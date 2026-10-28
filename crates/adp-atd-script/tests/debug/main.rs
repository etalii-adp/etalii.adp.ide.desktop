//! The debugger (etalii.adp spec 035, task T139, FR-070 to FR-072, contracts/debugger.md): three seeded defects in the
//! mind map's and the causal loop's script functions, each found by where the debugger stops and what it shows there;
//! a conditional breakpoint stops only where its condition holds, and one that does not evaluate is reported once and
//! treated as false; stepping over, into and out; and a stop discards the transaction, also when paused in an
//! operation run inside another operation's transaction.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adp_atd_check::{Files, FsFiles, Severity, Symbols, check};
use adp_atd_script::debug::{
    Breakpoint, DebugControl, DebugEvent, Frame, Inspect, PauseReason, Scope, ScopeKind, Session, StopReason, Variable,
    statement_lines, verify,
};
use adp_atd_script::{Action, Host, Options, Performed, Script, Transaction};
use adp_cel::{Environment, Map, Value};
use adp_history::{Applied, Command, History, Refusal};

const WAIT: Duration = Duration::from_secs(20);

// ------------------------------------------------------------------------------------------------ Descriptions

fn vendor(file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/etalii.adp/definitions/diagrams")
        .join(file)
}

/// The files on disk, but for one whose text is seeded with a defect.
struct Seeded {
    path: PathBuf,
    text: String,
}

impl Files for Seeded {
    fn read(&self, path: &Path) -> Option<String> {
        if path == self.path {
            return Some(self.text.clone());
        }
        FsFiles.read(path)
    }

    fn list(&self, folder: &Path) -> Vec<PathBuf> {
        FsFiles.list(folder)
    }
}

/// A vendored description with `correct` replaced by `defect`, and the line `at` is on in the seeded text.
struct Defect {
    path: PathBuf,
    text: String,
    line: usize,
}

fn seed(file: &str, correct: &str, defect: &str, at: &str) -> Defect {
    let path = vendor(file);
    let original = std::fs::read_to_string(&path).unwrap();
    assert_eq!(original.matches(correct).count(), 1, "`{correct}` is in {file} once");
    let text = original.replacen(correct, defect, 1);
    let line = line_of(&text, at);
    Defect { path, text, line }
}

/// The line `needle` is on, which it is on alone.
fn line_of(text: &str, needle: &str) -> usize {
    let lines: Vec<usize> = text
        .lines()
        .enumerate()
        .filter(|(_, l)| l.contains(needle))
        .map(|(i, _)| i + 1)
        .collect();
    assert_eq!(lines.len(), 1, "`{needle}` is on one line");
    lines[0]
}

fn checked(files: &dyn Files, path: &Path) -> Symbols {
    let checked = check(files, path);
    let errors: Vec<String> = checked
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .map(|f| f.to_line())
        .collect();
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    checked.symbols.expect("symbols")
}

/// A breakpoint at `line`, verified against the statements of the function or operation `name`.
fn breakpoint(symbols: &Symbols, name: &str, line: usize, condition: Option<&str>) -> Breakpoint {
    let tree = symbols
        .functions
        .get(name)
        .or_else(|| symbols.operations.get(name))
        .expect("the function or operation");
    let file = statement_lines(tree)
        .into_iter()
        .map(|(file, _)| file)
        .next()
        .expect("statements");
    let mut breakpoint = Breakpoint::new(file, line);
    if let Some(condition) = condition {
        breakpoint = breakpoint.when(condition);
    }
    let mut breakpoints = [breakpoint];
    verify(&mut breakpoints, &statement_lines(tree));
    let [breakpoint] = breakpoints;
    assert!(breakpoint.verified, "line {line} of `{name}` has a statement");
    breakpoint
}

const TEST: &str = "/test/debug.atd";

struct Memory(BTreeMap<PathBuf, String>);

impl Files for Memory {
    fn read(&self, path: &Path) -> Option<String> {
        self.0.get(path).cloned()
    }

    fn list(&self, folder: &Path) -> Vec<PathBuf> {
        self.0.keys().filter(|p| p.parent() == Some(folder)).cloned().collect()
    }
}

fn described(members: &str) -> String {
    format!(
        "atd 0.1\ndiagram test/debug version \"0.1.0\" {{\n  label \"Debug\"\n  model {{\n    element Node {{ attribute text: string }}\n  }}\n{members}\n}}\n"
    )
}

fn memory(text: &str) -> Memory {
    Memory(BTreeMap::from([(PathBuf::from(TEST), text.to_owned())]))
}

fn strings(items: &[&str]) -> Value {
    Value::list(items.iter().map(|s| Value::String((*s).into())).collect())
}

