//! Every FBL fixture vendored from etalii.adp, replayed (etalii.adp spec 030, FR-034, SC-003, tasks T038, T039).
//!
//! This first level checks what the splice catalogue alone decides, as etalii.adp's own validator does (FBL section
//! 15.3): each step's splices turn the body before it into its expected body and back by their inverses; an undo
//! gives back the body the undone edit started from and a redo the body it left; a refused edit and a save change
//! nothing. Reading the input and planning each edit's splices from its binding is `standalone.rs`, which runs the
//! standalone host's harness over the fixtures that host reads.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use adp_fbl::{Splice, apply};
use serde_json::Value;

fn fixtures() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/specifications/fbl/fixtures");
    let mut found = Vec::new();
    let mut folders = vec![root];
    while let Some(folder) = folders.pop() {
        for entry in fs::read_dir(&folder).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                folders.push(path);
            } else if path.file_name().is_some_and(|n| n == "fixture.json") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// The registration blocks a fixture's binding keeps outside the history (FBL 0.7, sections 7.1, 8.1).
fn blocks_outside_history(folder: &Path, reference: &str) -> Vec<String> {
    let (file, name) = reference.split_once('#').unwrap_or((reference, ""));
    let Ok(text) = fs::read_to_string(folder.join(file)) else {
        return Vec::new();
    };
    let Ok(document) = serde_json::from_str::<Value>(&text) else {
        return Vec::new();
    };
    document["bindings"][name]["registration"]["blocks"]
        .as_object()
        .map(|blocks| {
            blocks
                .iter()
                .filter(|(_, b)| b["history"] == Value::Bool(false))
                .map(|(k, _)| k.clone())
                .collect()
        })
        .unwrap_or_default()
}

type Changes = Vec<(Vec<u8>, Vec<u8>)>;

/// What each splice replaces and with what, in the order of their offsets.
fn changes(body: &[u8], splices: &[Splice]) -> Changes {
    let mut ordered: Vec<&Splice> = splices.iter().collect();
    ordered.sort_by_key(|s| (s.start, s.end));
    ordered
        .iter()
        .map(|s| (body[s.start..s.end].to_vec(), s.text.as_bytes().to_vec()))
        .collect()
}

/// Every file of a fixture as a step leaves it, by the name its splices give it; `None` is the input.
type Files = BTreeMap<Option<String>, Vec<u8>>;

/// A fixture whose splices name their files (FBL 0.10, section 15.3): the input and every file beside it that a step
/// names, each file's splices turning its bytes into those the step states, an undo or a redo checked over all of them.
fn replay_files(folder: &Path, fixture: &Value, steps: &[Value]) -> Result<usize, String> {
    let input = folder.join(fixture["input"].as_str().unwrap());
    let beside = input.parent().unwrap().to_path_buf();
    let mut files = Files::from([(None, fs::read(&input).map_err(|e| e.to_string())?)]);
    let (mut done, mut undone): (Vec<(Files, Files)>, Vec<(Files, Files)>) = (Vec::new(), Vec::new());
    for (index, step) in steps.iter().enumerate() {
        let number = index + 1;
        let expected_files = step["expectFiles"].as_object().cloned().unwrap_or_default();
        let splices = step["splices"].as_array().cloned().unwrap_or_default();
        let named = splices
            .iter()
            .filter_map(|s| s["file"].as_str())
            .chain(expected_files.keys().map(String::as_str));
        for name in named {
            if !files.contains_key(&Some(name.to_owned())) {
                let bytes = fs::read(beside.join(name)).map_err(|e| format!("step {number}: {name}: {e}"))?;
                files.insert(Some(name.to_owned()), bytes);
            }
        }
        let before = files.clone();
        let names: Vec<Option<String>> = files.keys().cloned().collect();
        for name in names {
            let own: Vec<Splice> = splices
                .iter()
                .filter(|s| s["file"].as_str() == name.as_deref())
                .map(|s| serde_json::from_value(s.clone()).map_err(|e| format!("step {number}: {e}")))
                .collect::<Result<_, _>>()?;
            let shown = name.as_deref().unwrap_or("the input");
            let edit = apply(&before[&name], &own).map_err(|e| format!("step {number}: {shown}: {e}"))?;
            if apply(&edit.after, &edit.inverse).map_err(|e| e.to_string())?.after != before[&name] {
                return Err(format!("step {number}: {shown}: the inverse splices do not restore it"));
            }
            let expected = match &name {
                Some(file) => expected_files.get(file).and_then(Value::as_str).map(str::as_bytes),
                None => step["expect"].as_str().map(str::as_bytes),
            };
            if expected.is_some_and(|e| e != edit.after) {
                return Err(format!("step {number}: {shown}: the splices give different bytes"));
            }
            files.insert(name, edit.after);
        }
        // A file first named by this step is compared as it stood, which the step's undo returns to as well.
        let same = |a: &Files, b: &Files| b.iter().all(|(n, bytes)| a.get(n).is_none_or(|x| x == bytes));
        if step["undo"] == Value::Bool(true) {
            let (start, end) = done.pop().ok_or(format!("step {number}: nothing to undo"))?;
            if !same(&before, &end) || !same(&files, &start) {
                return Err(format!(
                    "step {number}: the undo does not return what the edit started from"
                ));
            }
            undone.push((start, end));
        } else if step["redo"] == Value::Bool(true) {
            let (start, end) = undone.pop().ok_or(format!("step {number}: nothing to redo"))?;
            if !same(&before, &start) || !same(&files, &end) {
                return Err(format!(
                    "step {number}: the redo does not return what the edit produced"
                ));
            }
            done.push((start, end));
        } else if step.get("refused").is_some() {
            if files != before {
                return Err(format!("step {number}: a refused edit must leave every file as it is"));
            }
        } else {
            done.push((before, files.clone()));
            undone.clear();
        }
    }
    Ok(steps.len())
}

