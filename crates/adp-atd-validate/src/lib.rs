//! Validation of a tool description against its conformance fixtures (etalii.adp spec 035, T127, FR-041; ATD §2.3).
//!
//! A case is replayed through the description's runtime on working copies held in memory: its inputs opened from
//! their bytes, each step's gesture made with the ids the reference host generated, in that order. After the reading
//! and after each step, what the runtime gives is compared with what the case states: the bytes, byte for byte, the
//! reading, the findings, the refusal and the drawing (as ATD §2.4 defines its equality). Each difference is
//! reported with its step, its part, the expected and the actual value, for bytes the first line that differs.
//! Nothing is read from or written to the file system.

pub mod parts;

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};

use adp_atd_runtime::RuntimeTool;
use adp_context::Prompt;
use adp_diagram::session::{ActionOutcome, ActionTarget, DiagramSession};
use adp_diagram::workspace::DiagramWorkspace;
use adp_disl::{HostAction, IdSource};
use adp_history::Outcome;
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
    /// `bytes`, `reading`, `findings`, `refused` or `drawing`.
    pub part: &'static str,
    pub expected: String,
    pub actual: String,
}

impl std::fmt::Display for Difference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.step {
            Some(step) => write!(f, "step {step}, {}: ", self.part)?,
            None => write!(f, "read, {}: ", self.part)?,
        }
        write!(f, "expected {}, got {}", self.expected, self.actual)
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

/// Replays `case` through `runtime`, and every difference from what it states.
pub fn validate(runtime: &RuntimeTool, case: &Case) -> Vec<Difference> {
    let mut differences = Vec::new();
    let inputs = &case.fixture["inputs"];
    let Some(body_name) = inputs["body"].as_str() else {
        differences.push(Difference {
            step: None,
            part: "reading",
            expected: "a body among the inputs".into(),
            actual: "none".into(),
        });
        return differences;
    };
    let body_path = Path::new(ROOT).join(body_name);
    let registration_name = inputs["registration"].as_str();
    let registration_path = registration_name.map(|r| Path::new(ROOT).join(r));
    let mut workspace = DiagramWorkspace::in_memory(ROOT);
    let session = workspace.open_bytes(
        runtime.tool.clone(),
        &body_path,
        registration_path.as_deref(),
        case.files.get(body_name).cloned().unwrap_or_default(),
        registration_name.and_then(|r| case.files.get(r).cloned()),
    );
    let mut replay = Replay {
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
    compare(
        &mut differences,
        None,
        "reading",
        &read["elements"],
        &Json::Array(nodes),
    );
    compare(
        &mut differences,
        None,
        "reading",
        &read["relations"],
        &Json::Array(relations),
    );
    let findings = parts::findings(&replay.session, replay.workspace.documents());
    compare(
        &mut differences,
        None,
        "findings",
        &read["findings"],
        &Json::Array(findings),
    );
    if let Some(expected) = drawing_of(case, &case.fixture["drawing"]) {
        replay.compare_drawing(&mut differences, None, &expected);
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
        let outcome = replay.step(step);
        let expected_refusal = step["refused"]["reason"].as_str();
        match (&outcome, expected_refusal) {
            (Ok(()), None) => {}
            (Err(actual), Some(expected)) if actual == expected => {}
            (Err(actual), expected) => differences.push(Difference {
                step: number,
                part: "refused",
                expected: expected.map_or("the gesture made".to_owned(), |e| format!("refused: {e}")),
                actual: format!("refused: {actual}"),
            }),
            (Ok(()), Some(expected)) => differences.push(Difference {
                step: number,
                part: "refused",
                expected: format!("refused: {expected}"),
                actual: "the gesture made".into(),
            }),
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
            });
        }
        if let Some(expected) = step.get("findings") {
            let findings = parts::findings(&replay.session, replay.workspace.documents());
            compare(&mut differences, number, "findings", expected, &Json::Array(findings));
        }
        if let Some(expected) = step.get("drawing").and_then(|d| drawing_of(case, d)) {
            replay.compare_drawing(&mut differences, number, &expected);
        }
    }
    differences
}

/// A case's drawing: inline, or `{"$ref": "<file>"}` beside the case.
fn drawing_of(case: &Case, drawing: &Json) -> Option<Json> {
    match drawing.get("$ref").and_then(Json::as_str) {
        Some(file) => serde_json::from_slice(case.files.get(file)?).ok(),
        None => drawing.is_object().then(|| drawing.clone()),
    }
}

/// The first difference between two drawings, as ATD §2.4 defines their equality: their elements and their edges as
/// sets keyed by `id`, their chrome, and their `order`.
fn drawing_difference(expected: &Json, actual: &Json) -> Option<(String, String)> {
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
                    return Some((format!("{part} {id}: {}", show(w)), format!("{part} {id}: {}", show(g))));
                }
            }
        }
    }
    for part in ["chrome", "order"] {
        if expected[part] != actual[part] {
            return Some((
                format!("{part}: {}", expected[part]),
                format!("{part}: {}", actual[part]),
            ));
        }
    }
    None
}

fn compare(differences: &mut Vec<Difference>, step: Option<usize>, part: &'static str, expected: &Json, actual: &Json) {
    let expected = if expected.is_null() {
        json!([])
    } else {
        expected.clone()
    };
    if &expected != actual {
        differences.push(Difference {
            step,
            part,
            expected: expected.to_string(),
            actual: actual.to_string(),
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

    fn compare_drawing(&self, differences: &mut Vec<Difference>, step: Option<usize>, expected: &Json) {
        let difference = match self.drawing() {
            Ok(actual) => drawing_difference(expected, &actual),
            Err(reason) => Some(("a drawing".to_owned(), format!("none: {reason}"))),
        };
        if let Some((expected, actual)) = difference {
            differences.push(Difference {
                step,
                part: "drawing",
                expected,
                actual,
            });
        }
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
        if let Some(action) = step.get("action").and_then(Json::as_str) {
            return self.action(action, step.get("args").unwrap_or(&Json::Null));
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

    fn target(&self, args: &Json) -> Result<ActionTarget, String> {
        match args.get("target").and_then(Json::as_str) {
            Some(target) => Ok(ActionTarget::Element(target.to_owned())),
            None => self
                .selection
                .clone()
                .map(ActionTarget::Element)
                .ok_or_else(|| "nothing is selected, and the document's entry offers no diagram action".to_owned()),
        }
    }

    /// An action of the selection's menu, its prompt answered with `args.value` or `args.confirm`. An operation that
    /// asks nothing and ends by editing a label inline takes `args.value` as the text typed into it.
    fn action(&mut self, action: &str, args: &Json) -> Result<(), String> {
        let target = self.target(args)?;
        let value = args.get("value").and_then(Json::as_str);
        let _ = self.session.take_host_actions();
        let prompt = match self.session.execute(&mut self.workspace, &target, action, &self.ids) {
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
            .answer_with_ids(&mut self.workspace, &target, action, &answer, &self.ids)
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
        let x = movement.get("x").and_then(Json::as_f64).ok_or("A place has x.")?;
        let y = movement.get("y").and_then(Json::as_f64).ok_or("A place has y.")?;
        self.session.move_element_to(&mut self.workspace, id, x, y)
    }
}

fn history(outcome: Outcome) -> Result<(), String> {
    match outcome {
        Outcome::Refused(refusal) => Err(refusal.0),
        _ => Ok(()),
    }
}
