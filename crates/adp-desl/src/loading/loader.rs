//! Loads a DESL specification as DESL section 10.1 says, in its order, the way `adp-disl` loads DISL: **1** parse,
//! rejecting a byte-order mark and duplicate keys; **2** check the version, refusing a later major and warning about
//! a later minor; **3** validate against the vendored `desl.schema.json`; **4** resolve names - the metamodel's, the
//! surface's, and, when the specification's folder is known, its bindings' types; and **5** the checks the schema
//! cannot make that the runtime rests on (reserved attribute names, the namespace of types and enumerations, id
//! rules).
//!
//! Not done here, and not claimed: step 6, compiling CEL. The standalone host interprets no DESL expression either -
//! its Knowledge module carries its rules in code - so constraints, operations, functions and derived ids are kept as
//! written in [`DeslSpecification::root`] and checked by nothing but the schema.

use std::path::Path;

use super::diagnostic::{DeslDiagnostic, DeslSeverity};
use super::reading;
use super::specification::{DeslPersistence, DeslSpecification};
use crate::json::{self, Json};
use crate::schema;

/// The DESL major version this runtime reads, and the highest minor it knows (DESL section 2.7).
pub const MAJOR: i64 = 0;
pub const MINOR: i64 = 2;

/// What loading a specification gives: the specification, when it has no errors, and every diagnostic.
#[derive(Debug, Clone)]
pub struct DeslLoadResult {
    pub specification: Option<DeslSpecification>,
    pub diagnostics: Vec<DeslDiagnostic>,
}

impl DeslLoadResult {
    pub fn errors(&self) -> impl Iterator<Item = &DeslDiagnostic> {
        self.diagnostics.iter().filter(|d| d.severity == DeslSeverity::Error)
    }
}

/// The loader of DESL specifications.
pub struct DeslLoader;

fn has_errors(diagnostics: &[DeslDiagnostic]) -> bool {
    diagnostics.iter().any(|d| d.severity == DeslSeverity::Error)
}

impl DeslLoader {
    /// Loads the specification `json`, without resolving its bindings: it has no location to resolve them against.
    pub fn load(json: &str) -> DeslLoadResult {
        DeslLoader::load_bytes(json.as_bytes(), None)
    }

    /// Loads the specification at `path`, its bindings resolved against its folder (DESL section 5).
    pub fn load_file(path: &Path) -> DeslLoadResult {
        match std::fs::read(path) {
            Ok(bytes) => DeslLoader::load_bytes(&bytes, Some(path)),
            Err(e) => DeslLoadResult {
                specification: None,
                diagnostics: vec![DeslDiagnostic::error(
                    "",
                    format!("The specification '{}' cannot be read: {e}", path.display()),
                )],
            },
        }
    }

    /// Loads a specification from its bytes; with `location`, the path it was read from, its bindings are resolved.
    pub fn load_bytes(utf8: &[u8], location: Option<&Path>) -> DeslLoadResult {
        let mut diagnostics = Vec::new();
        let refused = |diagnostics| DeslLoadResult {
            specification: None,
            diagnostics,
        };

        // Step 1: parse, rejecting a byte-order mark and duplicate keys (section 2.1).
        if utf8.starts_with(&[0xEF, 0xBB, 0xBF]) {
            diagnostics.push(DeslDiagnostic::error(
                "",
                "A specification is UTF-8 without a byte-order mark (DESL §2.1).",
            ));
            return refused(diagnostics);
        }
        let (root, duplicates) = match json::parse(utf8) {
            Ok(parsed) => parsed,
            Err(e) => {
                diagnostics.push(DeslDiagnostic::error("", format!("The specification is not JSON: {e}")));
                return refused(diagnostics);
            }
        };
        if !root.is_object() {
            diagnostics.push(DeslDiagnostic::error(
                "",
                "A specification is a JSON object (DESL §2.1).",
            ));
            return refused(diagnostics);
        }
        for (pointer, key) in duplicates {
            diagnostics.push(DeslDiagnostic::error(
                pointer,
                format!("The key '{key}' appears more than once in one object (DESL §2.1)."),
            ));
        }

        // Step 2: the version (section 2.7). A later major is refused before anything else is read of it.
        let Some(newer_minor) = version(&root, &mut diagnostics) else {
            return refused(diagnostics);
        };

        // Step 3: the schema. A specification of a newer minor is checked as the newest this runtime knows, so what
        // its version adds is ignored rather than refused (section 2.7).
        let mut checked = root.clone();
        if newer_minor && let Some(object) = checked.as_object_mut() {
            object.insert("desl".into(), Json::String(format!("{MAJOR}.{MINOR}")));
        }
        for (pointer, message) in schema::check_specification(&checked) {
            diagnostics.push(DeslDiagnostic::error(pointer, format!("{message} (desl.schema.json)")));
        }
        if has_errors(&diagnostics) {
            return refused(diagnostics);
        }

        // Steps 4 and 5: the model read, and every name it uses resolved.
        let language = reading::language(&root);
        let metamodel = reading::metamodel(&root, &mut diagnostics);
        let persistence = reading::persistence(&root, &metamodel, &mut diagnostics);
        if persistence.format != "fbl" {
            diagnostics.push(DeslDiagnostic::error(
                "/persistence/format",
                format!(
                    "Documents are stored in '{}'; DESL 0.2 knows 'fbl' only (DESL §5).",
                    persistence.format
                ),
            ));
        }
        if let Some(location) = location {
            bindings(&persistence, &metamodel_types(&metamodel), location, &mut diagnostics);
        }
        let surface = reading::surface(&root, &metamodel, &mut diagnostics);
        if has_errors(&diagnostics) {
            return refused(diagnostics);
        }
        let Some(surface) = surface else {
            return refused(diagnostics);
        };
        DeslLoadResult {
            specification: Some(DeslSpecification::new(root, language, metamodel, persistence, surface)),
            diagnostics,
        }
    }
}

