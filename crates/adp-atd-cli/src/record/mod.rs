//! `atd record <origin> --cases <dir>` (etalii.adp spec 035, T050, research R17, contracts/conformance.md): one
//! conformance case per parity edit script of the tool type, recorded from the desktop host's existing module. The
//! script's documents are opened and its gestures replayed headlessly (`adp-parity`); the case keeps the documents
//! as its inputs, byte for byte, and after the reading and after each gesture what the module read, the findings it
//! gave, the drawing its canvas draws (the shell's recording painter) and the bytes it wrote. The replay must end in the
//! standalone host's golden bytes, so the bytes are the standalone host's; readings, findings and drawings are the
//! desktop host's. Each part names who produced it.
//!
//! A script's gestures are written in ATD §2.3's terms (`gesture`, etalii.adp spec 067, T018 to T020), and the script
//! is then replayed on the module once more as the case writes it, so that recording fails rather than writes a step
//! whose result is not the one it states (FR-006).

mod c4;
pub mod capability;
pub mod check;
mod editor;
mod gesture;
mod hierarchy;
mod w3c;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use adp_atd_validate::parts::{findings, reading};
use adp_diagram::ElementKind;
use adp_parity::{Case, Observed};
use adp_theme::Kind;
use gesture::{Before, Written};
use serde_json::{Map, Value as Json, json};

/// The desktop host's commit whose behaviour the readings and drawings stand for (research R17).
pub const REFERENCE: &str = "fe5feec";

/// What a reading or a step left behind.
#[derive(PartialEq)]
struct Seen {
    outcome: Result<(), String>,
    generated: Vec<String>,
    bytes: Option<Vec<u8>>,
    findings: Vec<Json>,
    drawing: Result<Json, String>,
    read: Option<(Vec<Json>, Vec<Json>)>,
}

impl Seen {
    fn of(observed: &Observed, origin: &str, body_path: &Path) -> Seen {
        let elements = observed.session.render(observed.documents).unwrap_or_default();
        Seen {
            outcome: observed.outcome.clone(),
            generated: observed.generated.clone(),
            bytes: fs::read(observed.project.join(body_path)).ok(),
            findings: findings(observed.session, observed.documents),
            drawing: adp_shell::recording::drawing_of(origin, observed.session, observed.documents, Kind::Light),
            read: observed.step.is_none().then(|| reading(&elements)),
        }
    }
}

/// A gesture's references with the ids of `$N` and `$cN` put in, from the ids the standalone host generated.
fn resolve(value: &Json, generated: &[(String, bool)]) -> Json {
    let nth = |n: usize, connection: bool| {
        generated
            .iter()
            .filter(|(_, c)| *c == connection)
            .nth(n.checked_sub(1)?)
            .map(|(id, _)| id.clone())
    };
    match value {
        Json::String(text) if text.contains('$') => {
            let mut out = String::new();
            let mut rest = text.as_str();
            while let Some(at) = rest.find('$') {
                out.push_str(&rest[..at]);
                let after = &rest[at + 1..];
                let connection = after.starts_with('c');
                let from = usize::from(connection);
                let digits: String = after[from..].chars().take_while(char::is_ascii_digit).collect();
                match digits.parse().ok().and_then(|n| nth(n, connection)) {
                    Some(id) => out.push_str(&id),
                    None => out.push_str(&rest[at..at + 1 + from + digits.len()]),
                }
                rest = &after[from + digits.len()..];
            }
            out.push_str(rest);
            Json::String(out)
        }
        Json::Array(items) => Json::Array(items.iter().map(|v| resolve(v, generated)).collect()),
        Json::Object(map) => Json::Object(map.iter().map(|(k, v)| (k.clone(), resolve(v, generated))).collect()),
        other => other.clone(),
    }
}

