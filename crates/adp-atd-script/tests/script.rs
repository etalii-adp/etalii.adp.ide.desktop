//! The script's semantics (etalii.adp spec 035, task T119, ATD §12, FR-013 to FR-016): actions go to the gesture's
//! transaction in order and land as one change; a budget overrun, a recursion past its bound, a loop at its limit and a
//! `fail` stop the call, discard the transaction and name the function; a `for` over a map goes in ascending key order;
//! `string(double)` is CEL's.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_atd_check::{Files, FsFiles, Severity, check};
use adp_atd_script::{Action, Control, Gesture, Host, Kind, Options, Performed, Script, Stop, Transaction};
use adp_cel::{Environment, Map, Value};
use adp_history::{Applied, Command, History, Refusal};
use serde_json::Value as Json;

// ------------------------------------------------------------------------------------------------ The test's model

/// Nodes under one root, in order: an id and a text each.
#[derive(Debug, Default, Clone, PartialEq)]
struct Model {
    nodes: Vec<(String, String)>,
}

impl Model {
    fn texts(&self) -> Vec<&str> {
        self.nodes.iter().map(|(_, t)| t.as_str()).collect()
    }
}

struct Insert {
    index: usize,
    id: String,
    text: String,
}

impl Command<Model> for Insert {
    fn label(&self) -> String {
        "Insert".into()
    }

    fn apply(&self, model: &mut Model) -> Result<Applied<Model>, Refusal> {
        model.nodes.insert(self.index, (self.id.clone(), self.text.clone()));
        Ok(Applied::undoable(Remove { id: self.id.clone() }))
    }
}

struct Remove {
    id: String,
}

impl Command<Model> for Remove {
    fn label(&self) -> String {
        "Remove".into()
    }

    fn apply(&self, model: &mut Model) -> Result<Applied<Model>, Refusal> {
        let index = model
            .nodes
            .iter()
            .position(|(id, _)| *id == self.id)
            .ok_or_else(|| Refusal::new("No such node."))?;
        let (id, text) = model.nodes.remove(index);
        Ok(Applied::undoable(Insert { index, id, text }))
    }
}

struct SetText {
    id: String,
    text: String,
}

impl Command<Model> for SetText {
    fn label(&self) -> String {
        "Set text".into()
    }

    fn apply(&self, model: &mut Model) -> Result<Applied<Model>, Refusal> {
        let node = model
            .nodes
            .iter_mut()
            .find(|(id, _)| *id == self.id)
            .ok_or_else(|| Refusal::new("No such node."))?;
        let old = std::mem::replace(&mut node.1, self.text.clone());
        Ok(Applied::undoable(SetText {
            id: self.id.clone(),
            text: old,
        }))
    }
}

/// A host over the model: `self` is the root, whose children are the nodes; `create`, `set` and `delete` are
/// commands of the transaction, and every other action is only logged.
struct TestHost {
    model: Model,
    transaction: Option<Transaction<Model>>,
    log: Vec<String>,
    next: usize,
}

impl TestHost {
    fn new(model: Model) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(TestHost {
            model,
            transaction: None,
            log: Vec::new(),
            next: 1,
        }))
    }

    fn node(id: &str, text: &str) -> Value {
        Value::map(Map::from_iter([
            ("id", Value::String(id.into())),
            ("text", Value::String(text.into())),
        ]))
    }

    fn apply(&mut self, command: &dyn Command<Model>) -> Result<(), String> {
        let transaction = self.transaction.as_mut().ok_or("No transaction is open.")?;
        transaction.apply(&mut self.model, command).map_err(|r| r.0)
    }
}