// ------------------------------------------------------------------------------------------------ The test's host

/// Nodes by their text, and the transaction's commands, shared with what the debugger inspects.
#[derive(Default)]
struct Shared {
    nodes: Vec<String>,
    commands: Vec<String>,
}

struct Append(String);

impl Command<Vec<String>> for Append {
    fn label(&self) -> String {
        format!("Create {}", self.0)
    }

    fn apply(&self, model: &mut Vec<String>) -> Result<Applied<Vec<String>>, Refusal> {
        model.push(self.0.clone());
        Ok(Applied::undoable(Pop))
    }
}

struct Pop;

impl Command<Vec<String>> for Pop {
    fn label(&self) -> String {
        "Remove".into()
    }

    fn apply(&self, model: &mut Vec<String>) -> Result<Applied<Vec<String>>, Refusal> {
        let text = model.pop().ok_or_else(|| Refusal::new("Nothing to remove."))?;
        Ok(Applied::undoable(Append(text)))
    }
}

struct TestHost {
    shared: Rc<RefCell<Shared>>,
    transaction: Option<Transaction<Vec<String>>>,
    operations: BTreeMap<String, serde_json::Value>,
}

impl Host for TestHost {
    fn variable(&self, name: &str) -> Option<Value> {
        (name == "self").then(|| Value::map(Map::from_iter([("text", Value::String("Root".into()))])))
    }