fn replay(path: &Path) -> Result<usize, String> {
    let folder = path.parent().unwrap();
    let fixture: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).map_err(|e| e.to_string())?;
    let mut body = fs::read(folder.join(fixture["input"].as_str().unwrap())).map_err(|e| e.to_string())?;
    let outside = blocks_outside_history(folder, fixture["binding"].as_str().unwrap_or_default());
    // (before, after, changes, crossed) of each edit that can be undone, and of each that can be redone.
    let mut done: Vec<(Vec<u8>, Vec<u8>, Changes, bool)> = Vec::new();
    let mut undone: Vec<(Vec<u8>, Vec<u8>, Changes, bool)> = Vec::new();
    let steps = fixture["steps"].as_array().cloned().unwrap_or_default();
    if steps
        .iter()
        .flat_map(|s| s["splices"].as_array().into_iter().flatten())
        .any(|s| s.get("file").is_some())
    {
        return replay_files(folder, &fixture, &steps);
    }
    for (index, step) in steps.iter().enumerate() {
        let number = index + 1;
        let expected = if let Some(file) = step["expectFile"].as_str() {
            fs::read(folder.join(file)).map_err(|e| e.to_string())?
        } else if let Some(text) = step["expect"].as_str() {
            text.as_bytes().to_vec()
        } else {
            body.clone()
        };
        let splices: Vec<Splice> =
            serde_json::from_value(step["splices"].clone()).map_err(|e| format!("step {number}: {e}"))?;
        let edit = apply(&body, &splices).map_err(|e| format!("step {number}: {e}"))?;
        if edit.after != expected {
            let at = edit
                .after
                .iter()
                .zip(&expected)
                .position(|(a, b)| a != b)
                .unwrap_or(edit.after.len().min(expected.len()));
            return Err(format!(
                "step {number}: the splices give a different body, first at byte {at}"
            ));
        }
        if apply(&edit.after, &edit.inverse).map_err(|e| e.to_string())?.after != body {
            return Err(format!(
                "step {number}: the inverse splices do not restore the body before the step"
            ));
        }
        let edits: Vec<Value> = match &step["edit"] {
            Value::Array(list) => list.clone(),
            Value::Null => Vec::new(),
            one => vec![one.clone()],
        };
        let step_changes = changes(&body, &splices);
        if step["undo"] == Value::Bool(true) {
            let (before, after, edit_changes, crossed) = done.pop().ok_or(format!("step {number}: nothing to undo"))?;
            let swapped: Changes = edit_changes
                .iter()
                .map(|(old, new)| (new.clone(), old.clone()))
                .collect();
            if crossed && step_changes != swapped {
                return Err(format!(
                    "step {number}: the undo does not put back what the undone edit changed"
                ));
            }
            if !crossed && (body != after || edit.after != before) {
                return Err(format!(
                    "step {number}: the undo does not return the body the undone edit started from"
                ));
            }
            undone.push((before, after, edit_changes, crossed));
        } else if step["redo"] == Value::Bool(true) {
            let (before, after, edit_changes, crossed) =
                undone.pop().ok_or(format!("step {number}: nothing to redo"))?;
            if crossed && step_changes != edit_changes {
                return Err(format!(
                    "step {number}: the redo does not make the changes the edit made"
                ));
            }
            if !crossed && (body != before || edit.after != after) {
                return Err(format!(
                    "step {number}: the redo does not return the body the edit produced"
                ));
            }
            done.push((before, after, edit_changes, crossed));
        } else if step.get("refused").is_some() || edits.iter().any(|e| e.get("save").is_some()) {
            if edit.after != body {
                return Err(format!(
                    "step {number}: a refused edit or a save without change must leave the body as it is"
                ));
            }
        } else if !edits.is_empty()
            && edits.iter().all(|e| {
                e["setting"]["block"]
                    .as_str()
                    .is_some_and(|b| outside.iter().any(|o| o == b))
            })
        {
            for entry in done.iter_mut().chain(undone.iter_mut()) {
                entry.3 = true;
            }
        } else {
            done.push((body.clone(), edit.after.clone(), step_changes, false));
            undone.clear();
        }
        body = edit.after;
    }
    Ok(steps.len())
}

