//! Replays the edit scripts against the goldens recorded from the standalone host (etalii.adp spec 030, SC-004, task
//! T050; contracts/parity.md, "The check").
//!
//! An edit script (`parity/scripts/<tool>/<name>.json`) is a person's edits to one document, step by step; its golden
//! (`parity/goldens/<tool>/<name>/`) holds the bytes the standalone host at `8b6ccfc` wrote for them, and
//! `recorded.json`, what each step sent and the ids the standalone host generated (`parity/recorder/README.md`). This
//! crate replays a script headless in the desktop core and compares each file the golden lists byte for byte:
//!
//! 1. The fixture's folder is copied whole into a temporary project, as the recorder copies it.
//! 2. The registration (else the fixture) is opened through `adp-diagram`'s [`DiagramWorkspace`], as
//!    `DiagramService.Open` opened it.
//! 3. Each step is mapped onto the [`DiagramSession`] the way the recorder mapped it onto the standalone host's RPCs:
//!    a selection is remembered, an action runs on its `target` or the selection and its prompt is answered with
//!    `value` or `confirm` or dismissed, a property row named by id or label is set, a move re-parents or places, and
//!    undo and redo run on the project's history. See [`replay`].
//! 4. An element or connection a step creates gets the id the standalone host generated at that step, from
//!    `recorded.json`, so the bytes can match; `$N` and `$cN` name the N-th generated element and connection.
//! 5. Every file in the golden's `changed` list is compared with the project's file of the same path, and every file
//!    in `removed` must be gone. A mismatch names the first differing byte offset and both lines ([`compare`]).
//!
//! Line endings: a golden is the standalone host's bytes on the platform `recorded.json` names. The C4 layout sidecar
//! (`*.layout.json`) is the one file the standalone host writes with the platform's line ending
//! (`System.Text.Json`'s indented output uses `Environment.NewLine`), so a desktop replay is compared in the line
//! endings of the platform the golden was recorded on: when the replay runs on another platform, the sidecar's line
//! endings are turned into the recorded platform's before the comparison ([`in_recorded_line_endings`]). Every other
//! file is compared as written, and on the recorded platform the sidecar is too.
//!
//! A designer's script - the Knowledge designer's, whose document is a table - is replayed through its session rather
//! than through `adp-diagram` ([`table`]).

pub mod table;

use std::cell::{Cell, RefCell};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_context::Prompt;
use adp_core::Catalogue;
use adp_diagram::session::{ActionOutcome, ActionTarget};
use adp_diagram::tool::DiagramTools;
use adp_diagram::workspace::DiagramWorkspace;
use adp_diagram::{DiagramSession, ElementKind};
use adp_disl::{DislPluginFunctions, IdSource};
use adp_history::Outcome;
use serde::Deserialize;
use serde_json::Value as Json;

/// The repository's root, where `parity/` and `vendor/` are.
pub fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// An edit script (contracts/parity.md, "Edit scripts").
#[derive(Debug, Clone, Deserialize)]
pub struct Script {
    pub tool: String,
    /// The document, under `parity/fixtures/`; its folder is the project.
    pub fixture: String,
    /// The registration opened instead of the fixture, under `parity/fixtures/`.
    #[serde(default)]
    pub registration: Option<String>,
    pub steps: Vec<Json>,
}

/// What the recorder wrote beside a golden (`parity/recorder/README.md`, "Goldens").
#[derive(Debug, Clone, Deserialize)]
pub struct Recorded {
    /// `windows` or `linux`: the platform whose line endings the platform-dependent files were written in.
    pub platform: String,
    pub changed: Vec<String>,
    #[serde(default)]
    pub removed: Vec<String>,
    #[serde(default)]
    pub generated: Vec<Generated>,
}

/// An id the standalone host generated, at the step that made it.
#[derive(Debug, Clone, Deserialize)]
pub struct Generated {
    pub step: usize,
    pub id: String,
    /// `<origin>+<type in lower case>`, as the diagram stream reported it.
    #[serde(rename = "type")]
    pub element_type: String,
    pub connection: bool,
}