    fn perform(&mut self, action: &Action<'_>) -> Result<Performed, String> {
        match action.kind {
            "create" => {
                let text = action
                    .values
                    .get("assignments")
                    .and_then(Value::as_map)
                    .and_then(|m| m.get("text"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned();
                let command = Append(text);
                let mut shared = self.shared.borrow_mut();
                shared.commands.push(command.label());
                let transaction = self.transaction.as_mut().ok_or("No transaction is open.")?;
                transaction.apply(&mut shared.nodes, &command).map_err(|r| r.0)?;
                Ok(Performed::Done(None))
            }
            "call" => {
                let name = action.node["operation"]["text"].as_str().unwrap_or_default().to_owned();
                let body = self.operations[&name]["body"].clone();
                Ok(Performed::Run {
                    name,
                    body,
                    bindings: Map::default(),
                })
            }
            _ => Ok(Performed::Done(None)),
        }
    }
}

struct Inspector(Rc<RefCell<Shared>>);

impl Inspect for Inspector {
    fn elements(&self) -> Vec<Variable> {
        let shared = self.0.borrow();
        shared
            .nodes
            .iter()
            .enumerate()
            .map(|(i, text)| Variable {
                name: format!("n{}", i + 1),
                value: "Node".into(),
                type_name: "Node".into(),
                children: vec![Variable::text("text", text.clone())],
            })
            .collect()
    }

    fn transaction(&self) -> Vec<Variable> {
        let shared = self.0.borrow();
        shared
            .commands
            .iter()
            .enumerate()
            .map(|(i, label)| Variable::text(format!("{}", i + 1), label.clone()))
            .collect()
    }
}

// ------------------------------------------------------------------------------------------------ Driving a session

/// The events up to the next pause or the end.
fn until_pause(session: &Session) -> Vec<DebugEvent> {
    let mut events = Vec::new();
    loop {
        let event = session.next_event(WAIT).expect("an event");
        let last = matches!(
            event,
            DebugEvent::Paused { .. } | DebugEvent::Finished { .. } | DebugEvent::Stopped { .. }
        );
        events.push(event);
        if last {
            return events;
        }
    }
}

/// The pause the events end in: its reason, frames and scopes.
fn paused(events: &[DebugEvent]) -> (PauseReason, Vec<Frame>, Vec<Scope>) {
    match events.last() {
        Some(DebugEvent::Paused { reason, frames, scopes }) => (*reason, frames.clone(), scopes.clone()),
        _ => panic!("not paused: {events:?}"),
    }
}

/// A scope's variables as name and value.
fn shown(scopes: &[Scope], frame: usize, kind: ScopeKind) -> Vec<(String, String)> {
    scopes
        .iter()
        .find(|s| s.frame == frame && s.kind == kind)
        .map(|s| s.variables.iter().map(|v| (v.name.clone(), v.value.clone())).collect())
        .unwrap_or_default()
}

fn value(scopes: &[Scope], name: &str) -> String {
    shown(scopes, 0, ScopeKind::Locals)
        .into_iter()
        .chain(shown(scopes, 0, ScopeKind::Arguments))
        .find(|(n, _)| n == name)
        .map(|(_, v)| v)
        .unwrap_or_else(|| panic!("`{name}` is not shown"))
}

fn names(frames: &[Frame]) -> Vec<&str> {
    frames.iter().map(|f| f.name.as_str()).collect()
}

/// A function of a vendored description, seeded, called under the debugger with a breakpoint at the defect's line.
fn debug_call(
    defect: Defect,
    function: &'static str,
    arguments: impl FnOnce() -> Vec<Value> + Send + 'static,
    condition: Option<&'static str>,
) -> (Session, Arc<Mutex<Option<String>>>) {
    let result = Arc::new(Mutex::new(None));
    let returned = Arc::clone(&result);
    let session = Session::start(Vec::new(), false, move |debugger| {
        let files = Seeded {
            path: defect.path.clone(),
            text: defect.text.clone(),
        };
        let symbols = checked(&files, &defect.path);
        let debugger = debugger.with_breakpoints(vec![breakpoint(&symbols, function, defect.line, condition)]);
        let script = Script::new(&symbols, Environment::standard_library(), Options::default());
        let shared = Rc::new(RefCell::new(Shared::default()));
        let mut host = TestHost {
            shared: Rc::clone(&shared),
            transaction: None,
            operations: BTreeMap::new(),
        };
        let gesture = script.gesture(&mut host);
        debugger.attach(&gesture, Inspector(shared));
        let value = gesture.call(function, &arguments())?;
        *returned.lock().unwrap() = Some(value.to_string());
        Ok(())
    });
    (session, result)
}

// ------------------------------------------------------------------------------------------------ The seeded defects

#[test]
fn a_wrong_comparison_in_the_mind_maps_sibling_naming_shows_at_its_line() {
    // `>` became `<`: no number is ever higher than -1, so the holder stays empty.
    let defect = seed(
        "mindmap.atd",
        "if numberOf(n) > high {",
        "if numberOf(n) < high {",
        "if numberOf(n) < high {",
    );
    let (session, result) = debug_call(
        defect,
        "siblingName",
        || vec![strings(&["Phase 1", "Phase 3"])],
        Some("n == 'Phase 3'"),
    );
    let (reason, frames, scopes) = paused(&until_pause(&session));
    assert_eq!(reason, PauseReason::Breakpoint);
    assert_eq!(names(&frames), ["siblingName"]);
    assert_eq!(value(&scopes, "n"), "Phase 3");
    // The cause: after "Phase 1", the highest is still -1 and nobody holds it.
    assert_eq!(value(&scopes, "high"), "-1");
    assert_eq!(value(&scopes, "holder"), "");
    assert_eq!(
        shown(&scopes, 0, ScopeKind::Arguments),
        [("names".to_owned(), "list of 2".to_owned())]
    );
    assert!(
        shown(&scopes, 0, ScopeKind::Transaction).is_empty(),
        "a function changes nothing"
    );
    let budget = shown(&scopes, 0, ScopeKind::Budget);
    assert_eq!(budget[0].0, "steps");
    assert!(budget[0].1.ends_with(" of 100000"), "{budget:?}");
    assert_eq!(budget[1], ("depth".to_owned(), "1 of 64".to_owned()));

    session.control(DebugControl::Continue);
    let rest = until_pause(&session);
    assert_eq!(rest.first(), Some(&DebugEvent::Continued));
    assert_eq!(rest.last(), Some(&DebugEvent::Finished { committed: true }));
    // The condition held once: the one stop. The name the defect gives is wrong.
    assert_eq!(result.lock().unwrap().as_deref(), Some(" 0"));
}

#[test]
fn an_off_by_one_limit_in_the_causal_loops_cycle_enumeration_stops_where_the_bound_is_passed() {
    // `>=` became `>`: one cycle more than the bound is kept.
    let defect = seed(
        "causal-loop-diagram.atd",
        "if cycles.size() >= bound {",
        "if cycles.size() > bound {",
        "cycles = cycles + [path]",
    );
    let (session, result) = debug_call(
        defect,
        "causalCycles",
        || {
            let element = |pairs: &[(&str, &str)]| {
                Value::map(Map::from_iter(
                    pairs.iter().map(|(k, v)| (*k, Value::String((*v).into()))),
                ))
            };
            let variables = Value::list(["a", "b", "c"].iter().map(|n| element(&[("name", n)])).collect());
            let links = Value::list(
                [("a", "b"), ("b", "a"), ("b", "c"), ("c", "b")]
                    .iter()
                    .map(|(c, e)| element(&[("causeName", c), ("effectName", e)]))
                    .collect(),
            );
            vec![variables, links, Value::Int(1)]
        },
        Some("cycles.size() >= bound"),
    );
    let (reason, frames, scopes) = paused(&until_pause(&session));
    assert_eq!(reason, PauseReason::Breakpoint);
    assert_eq!(names(&frames), ["causalCycles"]);
    // The cause: a cycle is kept although the bound of one is already reached.
    assert_eq!(value(&scopes, "bound"), "1");
    assert_eq!(value(&scopes, "cycles"), "list of 1");
    assert_eq!(value(&scopes, "path"), "list of 2");
    session.control(DebugControl::Continue);
    let rest = until_pause(&session);
    assert_eq!(rest.last(), Some(&DebugEvent::Finished { committed: true }));
    assert!(result.lock().unwrap().is_some());
}

#[test]
fn a_missing_else_in_the_mind_maps_sibling_naming_runs_the_branch_it_should_skip() {
    // `} else {` lost its `else`: the worded names are counted after the numbered ones too.
    let defect = seed(
        "mindmap.atd",
        "      name = bestName\n    } else {",
        "      name = bestName\n    }\n    if true {",
        "name = \"New \" + word",
    );
    let (session, result) = debug_call(
        defect,
        "siblingName",
        || vec![strings(&["Phase 1", "Measure pass", "Arrange pass"])],
        None,
    );
    let (reason, frames, scopes) = paused(&until_pause(&session));
    assert_eq!(reason, PauseReason::Breakpoint);
    assert_eq!(names(&frames), ["siblingName"]);
    // The cause: the numbered name is already chosen when the worded branch overwrites it.
    assert_eq!(value(&scopes, "name"), "Phase 2");
    assert_eq!(value(&scopes, "numbered"), "list of 1");
    assert_eq!(value(&scopes, "word"), "pass");
    session.control(DebugControl::Continue);
    let rest = until_pause(&session);
    assert_eq!(rest.last(), Some(&DebugEvent::Finished { committed: true }));
    assert_eq!(result.lock().unwrap().as_deref(), Some("New pass"));
}

// ------------------------------------------------------------------------------------------------ Breakpoints and steps

/// A description of `members`, its operation `operation` run under the debugger in a transaction, with breakpoints at
/// the lines of `at` in the operations named beside them; what the model is after the gesture ends.
fn debug_operation(
    members: &'static str,
    operation: &'static str,
    at: Vec<(&'static str, &'static str, Option<&'static str>)>,
    stop_on_entry: bool,
) -> (Session, Arc<Mutex<Option<Vec<String>>>>) {
    let after = Arc::new(Mutex::new(None));
    let model = Arc::clone(&after);
    let session = Session::start(Vec::new(), stop_on_entry, move |debugger| {
        let text = described(members);
        let symbols = checked(&memory(&text), Path::new(TEST));
        let breakpoints = at
            .iter()
            .map(|(name, needle, condition)| breakpoint(&symbols, name, line_of(&text, needle), *condition))
            .collect();
        let debugger = debugger.with_breakpoints(breakpoints);
        let script = Script::new(&symbols, Environment::standard_library(), Options::default());
        let shared = Rc::new(RefCell::new(Shared::default()));
        let mut host = TestHost {
            shared: Rc::clone(&shared),
            transaction: Some(Transaction::new(operation)),
            operations: symbols.operations.clone(),
        };
        let mut history = History::default();
        let result = {
            let gesture = script.gesture(&mut host);
            debugger.attach(&gesture, Inspector(Rc::clone(&shared)));
            gesture.run(operation, &symbols.operations[operation]["body"], Map::default())
        };
        let transaction = host.transaction.take().expect("the transaction is open");
        match &result {
            Ok(()) => {
                transaction.commit(&mut history);
            }
            Err(_) => transaction.discard(&mut shared.borrow_mut().nodes),
        }
        *model.lock().unwrap() = Some(shared.borrow().nodes.clone());
        result
    });
    (session, after)
}

const NESTED: &str = r#"
  operation add "Add" for Node {
    create Node { text: "a" } in self
    call mark()
    create Node { text: "c" } in self
  }

  operation mark "Mark" for Node {
    create Node { text: "b" } in self
    notify "marked"
  }
"#;

#[test]
fn a_stop_while_paused_in_an_operation_run_by_another_discards_the_whole_transaction() {
    let (session, after) = debug_operation(NESTED, "add", vec![("mark", "notify \"marked\"", None)], false);
    let (reason, frames, scopes) = paused(&until_pause(&session));
    assert_eq!(reason, PauseReason::Breakpoint);
    assert_eq!(names(&frames), ["mark", "add"]);
    assert_eq!(frames[1].line, line_of(&described(NESTED), "call mark()"));
    assert_eq!(
        shown(&scopes, 0, ScopeKind::Transaction),
        [
            ("1".to_owned(), "Create a".to_owned()),
            ("2".to_owned(), "Create b".to_owned())
        ]
    );
    assert_eq!(shown(&scopes, 0, ScopeKind::Elements).len(), 2);
    session.control(DebugControl::Stop);
    let rest = until_pause(&session);
    assert_eq!(
        rest.last(),
        Some(&DebugEvent::Stopped {
            reason: StopReason::User
        })
    );
    assert_eq!(
        after.lock().unwrap().as_deref(),
        Some(&[][..]),
        "nothing is left of the gesture"
    );
}

#[test]
fn stepping_goes_over_into_and_out_of_a_called_operation() {
    let (session, after) = debug_operation(NESTED, "add", Vec::new(), true);
    let text = described(NESTED);
    let at = |events: &[DebugEvent]| {
        let (_, frames, _) = paused(events);
        (frames[0].name.clone(), frames[0].line)
    };
    let (reason, _, _) = paused(&until_pause(&session));
    assert_eq!(reason, PauseReason::Entry);
    session.control(DebugControl::StepOver);
    assert_eq!(
        at(&until_pause(&session)),
        ("add".to_owned(), line_of(&text, "call mark()"))
    );
    session.control(DebugControl::StepInto);
    assert_eq!(
        at(&until_pause(&session)),
        ("mark".to_owned(), line_of(&text, "text: \"b\""))
    );
    session.control(DebugControl::StepOut);
    assert_eq!(
        at(&until_pause(&session)),
        ("add".to_owned(), line_of(&text, "text: \"c\""))
    );
    session.control(DebugControl::Continue);
    let rest = until_pause(&session);
    assert_eq!(rest.last(), Some(&DebugEvent::Finished { committed: true }));
    assert_eq!(
        after.lock().unwrap().clone(),
        Some(vec!["a".to_owned(), "b".to_owned(), "c".to_owned()])
    );
}

const COUNTING: &str = r#"
  fn total(n: int) -> int {
    var sum = 0
    for i in range(n) limit 100 {
      sum = sum + i
    }
    return sum
  }

  operation count "Count" for Node {
    let t = total(5)
    notify string(t)
  }
"#;

#[test]
fn a_conditional_breakpoint_stops_only_where_its_condition_holds() {
    let (session, _) = debug_operation(
        COUNTING,
        "count",
        vec![("total", "sum = sum + i", Some("i % 2 == 1"))],
        false,
    );
    let mut seen = Vec::new();
    loop {
        let events = until_pause(&session);
        match events.last() {
            Some(DebugEvent::Paused { scopes, frames, .. }) => {
                assert_eq!(names(frames), ["total", "count"]);
                seen.push(value(scopes, "i"));
                session.control(DebugControl::Continue);
            }
            Some(DebugEvent::Finished { committed }) => {
                assert!(committed);
                break;
            }
            other => panic!("{other:?}"),
        }
    }
    assert_eq!(seen, ["1", "3"]);
}

#[test]
fn a_condition_that_does_not_evaluate_is_reported_once_and_does_not_hold() {
    let (session, _) = debug_operation(
        COUNTING,
        "count",
        vec![("total", "sum = sum + i", Some("i > 'one'"))],
        false,
    );
    let events = until_pause(&session);
    let failed: Vec<&DebugEvent> = events
        .iter()
        .filter(|e| matches!(e, DebugEvent::ConditionFailed { .. }))
        .collect();
    assert_eq!(failed.len(), 1, "{events:?}");
    assert_eq!(events.last(), Some(&DebugEvent::Finished { committed: true }));
}

#[test]
fn a_breakpoint_on_a_line_without_a_statement_is_not_verified() {
    let text = described(COUNTING);
    let symbols = checked(&memory(&text), Path::new(TEST));
    let lines = statement_lines(&symbols.functions["total"]);
    let file = lines.iter().next().unwrap().0.clone();
    let mut breakpoints = vec![
        Breakpoint::new(file.clone(), line_of(&text, "fn total")),
        Breakpoint::new(file, line_of(&text, "return sum")),
    ];
    verify(&mut breakpoints, &lines);
    assert_eq!(
        breakpoints.iter().map(|b| b.verified).collect::<Vec<_>>(),
        [false, true]
    );
}

#[test]
fn a_dropped_session_stops_its_gesture() {
    let (session, after) = debug_operation(NESTED, "add", vec![("mark", "notify \"marked\"", None)], false);
    paused(&until_pause(&session));
    drop(session);
    assert_eq!(after.lock().unwrap().as_deref(), Some(&[][..]));
}