fn metamodel_types(metamodel: &super::specification::DeslMetamodel) -> Vec<String> {
    metamodel.types.keys().cloned().collect()
}

/// Every binding exists, and every rule's type in it names an element type or a key of the type map (step 4).
fn bindings(persistence: &DeslPersistence, types: &[String], location: &Path, diagnostics: &mut Vec<DeslDiagnostic>) {
    for (index, reference) in persistence.bindings.iter().enumerate() {
        let pointer = json::pointer_at("/persistence/bindings", index);
        match adp_fbl::documents::resolve_reference(reference, location) {
            // A binding this FBL runtime cannot read - one of a later FBL than it knows - is the runtime's limit,
            // not the specification's mistake: the specification is read, and its documents do not open here.
            Err(problem) if declares(reference, location) => diagnostics.push(DeslDiagnostic::warning(
                pointer,
                format!(
                    "This runtime cannot read the binding '{reference}', so its documents do not open here: {problem}"
                ),
            )),
            Err(problem) => diagnostics.push(DeslDiagnostic::error(pointer, problem)),
            Ok((binding, _)) => {
                for rule in binding.all_rules() {
                    if !types.contains(&rule.type_name) && !persistence.type_map.contains_key(&rule.type_name) {
                        diagnostics.push(DeslDiagnostic::error(
                            pointer.clone(),
                            format!(
                                "The rule '{}' of '{reference}' produces '{}', which is no element type and no key of the type map (DESL §5.1).",
                                rule.name, rule.type_name
                            ),
                        ));
                    }
                }
            }
        }
    }
}

/// Whether the FBL document a reference names is there and declares a binding of that name, read as JSON only.
fn declares(reference: &str, location: &Path) -> bool {
    let Some((file, name)) = reference.rsplit_once('#') else {
        return false;
    };
    let path = if file.is_empty() {
        location.to_path_buf()
    } else {
        location.parent().unwrap_or(Path::new(".")).join(file)
    };
    std::fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Json>(&bytes).ok())
        .is_some_and(|document| json::get(json::member(&document, "bindings"), name).is_some())
}

/// The `desl` version (section 2.7): none when it refuses the specification, else whether its minor is newer than
/// this runtime knows, which is warned about.
fn version(root: &Json, diagnostics: &mut Vec<DeslDiagnostic>) -> Option<bool> {
    let Some(version) = json::string_of(root, "desl") else {
        diagnostics.push(DeslDiagnostic::error(
            "/desl",
            "A specification declares the DESL version it is written in (DESL §2.7).",
        ));
        return None;
    };
    let number = |p: &str| -> Option<i64> {
        if p.is_empty() || !p.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        p.parse().ok()
    };
    let parts: Vec<&str> = version.split('.').collect();
    let (Some(major), Some(minor), 2) = (
        parts.first().and_then(|p| number(p)),
        parts.get(1).and_then(|p| number(p)),
        parts.len(),
    ) else {
        diagnostics.push(DeslDiagnostic::error(
            "/desl",
            format!("'{version}' is not a DESL version written as major.minor (DESL §2.7)."),
        ));
        return None;
    };
    if major > MAJOR {
        diagnostics.push(DeslDiagnostic::error(
            "/desl",
            format!("This runtime reads DESL {MAJOR}.x; DESL {version} is refused (DESL §2.7)."),
        ));
        return None;
    }
    let newer = major == MAJOR && minor > MINOR;
    if newer {
        diagnostics.push(DeslDiagnostic::warning(
            "/desl",
            format!("This runtime knows DESL up to {MAJOR}.{MINOR}; what DESL {version} adds is not read (DESL §2.7)."),
        ));
    }
    Some(newer)
}
