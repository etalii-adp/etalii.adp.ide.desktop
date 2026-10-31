//! Validation of a tool description against its conformance fixtures (etalii.adp spec 035, T127, FR-041; ATD §2.3).
//!
//! A case is replayed through the description's runtime on working copies held in memory: its inputs opened from
//! their bytes, each step's gesture made with the ids the reference host generated, in that order. After the reading
//! and after each step, what the runtime gives is compared with what the case states: the bytes, byte for byte, the
//! splices, the reading, the findings, the refusal and the drawing (as ATD §2.4 defines its equality). Each
//! difference is reported with its step, its part, the expected and the actual value, for bytes the first line that
//! differs, and the construct of the description that produced the value (`attribution`). A case recorded from a module
//! that opens its own documents states its reading and its drawing in that module's terms, and they are compared by what
//! they mean (`meaning`). Nothing is read from or written to the file system.

pub mod attribution;
pub mod cases;
pub mod meaning;
pub mod parts;

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_atd_check::Description;
use adp_atd_runtime::RuntimeTool;
use adp_context::Prompt;
use adp_diagram::ElementKind;
use adp_diagram::document::EditedFile;
use adp_diagram::session::{ActionOutcome, ActionTarget, DiagramSession};
use adp_diagram::tool::DiagramTool;
use adp_diagram::workspace::DiagramWorkspace;
use adp_disl::{HostAction, IdSource};
use adp_history::Outcome;
pub use attribution::{At, Constructs};
pub use cases::{CaseResult, case_of, cases_of, read_case, validate_description};
use serde_json::{Value as Json, json};

/// A case: its fixture (`conformance.json`) and its files by their names in the fixture, all in memory.
#[derive(Debug, Clone)]
pub struct Case {
    pub name: String,
    pub fixture: Json,
    pub files: BTreeMap<String, Vec<u8>>,
}

/// One difference between what the runtime gives and what the case states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Difference {
    /// The step, from 1; none for the reading.
    pub step: Option<usize>,
    /// `bytes`, `splices`, `reading`, `findings`, `refused` or `drawing`.
    pub part: &'static str,
    pub expected: String,
    pub actual: String,
    /// The construct of the description that produced the value, when one is known.
    pub at: Option<At>,
}

impl Difference {
    /// The difference without its construct: its step, its part, what was expected and what was given.
    pub fn what(&self) -> String {
        let place = match self.step {
            Some(step) => format!("step {step}, {}", self.part),
            None => format!("read, {}", self.part),
        };
        format!("{place}: expected {}, got {}", self.expected, self.actual)
    }
}

impl std::fmt::Display for Difference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.what())?;
        match &self.at {
            Some(at) => write!(f, " ({at})"),
            None => Ok(()),
        }
    }
}

/// Where the case's files are named: under a root that is never read.
const ROOT: &str = "/case";

/// The ids the reference host generated in a step, handed out in order.
#[derive(Default)]
struct StepIds {
    ids: RefCell<VecDeque<String>>,
}

impl IdSource for StepIds {
    fn next(&self, _type_name: &str) -> Result<String, String> {
        self.ids
            .borrow_mut()
            .pop_front()
            .ok_or_else(|| "The case gives no further id for this step.".to_owned())
    }
}