/// A script and its golden, loaded.
#[derive(Debug, Clone)]
pub struct Case {
    /// The script's path under `parity/scripts/`, without `.json` (`generic/timeline/roadmap`).
    pub name: String,
    pub script: Script,
    pub recorded: Recorded,
    /// `parity/goldens/<name>/`.
    pub golden: PathBuf,
}

impl Case {
    /// Loads `parity/scripts/<name>.json` and its golden.
    pub fn load(name: &str) -> Result<Case, String> {
        let parity = repository_root().join("parity");
        let script_path = parity.join("scripts").join(format!("{name}.json"));
        let golden = parity.join("goldens").join(name);
        let script: Script = read_json(&script_path)?;
        let recorded: Recorded = read_json(&golden.join("recorded.json"))?;
        Ok(Case {
            name: name.to_owned(),
            script,
            recorded,
            golden,
        })
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// Every script under `parity/scripts/`, by its name without `.json`, sorted.
pub fn script_names() -> Vec<String> {
    let scripts = repository_root().join("parity/scripts");
    let mut names = Vec::new();
    let mut folders = vec![scripts.clone()];
    while let Some(folder) = folders.pop() {
        let Ok(entries) = fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                folders.push(path);
            } else if path.extension().is_some_and(|e| e == "json")
                && let Ok(relative) = path.with_extension("").strip_prefix(&scripts)
            {
                names.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    names.sort();
    names
}

// ---- the comparison ------------------------------------------------------------------------------------------------

/// Where two files first differ: the byte offset, and the line holding it in each.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mismatch {
    pub offset: usize,
    pub golden_line: String,
    pub replay_line: String,
    pub golden_length: usize,
    pub replay_length: usize,
}

impl fmt::Display for Mismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "differs at byte offset {} (golden {} bytes, replay {} bytes)\n    golden: {:?}\n    replay: {:?}",
            self.offset, self.golden_length, self.replay_length, self.golden_line, self.replay_line
        )
    }
}

/// Compares `replay` with `golden` byte for byte: none when they are equal, else the first differing offset (the
/// shorter one's length when one is a prefix of the other) and the line around it in each, its line ending included.
pub fn compare(golden: &[u8], replay: &[u8]) -> Option<Mismatch> {
    if golden == replay {
        return None;
    }
    let offset = golden
        .iter()
        .zip(replay)
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| golden.len().min(replay.len()));
    Some(Mismatch {
        offset,
        golden_line: line_at(golden, offset),
        replay_line: line_at(replay, offset),
        golden_length: golden.len(),
        replay_length: replay.len(),
    })
}

/// The line of `bytes` that holds `offset`, with its line ending; past the end, the last line.
fn line_at(bytes: &[u8], offset: usize) -> String {
    let mut offset = offset.min(bytes.len());
    if offset == bytes.len() {
        offset = offset.saturating_sub(1);
    }
    let start = bytes[..offset].iter().rposition(|b| *b == b'\n').map_or(0, |i| i + 1);
    let end = bytes[start..]
        .iter()
        .position(|b| *b == b'\n')
        .map_or(bytes.len(), |i| start + i + 1);
    String::from_utf8_lossy(&bytes[start..end]).into_owned()
}

/// The platform this replay runs on, in `recorded.json`'s words.
pub fn this_platform() -> &'static str {
    if cfg!(windows) { "windows" } else { "linux" }
}

/// Whether the standalone host writes `path` with the platform's line ending: the C4 layout sidecar and the Wardley
/// map's identity sidecar, which `System.Text.Json` writes with `Environment.NewLine` (`parity/recorder/README.md`).
pub fn platform_line_endings(path: &str) -> bool {
    path.ends_with(".layout.json") || path.ends_with(".identities.json")
}