thread_local! {
    /// The commit `recordedWith` names in place of the one `atd record` runs at: `atd record --check` records each case
    /// as it was recorded.
    static RECORDED_WITH: RefCell<Option<String>> = const { RefCell::new(None) };
    /// The gestures ATD's steps cannot state, as recording found them (spec 067, FR-007).
    static UNSTATED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// The gestures recorded since the last call that ATD's steps cannot state, one line each, and forgets them.
pub fn take_unstated() -> Vec<String> {
    UNSTATED.with(|u| std::mem::take(&mut *u.borrow_mut()))
}

pub(crate) fn override_recorded_with(commit: Option<String>) {
    RECORDED_WITH.with(|r| *r.borrow_mut() = commit);
}

/// The short commit of the desktop host `atd record` runs at.
fn recorded_with() -> String {
    if let Some(commit) = RECORDED_WITH.with(|r| r.borrow().clone()) {
        return commit;
    }
    if let Ok(commit) = std::env::var("ATD_RECORDED_WITH") {
        return commit;
    }
    std::process::Command::new("git")
        .args(["rev-parse", "--short=7", "HEAD"])
        .current_dir(adp_parity::repository_root())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

pub(crate) fn write_json(path: &Path, value: &Json) -> Result<(), String> {
    let mut text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    text.push('\n');
    write(path, text.as_bytes())
}

/// Records every parity script of `origin` as a case folder under `cases`, named after the script; returns the
/// folders written. `description` is the path of the tool description from a case's folder.
pub fn record(origin: &str, cases: &Path, description: Option<&str>) -> Result<Vec<PathBuf>, String> {
    if editor::is_editor(origin) {
        let short = origin.rsplit('/').next().unwrap_or(origin);
        let description = description.map_or_else(|| format!("../../../{short}.atd"), str::to_owned);
        return editor::record(origin, cases, &description);
    }
    let of_origin = |name: &String| name.rsplit_once('/').is_some_and(|(tool, _)| tool == origin);
    let names: Vec<String> = adp_parity::script_names().into_iter().filter(of_origin).collect();
    let minted: Vec<String> = adp_parity::case_names().into_iter().filter(of_origin).collect();
    if names.is_empty() && minted.is_empty() {
        return Err(format!("no parity script records `{origin}`"));
    }
    let short = origin.rsplit('/').next().unwrap_or(origin);
    let description = description.map_or_else(|| format!("../../../{short}.atd"), str::to_owned);
    let mut written = Vec::new();
    let loaded = names
        .iter()
        .map(|name| Case::load(name))
        .chain(minted.iter().map(|name| Case::load_minted(name)));
    for case in loaded {
        let case = case?;
        let name = case.name.clone();
        if adp_parity::table::is_table(&case.script.tool) {
            return Err(format!("{name}: a designer's table is not recorded yet"));
        }
        let folder = cases.join(name.rsplit('/').next().unwrap_or(&name));
        if adp_parity::hierarchy::is_hierarchy(&case.script.tool) {
            hierarchy::record_case(&case, &folder, &description)?;
        } else {
            record_case(&case, &folder, &description)?;
        }
        written.push(folder);
    }
    Ok(written)
}

/// Copies the case's fixture's folder to `folder`, made anew, and names its inputs: the body, the registration, and
/// every other file of the folder. Returns the body's file name and the inputs.
fn copy_inputs(case: &Case, folder: &Path) -> Result<(String, Map<String, Json>), String> {
    let fixtures = adp_parity::repository_root().join(if case.minted { "parity/cases" } else { "parity/fixtures" });
    let fixture = fixtures.join(&case.script.fixture);
    let source = fixture.parent().ok_or("a fixture has a folder")?;
    let body = fixture
        .file_name()
        .ok_or("a fixture is a file")?
        .to_string_lossy()
        .into_owned();
    if folder.exists() {
        fs::remove_dir_all(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    }
    adp_parity::copy_folder(source, folder)?;

    // The inputs: the body, the registration, and every other file of the fixture's folder.
    let mut inputs = Map::new();
    inputs.insert("body".into(), json!(body));
    let registration = case
        .script
        .registration
        .as_deref()
        .and_then(|r| Path::new(r).file_name())
        .map(|r| r.to_string_lossy().into_owned());
    if let Some(registration) = &registration {
        inputs.insert("registration".into(), json!(registration));
    }
    let mut others: Vec<String> = Vec::new();
    collect_files(folder, folder, &mut others);
    others.sort();
    for other in others {
        if other != body && Some(&other) != registration.as_ref() {
            inputs.insert(other.clone(), json!(other));
        }
    }

    Ok((body, inputs))
}

/// Records one script as the case in `folder`, which is made anew.
fn record_case(case: &Case, folder: &Path, description: &str) -> Result<(), String> {
    let (body, inputs) = copy_inputs(case, folder)?;
    let origin = case.script.tool.clone();
    let body_path = PathBuf::from(&body);
    let generated: Vec<(String, bool)> = case
        .recorded
        .generated
        .iter()
        .map(|g| (g.id.clone(), g.connection))
        .collect();
    let mut seen: Vec<Seen> = Vec::new();
    // What the module said of each step's gesture just before it was made, and what its gesture made.
    let mut before: Vec<Before> = Vec::new();
    let mut made: Vec<BTreeMap<String, (String, bool)>> = Vec::new();
    // Whether the tool's module opens its own documents, which the case says of itself (ATD §2.3, spec 067, FR-012).
    let mut from_module = false;
    let verdict = adp_parity::replay_observed(case, &mut |observed: &Observed| {
        from_module = observed.session.module_view().is_some();
        let elements = observed.session.render(observed.documents).unwrap_or_default();
        made.push(
            elements
                .iter()
                .filter(|e| observed.generated.contains(&e.id))
                .map(|e| (e.id.clone(), (e.element_type.clone(), e.kind == ElementKind::Relation)))
                .collect(),
        );
        seen.push(Seen::of(observed, &origin, &body_path));
        let next = observed.step.map_or(0, |step| step + 1);
        if let Some(step) = case.script.steps.get(next) {
            before.push(Before::of(
                &resolve(step, &generated),
                observed.session,
                observed.documents,
            ));
        }
    })?;
    if !verdict.matches() {
        return Err(format!(
            "{}: the replay does not end in the golden's bytes:\n{verdict}",
            case.name
        ));
    }
    let Some(opened) = seen.first() else {
        return Err(format!("{}: the document did not open", case.name));
    };
    let (elements, relations) = opened.read.clone().unwrap_or_default();
    write_json(
        &folder.join("drawing.read.json"),
        opened.drawing.as_ref().map_err(String::clone)?,
    )?;

    // The bytes each step leaves, each distinct text written once: the inputs' own name for the bytes the case
    // started with, `after-<n>` for the others.
    // A folder's own registration is named `.adp` and has no extension by Rust's reckoning; its bytes after a step are
    // `after-N.adp`, a name every platform can check out.
    let extension = Path::new(&body)
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .or_else(|| {
            Path::new(&body)
                .file_name()
                .and_then(|n| n.to_str()?.strip_prefix('.').map(str::to_owned))
        })
        .unwrap_or_default();
    let mut texts: BTreeMap<Vec<u8>, String> = BTreeMap::new();
    if let Some(bytes) = &opened.bytes {
        texts.insert(bytes.clone(), body.clone());
    }

    let mut steps: Vec<Json> = Vec::new();
    // The script's steps as the case writes its gestures, replayed below.
    let mut written: Vec<Json> = Vec::new();
    let mut select: Option<Json> = None;
    // What the module has selected, and what a replay of the case has: a gesture's own target is not the module's
    // selection, and is the case's.
    let mut selected = json!([]);
    let mut replayed = json!([]);
    for (index, (step, after)) in case.script.steps.iter().zip(seen.iter().skip(1)).enumerate() {
        let step = resolve(step, &generated);
        let mut gesture = Map::new();
        let mut repeat = 1;
        let mut targeted = None;
        // Whether the gesture acts on the selection.
        let mut on_selection = false;
        if let Some(refs) = step.get("select") {
            select = Some(refs.clone());
            selected = refs.clone();
            written.push(step);
            continue;
        } else if step.get("action").is_some() {
            match gesture::write(&step, &before[index], &made[index + 1]) {
                Written::Gesture { fields, select } => {
                    on_selection =
                        fields.contains_key("action") && fields.get("args").is_none_or(|a| a.get("at").is_none());
                    gesture = fields;
                    targeted = select;
                }
                Written::Unstated { reason, fields } => {
                    let name = &case.name;
                    UNSTATED.with(|u| {
                        u.borrow_mut()
                            .push(format!("{name}, script step {}: {reason}", index + 1))
                    });
                    gesture = fields;
                }
            }
        } else if let Some(property) = step.get("property") {
            let mut set = Map::new();
            if let Some(target) = property.get("target") {
                set.insert("element".into(), target.clone());
            }
            set.insert("attribute".into(), property.get("row").cloned().unwrap_or_default());
            set.insert("value".into(), property.get("value").cloned().unwrap_or_default());
            on_selection = property.get("target").is_none();
            gesture.insert("property".into(), Json::Object(set));
        } else if let Some(movement) = step.get("move") {
            let mut to = Map::new();
            if let Some(element) = movement["refs"].as_array().and_then(|r| r.first()) {
                to.insert("element".into(), element.clone());
            }
            // A move into a parent goes before the child at `index` there, or last when it is -1 (ATD §2.3).
            for key in ["parent", "index", "x", "y", "dx", "dy"] {
                if let Some(value) = movement.get(key)
                    && (key != "index" || movement.get("parent").is_some())
                {
                    to.insert(key.into(), value.clone());
                }
            }
            // A move to a place: the parity script's `to`, a step's own `x` and `y` in the case (ATD §2.3).
            for key in ["x", "y"] {
                if let Some(value) = movement["to"].get(key) {
                    to.insert(key.into(), value.clone());
                }
            }
            gesture.insert("move".into(), Json::Object(to));
            gesture.insert("doc".into(), json!(format!("The parity script's gesture: {movement}")));
        } else if let Some(n) = step.get("undo").and_then(Json::as_u64) {
            gesture.insert("undo".into(), json!(true));
            repeat = n.max(1) as usize;
        } else if let Some(n) = step.get("redo").and_then(Json::as_u64) {
            gesture.insert("redo".into(), json!(true));
            repeat = n.max(1) as usize;
        } else {
            // A save writes nothing a case states.
            written.push(step);
            continue;
        }
        // The script's selection goes with the next gesture, unless that gesture names what it is made on; a gesture
        // on the selection after one that named its own says again what the module still has selected.
        let carried = select.take();
        let refs = match targeted {
            Some(target) => Some(json!([target])),
            None if carried.is_some() => carried,
            None if on_selection && replayed != selected => Some(selected.clone()),
            None => None,
        };
        let mut as_written = if step.get("action").is_some() {
            Json::Object(gesture.clone())
        } else {
            step
        };
        if let Some(refs) = refs {
            replayed = refs.clone();
            as_written["select"] = refs.clone();
            gesture.insert("select".into(), refs);
        }
        written.push(as_written);
        if !after.generated.is_empty() {
            gesture.insert("ids".into(), json!(after.generated));
        }
        if let Err(reason) = &after.outcome {
            gesture.insert("refused".into(), json!({"reason": reason}));
        }
        // Several undos or redos at once are that many steps; what they leave is stated on the last.
        for _ in 1..repeat {
            steps.push(Json::Object(gesture.clone()));
        }
        if let Some(bytes) = &after.bytes {
            let next = texts.len();
            let name = texts
                .entry(bytes.clone())
                .or_insert_with(|| format!("after-{next}.{extension}"))
                .clone();
            if !folder.join(&name).exists() {
                write(&folder.join(&name), bytes)?;
            }
            gesture.insert("expectFile".into(), json!(name));
        }
        gesture.insert("findings".into(), json!(after.findings));
        if let Ok(drawing) = &after.drawing {
            let file = format!("drawing.{}.json", steps.len() + 1);
            write_json(&folder.join(&file), drawing)?;
            gesture.insert("drawing".into(), json!({"$ref": file}));
        }
        steps.push(Json::Object(gesture));
    }

    same_when_replayed_as_written(case, written, &seen, &origin, &body_path)?;

    let desktop = |part: &str| {
        json!({
            "host": "desktop", "repository": "etalii.adp.ide.desktop", "commit": REFERENCE,
            "recordedWith": recorded_with(), "source": format!("atd record ({part})"),
        })
    };
    let standalone = case.recorded.standalone_commit.chars().take(7).collect::<String>();
    // A case without a golden has the desktop host's bytes; a parity script's are the standalone host's golden.
    let bytes = if case.minted {
        desktop("bytes")
    } else {
        json!({
            "host": "standalone", "repository": "etalii.adp.ide.standalone", "commit": standalone,
            "source": format!("etalii.adp.ide.desktop/parity/goldens/{}/", case.name),
        })
    };
    let source = if case.minted { "parity/cases" } else { "parity/scripts" };
    let mut fixture = json!({
        "$schema": "https://etalii.net/adp/atd/schema/0.1/atd.schema.json#/$defs/ConformanceFixture",
        "doc": format!(
            "Recorded by `atd record {}` from {source}/{}.json of etalii.adp.ide.desktop; never edited by hand.",
            case.script.tool, case.name
        ),
        "tool": case.script.tool,
        "description": description,
        "inputs": inputs,
        "read": {"elements": elements, "relations": relations, "findings": opened.findings},
        "drawing": {"$ref": "drawing.read.json"},
        "steps": steps,
        "provenance": {
            "bytes": bytes,
            "reading": desktop("reading"),
            "findings": desktop("findings"),
            "drawing": desktop("drawing"),
        },
    });
    if from_module {
        fixture["recordedFrom"] = json!("module");
    }
    write_json(&folder.join("conformance.json"), &fixture)
}

/// Replays the script on the module with its steps as the case writes them: every step must leave what the script's
/// own step left (spec 067, FR-006), or the case would state a result its gesture does not give.
fn same_when_replayed_as_written(
    case: &Case,
    written: Vec<Json>,
    seen: &[Seen],
    origin: &str,
    body_path: &Path,
) -> Result<(), String> {
    let mut as_written = case.clone();
    as_written.script.steps = written;
    let mut again: Vec<Seen> = Vec::new();
    adp_parity::replay_observed(&as_written, &mut |observed: &Observed| {
        again.push(Seen::of(observed, origin, body_path));
    })?;
    let different = (0..seen.len()).find(|&n| seen.get(n) != again.get(n));
    match different {
        None if seen.len() == again.len() => Ok(()),
        None => Err(format!("{}: replayed as written, its steps do not line up", case.name)),
        Some(0) => Err(format!("{}: replayed as written, the reading differs", case.name)),
        Some(n) => Err(format!(
            "{}: script step {n}, written as {}, does not leave what it left: {:?}, not {:?}",
            case.name,
            as_written.script.steps[n - 1],
            again.get(n).map(|s| &s.outcome),
            seen[n].outcome,
        )),
    }
}

pub(crate) fn collect_files(root: &Path, folder: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(folder) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, out);
        } else if let Ok(relative) = path.strip_prefix(root) {
            out.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
}