/// Replays `case` through `runtime`, the tool of `description`, and every difference from what it states.
pub fn validate(runtime: &RuntimeTool, description: &Description, case: &Case) -> Vec<Difference> {
    let constructs = Constructs::of(description, &runtime.loaded.json);
    let mut differences = Vec::new();
    let inputs = &case.fixture["inputs"];
    let Some(body_name) = inputs["body"].as_str() else {
        differences.push(Difference {
            step: None,
            part: "reading",
            expected: "a body among the inputs".into(),
            actual: "none".into(),
            at: None,
        });
        return differences;
    };
    let body_path = Path::new(ROOT).join(body_name);
    let registration_name = inputs["registration"].as_str();
    let registration_path = registration_name.map(|r| Path::new(ROOT).join(r));
    let mut workspace = DiagramWorkspace::in_memory(ROOT);
    let session = workspace.open_bytes_beside(
        runtime.tool.clone(),
        &body_path,
        registration_path.as_deref(),
        case.files.get(body_name).cloned().unwrap_or_default(),
        registration_name.and_then(|r| case.files.get(r).cloned()),
        &|path| {
            // A file beside the body is named by its absolute path, which on Windows has a drive before the root.
            let root = std::path::absolute(ROOT).unwrap_or_else(|_| PathBuf::from(ROOT));
            let name = path.strip_prefix(root).ok()?.to_str()?.replace('\\', "/");
            case.files.get(&name).cloned()
        },
    );
    let mut replay = Replay {
        tool: runtime.tool.clone(),
        workspace,
        session,
        body_path,
        ids: StepIds::default(),
        selection: None,
    };

    // The reading.
    let elements = replay.session.render(replay.workspace.documents()).unwrap_or_default();
    let (nodes, relations) = parts::reading(&elements);
    let read = &case.fixture["read"];
    let read_by = |item: &Json| constructs.rule(item["type"].as_str());
    let found_by = |item: &Json| constructs.code(item["rule"].as_str());
    let from_module = case.fixture["recordedFrom"] == "module";
    let (nodes, relations) = (Json::Array(nodes), Json::Array(relations));
    if from_module {
        let case_draws = drawing_of(case, &case.fixture["drawing"]).map(|drawing| meaning::drawn_ids(&drawing));
        let description_draws = replay.drawing().ok().map(|drawing| meaning::drawn_ids(&drawing));
        let stated = meaning::stated_by(read);
        for (expected, actual) in [(&read["elements"], &nodes), (&read["relations"], &relations)] {
            let reading = meaning::Reading {
                expected,
                actual,
                case_draws: case_draws.as_ref(),
                description_draws: description_draws.as_ref(),
                stated: &stated,
            };
            let specification = runtime.tool.definition.specification();
            for (want, got, id) in meaning::reading_differences(specification, &reading) {
                let read_as = actual
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|item| item["id"] == id.as_str());
                differences.push(Difference {
                    step: None,
                    part: "reading",
                    expected: want,
                    actual: got,
                    at: read_as.and_then(read_by),
                });
            }
        }
    } else {
        compare(&mut differences, (None, "reading"), &read["elements"], &nodes, &read_by);
        compare(
            &mut differences,
            (None, "reading"),
            &read["relations"],
            &relations,
            &read_by,
        );
    }
    let findings = parts::findings(&replay.session, replay.workspace.documents());
    compare(
        &mut differences,
        (None, "findings"),
        &read["findings"],
        &Json::Array(findings),
        &found_by,
    );
    if let Some(expected) = drawing_of(case, &case.fixture["drawing"]) {
        replay.compare_drawing(&mut differences, None, &expected, &constructs, from_module);
    }

    // The steps.
    for (index, step) in case.fixture["steps"].as_array().into_iter().flatten().enumerate() {
        let number = Some(index + 1);
        *replay.ids.ids.borrow_mut() = step["ids"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Json::as_str)
            .map(str::to_owned)
            .collect();
        let element = step["select"][0]
            .as_str()
            .map(str::to_owned)
            .or_else(|| replay.selection.clone());
        let element = ["property", "move"]
            .iter()
            .find_map(|gesture| step[gesture]["element"].as_str().map(str::to_owned))
            .or_else(|| step["delete"][0].as_str().map(str::to_owned))
            .or(element);
        let element_type = element.and_then(|id| replay.types().remove(&id));
        let element_type = element_type.as_deref();
        let _ = replay.workspace.take_executed();
        let outcome = replay.step(step);
        let executed = replay.workspace.take_executed();
        let expected_refusal = step["refused"]["reason"].as_str();
        let refused_at = |actual: Option<&str>| {
            let reasons: Vec<&str> = expected_refusal.into_iter().chain(actual).collect();
            constructs.refusal(step, &reasons, element_type)
        };
        match (&outcome, expected_refusal) {
            (Ok(()), None) => {}
            (Err(actual), Some(expected)) if same_refusal(expected, actual) => {}
            (Err(actual), expected) => differences.push(Difference {
                step: number,
                part: "refused",
                expected: expected.map_or("the gesture made".to_owned(), |e| format!("refused: {e}")),
                actual: format!("refused: {actual}"),
                at: refused_at(Some(actual)),
            }),
            (Ok(()), Some(expected)) => differences.push(Difference {
                step: number,
                part: "refused",
                expected: format!("refused: {expected}"),
                actual: "the gesture made".into(),
                at: refused_at(None),
            }),
        }
        if let Some(expected) = step.get("splices") {
            let splices: Vec<Json> = executed
                .iter()
                .filter(|edit| edit.body_path == replay.body_path)
                .flat_map(|edit| &edit.steps)
                .filter(|edit_step| edit_step.file == EditedFile::Body)
                .flat_map(|edit_step| &edit_step.splices)
                .map(|splice| serde_json::to_value(splice).expect("a splice writes"))
                .collect();
            let made_by = |_: &Json| constructs.gesture(step, element_type);
            compare(
                &mut differences,
                (number, "splices"),
                expected,
                &Json::Array(splices),
                &made_by,
            );
        }
        let expected_bytes = match (step.get("expectFile").and_then(Json::as_str), step.get("expect")) {
            (Some(file), _) => case.files.get(file).cloned(),
            (None, Some(Json::String(text))) => Some(text.as_bytes().to_vec()),
            _ => None,
        };
        if let Some(expected) = expected_bytes
            && let Some((expected, actual)) = first_difference(&expected, &replay.bytes())
        {
            differences.push(Difference {
                step: number,
                part: "bytes",
                expected,
                actual,
                at: constructs.gesture(step, element_type),
            });
        }
        if let Some(expected) = step.get("findings") {
            let findings = parts::findings(&replay.session, replay.workspace.documents());
            compare(
                &mut differences,
                (number, "findings"),
                expected,
                &Json::Array(findings),
                &found_by,
            );
        }
        if let Some(expected) = step.get("drawing").and_then(|d| drawing_of(case, d)) {
            replay.compare_drawing(&mut differences, number, &expected, &constructs, from_module);
        }
    }
    differences
}