#[test]
fn every_fixture_replays_by_its_splices() {
    let fixtures = fixtures();
    assert!(fixtures.len() >= 50, "only {} fixtures were found", fixtures.len());
    let mut failures = Vec::new();
    let mut steps = 0;
    for path in &fixtures {
        match replay(path) {
            Ok(count) => steps += count,
            Err(problem) => failures.push(format!("{}: {problem}", path.display())),
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} fixtures failed:\n{}",
        failures.len(),
        fixtures.len(),
        failures.join("\n")
    );
    assert!(steps > 700, "only {steps} steps were replayed");
}

#[test]
fn every_operation_of_the_catalogue_is_exercised_by_the_fixtures() {
    let mut seen = std::collections::HashSet::new();
    for path in fixtures() {
        let fixture: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        for step in fixture["steps"].as_array().into_iter().flatten() {
            let splices: Vec<Splice> = serde_json::from_value(step["splices"].clone()).unwrap();
            seen.extend(splices.into_iter().map(|s| s.operation));
        }
    }
    for operation in adp_fbl::Operation::ALL {
        assert!(seen.contains(&operation), "no fixture uses {operation}");
    }
}

/// A planted defect in one expected body is caught (SC-003).
#[test]
fn a_planted_defect_is_caught() {
    let path = fixtures()
        .into_iter()
        .find(|p| p.to_string_lossy().contains("timeline-edits"))
        .unwrap();
    let folder = tempfile_copy(path.parent().unwrap());
    let fixture_path = folder.join("fixture.json");
    let mut fixture: Value = serde_json::from_str(&fs::read_to_string(&fixture_path).unwrap()).unwrap();
    let steps = fixture["steps"].as_array_mut().unwrap();
    let step = steps.iter_mut().find(|s| s["expect"].is_string()).unwrap();
    let expect = step["expect"].as_str().unwrap().replacen(' ', "_", 1);
    step["expect"] = Value::String(expect);
    fs::write(&fixture_path, serde_json::to_string(&fixture).unwrap()).unwrap();
    assert!(replay(&fixture_path).unwrap_err().contains("different body"));
    fs::remove_dir_all(folder).unwrap();
}

fn tempfile_copy(folder: &Path) -> PathBuf {
    let target = std::env::temp_dir().join(format!("adp-conformance-{}", std::process::id()));
    fs::create_dir_all(&target).unwrap();
    for entry in fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            fs::copy(&path, target.join(path.file_name().unwrap())).unwrap();
        }
    }
    target
}