fn id_of(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_map)
        .and_then(|m| m.get("id"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

impl Host for TestHost {
    fn variable(&self, name: &str) -> Option<Value> {
        (name == "self").then(|| {
            let children = self.model.nodes.iter().map(|(id, text)| Self::node(id, text)).collect();
            Value::map(Map::from_iter([
                ("id", Value::String("root".into())),
                ("text", Value::String("Root".into())),
                ("children", Value::list(children)),
            ]))
        })
    }

    fn perform(&mut self, action: &Action<'_>) -> Result<Performed, String> {
        match action.kind {
            "create" => {
                let text = action
                    .values
                    .get("assignments")
                    .and_then(Value::as_map)
                    .and_then(|m| m.get("text"));
                let text = text.and_then(Value::as_str).unwrap_or_default().to_owned();
                let id = format!("n{}", self.next);
                self.next += 1;
                let index = match action.values.get("after") {
                    Some(after) => {
                        let after = id_of(Some(after));
                        self.model
                            .nodes
                            .iter()
                            .position(|(id, _)| *id == after)
                            .map_or(0, |i| i + 1)
                    }
                    None => self.model.nodes.len(),
                };
                self.log.push(format!("create {text}"));
                self.apply(&Insert {
                    index,
                    id: id.clone(),
                    text: text.clone(),
                })?;
                Ok(Performed::Done(Some(Self::node(&id, &text))))
            }
            "set" => {
                let id = id_of(action.values.get("target"));
                let text = action.values.get("value").map(|v| v.to_string()).unwrap_or_default();
                self.log.push(format!("set {}.{} = {text}", id, action.path.join(".")));
                self.apply(&SetText { id, text })?;
                Ok(Performed::Done(None))
            }
            "delete" => {
                let id = id_of(action.values.get("target"));
                self.log.push(format!("delete {id}"));
                self.apply(&Remove { id })?;
                Ok(Performed::Done(None))
            }
            other => {
                let target = action.values.get("target").map(|t| id_of(Some(t)));
                let value = action.values.get("value").map(Value::to_string);
                self.log
                    .push(format!("{other} {}", target.or(value).unwrap_or_default()));
                Ok(Performed::Done(None))
            }
        }
    }
}

// ------------------------------------------------------------------------------------------------ Descriptions

struct Memory(BTreeMap<PathBuf, String>);

impl Files for Memory {
    fn read(&self, path: &Path) -> Option<String> {
        self.0.get(path).cloned()
    }

    fn list(&self, folder: &Path) -> Vec<PathBuf> {
        self.0.keys().filter(|p| p.parent() == Some(folder)).cloned().collect()
    }
}

/// The description `members`, checked without errors, with its operations by name.
fn describe(members: &str, options: Options) -> (Script, BTreeMap<String, Json>) {
    describe_in(members, Environment::standard_library(), options)
}

fn describe_in(members: &str, environment: Environment, options: Options) -> (Script, BTreeMap<String, Json>) {
    let text = format!(
        "atd 0.1\ndiagram test/script version \"0.1.0\" {{\n  label \"Script\"\n  model {{\n    element Node {{ attribute text: string }}\n  }}\n{members}\n}}\n"
    );
    let path = PathBuf::from("/test/script.atd");
    let checked = check(&Memory(BTreeMap::from([(path.clone(), text)])), &path);
    let errors: Vec<String> = checked
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .map(|f| f.to_line())
        .collect();
    assert!(errors.is_empty(), "the description has errors:\n{}", errors.join("\n"));
    let symbols = checked.symbols.expect("the description has symbols");
    let script = Script::new(&symbols, environment, options);
    (script, symbols.operations)
}

/// Runs an operation as a host does: in a transaction that lands as one change, or is discarded when the call stops.
fn operate(
    script: &Script,
    operations: &BTreeMap<String, Json>,
    name: &str,
    host: &Rc<RefCell<TestHost>>,
    history: &mut History<Model>,
) -> Result<(), Stop> {
    host.borrow_mut().transaction = Some(Transaction::new(name));
    let gesture = script.gesture(host.clone());
    let result = gesture.run(name, &operations[name]["body"], Map::default());
    let mut host = host.borrow_mut();
    let transaction = host.transaction.take().expect("the transaction is open");
    match &result {
        Ok(()) => {
            transaction.commit(history);
        }
        Err(_) => transaction.discard(&mut host.model),
    }
    result
}

fn call(script: &Script, function: &str, arguments: &[Value]) -> Result<Value, Stop> {
    let gesture: Gesture = script.gesture(TestHost::new(Model::default()));
    gesture.call(function, arguments)
}

fn strings(items: &[&str]) -> Value {
    Value::list(items.iter().map(|s| Value::String((*s).into())).collect())
}

// ------------------------------------------------------------------------------------------------ Tests

#[test]
fn actions_go_to_the_transaction_in_order_and_land_as_one_change() {
    let (script, operations) = describe(
        r#"
  fn nextName(names: list<string>) -> string {
    return "Node " + string(names.size() + 1)
  }

  operation add "Add" for Node {
    create Node { text: nextName(self.children.map(c, c.text)) } in self as first
    create Node { text: nextName(self.children.map(c, c.text)) } in self after first as second
    set second.text = second.text + "!"
    select second
    notify "Added " + first.text
  }
"#,
        Options::default(),
    );
    let host = TestHost::new(Model::default());
    let mut history = History::default();
    operate(&script, &operations, "add", &host, &mut history).expect("the operation runs");

    let h = host.borrow();
    // The second `create` sees the first: each expression sees the actions before it.
    assert_eq!(
        h.log,
        [
            "create Node 1",
            "create Node 2",
            "set n2.text = Node 2!",
            "select n2",
            "notify Added Node 1"
        ]
    );
    assert_eq!(h.model.texts(), ["Node 1", "Node 2!"]);
    drop(h);

    // One change, undone exactly.
    assert_eq!(history.availability().undo_count, 1);
    history.undo(&mut host.borrow_mut().model);
    assert_eq!(host.borrow().model, Model::default());
    history.redo(&mut host.borrow_mut().model);
    assert_eq!(host.borrow().model.texts(), ["Node 1", "Node 2!"]);
}

#[test]
fn a_budget_overrun_stops_the_call_and_discards_the_transaction() {
    let (script, operations) = describe(
        r#"
  fn spin(n: int) -> int {
    var i = 0
    while i < n limit 1000000 {
      i = i + 1
    }
    return i
  }

  operation add "Add" for Node {
    create Node { text: "kept?" } in self
    set text = string(spin(100000))
  }
"#,
        Options { steps: 1000 },
    );
    assert_eq!(call(&script, "spin", &[Value::Int(10)]).unwrap().as_int(), Some(10));

    let host = TestHost::new(Model::default());
    let mut history = History::default();
    let stop = operate(&script, &operations, "add", &host, &mut history).unwrap_err();
    assert_eq!(stop.kind, Kind::Budget);
    assert_eq!(stop.code(), Some("ATD7102"));
    assert_eq!(stop.function, "spin");
    assert!(stop.message.contains("`spin`"), "{}", stop.message);
    let finding = stop.finding().expect("a budget overrun is a finding");
    assert_eq!(finding.code, "ATD7102");
    assert_eq!(finding.file, "/test/script.atd");
    assert!(finding.line > 0);

    assert_eq!(host.borrow().model, Model::default(), "the transaction is discarded");
    assert_eq!(history.availability().undo_count, 0);
}

#[test]
fn a_value_past_its_length_or_an_expression_past_its_budget_is_a_budget_overrun() {
    let members = r#"
  fn big() -> int {
    let xs = range(0, 600000)
    let ys = xs + xs
    return ys.size()
  }
"#;
    // An expression runs within CEL's budget of one evaluation.
    let (script, _) = describe(members, Options::default());
    let stop = call(&script, "big", &[]).unwrap_err();
    assert_eq!(
        (stop.kind, stop.code(), stop.function.as_str()),
        (Kind::Budget, Some("ATD7102"), "big")
    );

    // Within a larger one, the list of 1,200,000 ints is past the length a value may have.
    let mut environment = Environment::standard_library();
    environment.budget = 10_000_000;
    let (script, _) = describe_in(members, environment, Options::default());
    let stop = call(&script, "big", &[]).unwrap_err();
    assert_eq!((stop.kind, stop.function.as_str()), (Kind::Budget, "big"), "{stop:?}");
    assert!(stop.message.contains("1000000 elements"), "{}", stop.message);
}

#[test]
fn a_recursion_past_its_bound_stops_the_call() {
    let (script, _) = describe(
        r#"
  fn down(n: int) -> int recursion 8 {
    if n == 0 {
      return 0
    }
    return down(n - 1)
  }

  fn deep(n: int) -> int {
    if n == 0 {
      return 0
    }
    return deep(n - 1)
  }
"#,
        Options::default(),
    );
    assert_eq!(call(&script, "down", &[Value::Int(7)]).unwrap().as_int(), Some(0));
    let stop = call(&script, "down", &[Value::Int(8)]).unwrap_err();
    assert_eq!(stop.kind, Kind::Recursion);
    assert_eq!(stop.code(), Some("ATD7103"));
    assert_eq!(stop.function, "down");
    assert!(stop.message.contains("bound of 8"), "{}", stop.message);

    assert_eq!(call(&script, "deep", &[Value::Int(63)]).unwrap().as_int(), Some(0));
    let stop = call(&script, "deep", &[Value::Int(64)]).unwrap_err();
    assert_eq!((stop.kind, stop.function.as_str()), (Kind::Recursion, "deep"));
}

#[test]
fn a_loop_at_its_limit_has_its_own_finding() {
    let (script, _) = describe(
        r#"
  fn total(xs: list<int>) -> int {
    var sum = 0
    for x in xs limit 3 {
      sum = sum + x
    }
    return sum
  }

  fn halve(n: int) -> int {
    var m = n
    while m > 1 limit 4 {
      m = m / 2
    }
    return m
  }
"#,
        Options::default(),
    );
    let ints = |xs: &[i64]| Value::list(xs.iter().copied().map(Value::Int).collect());
    assert_eq!(call(&script, "total", &[ints(&[1, 2, 3])]).unwrap().as_int(), Some(6));
    let stop = call(&script, "total", &[ints(&[1, 2, 3, 4])]).unwrap_err();
    assert_eq!((stop.kind, stop.code()), (Kind::Limit, Some("ATD7101")));
    assert_eq!(stop.function, "total");
    assert!(stop.message.contains("limit of 3"), "{}", stop.message);

    assert_eq!(call(&script, "halve", &[Value::Int(16)]).unwrap().as_int(), Some(1));
    let stop = call(&script, "halve", &[Value::Int(32)]).unwrap_err();
    assert_eq!((stop.kind, stop.function.as_str()), (Kind::Limit, "halve"));
}

#[test]
fn a_for_over_a_map_visits_its_keys_in_ascending_order() {
    let (script, _) = describe(
        r#"
  fn keys(m: map<string, int>) -> list<string> {
    var out = []
    for k in m limit 10 {
      out = out + [k]
    }
    return out
  }

  fn pairs(m: map<string, int>) -> list<string> {
    var out = []
    for k, v in m limit 10 {
      out = out + [k + "=" + string(v)]
    }
    return out
  }
"#,
        Options::default(),
    );
    let map = Value::map(Map::from_iter([
        ("b", Value::Int(2)),
        ("é", Value::Int(3)),
        ("a", Value::Int(1)),
        ("Z", Value::Int(0)),
    ]));
    let texts = |v: Value| -> Vec<String> { v.as_list().unwrap().iter().map(|i| i.to_string()).collect() };
    assert_eq!(
        texts(call(&script, "keys", std::slice::from_ref(&map)).unwrap()),
        ["Z", "a", "b", "é"]
    );
    assert_eq!(
        texts(call(&script, "pairs", &[map]).unwrap()),
        ["Z=0", "a=1", "b=2", "é=3"]
    );
}

#[test]
fn string_of_a_double_is_cels() {
    let (script, _) = describe(
        r#"
  fn show(d: double) -> string {
    return string(d)
  }
"#,
        Options::default(),
    );
    for (value, text) in [
        (1.5, "1.5"),
        (2.0, "2"),
        (-0.25, "-0.25"),
        (100000.0, "100000"),
        (1e6, "1e+06"),
        (0.0001, "0.0001"),
        (0.00001, "1e-05"),
        (1.0 / 3.0, "0.3333333333333333"),
    ] {
        assert_eq!(
            call(&script, "show", &[Value::Double(value)]).unwrap().to_string(),
            text
        );
    }
}

#[test]
fn a_fail_refuses_with_its_message() {
    let (script, operations) = describe(
        r#"
  fn checked(n: int) -> int {
    if n > 2 {
      fail "At most two, not " + string(n) + "."
    }
    return n
  }

  operation add "Add" for Node {
    create Node { text: "first" } in self
    fail "No room."
  }

  operation addMany "Add many" for Node {
    create Node { text: string(checked(self.children.size() + 3)) } in self
  }
"#,
        Options::default(),
    );
    let host = TestHost::new(Model::default());
    let mut history = History::default();

    let stop = operate(&script, &operations, "add", &host, &mut history).unwrap_err();
    assert_eq!(
        (stop.kind, stop.message.as_str(), stop.function.as_str()),
        (Kind::Fail, "No room.", "add")
    );
    assert_eq!(stop.refusal().0, "No room.");
    assert_eq!(stop.code(), None);
    assert_eq!(host.borrow().model, Model::default());

    // A `fail` in a function an expression calls stops the operation with the function's message and name.
    let stop = operate(&script, &operations, "addMany", &host, &mut history).unwrap_err();
    assert_eq!(
        (stop.kind, stop.message.as_str(), stop.function.as_str()),
        (Kind::Fail, "At most two, not 3.", "checked")
    );
    assert_eq!(history.availability().undo_count, 0);
}

#[test]
fn the_debugger_sees_each_statement_and_can_stop_the_call() {
    let (script, _) = describe(
        r#"
  fn sum(n: int) -> int {
    var total = 0
    for i in range(n) limit 100 {
      total = total + i
    }
    return total
  }
"#,
        Options::default(),
    );
    let gesture = script.gesture(TestHost::new(Model::default()));
    let seen = Rc::new(RefCell::new(Vec::new()));
    let log = Rc::clone(&seen);
    gesture.on_statement(move |event| {
        let total = event
            .locals
            .iter()
            .find(|(n, _)| *n == "total")
            .map(|(_, v)| v.to_string());
        log.borrow_mut()
            .push((event.statement["kind"].as_str().unwrap_or_default().to_owned(), total));
        if log.borrow().len() == 5 {
            Control::Stop
        } else {
            Control::Continue
        }
    });
    let stop = gesture.call("sum", &[Value::Int(3)]).unwrap_err();
    assert_eq!((stop.kind, stop.function.as_str()), (Kind::Debugger, "sum"));
    let seen = seen.borrow();
    let kinds: Vec<&str> = seen.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(kinds, ["var", "for", "assign", "assign", "assign"]);
    assert_eq!(seen[4].1.as_deref(), Some("1"), "the locals are the debugger's to read");
}

#[test]
fn the_steps_are_counted_per_statement_iteration_expression_and_call() {
    let (script, _) = describe(
        r#"
  fn one() -> int {
    return 1
  }

  fn two() -> int {
    let a = one()
    return a + one()
  }
"#,
        Options::default(),
    );
    let gesture = script.gesture(TestHost::new(Model::default()));
    assert_eq!(gesture.call("one", &[]).unwrap().as_int(), Some(1));
    // A call, a statement, an expression.
    assert_eq!(gesture.steps(), 3);
    assert_eq!(gesture.call("two", &[]).unwrap().as_int(), Some(2));
    // The call of `two`, two statements and two expressions, and three steps for each call of `one`.
    assert_eq!(gesture.steps(), 3 + 5 + 6);
}

#[test]
fn the_mind_maps_sibling_names() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/definitions/diagrams/mindmap.atd");
    let checked = check(&FsFiles, &path);
    let errors: Vec<String> = checked
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .map(|f| f.to_line())
        .collect();
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    let script = Script::new(
        &checked.symbols.expect("symbols"),
        Environment::standard_library(),
        Options::default(),
    );
    for (names, expected) in [
        (&[][..], "Node"),
        (&["Node"][..], "Node 2"),
        (&["node", "Node 2"][..], "Node 3"),
        (&["Phase 1", "Phase 3"][..], "Phase 4"),
        (&["Phase 1", "Step 7", "phase 2"][..], "phase 3"),
        (&["Measure pass", "Arrange pass", "Idea"][..], "New pass"),
        (&["  ", "Idea", "Plan"][..], "Node"),
    ] {
        let name = call(&script, "siblingName", &[strings(names)]).unwrap_or_else(|s| panic!("{names:?}: {s}"));
        assert_eq!(name.to_string(), expected, "{names:?}");
    }
}
