//! A description validated against its cases, as `atd validate` and the language server's `atd/validate` run it
//! (etalii.adp spec 035, T128, contracts/language-server.md): the cases found beside the description or handed in,
//! each replayed through the description's runtime.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use adp_atd_check::{Checked, Severity};
use adp_atd_runtime::{RuntimeHost, diagram_tool};
use serde_json::{Value as Json, json};

use crate::{At, Case, Difference, validate};

/// One case's result: its name and its differences, none when it passed.
#[derive(Debug, Clone)]
pub struct CaseResult {
    pub name: String,
    pub differences: Vec<Difference>,
}

impl CaseResult {
    pub fn passed(&self) -> bool {
        self.differences.is_empty()
    }

    /// The result as `atd/validate` gives it, each construct placed by `at`.
    pub fn json(&self, at: &dyn Fn(&At) -> Json) -> Json {
        json!({
            "name": self.name,
            "passed": self.passed(),
            "differences": self.differences.iter().map(|d| {
                json!({
                    "step": d.step,
                    "part": d.part,
                    "expected": d.expected,
                    "actual": d.actual,
                    "at": d.at.as_ref().map_or(Json::Null, at),
                })
            }).collect::<Vec<_>>(),
        })
    }
}

/// Replays each of `cases` through the runtime of the checked description, on `host`. A description that has errors,
/// or that is not a diagram's, has no runtime to validate, and the reason is given instead.
pub fn validate_description(checked: &Checked, host: &RuntimeHost, cases: &[Case]) -> Result<Vec<CaseResult>, String> {
    let errors = checked
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count();
    if errors > 0 {
        return Err(format!(
            "The description has {errors} error(s); a description is validated once it checks without errors."
        ));
    }
    let symbols = checked.symbols.as_ref().ok_or("The description was not checked.")?;
    match checked.description.kind() {
        Some("diagram") => {}
        Some(kind) => return Err(format!("A {kind}'s description has no runtime yet; diagrams' have.")),
        None => return Err("The file is not part of a tool description.".to_owned()),
    }
    let runtime = diagram_tool(&checked.description, symbols, host)?;
    Ok(cases
        .iter()
        .map(|case| CaseResult {
            name: case.name.clone(),
            differences: validate(&runtime, &checked.description, case),
        })
        .collect())
}

/// The cases of the description whose root is `root` (ATD §2.3): each `conformance/<type>/<case>/` folder beside it
/// whose fixture names `root` as its description, in the order of their paths.
pub fn cases_of(root: &Path) -> Vec<PathBuf> {
    let Some(folder) = root.parent() else {
        return Vec::new();
    };
    let Ok(root) = root.canonicalize() else {
        return Vec::new();
    };
    let mut cases = Vec::new();
    for tool_type in entries(&folder.join("conformance")) {
        for case in entries(&tool_type) {
            let Ok(text) = std::fs::read_to_string(case.join("conformance.json")) else {
                continue;
            };
            let Ok(fixture) = serde_json::from_str::<Json>(&text) else {
                continue;
            };
            let described = fixture["description"]
                .as_str()
                .and_then(|d| case.join(d).canonicalize().ok());
            if described.as_deref() == Some(root.as_path()) {
                cases.push(case);
            }
        }
    }
    cases
}

/// The folders in `folder`, sorted.
fn entries(folder: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    found.sort();
    found
}

/// The case in folder `dir`, its fixture and its files read into memory.
pub fn read_case(dir: &Path) -> Result<Case, String> {
    let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut files = BTreeMap::new();
    for entry in std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .flatten()
    {
        let path = entry.path();
        if path.is_file() {
            let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            files.insert(entry.file_name().to_string_lossy().into_owned(), bytes);
        }
    }
    case_of(name, files)
}

/// The case `name` of `files`, by their names in the case: its fixture is `conformance.json`.
pub fn case_of(name: String, files: BTreeMap<String, Vec<u8>>) -> Result<Case, String> {
    let fixture = files
        .get("conformance.json")
        .ok_or_else(|| format!("The case '{name}' has no conformance.json."))?;
    let fixture = serde_json::from_slice(fixture).map_err(|e| format!("The case '{name}': conformance.json: {e}"))?;
    Ok(Case { name, fixture, files })
}
