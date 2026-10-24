//! `atd record --check` (etalii.adp spec 035, T163, research R14, R17): every vendored conformance case and capability
//! fixture recorded again into a temporary folder and compared byte for byte with the vendored files, so that code
//! moved into a capability (not rewritten) keeps recording what the reference recorded. A case is recorded with the
//! `recordedWith` it was recorded with, so only what the code produces can differ.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value as Json;

use super::{collect_files, override_recorded_with};

/// The differences between what records now and what is vendored under `vendor` (etalii.adp's copy), one line each;
/// empty when there are none.
pub fn check(vendor: &Path) -> Result<Vec<String>, String> {
    let scratch = scratch()?;
    let result = differences(vendor, &scratch);
    let _ = fs::remove_dir_all(&scratch);
    result
}

fn differences(vendor: &Path, scratch: &Path) -> Result<Vec<String>, String> {
    let mut differences = Vec::new();

    // The cases, one folder of cases per tool type and description.
    for (folder, (origin, description, recorded_with)) in case_folders(vendor)? {
        let into = scratch
            .join("cases")
            .join(folder.strip_prefix(vendor).unwrap_or(&folder));
        override_recorded_with(Some(recorded_with));
        let recorded = super::record(&origin, &into, Some(&description));
        override_recorded_with(None);
        recorded?;
        compare(&folder, &into, vendor, &mut differences);
    }

    // The capabilities' fixtures.
    let catalogue = vendor.join("specifications/atd/capabilities");
    for entry in fs::read_dir(&catalogue)
        .map_err(|e| format!("{}: {e}", catalogue.display()))?
        .flatten()
    {
        let fixtures = entry.path().join("fixtures");
        if !fixtures.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let into = scratch.join("capabilities").join(&name);
        super::capability::record(&name, &into)?;
        compare(&fixtures, &into, vendor, &mut differences);
    }
    Ok(differences)
}

/// Every folder of vendored cases, with the tool type its cases record, their description and the commit they were
/// recorded with.
fn case_folders(vendor: &Path) -> Result<BTreeMap<PathBuf, (String, String, String)>, String> {
    let mut folders = BTreeMap::new();
    let definitions = vendor.join("definitions");
    for kind in fs::read_dir(&definitions)
        .map_err(|e| format!("{}: {e}", definitions.display()))?
        .flatten()
    {
        let Ok(tools) = fs::read_dir(kind.path().join("conformance")) else {
            continue;
        };
        for tool in tools.flatten().filter(|t| t.path().is_dir()) {
            let Ok(cases) = fs::read_dir(tool.path()) else { continue };
            for case in cases.flatten() {
                let file = case.path().join("conformance.json");
                let Ok(text) = fs::read_to_string(&file) else { continue };
                let fixture: Json = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", file.display()))?;
                let origin = fixture["tool"].as_str().unwrap_or_default().to_owned();
                let description = fixture["description"].as_str().unwrap_or_default().to_owned();
                let recorded_with = recorded_with_of(&fixture).unwrap_or_default();
                folders
                    .entry(tool.path())
                    .or_insert((origin, description, recorded_with));
            }
        }
    }
    Ok(folders)
}

/// The `recordedWith` of a case's first provenance entry.
fn recorded_with_of(fixture: &Json) -> Option<String> {
    let mut found = None;
    let mut stack = vec![fixture];
    while let Some(value) = stack.pop() {
        match value {
            Json::Object(map) => {
                if let Some(commit) = map.get("recordedWith").and_then(Json::as_str) {
                    found = Some(commit.to_owned());
                    break;
                }
                stack.extend(map.values().rev());
            }
            Json::Array(items) => stack.extend(items.iter().rev()),
            _ => {}
        }
    }
    found
}

/// Every file that is in one folder and not the other, or differs between them.
fn compare(vendored: &Path, recorded: &Path, vendor: &Path, differences: &mut Vec<String>) {
    let files = |root: &Path| -> BTreeSet<String> {
        let mut out = Vec::new();
        collect_files(root, root, &mut out);
        out.into_iter().collect()
    };
    let (theirs, ours) = (files(vendored), files(recorded));
    let shown = |file: &str| {
        let path = vendored.join(file);
        path.strip_prefix(vendor)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/")
    };
    for file in theirs.union(&ours) {
        match (theirs.contains(file), ours.contains(file)) {
            (true, false) => differences.push(format!("{}: vendored, no longer recorded", shown(file))),
            (false, true) => differences.push(format!("{}: recorded, not vendored", shown(file))),
            _ => {
                if fs::read(vendored.join(file)).ok() != fs::read(recorded.join(file)).ok() {
                    differences.push(format!("{}: recorded differently", shown(file)));
                }
            }
        }
    }
}

fn scratch() -> Result<PathBuf, String> {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    let folder = std::env::temp_dir().join(format!("atd-record-check-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    Ok(folder)
}
