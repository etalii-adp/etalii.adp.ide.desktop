//! The desktop host's editors' checks, handed to the runtime as check sources (ATD §10.2; etalii.adp spec 083, research
//! R1): each editor module keeps its analysis, which this crate calls as the shell does (`crates/adp-shell/src/
//! documents.rs`), one source per editor covering the names its description's `check with` gives. The XML editor's
//! checks rest on libxml2, which is not built for WebAssembly, so a WebAssembly host has no XML source.

use std::rc::Rc;
use std::sync::Arc;

use adp_atd_runtime::Capabilities;
use adp_atd_runtime::capabilities::{CheckFinding, CheckSource, ProjectFiles};

/// `capabilities` with every editor check source this build has.
pub fn contribute(capabilities: Capabilities) -> Capabilities {
    let capabilities = capabilities
        .with_check_source(Rc::new(Json))
        .with_check_source(Rc::new(Yaml));
    #[cfg(not(target_arch = "wasm32"))]
    let capabilities = capabilities.with_check_source(Rc::new(Xml));
    capabilities
}

/// The files of the project, as the editors read a schema a document names: by its path segments.
fn reader(files: ProjectFiles<'_>) -> adp_editor_json::ReadFile {
    let held = Arc::new(files.clone());
    Arc::new(move |path: &[String]| held.get(&path.join("/")).cloned())
}

fn severity(error: bool, warning: bool) -> &'static str {
    if error {
        "error"
    } else if warning {
        "warning"
    } else {
        "information"
    }
}

/// The JSON editor's checks: JSON itself, and the schema its `$schema` names.
struct Json;

impl CheckSource for Json {
    fn names(&self) -> &'static [&'static str] {
        &["json", "schema"]
    }

    fn check(&self, text: &str, document: &str, _encoding: &str, files: ProjectFiles<'_>) -> Vec<CheckFinding> {
        adp_editor_json::analyse(text, &[document.to_owned()], reader(files))
            .findings
            .into_iter()
            .map(|f| {
                use adp_editor_json::Severity::*;
                CheckFinding {
                    start: f.span.start,
                    severity: severity(f.severity == Error, f.severity == Warning),
                    message: f.message,
                }
            })
            .collect()
    }
}

/// The YAML editor's checks: YAML itself, and the schema a document's first comment names.
struct Yaml;

impl CheckSource for Yaml {
    fn names(&self) -> &'static [&'static str] {
        &["yaml", "schema"]
    }

    fn check(&self, text: &str, document: &str, _encoding: &str, files: ProjectFiles<'_>) -> Vec<CheckFinding> {
        adp_editor_yaml::analyse(text, &[document.to_owned()], reader(files))
            .findings
            .into_iter()
            .map(|f| {
                use adp_editor_yaml::Severity::*;
                CheckFinding {
                    start: f.span.start,
                    severity: severity(f.severity == Error, f.severity == Warning),
                    message: f.message,
                }
            })
            .collect()
    }
}

/// The XML editor's checks: well-formedness, the encoding, and the XSD or DTD a document names.
#[cfg(not(target_arch = "wasm32"))]
struct Xml;

#[cfg(not(target_arch = "wasm32"))]
impl CheckSource for Xml {
    fn names(&self) -> &'static [&'static str] {
        &["xml", "encoding", "xsd", "dtd"]
    }

    fn check(&self, text: &str, document: &str, encoding: &str, files: ProjectFiles<'_>) -> Vec<CheckFinding> {
        adp_editor_xml::analyse(text, &[document.to_owned()], encoding, reader(files))
            .findings
            .into_iter()
            .map(|f| {
                use adp_editor_xml::Severity::*;
                CheckFinding {
                    start: f.span.start,
                    severity: severity(f.severity == Error, f.severity == Warning),
                    message: f.message,
                }
            })
            .collect()
    }
}
