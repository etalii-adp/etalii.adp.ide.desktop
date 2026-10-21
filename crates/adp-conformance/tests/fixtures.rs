//! Every FBL fixture vendored from etalii.adp, replayed (etalii.adp spec 030, FR-034, SC-003, tasks T038, T039).
//!
//! This first level checks what the splice catalogue alone decides, as etalii.adp's own validator does (FBL section
//! 15.3): each step's splices turn the body before it into its expected body and back by their inverses; an undo
//! gives back the body the undone edit started from and a redo the body it left; a refused edit and a save change
//! nothing. Reading the input and planning each edit's splices from its binding follow with the families (T040 to
//! T044), and then the same fixtures are replayed from the edits rather than from their listed splices.

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

fn replay(path: &Path) -> Result<usize, String> {
    let folder = path.parent().unwrap();
    let fixture: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).map_err(|e| e.to_string())?;
    let mut body = fs::read(folder.join(fixture["input"].as_str().unwrap())).map_err(|e| e.to_string())?;
    let outside = blocks_outside_history(folder, fixture["binding"].as_str().unwrap_or_default());
    // (before, after, changes, crossed) of each edit that can be undone, and of each that can be redone.
    let mut done: Vec<(Vec<u8>, Vec<u8>, Changes, bool)> = Vec::new();
    let mut undone: Vec<(Vec<u8>, Vec<u8>, Changes, bool)> = Vec::new();
    let steps = fixture["steps"].as_array().cloned().unwrap_or_default();
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