/// The bytes the desktop host wrote for `path`, in the line endings of the platform the golden was recorded on: a
/// file written with the platform's line ending whose replay runs on another platform has its line endings turned
/// into the recorded platform's (CRLF for `windows`, LF otherwise); every other file is returned as written.
pub fn in_recorded_line_endings(path: &str, bytes: Vec<u8>, recorded_platform: &str, replay_platform: &str) -> Vec<u8> {
    if !platform_line_endings(path) || recorded_platform == replay_platform {
        return bytes;
    }
    let mut lf = Vec::with_capacity(bytes.len());
    for (i, b) in bytes.iter().enumerate() {
        if *b == b'\r' && bytes.get(i + 1) == Some(&b'\n') {
            continue;
        }
        lf.push(*b);
    }
    if recorded_platform != "windows" {
        return lf;
    }
    let mut crlf = Vec::with_capacity(lf.len() + lf.len() / 16);
    for b in lf {
        if b == b'\n' {
            crlf.push(b'\r');
        }
        crlf.push(b);
    }
    crlf
}

// ---- the replay ----------------------------------------------------------------------------------------------------

/// A step that went otherwise in the desktop core than in the standalone host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepDifference {
    pub step: usize,
    pub step_text: String,
    pub what: String,
}

/// What one file of the golden came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileResult {
    Matches,
    Differs(Mismatch),
    /// The replay did not write it, or removed it.
    Missing,
    /// The golden says the standalone host removed it; the replay kept it.
    NotRemoved,
}

/// The outcome of replaying one script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The document did not open in the desktop host, with the reason it gave.
    CannotOpen(String),
    /// The steps ran; the differences per step and per file.
    Replayed {
        steps: Vec<StepDifference>,
        files: Vec<(String, FileResult)>,
    },
}

impl Verdict {
    /// Whether every step went as the standalone host's did and every file matches its golden.
    pub fn matches(&self) -> bool {
        match self {
            Verdict::CannotOpen(_) => false,
            Verdict::Replayed { steps, files } => {
                steps.is_empty() && files.iter().all(|(_, r)| *r == FileResult::Matches)
            }
        }
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Verdict::CannotOpen(reason) => write!(f, "cannot open: {reason}"),
            Verdict::Replayed { steps, files } => {
                for difference in steps {
                    writeln!(
                        f,
                        "step {} {}: {}",
                        difference.step, difference.step_text, difference.what
                    )?;
                }
                for (path, result) in files {
                    match result {
                        FileResult::Matches => writeln!(f, "{path}: matches")?,
                        FileResult::Differs(mismatch) => writeln!(f, "{path}: {mismatch}")?,
                        FileResult::Missing => writeln!(f, "{path}: the replay did not write it")?,
                        FileResult::NotRemoved => {
                            writeln!(f, "{path}: the standalone host removed it; the replay kept it")?
                        }
                    }
                }
                Ok(())
            }
        }
    }
}

/// The ids the standalone host generated, handed out at the step that generated them: a create asks for an id of its
/// type and gets the next unused one of that step whose recorded type is that type, else the next unused one of the
/// step; a create the standalone host did not make at that step is refused with a sentence.
struct RecordedIds {
    generated: Vec<Generated>,
    used: RefCell<Vec<bool>>,
    step: Cell<usize>,
}

impl RecordedIds {
    fn new(generated: Vec<Generated>) -> Self {
        RecordedIds {
            used: RefCell::new(vec![false; generated.len()]),
            generated,
            step: Cell::new(0),
        }
    }

    /// The N-th (from 1) generated element, or connection when `connection`.
    fn nth(&self, n: usize, connection: bool) -> Option<&str> {
        self.generated
            .iter()
            .filter(|g| g.connection == connection)
            .nth(n.checked_sub(1)?)
            .map(|g| g.id.as_str())
    }

    /// `reference` with each `$N` and `$cN` replaced by the id it names.
    fn resolve(&self, reference: &str) -> Result<String, String> {
        let mut out = String::new();
        let mut rest = reference;
        while let Some(at) = rest.find('$') {
            out.push_str(&rest[..at]);
            let after = &rest[at + 1..];
            let connection = after.starts_with('c');
            let from = usize::from(connection);
            let digits: String = after[from..].chars().take_while(char::is_ascii_digit).collect();
            let n: usize = digits
                .parse()
                .map_err(|_| format!("'{reference}' names no generated id"))?;
            let id = self
                .nth(n, connection)
                .ok_or_else(|| format!("'{reference}': recorded.json lists no generated id for it"))?;
            out.push_str(id);
            rest = &after[from + digits.len()..];
        }
        out.push_str(rest);
        Ok(out)
    }
}

