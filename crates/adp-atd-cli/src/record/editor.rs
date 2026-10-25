//! `atd record editor/<name> --cases <dir>` (etalii.adp spec 035, T066): the conformance cases of the five editors, one
//! per document of the editor's examples, the folder the conformance tests round-trip
//! (`crates/adp-conformance/tests/round_trip/editors/<name>/examples/`, the standalone host's examples). An editor has
//! no parity script and no recording painter: a case is the document opened as the shell opens it (`adp-documents`'
//! `TextFile`), with what the editor's module reads of it (the Markdown editor's headings) and the findings the shell
//! shows for it, each at its 1-based line and column in characters. Every file of the examples folder is an input, so
//! that a schema a document names is there to be read.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{Map, Value as Json, json};

use super::{REFERENCE, collect_files, recorded_with, write_json};

/// The editors, by origin: the folder of their examples and the extensions the shell opens with them. The plain text
/// editor takes what no other editor takes.
const EDITORS: &[(&str, &str, &[&str])] = &[
    ("editor/plain", "plain", &[]),
    ("editor/markdown", "markdown", &["md", "markdown"]),
    ("editor/json", "json", &["json"]),
    ("editor/xml", "xml", &["xml", "xsd"]),
    ("editor/yaml", "yaml", &["yaml", "yml"]),
];

/// Whether `origin` is an editor's.
pub fn is_editor(origin: &str) -> bool {
    EDITORS.iter().any(|(o, _, _)| *o == origin)
}

/// The editor whose extensions take `extension`, as the shell chooses one (`crates/adp-shell/src/documents.rs`).
fn editor_of(extension: &str) -> &'static str {
    EDITORS
        .iter()
        .find(|(_, _, extensions)| extensions.contains(&extension))
        .map_or("editor/plain", |(origin, _, _)| origin)
}

/// The examples of the editors, the documents their conformance tests round-trip.
pub fn examples() -> PathBuf {
    adp_parity::repository_root().join("crates/adp-conformance/tests/round_trip/editors")
}

/// Records one case per document of `origin`'s examples into `cases`; the folders written.
pub fn record(origin: &str, cases: &Path, description: &str) -> Result<Vec<PathBuf>, String> {
    let (_, folder, _) = EDITORS
        .iter()
        .find(|(o, _, _)| *o == origin)
        .ok_or_else(|| format!("`{origin}` is not an editor"))?;
    let source = examples().join(folder).join("examples");
    let mut files = Vec::new();
    collect_files(&source, &source, &mut files);
    files.sort();
    let documents: Vec<&String> = files
        .iter()
        .filter(|f| !f.contains('/') && editor_of(&extension_of(f)) == origin)
        .collect();
    if documents.is_empty() {
        return Err(format!("{}: no document of `{origin}`", source.display()));
    }
    let mut written = Vec::new();
    for document in documents {
        let into = cases.join(document.replace('.', "-"));
        record_case(origin, &source, document, &files, &into, description)?;
        written.push(into);
    }
    Ok(written)
}

fn extension_of(file: &str) -> String {
    Path::new(file)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// Records `document` of `source` as the case in `folder`, which is made anew with every file of `source`.
fn record_case(
    origin: &str,
    source: &Path,
    document: &str,
    files: &[String],
    folder: &Path,
    description: &str,
) -> Result<(), String> {
    if folder.exists() {
        fs::remove_dir_all(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    }
    adp_parity::copy_folder(source, folder)?;
    let mut inputs = Map::new();
    inputs.insert("body".into(), json!(document));
    for other in files.iter().filter(|f| *f != document) {
        inputs.insert(other.clone(), json!(other));
    }

    let file = adp_documents::TextFile::open(folder.join(document))?;
    let text = file.content().to_owned();
    let (elements, findings) = read(origin, &text, folder, document, file.encoding_name());

    let desktop = |part: &str| {
        json!({
            "host": "desktop", "repository": "etalii.adp.ide.desktop", "commit": REFERENCE,
            "recordedWith": recorded_with(), "source": format!("atd record ({part})"),
        })
    };
    let fixture = json!({
        "$schema": "https://etalii.net/adp/atd/schema/0.1/atd.schema.json#/$defs/ConformanceFixture",
        "doc": format!(
            "Recorded by `atd record {origin}` from crates/adp-conformance/tests/round_trip/editors/{}/{document} of etalii.adp.ide.desktop, the standalone host's example; never edited by hand.",
            source.strip_prefix(examples()).unwrap_or(source).to_string_lossy().replace('\\', "/")
        ),
        "tool": origin,
        "description": description,
        "inputs": inputs,
        "read": {"elements": elements, "relations": [], "findings": findings},
        "provenance": {"reading": desktop("reading"), "findings": desktop("findings")},
    });
    write_json(&folder.join("conformance.json"), &fixture)
}

/// What the editor reads of `text` and the findings the shell shows for it, as `crates/adp-shell/src/documents.rs`
/// runs the editors' analyses: the project is the case's folder and the document's path in it its name.
fn read(origin: &str, text: &str, folder: &Path, document: &str, stored: &str) -> (Vec<Json>, Vec<Json>) {
    let root = folder.to_path_buf();
    let reader: adp_editor_json::ReadFile =
        Arc::new(move |path: &[String]| fs::read_to_string(root.join(path.join("/"))).ok());
    let path = vec![document.to_owned()];
    let found: Vec<(std::ops::Range<usize>, &str, String)> = match origin {
        "editor/markdown" => {
            let elements = adp_editor_markdown::headings_of(text)
                .into_iter()
                .map(|h| {
                    json!({
                        "id": format!("heading-{}", h.line), "type": "Heading",
                        "attributes": {"level": h.level, "title": h.title, "line": h.line},
                    })
                })
                .collect();
            return (elements, Vec::new());
        }
        "editor/json" => adp_editor_json::analyse(text, &path, reader)
            .findings
            .into_iter()
            .map(|f| {
                use adp_editor_json::Severity::*;
                (f.span, severity(f.severity == Error, f.severity == Warning), f.message)
            })
            .collect(),
        "editor/xml" => adp_editor_xml::analyse(text, &path, stored, reader)
            .findings
            .into_iter()
            .map(|f| {
                use adp_editor_xml::Severity::*;
                (f.span, severity(f.severity == Error, f.severity == Warning), f.message)
            })
            .collect(),
        "editor/yaml" => adp_editor_yaml::analyse(text, &path, reader)
            .findings
            .into_iter()
            .map(|f| {
                use adp_editor_yaml::Severity::*;
                (f.span, severity(f.severity == Error, f.severity == Warning), f.message)
            })
            .collect(),
        _ => Vec::new(),
    };
    let findings = found
        .into_iter()
        .map(|(span, severity, message)| {
            let (line, column) = position(text, span.start);
            json!({"severity": severity, "line": line, "column": column, "message": message})
        })
        .collect();
    (Vec::new(), findings)
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

/// The 1-based line and column, in characters, of byte `offset` of `text` (ATD §3.1).
fn position(text: &str, offset: usize) -> (usize, usize) {
    let before = text.get(..offset.min(text.len())).unwrap_or(text);
    let line = before.matches('\n').count() + 1;
    let start = before.rfind('\n').map_or(0, |at| at + 1);
    (line, before[start..].chars().count() + 1)
}