/// Whether two reasons a gesture was refused with are the same refusal (ATD §2.3, spec 067, FR-011): a reason compares
/// exactly, unless both are the host's own sentence that the gesture is not offered, which says nothing a description
/// wrote.
fn same_refusal(expected: &str, actual: &str) -> bool {
    expected == actual || (not_offered(expected) && not_offered(actual))
}

/// Whether `reason` is one of the host's "not offered" sentences, a closed set (ATD §2.3).
fn not_offered(reason: &str) -> bool {
    // What the host names in its sentence is an action's id: one word, in quotes it does not itself hold.
    let an_id = |id: &str| !id.is_empty() && !id.contains(|c: char| c == '\'' || c.is_whitespace());
    reason == "That action is not available for this item."
        || reason
            .strip_prefix("Unknown action '")
            .and_then(|r| r.strip_suffix("'."))
            .is_some_and(an_id)
        || reason
            .strip_prefix('\'')
            .and_then(|r| r.strip_suffix("' does not apply to this selection."))
            .is_some_and(an_id)
}

/// A case's drawing: inline, or `{"$ref": "<file>"}` beside the case.
fn drawing_of(case: &Case, drawing: &Json) -> Option<Json> {
    match drawing.get("$ref").and_then(Json::as_str) {
        Some(file) => serde_json::from_slice(case.files.get(file)?).ok(),
        None => drawing.is_object().then(|| drawing.clone()),
    }
}

/// The first difference between two drawings, as ATD §2.4 defines their equality: their elements and their edges as
/// sets keyed by `id`, their chrome, and their `order`; with the type of the element or edge that differs.
fn drawing_difference(expected: &Json, actual: &Json) -> Option<(String, String, Option<String>)> {
    for part in ["elements", "edges"] {
        let keyed = |drawing: &Json| -> BTreeMap<String, Json> {
            drawing[part]
                .as_array()
                .into_iter()
                .flatten()
                .map(|item| (item["id"].as_str().unwrap_or_default().to_owned(), item.clone()))
                .collect()
        };
        let (want, got) = (keyed(expected), keyed(actual));
        for id in want.keys().chain(got.keys()) {
            match (want.get(id), got.get(id)) {
                (Some(w), Some(g)) if w == g => {}
                (w, g) => {
                    let show = |v: Option<&Json>| v.map_or("nothing".to_owned(), Json::to_string);
                    let type_name = w.or(g).and_then(|item| item["type"].as_str()).map(str::to_owned);
                    return Some((
                        format!("{part} {id}: {}", show(w)),
                        format!("{part} {id}: {}", show(g)),
                        type_name,
                    ));
                }
            }
        }
    }
    for part in ["chrome", "order"] {
        if expected[part] != actual[part] {
            return Some((
                format!("{part}: {}", expected[part]),
                format!("{part}: {}", actual[part]),
                None,
            ));
        }
    }
    None
}