impl IdSource for RecordedIds {
    fn next(&self, type_name: &str) -> Result<String, String> {
        let step = self.step.get();
        let mut used = self.used.borrow_mut();
        let candidates: Vec<usize> = (0..self.generated.len())
            .filter(|i| self.generated[*i].step == step && !used[*i])
            .collect();
        let wanted = type_name.to_lowercase();
        let pick = candidates
            .iter()
            .copied()
            .find(|i| {
                self.generated[*i]
                    .element_type
                    .rsplit_once('+')
                    .is_some_and(|(_, t)| t == wanted)
            })
            .or_else(|| candidates.first().copied());
        match pick {
            Some(i) => {
                used[i] = true;
                Ok(self.generated[i].id.clone())
            }
            None => Err(format!(
                "the standalone host generated no further id at step {step}, but the desktop host creates a {type_name}"
            )),
        }
    }
}

/// What a replay keeps between steps, as the recorder's connection kept it: the session and the selection.
struct Replay<'a> {
    workspace: DiagramWorkspace,
    session: DiagramSession,
    ids: &'a RecordedIds,
    selection: Option<String>,
}

/// Replays `case` in a copy of its fixture's folder, and compares the files its golden lists; an error is a script
/// or golden the runner cannot read.
pub fn replay(case: &Case) -> Result<Verdict, String> {
    let fixtures = repository_root().join("parity/fixtures");
    let fixture = fixtures.join(&case.script.fixture);
    let folder = fixture
        .parent()
        .ok_or_else(|| format!("{} has no folder", fixture.display()))?;
    let project = tempfile::tempdir().map_err(|e| e.to_string())?;
    copy_folder(folder, project.path())?;
    let opened = case.script.registration.as_deref().unwrap_or(&case.script.fixture);
    let document = fixtures
        .join(opened)
        .strip_prefix(folder)
        .map_err(|_| format!("{opened} is not in the fixture's folder"))?
        .to_path_buf();
    if table::is_table(&case.script.tool) {
        return table::replay(case, project.path(), &document);
    }

    // The ids the standalone host generated, handed out at the steps that made them; the Wardley map mints its ids
    // itself, when its identities are reconciled, so its module is handed them too.
    let ids = Rc::new(RecordedIds::new(case.recorded.generated.clone()));
    let catalogue = Catalogue::load(&repository_root().join("vendor"));
    let tools = adp_diagram_mindmap::register(DiagramTools::new(catalogue, |_| DislPluginFunctions::none()))
        .with_module(adp_diagram_timeline::ORIGIN, adp_diagram_timeline::module())
        .with_module(adp_diagram_causal_loop::ORIGIN, adp_diagram_causal_loop::module())
        .with_module(
            adp_diagram_wardley_map::ORIGIN,
            Rc::new(adp_diagram_wardley_map::WardleyModule::with_ids(ids.clone())),
        );
    let tools = adp_diagram_dependency_graph::register(tools);
    let mut tools = adp_diagram_c4::register(tools);
    let mut workspace = DiagramWorkspace::new(project.path());
    let (_, mut session) = match workspace.open(&mut tools, &document) {
        Ok(opened) => opened,
        Err(reason) => return Ok(Verdict::CannotOpen(reason)),
    };
    // The recorder opened the document and asked for everything, as a browser tab does.
    if let Err(reason) = session.baseline(workspace.documents()) {
        return Ok(Verdict::CannotOpen(reason));
    }

    let mut replay = Replay {
        workspace,
        session,
        ids: &ids,
        selection: None,
    };
    let mut steps = Vec::new();
    for (index, step) in case.script.steps.iter().enumerate() {
        ids.step.set(index);
        let refusal_expected = step.get("expect").and_then(Json::as_str) == Some("refused");
        let what = match (replay.step(step), refusal_expected) {
            (Ok(()), false) | (Err(_), true) => None,
            (Ok(()), true) => Some("the standalone host refused it; the desktop host accepted it".to_owned()),
            (Err(reason), false) => Some(format!("the standalone host accepted it; the desktop host: {reason}")),
        };
        if let Some(what) = what {
            steps.push(StepDifference {
                step: index,
                step_text: compact(step),
                what,
            });
        }
    }

    let files = compare_files(case, project.path())?;
    Ok(Verdict::Replayed { steps, files })
}

