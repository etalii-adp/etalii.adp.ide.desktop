//! The SPARQL query diagram's residue (etalii.adp spec 030, FR-018, FR-031 and FR-032, task T084): the tool type
//! `w3c/sparql`, read-only, after the standalone host's `src/diagrams/sparql/backend/EtAlii.Adp.Diagram.Sparql` at
//! `8b6ccfc`.
//!
//! A `.rq` file is a question, not a serialization of a graph, so this module shares no engine with the RDF family:
//! its own parser, its own model, its own projection, all in Rust. The query is never written; the one edit is a
//! position, written into the registration's `layout:` block. The standalone host read the query with its module's
//! own code, not through a binding, so [`SparqlModule`] [opens its documents
//! itself](adp_diagram::DiagramModule::opens_documents) and each open view is a [`view::SparqlView`].
//!
//! - [`terminals`], [`tokenizer`], [`parser`] and [`model`]: the query as read.
//! - [`projection`], [`layout`] and [`mapper`]: what is drawn, where, and what the shell is handed.
//! - [`findings`] and [`rows`]: the problems, the selection's words and the read-only rows.
//! - [`registration_layout`] and [`commands`]: the stored positions and the command that stores one.
//! - [`store`] and [`view`]: the open queries, and one open view.

pub mod commands;
pub mod findings;
pub mod layout;
pub mod mapper;
pub mod model;
pub mod parser;
pub mod projection;
pub mod registration_layout;
pub mod rows;
pub mod store;
pub mod terminals;
pub mod tokenizer;
pub mod view;

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;

use adp_diagram::workspace::{CANNOT_OPEN, OpenedDiagram};
use adp_diagram::{DiagramDocuments, DiagramModule, DiagramTools, ModuleView};

use crate::view::SparqlView;

/// The tool type this module serves.
pub const ORIGIN: &str = "w3c/sparql";

/// The extension a registration without a `body:` header derives its body by.
pub const DOCUMENT_EXTENSION: &str = ".rq";

/// The query a new SPARQL diagram starts as (`ServiceCollectionAddSparqlExtension.EmptyQuery`): a `SELECT` that
/// parses and validates clean, in CRLF.
pub const EMPTY_DOCUMENT: &str = "PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>\r\n\r\nSELECT ?subject ?label\r\nWHERE {\r\n  ?subject rdfs:label ?label .\r\n}\r\nLIMIT 100\r\n";

/// How many lines after the first a registration's headers are looked for in (`DiagramFilePair.HeaderScanLimit`).
const HEADER_SCAN_LIMIT: usize = 16;

/// The refusal of a `body:` header naming a file outside the project.
pub const OUTSIDE: &str =
    "Its body: header names a document outside the project, which is refused rather than followed.";

/// The SPARQL query module.
#[derive(Debug, Clone, Copy, Default)]
pub struct SparqlModule;

/// The SPARQL query module, as a tool type's module.
pub fn module() -> Rc<dyn DiagramModule> {
    Rc::new(SparqlModule)
}

/// `tools` with the SPARQL query module added to its tool type.
pub fn register(tools: DiagramTools) -> DiagramTools {
    tools.with_module(ORIGIN, module())
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The `body:` header of a registration's text, read from the lines after its first, blank lines skipped, up to the
/// first line that is neither a header nor blank (`DiagramFilePair.ReadBodyAndViewHeaders`).
fn body_header(text: &str) -> Option<String> {
    let mut body = None;
    for line in text
        .trim_start_matches('\u{feff}')
        .lines()
        .skip(1)
        .take(HEADER_SCAN_LIMIT)
    {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let starts = |header: &str| {
            trimmed
                .get(..header.len())
                .is_some_and(|h| h.eq_ignore_ascii_case(header))
        };
        if starts("body:") {
            let value = trimmed["body:".len()..].trim();
            body = (!value.is_empty()).then(|| value.to_owned());
        } else if !starts("view:") {
            break;
        }
    }
    body
}

/// `path` with `.` and `..` resolved without touching the file system.
fn normalise(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// `relative` resolved against `folder`, or none when it is rooted or leaves `root` (`DiagramFilePair.ResolveWithin`).
fn resolve_within(root: &Path, folder: &Path, relative: &str) -> Option<PathBuf> {
    if Path::new(relative).has_root() || Path::new(relative).is_absolute() {
        return None;
    }
    let full = normalise(&folder.join(relative));
    full.starts_with(normalise(root)).then_some(full)
}

/// The query the registration at `registration` names: its `body:` header, else its sibling `.rq`.
fn route(root: &Path, registration: &Path) -> Result<PathBuf, String> {
    let text = fs::read(registration)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .map_err(|_| "The registration file could not be read.".to_owned())?;
    let folder = registration.parent().unwrap_or(root);
    match body_header(&text) {
        Some(body) => resolve_within(root, folder, &body).ok_or_else(|| OUTSIDE.to_owned()),
        None => {
            let stem = registration
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .filter(|s| !s.is_empty() && s != ".adp")
                .ok_or_else(|| CANNOT_OPEN.to_owned())?;
            Ok(folder.join(format!("{stem}{DOCUMENT_EXTENSION}")))
        }
    }
}

impl DiagramModule for SparqlModule {
    fn opens_documents(&self) -> bool {
        true
    }

    /// Opens the registration at `path` against the query it names (`AddDiagramSession`); a query opened on its own
    /// has no registration to store a position in.
    fn open(
        &self,
        root: &Path,
        path: &Path,
        origin: &str,
        documents: &mut DiagramDocuments,
    ) -> Result<(OpenedDiagram, Rc<dyn ModuleView>), String> {
        let is_registration = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("adp"));
        let (query, registration) = if is_registration {
            (route(root, path)?, Some(path.to_path_buf()))
        } else {
            (path.to_path_buf(), None)
        };
        if !query.is_file() {
            return Err(format!(
                "The body of '{}' is missing.",
                file_name(registration.as_deref().unwrap_or(&query))
            ));
        }
        store::open(documents, &query, registration.as_deref());
        let view: Rc<dyn ModuleView> = Rc::new(SparqlView::new(&query, registration.as_deref()));
        Ok((
            OpenedDiagram {
                origin: origin.to_owned(),
                body_path: query,
                registration_path: registration,
            },
            view,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_body_header_is_read_after_the_first_line_and_blank_lines() {
        assert_eq!(
            body_header("w3c/sparql\r\n\r\nBODY: q.rq \r\n").as_deref(),
            Some("q.rq")
        );
        assert_eq!(body_header("w3c/sparql\r\nlayout:\r\nbody: q.rq\r\n"), None);
        assert_eq!(body_header("w3c/sparql\n"), None);
    }

    #[test]
    fn the_starter_query_parses_and_reports_nothing() {
        let model = parser::parse(EMPTY_DOCUMENT).expect("the starter query parses");
        assert_eq!(model.projection.len(), 2);
        assert!(findings::judge(&model, EMPTY_DOCUMENT).is_empty());
    }
}