/// Compares two lists; a difference is attributed by `attribute` to the construct of the first item that differs.
fn compare(
    differences: &mut Vec<Difference>,
    (step, part): (Option<usize>, &'static str),
    expected: &Json,
    actual: &Json,
    attribute: &dyn Fn(&Json) -> Option<At>,
) {
    let expected = if expected.is_null() {
        json!([])
    } else {
        expected.clone()
    };
    if &expected != actual {
        let (want, got) = (expected.as_array(), actual.as_array());
        let length = want.map_or(0, Vec::len).max(got.map_or(0, Vec::len));
        let first = (0..length).find_map(|i| {
            let (w, g) = (want.and_then(|l| l.get(i)), got.and_then(|l| l.get(i)));
            (w != g).then(|| w.or(g)).flatten()
        });
        differences.push(Difference {
            step,
            part,
            expected: expected.to_string(),
            actual: actual.to_string(),
            at: first.and_then(attribute),
        });
    }
}

/// The first line at which two texts differ, as `line N: …` of each; none when they are equal.
fn first_difference(expected: &[u8], actual: &[u8]) -> Option<(String, String)> {
    if expected == actual {
        return None;
    }
    let (expected, actual) = (String::from_utf8_lossy(expected), String::from_utf8_lossy(actual));
    let mut a = expected.split_inclusive('\n');
    let mut b = actual.split_inclusive('\n');
    let mut line = 1;
    loop {
        match (a.next(), b.next()) {
            (Some(x), Some(y)) if x == y => line += 1,
            (x, y) => {
                let show = |l: Option<&str>| l.map_or("(the end)".to_owned(), |l| format!("{l:?}"));
                return Some((format!("line {line}: {}", show(x)), format!("line {line}: {}", show(y))));
            }
        }
    }
}

struct Replay {
    tool: Rc<DiagramTool>,
    workspace: DiagramWorkspace,
    session: DiagramSession,
    body_path: PathBuf,
    ids: StepIds,
    selection: Option<String>,
}

impl Replay {
    /// The drawing the description's notation draws now, recorded as a case records it.
    fn drawing(&self) -> Result<Json, String> {
        let documents = self.workspace.documents();
        let elements = self.session.render(documents)?;
        let scene = adp_drawing::notation_scene(&self.session, documents, &elements)?;
        Ok(adp_drawing::record(&scene, &elements, "canvas"))
    }

    fn compare_drawing(
        &self,
        differences: &mut Vec<Difference>,
        step: Option<usize>,
        expected: &Json,
        constructs: &Constructs,
        from_module: bool,
    ) {
        let difference = match self.drawing() {
            Ok(actual) if from_module => meaning::drawing_difference(expected, &actual).map(|(want, got, id)| {
                let type_of =
                    |item: &Json| (item["id"] == id.as_str()).then(|| item["type"].as_str().map(str::to_owned));
                let drawn = ["elements", "edges"]
                    .iter()
                    .flat_map(|part| actual[*part].as_array().into_iter().flatten())
                    .find_map(type_of)
                    .flatten();
                (want, got, drawn)
            }),
            Ok(actual) => drawing_difference(expected, &actual),
            Err(reason) => Some(("a drawing".to_owned(), format!("none: {reason}"), None)),
        };
        if let Some((expected, actual, type_name)) = difference {
            differences.push(Difference {
                step,
                part: "drawing",
                expected,
                actual,
                at: constructs.drawn(type_name.as_deref()),
            });
        }
    }

    /// Each element's type, by its id, as the document reads now.
    fn types(&self) -> BTreeMap<String, String> {
        self.session
            .render(self.workspace.documents())
            .unwrap_or_default()
            .into_iter()
            .map(|element| (element.id, element.element_type))
            .collect()
    }

    fn bytes(&self) -> Vec<u8> {
        self.workspace
            .documents()
            .get(&self.body_path)
            .map(|d| d.body_bytes().to_vec())
            .unwrap_or_default()
    }

    /// One gesture of a case (ATD §2.3), made as the reference host's recorder made it.
    fn step(&mut self, step: &Json) -> Result<(), String> {
        if let Some(select) = step.get("select") {
            self.select(select)?;
        }
        let args = step.get("args").unwrap_or(&Json::Null);
        if let Some(action) = step.get("action").and_then(Json::as_str) {
            let target = self.target(args)?;
            return self.action(&target, action, args);
        }
        // A connect that ends on empty canvas states that new end as the `create` of its step.
        if let Some(connect) = step.get("connect") {
            return self.connect(connect, step.get("create"), args);
        }
        if let Some(create) = step.get("create") {
            return self.create(create, args);
        }
        if let Some(delete) = step.get("delete") {
            return self.delete(delete);
        }
        if let Some(property) = step.get("property") {
            return self.property(property);
        }
        if let Some(movement) = step.get("move") {
            return self.movement(movement);
        }
        if step.get("undo").is_some() {
            return history(self.workspace.undo());
        }
        if step.get("redo").is_some() {
            return history(self.workspace.redo());
        }
        Err(format!("The runner knows no step {step}."))
    }

    fn select(&mut self, select: &Json) -> Result<(), String> {
        let refs = select.as_array().ok_or("A selection is a list.")?;
        match refs.as_slice() {
            [] => {
                self.selection = None;
                Ok(())
            }
            [one] => {
                let id = one.as_str().ok_or("An element is named by a string.")?.to_owned();
                if self.session.select(self.workspace.documents(), &id).is_none() {
                    return Err(format!("nothing has the id '{id}'"));
                }
                self.selection = Some(id);
                Ok(())
            }
            _ => Err("A selection carries one element.".to_owned()),
        }
    }

    /// What an action is made on: the canvas at `args.at`, the element `args.target`, else the selection.
    fn target(&self, args: &Json) -> Result<ActionTarget, String> {
        if let Some(at) = args.get("at") {
            let (x, y) = point_of(at)?;
            return Ok(ActionTarget::Canvas { x, y });
        }
        match args.get("target").and_then(Json::as_str) {
            Some(target) => Ok(ActionTarget::Element(target.to_owned())),
            None => self
                .selection
                .clone()
                .map(ActionTarget::Element)
                .ok_or_else(|| "nothing is selected, and the document's entry offers no diagram action".to_owned()),
        }
    }

    /// A create gesture on the description's own tool: the entry of its toolbox that `type` names, by its id or by the
    /// element type it creates, dropped at `at`. An entry that names an action runs it on the canvas there, its prompt
    /// answered from `args`, as a toolbox drop and the canvas's menu share one implementation.
    fn create(&mut self, create: &Json, args: &Json) -> Result<(), String> {
        let type_name = create["type"].as_str().ok_or("A create names its type.")?;
        if create.get("parent").is_some() || create.get("attributes").is_some() {
            return Err(format!("The runner knows no step {create}."));
        }
        let (x, y) = point_of(&create["at"])?;
        let tools = self.session.toolbox(self.workspace.documents());
        let creating = self.tool_creating(type_name);
        let Some(tool) = tools
            .iter()
            .find(|tool| tool.id == type_name || creating.as_deref() == Some(tool.id.as_str()))
        else {
            return Err(format!("Unknown action '{type_name}'."));
        };
        if tool.drop_action_id.is_empty() {
            return self.session.drop_tool(&mut self.workspace, &tool.id, x, y, &self.ids);
        }
        self.action(&ActionTarget::Canvas { x, y }, &tool.drop_action_id, args)
    }

    /// The id of the toolbox's first tool that creates the element type `type_name`.
    fn tool_creating(&self, type_name: &str) -> Option<String> {
        let groups = self
            .tool
            .definition
            .specification()
            .root()
            .get("toolbox")?
            .get("groups")?;
        let tool = groups
            .as_array()?
            .iter()
            .filter_map(|group| group.get("tools")?.as_array())
            .flatten()
            .find(|tool| tool.get("creates").and_then(|c| c.as_str()) == Some(type_name))?;
        Some(self.tool.definition.ids().tool_id(tool.get("id")?.as_str()?))
    }

    /// A connect gesture on the description's own tool: a relation of the type `type` names; else the entry of that id
    /// in the menu of a connection between the two ends, as a finished connect gesture runs it. With the `create` of
    /// its step, the gesture is released on empty canvas at that create's `at`, and the tool makes the new end.
    fn connect(&mut self, connect: &Json, created: Option<&Json>, args: &Json) -> Result<(), String> {
        let name = connect["type"].as_str().ok_or("A connect names its type.")?;
        let source = connect["source"].as_str().ok_or("A connect names its source.")?;
        let end = connect["target"].as_str().ok_or("A connect names its target.")?;
        let relations = self
            .session
            .relation_types_between(self.workspace.documents(), source, None);
        let named = relations.iter().find(|relation| *relation == name);
        if let Some(created) = created {
            let (x, y) = point_of(&created["at"])?;
            let relation = named
                .or(relations.first())
                .ok_or_else(|| format!("no relation leaves '{source}' towards empty canvas"))?;
            return self
                .session
                .connect_to_new(&mut self.workspace, relation, source, x, y, &self.ids);
        }
        if named.is_some() {
            return self.session.connect(&mut self.workspace, name, source, end, &self.ids);
        }
        let connection = ActionTarget::Connection {
            source: Some(source.to_owned()),
            target: Some(end.to_owned()),
        };
        self.action(&connection, name, args)
    }

    /// A delete gesture on the description's own tool: the elements named, as one command, by the deletion each one's
    /// menu offers; the step states that it was made, so whatever the tool would ask first is confirmed.
    fn delete(&mut self, delete: &Json) -> Result<(), String> {
        let ids: Vec<String> = delete
            .as_array()
            .ok_or("A delete is a list.")?
            .iter()
            .map(|id| id.as_str().map(str::to_owned).ok_or("An element is named by a string."))
            .collect::<Result<_, _>>()?;
        let documents = self.workspace.documents();
        let deletion = ids.first().and_then(|id| {
            let element = ActionTarget::Element(id.clone());
            self.session
                .actions(documents, &element)
                .into_iter()
                .flat_map(|group| group.actions)
                .map(|action| action.id)
                .find(|action| self.session.is_deletion(documents, &element, action))
        });
        let deletion = deletion.unwrap_or_default();
        match self.session.delete_elements(&mut self.workspace, &ids, &deletion, true) {
            ActionOutcome::Refused(reason) => Err(reason),
            _ => Ok(()),
        }
    }

    /// An action of `target`'s menu, its prompt answered with `args.value` or `args.confirm`. An operation that asks
    /// nothing and ends by editing a label inline takes `args.value` as the text typed into it.
    fn action(&mut self, target: &ActionTarget, action: &str, args: &Json) -> Result<(), String> {
        let value = args.get("value").and_then(Json::as_str);
        let _ = self.session.take_host_actions();
        let prompt = match self.session.execute(&mut self.workspace, target, action, &self.ids) {
            ActionOutcome::Done => {
                let Some(value) = value else { return Ok(()) };
                let edited = self.session.take_host_actions().into_iter().find_map(|a| match a {
                    HostAction::EditLabel { element_id, label } => Some((element_id, label)),
                    _ => None,
                });
                let Some((element, label)) = edited else {
                    return Err(format!(
                        "it ran without asking, so the answer '{value}' the reference host's prompt took was not given"
                    ));
                };
                return self.edit_label(&element, label.as_deref(), value);
            }
            ActionOutcome::Refused(reason) => return Err(reason),
            ActionOutcome::Prompt(prompt) => prompt,
        };
        let answer = match prompt {
            Prompt::Confirm { .. } => (args.get("confirm").and_then(Json::as_bool) == Some(true)).then(String::new),
            Prompt::Closed { message } => return Err(format!("the prompt closed: {message}")),
            _ => value.map(str::to_owned),
        };
        let Some(answer) = answer else {
            // Dismissed, as Escape dismisses it.
            return Ok(());
        };
        match self
            .session
            .answer_with_ids(&mut self.workspace, target, action, &answer, &self.ids)
        {
            ActionOutcome::Done => Ok(()),
            ActionOutcome::Refused(reason) => Err(reason),
            ActionOutcome::Prompt(_) => Err("the answer asked again".to_owned()),
        }
    }

    /// The inline editor of `element`'s label committed with `value`: the property the label shows set to it.
    fn edit_label(&mut self, element: &str, label: Option<&str>, value: &str) -> Result<(), String> {
        let rows = self.session.properties(self.workspace.documents(), &[element]);
        let attribute = label.unwrap_or("text");
        let row = rows
            .iter()
            .find(|r| r.id.rsplit(['.', ':']).next() == Some(attribute))
            .or_else(|| rows.first())
            .map(|r| r.id.clone())
            .ok_or_else(|| format!("'{element}' has no label to edit"))?;
        self.session.set_property(&mut self.workspace, &[element], &row, value)
    }

    /// A property row set, named by its id or by its label as the grid shows it.
    fn property(&mut self, property: &Json) -> Result<(), String> {
        let row = property
            .get("attribute")
            .and_then(Json::as_str)
            .ok_or("A property step names its row.")?;
        let value = property
            .get("value")
            .and_then(Json::as_str)
            .ok_or("A property step gives a value.")?;
        let id = match property.get("element").and_then(Json::as_str) {
            Some(id) => id.to_owned(),
            None => self.selection.clone().ok_or("nothing is selected")?,
        };
        let rows = self.session.properties(self.workspace.documents(), &[id.as_str()]);
        let row_id = rows
            .iter()
            .find(|r| r.id == row)
            .or_else(|| rows.iter().find(|r| r.label == row))
            .map(|r| r.id.clone())
            .ok_or_else(|| format!("'{id}' has no property row '{row}'"))?;
        self.session
            .set_property(&mut self.workspace, &[id.as_str()], &row_id, value)
    }

    /// A move: a re-parent, or a place.
    fn movement(&mut self, movement: &Json) -> Result<(), String> {
        let id = movement
            .get("element")
            .and_then(Json::as_str)
            .ok_or("A move names its element.")?;
        if let Some(parent) = movement.get("parent").and_then(Json::as_str) {
            let index = movement.get("index").and_then(Json::as_i64).unwrap_or(-1);
            return self.session.move_element(&mut self.workspace, id, Some(parent), index);
        }
        // A move by a distance goes from where the element is drawn, as the reference host's drag did.
        if movement.get("dx").is_some() || movement.get("dy").is_some() {
            let distance = |key: &str| movement.get(key).and_then(Json::as_f64).unwrap_or(0.0);
            let elements = self.session.render(self.workspace.documents())?;
            let element = elements
                .iter()
                .find(|e| e.id == id && e.kind == ElementKind::Node)
                .ok_or_else(|| format!("no node '{id}' is on the diagram"))?;
            let (x, y) = (element.x + distance("dx"), element.y + distance("dy"));
            return self.session.move_element_to(&mut self.workspace, id, x, y);
        }
        let x = movement.get("x").and_then(Json::as_f64).ok_or("A place has x.")?;
        let y = movement.get("y").and_then(Json::as_f64).ok_or("A place has y.")?;
        self.session.move_element_to(&mut self.workspace, id, x, y)
    }
}

/// A place as a step writes it: `x` and `y`.
fn point_of(at: &Json) -> Result<(f64, f64), String> {
    let x = at.get("x").and_then(Json::as_f64).ok_or("A place has x.")?;
    let y = at.get("y").and_then(Json::as_f64).ok_or("A place has y.")?;
    Ok((x, y))
}

fn history(outcome: Outcome) -> Result<(), String> {
    match outcome {
        Outcome::Refused(refusal) => Err(refusal.0),
        _ => Ok(()),
    }
}