/// Compares each file of the golden's `changed` list with the project's file of that path, and checks that each file
/// of `removed` is gone.
fn compare_files(case: &Case, project: &Path) -> Result<Vec<(String, FileResult)>, String> {
    let mut files = Vec::new();
    for path in &case.recorded.changed {
        let golden = fs::read(case.golden.join(path)).map_err(|e| format!("the golden {path}: {e}"))?;
        let result = match fs::read(project.join(path)) {
            Err(_) => FileResult::Missing,
            Ok(bytes) => {
                let bytes = in_recorded_line_endings(path, bytes, &case.recorded.platform, this_platform());
                compare(&golden, &bytes).map_or(FileResult::Matches, FileResult::Differs)
            }
        };
        files.push((path.clone(), result));
    }
    for path in &case.recorded.removed {
        if project.join(path).exists() {
            files.push((path.clone(), FileResult::NotRemoved));
        }
    }
    Ok(files)
}

impl Replay<'_> {
    fn step(&mut self, step: &Json) -> Result<(), String> {
        if let Some(select) = step.get("select") {
            return self.select(select);
        }
        if let Some(action) = step.get("action").and_then(Json::as_str) {
            let args = step.get("args").cloned().unwrap_or(Json::Null);
            return self.action(action, &args);
        }
        if let Some(property) = step.get("property") {
            return self.property(property);
        }
        if let Some(movement) = step.get("move") {
            return self.movement(movement);
        }
        if let Some(n) = step.get("undo").and_then(Json::as_u64) {
            return self.history(n, true);
        }
        if let Some(n) = step.get("redo").and_then(Json::as_u64) {
            return self.history(n, false);
        }
        if let Some(save) = step.get("save") {
            return self.save(save);
        }
        Err(format!("the runner knows no step {}", compact(step)))
    }

    fn element_id(&self, reference: &Json) -> Result<String, String> {
        let reference = reference.as_str().ok_or("an element is named by a string")?;
        self.ids.resolve(reference)
    }

    /// `ContextService.Select`: the document, or one element of it.
    fn select(&mut self, select: &Json) -> Result<(), String> {
        let refs = select.as_array().ok_or("a selection is a list")?;
        match refs.as_slice() {
            [] => {
                self.selection = None;
                Ok(())
            }
            [one] => {
                let id = self.element_id(one)?;
                if self.session.select(self.workspace.documents(), &id).is_none() {
                    return Err(format!("nothing has the id '{id}'"));
                }
                self.selection = Some(id);
                Ok(())
            }
            _ => Err("a selection carries one element".to_owned()),
        }
    }

    /// The source of an action: `args.target` (an element, `new:x,y` for the canvas, `rel:a->b` for a connect
    /// gesture), else the selection.
    fn target(&self, target: Option<&Json>) -> Result<ActionTarget, String> {
        let Some(target) = target else {
            return self
                .selection
                .clone()
                .map(ActionTarget::Element)
                .ok_or_else(|| "nothing is selected, and the document's entry offers no diagram action".to_owned());
        };
        let target = self.element_id(target)?;
        if let Some(point) = target.strip_prefix("new:") {
            let (x, y) = point.split_once(',').ok_or("a canvas target is new:x,y")?;
            let number = |n: &str| n.trim().parse::<f64>().map_err(|e| format!("'{n}': {e}"));
            return Ok(ActionTarget::Canvas {
                x: number(x)?,
                y: number(y)?,
            });
        }
        if let Some(ends) = target.strip_prefix("rel:") {
            let (source, target) = ends.split_once("->").ok_or("a connection target is rel:a->b")?;
            return Ok(ActionTarget::Connection {
                source: Some(source.to_owned()),
                target: Some(target.to_owned()),
            });
        }
        Ok(ActionTarget::Element(target))
    }

    /// `ContextService.ExecuteAction`, its prompt answered with `args.value` or `args.confirm`, or dismissed.
    fn action(&mut self, action: &str, args: &Json) -> Result<(), String> {
        if args.get("answers").is_some() {
            return Err("the runner does not map a prompt that leads to another onto the desktop session".to_owned());
        }
        // The explorer's add, on the entry the step names (T068).
        if action == "hierarchy.add" {
            return self.add(args);
        }
        // A history action without a target is sent against the project.
        if args.get("target").is_none() {
            match action {
                "history.undo" => return self.history(1, true),
                "history.redo" => return self.history(1, false),
                _ => {}
            }
        }
        let target = self.target(args.get("target"))?;
        if let ActionTarget::Connection {
            source: Some(source),
            target: Some(end),
        } = &target
            && let Some(point) = end.strip_prefix("new:")
        {
            return self.connect_to_new(source, point);
        }
        let prompt = match self.session.execute(&mut self.workspace, &target, action, self.ids) {
            ActionOutcome::Done => {
                return match args.get("value").and_then(Json::as_str) {
                    Some(value) => Err(format!(
                        "it ran without asking, so the answer '{value}' the standalone host's prompt took was not given"
                    )),
                    None => Ok(()),
                };
            }
            ActionOutcome::Refused(reason) => return Err(reason),
            ActionOutcome::Prompt(prompt) => prompt,
        };
        let answer = match prompt {
            Prompt::Confirm { .. } => (args.get("confirm").and_then(Json::as_bool) == Some(true)).then(String::new),
            Prompt::Closed { message } => return Err(format!("the prompt closed: {message}")),
            _ => args.get("value").and_then(Json::as_str).map(str::to_owned),
        };
        let Some(answer) = answer else {
            // Dismissed, as Escape dismisses it.
            return Ok(());
        };
        match self.session.answer(&mut self.workspace, &target, action, &answer) {
            ActionOutcome::Done => Ok(()),
            ActionOutcome::Refused(reason) => Err(reason),
            ActionOutcome::Prompt(_) => Err("the answer asked again".to_owned()),
        }
    }

    /// The explorer's add on the entry named `args.entry` - a folder to create in, a file to register - answered
    /// with `value` and `text`, as `adp-add` does it for the shell.
    fn add(&self, args: &Json) -> Result<(), String> {
        let entry = args
            .get("entry")
            .and_then(Json::as_str)
            .ok_or("an add names its entry")?;
        let path = entry_named(self.workspace.root(), entry).ok_or_else(|| format!("no entry '{entry}'"))?;
        let target = if path.is_dir() {
            adp_add::AddTarget::Folder(path)
        } else {
            adp_add::AddTarget::File(path)
        };
        let value = args.get("value").and_then(Json::as_str).unwrap_or_default();
        let text = args.get("text").and_then(Json::as_str).unwrap_or_default();
        adp_add::AddCatalogue::standalone()
            .commit(&target, value, text)
            .map(|_| ())
    }

    /// A connect gesture from `source` released on empty canvas at `point` (`rel:a->new:x,y`), as the canvas runs it:
    /// the first relation type that may leave `source` towards empty canvas, its target made at the point.
    fn connect_to_new(&mut self, source: &str, point: &str) -> Result<(), String> {
        let (x, y) = point.split_once(',').ok_or("a canvas end is new:x,y")?;
        let number = |n: &str| n.trim().parse::<f64>().map_err(|e| format!("'{n}': {e}"));
        let relation = self
            .session
            .relation_types_between(self.workspace.documents(), source, None)
            .into_iter()
            .next()
            .ok_or_else(|| format!("no relation leaves '{source}' towards empty canvas"))?;
        self.session
            .connect_to_new(&mut self.workspace, &relation, source, number(x)?, number(y)?, self.ids)
    }

    /// `ContextService.SetProperty`, the row named by its id or by its label as the grid shows it.
    fn property(&mut self, property: &Json) -> Result<(), String> {
        let row = property
            .get("row")
            .and_then(Json::as_str)
            .ok_or("a property step names its row")?;
        let value = property
            .get("value")
            .and_then(Json::as_str)
            .ok_or("a property step gives a value")?;
        let id = match property.get("target") {
            Some(target) => self.element_id(target)?,
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

    /// `DiagramService.MoveElement`: a re-parent, a place, or the element's place plus a distance.
    fn movement(&mut self, movement: &Json) -> Result<(), String> {
        let refs = movement
            .get("refs")
            .and_then(Json::as_array)
            .ok_or("a move names its refs")?;
        let ids = refs.iter().map(|r| self.element_id(r)).collect::<Result<Vec<_>, _>>()?;
        if let Some(parent) = movement.get("parent") {
            let parent = self.element_id(parent)?;
            let index = movement.get("index").and_then(Json::as_i64).unwrap_or(-1);
            for id in &ids {
                self.session
                    .move_element(&mut self.workspace, id, Some(&parent), index)?;
            }
            return Ok(());
        }
        let moves = if let Some(to) = movement.get("to") {
            let x = to.get("x").and_then(Json::as_f64).ok_or("a place has x")?;
            let y = to.get("y").and_then(Json::as_f64).ok_or("a place has y")?;
            ids.iter().map(|id| (id.clone(), x, y)).collect()
        } else {
            let dx = movement.get("dx").and_then(Json::as_f64).unwrap_or(0.0);
            let dy = movement.get("dy").and_then(Json::as_f64).unwrap_or(0.0);
            let elements = self.session.render(self.workspace.documents())?;
            ids.iter()
                .map(|id| {
                    let element = elements
                        .iter()
                        .find(|e| e.id == *id && e.kind == ElementKind::Node)
                        .ok_or_else(|| format!("no node '{id}' is on the diagram"))?;
                    Ok((id.clone(), element.x + dx, element.y + dy))
                })
                .collect::<Result<Vec<_>, String>>()?
        };
        self.session.move_elements(&mut self.workspace, &moves)
    }

    /// `history.undo` or `history.redo` against the project, `n` times.
    fn history(&mut self, n: u64, undo: bool) -> Result<(), String> {
        for _ in 0..n {
            let outcome = if undo {
                self.workspace.undo()
            } else {
                self.workspace.redo()
            };
            if let Outcome::Refused(refusal) = outcome {
                return Err(refusal.0);
            }
        }
        Ok(())
    }

    /// `{ "save": true }` sends nothing, since every command is written as it runs; `{ "save": { "path", "content" } }`
    /// is a text editor's save, which a diagram session has no editor for.
    fn save(&mut self, save: &Json) -> Result<(), String> {
        if save.as_bool() == Some(true) {
            return Ok(());
        }
        Err("the runner does not map a text editor's save onto the desktop session".to_owned())
    }
}

/// The first entry under `root` named `name`, breadth first in name order, as the recorder looks an entry up.
fn entry_named(root: &Path, name: &str) -> Option<PathBuf> {
    let mut pending = std::collections::VecDeque::from([root.to_path_buf()]);
    while let Some(folder) = pending.pop_front() {
        let mut entries: Vec<PathBuf> = fs::read_dir(&folder).ok()?.flatten().map(|e| e.path()).collect();
        entries.sort();
        for path in entries {
            if path.file_name().is_some_and(|n| n == name) {
                return Some(path);
            }
            if path.is_dir() {
                pending.push_back(path);
            }
        }
    }
    None
}

fn compact(step: &Json) -> String {
    serde_json::to_string(step).unwrap_or_default()
}

fn copy_folder(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|e| format!("{}: {e}", to.display()))?;
    let entries = fs::read_dir(from).map_err(|e| format!("{}: {e}", from.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let target = to.join(entry.file_name());
        if path.is_dir() {
            copy_folder(&path, &target)?;
        } else {
            fs::copy(&path, &target).map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }
    Ok(())
}

/// Replays the script `name` and fails with everything that differs: the test each script under `parity/scripts/` is.
pub fn check(name: &str) {
    let case = Case::load(name).unwrap_or_else(|e| panic!("{name}: {e}"));
    let verdict = replay(&case).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(verdict.matches(), "{name} does not match its golden:\n{verdict}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_bytes_match() {
        assert_eq!(compare(b"a\nb\n", b"a\nb\n"), None);
    }

    #[test]
    fn a_mismatch_names_the_first_differing_offset_and_both_lines() {
        let mismatch = compare(b"one\ntwo\nthree\n", b"one\ntwice\nthree\n").unwrap();
        assert_eq!(mismatch.offset, 6);
        assert_eq!(mismatch.golden_line, "two\n");
        assert_eq!(mismatch.replay_line, "twice\n");
    }

    #[test]
    fn a_prefix_differs_at_the_shorter_ones_end() {
        let mismatch = compare(b"one\ntwo\n", b"one\n").unwrap();
        assert_eq!(mismatch.offset, 4);
        assert_eq!(mismatch.golden_line, "two\n");
        assert_eq!(mismatch.replay_line, "one\n");
    }

    #[test]
    fn a_line_ending_is_a_difference() {
        let mismatch = compare(b"a\r\nb\r\n", b"a\nb\n").unwrap();
        assert_eq!(mismatch.offset, 1);
        assert_eq!(mismatch.golden_line, "a\r\n");
        assert_eq!(mismatch.replay_line, "a\n");
    }

    #[test]
    fn the_layout_sidecar_is_compared_in_the_recorded_platforms_line_endings() {
        let lf = b"{\n  \"a\": 1\n}".to_vec();
        let crlf = b"{\r\n  \"a\": 1\r\n}".to_vec();
        assert_eq!(
            in_recorded_line_endings("x.layout.json", lf.clone(), "windows", "linux"),
            crlf
        );
        assert_eq!(
            in_recorded_line_endings("x.layout.json", crlf.clone(), "linux", "windows"),
            lf
        );
        assert_eq!(
            in_recorded_line_endings("x.layout.json", lf.clone(), "windows", "windows"),
            lf,
            "on the recorded platform the bytes are compared as written"
        );
        assert_eq!(
            in_recorded_line_endings("map.identities.json", lf.clone(), "windows", "linux"),
            crlf,
            "the Wardley map's identity sidecar is written with the platform's line ending too"
        );
        assert_eq!(
            in_recorded_line_endings("courier.dsl", lf.clone(), "windows", "linux"),
            lf,
            "only a file the standalone host writes with the platform's line ending is converted"
        );
    }

    fn generated(step: usize, id: &str, element_type: &str, connection: bool) -> Generated {
        Generated {
            step,
            id: id.to_owned(),
            element_type: element_type.to_owned(),
            connection,
        }
    }

    #[test]
    fn generated_ids_are_named_by_their_order_among_elements_and_among_connections() {
        let ids = RecordedIds::new(vec![
            generated(1, "e1", "generic/timeline+period", false),
            generated(1, "k1", "generic/timeline+connection", true),
            generated(2, "e2", "generic/timeline+period", false),
        ]);
        assert_eq!(ids.resolve("$2").unwrap(), "e2");
        assert_eq!(ids.resolve("$c1").unwrap(), "k1");
        assert_eq!(ids.resolve("rel:discovery->$1").unwrap(), "rel:discovery->e1");
        assert_eq!(ids.resolve("discovery").unwrap(), "discovery");
        assert!(ids.resolve("$3").is_err());
    }

    #[test]
    fn a_create_gets_the_id_its_step_generated_for_its_type() {
        let ids = RecordedIds::new(vec![
            generated(1, "p", "generic/timeline+period", false),
            generated(1, "c", "generic/timeline+connection", true),
        ]);
        ids.step.set(1);
        assert_eq!(ids.next("Connection").unwrap(), "c");
        assert_eq!(ids.next("Period").unwrap(), "p");
        assert!(
            ids.next("Period").is_err(),
            "a step creates no more than it did in the standalone host"
        );
        ids.step.set(2);
        assert!(ids.next("Period").is_err());
    }

    #[test]
    fn every_script_has_a_golden() {
        let names = script_names();
        assert!(!names.is_empty());
        for name in names {
            Case::load(&name).unwrap_or_else(|e| panic!("{e}"));
        }
    }
}
