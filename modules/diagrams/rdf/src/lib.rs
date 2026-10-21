//! The RDF family's residue (etalii.adp spec 030, FR-018, FR-031 and FR-032, task T083): the tool types `w3c/rdf`,
//! `w3c/owl`, `w3c/skos` and `w3c/shacl` over `.ttl` and `.nt` documents, after the standalone host's
//! `src/diagrams/rdf/backend/EtAlii.Adp.Diagram.Rdf` at `8b6ccfc`.
//!
//! The standalone host read, drew and edited RDF with its module's own code: a Turtle and N-Triples parser that keeps
//! where every triple is stated, a writer that splices only the lines a change touches, one command per edit, and four
//! readings of the same bytes, each a diagram type of its own over one store (the data graph, the ontology, the
//! scheme and the shapes). So the family is routed through its module here too: [`RdfModule`]
//! [opens its documents itself](adp_diagram::DiagramModule::opens_documents), and each open view is a
//! [`view::RdfView`] of one [`reading::Reading`].
//!
//! `.ttl` and `.nt` belong to the data graph the way `.tml` belongs to the timeline, so a bare file of either opens as
//! one; the other three readings never claim a bare body (`SharedExtension: true`), and open a file only through a
//! registration that names them.
//!
//! - [`lines`], [`scanner`], [`iri`], [`parser`] and [`model`]: the file, read with where every triple is stated.
//! - [`writer`], [`commands`] and [`registration_layout`]: every edit, as a command on the project's history, and a
//!   dragged position, stored in the registration.
//! - [`reading`], [`graph`], [`owl`], [`skos`] and [`shacl`]: what each reading draws, where, and its findings.
//! - [`actions`], [`properties`], [`selection`] and [`family`]: the menus and rows, which are the family's whichever
//!   reading a file is opened under.
//! - [`store`]: one reading per body, shared by every view of it, kept in the project's documents.

pub mod actions;
pub mod commands;
pub mod family;
pub mod graph;
pub mod iri;
pub mod lines;
pub mod model;
pub mod owl;
pub mod parser;
pub mod payload;
pub mod projection;
pub mod properties;
pub mod reading;
pub mod registration_layout;
pub mod scanner;
pub mod selection;
pub mod shacl;
pub mod skos;
pub mod store;
pub mod toolbox;
pub mod validator;
pub mod view;
pub mod vocabulary;
pub mod writer;

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;

use adp_diagram::workspace::{CANNOT_OPEN, OpenedDiagram};
use adp_diagram::{DiagramDocuments, DiagramModule, DiagramTools, ModuleView};

use crate::view::RdfView;

/// Every reading of the family, the anchor first.
pub const ORIGINS: [&str; 4] = [graph::ORIGIN, owl::ORIGIN, skos::ORIGIN, shacl::ORIGIN];

/// How many lines after its first a registration's headers are looked for in.
const HEADER_SCAN_LIMIT: usize = 8;

/// What opening a body under a reading no registration names answers: only the data graph claims a bare file.
pub const NOT_REGISTERED: &str =
    "A .ttl or .nt file opens as an RDF graph; open it as another reading through a registration that names it.";

/// The family's module, serving every reading.
#[derive(Debug, Clone, Copy, Default)]
pub struct RdfModule;

/// `tools` with the family's module added to each of its tool types.
pub fn register(tools: DiagramTools) -> DiagramTools {
    ORIGINS
        .iter()
        .fold(tools, |tools, origin| tools.with_module(origin, Rc::new(RdfModule)))
}

/// The view of `body_path` under the reading `origin`.
pub fn view_of(origin: &str, root: &Path, body_path: &Path, registration: Option<&Path>) -> Rc<dyn ModuleView> {
    match origin {
        owl::ORIGIN => Rc::new(RdfView::new(owl::OwlReading, root, body_path, registration)),
        skos::ORIGIN => Rc::new(RdfView::new(
            skos::SkosReading::for_registration(registration),
            root,
            body_path,
            registration,
        )),
        shacl::ORIGIN => Rc::new(RdfView::new(shacl::ShaclReading, root, body_path, registration)),
        _ => Rc::new(RdfView::new(graph::RdfReading, root, body_path, registration)),
    }
}

impl DiagramModule for RdfModule {
    fn opens_documents(&self) -> bool {
        true
    }

    fn open(
        &self,
        root: &Path,
        path: &Path,
        origin: &str,
        documents: &mut DiagramDocuments,
    ) -> Result<(OpenedDiagram, Rc<dyn ModuleView>), String> {
        let mut origin = origin;
        let is_registration = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("adp"));
        let (body_path, registration) = if is_registration {
            (route_registration(root, path)?, Some(path.to_path_buf()))
        } else {
            let beside = path.with_extension("adp");
            let registered = beside.is_file() && fs::read_to_string(&beside).is_ok_and(|t| first_line(&t) == origin);
            if registered {
                (path.to_path_buf(), Some(beside))
            } else {
                // A bare body is the anchor's, whichever reading the workspace found first for its extension: the
                // other readings never claim a bare body (`SharedExtension: true`).
                origin = graph::ORIGIN;
                (path.to_path_buf(), None)
            }
        };
        if !body_path.is_file() {
            let named = registration.as_deref().unwrap_or(&body_path);
            return Err(format!("The body of '{}' is missing.", store::file_name(named)));
        }
        store::get_or_load(documents, &body_path);
        let view = view_of(origin, root, &body_path, registration.as_deref());
        Ok((
            OpenedDiagram {
                origin: origin.to_owned(),
                body_path,
                registration_path: registration,
            },
            view,
        ))
    }
}

/// The body a new data graph starts as: one prefix, one typed resource with a label, parsing and validating clean, in
/// CRLF (`ServiceCollectionAddRdfExtension.EmptyRdfDocument`).
pub fn empty_rdf_document(base_name: &str) -> String {
    let local = local_name(base_name);
    format!(
        "@prefix ex: <http://example.org/> .\r\n@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\r\n\r\nex:{local} a ex:Resource ;\r\n    rdfs:label \"{base_name}\" .\r\n"
    )
}

/// The body a new ontology starts as: an `owl:Ontology` header and one labeled class
/// (`ServiceCollectionAddRdfExtension.EmptyOwlDocument`).
pub fn empty_owl_document(base_name: &str) -> String {
    let local = local_name(base_name);
    format!(
        "@prefix : <http://example.org/{local}#> .\r\n@prefix owl: <http://www.w3.org/2002/07/owl#> .\r\n@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\r\n\r\n<http://example.org/{local}> a owl:Ontology ;\r\n    rdfs:label \"{base_name}\" .\r\n\r\n:Thing1 a owl:Class ;\r\n    rdfs:label \"First class\" .\r\n"
    )
}

/// The body a new scheme starts as: one scheme and one top concept with a preferred label
/// (`ServiceCollectionAddSkosExtension.EmptyDocument`).
pub fn empty_skos_document(base_name: &str) -> String {
    let local = local_name(base_name);
    format!(
        "@prefix skos: <http://www.w3.org/2004/02/skos/core#> .\r\n@prefix ex: <http://example.org/> .\r\n\r\nex:{local} a skos:ConceptScheme ;\r\n    skos:prefLabel \"{base_name}\"@en ;\r\n    skos:hasTopConcept ex:{local}_top .\r\n\r\nex:{local}_top a skos:Concept ;\r\n    skos:prefLabel \"Top concept\"@en ;\r\n    skos:topConceptOf ex:{local} .\r\n"
    )
}

/// The body a new shapes graph starts as: one node shape with a class target and one property row
/// (`ServiceCollectionAddShaclExtension.EmptyDocument`).
pub fn empty_shacl_document(base_name: &str) -> String {
    let local = local_name(base_name);
    format!(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .\r\n@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\r\n@prefix ex: <http://example.org/> .\r\n\r\nex:{local}Shape a sh:NodeShape ;\r\n    sh:targetClass ex:{local} ;\r\n    sh:name \"{base_name}\" ;\r\n    sh:property [\r\n        sh:path ex:name ;\r\n        sh:datatype xsd:string ;\r\n        sh:minCount 1 ;\r\n        sh:maxCount 1 ;\r\n    ] .\r\n"
    )
}

/// A base name as a Turtle local name (`RdfLocalName.Of`). STUB: ported with the family's remaining parts.
pub fn local_name(base_name: &str) -> String {
    base_name.to_owned()
}

fn first_line(text: &str) -> String {
    text.trim_start_matches('\u{feff}')
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_owned()
}

/// The body a registration opens (the standalone host's `DiagramFilePair.BodyOf`): its `body:` header, resolved
/// against its folder and refused outside the project, else the `.ttl` beside it named after it.
fn route_registration(root: &Path, registration: &Path) -> Result<PathBuf, String> {
    let text = fs::read_to_string(registration).map_err(|_| "The registration file could not be read.".to_owned())?;
    let folder = registration.parent().unwrap_or(root);
    match body_header(&text) {
        Some(body) => resolve_within(root, folder, &body).ok_or_else(|| {
            "Its body: header names a document outside the project, which is refused rather than followed.".to_owned()
        }),
        None => {
            let name = store::file_name(registration);
            let base = name
                .len()
                .checked_sub(4)
                .filter(|at| name[*at..].eq_ignore_ascii_case(".adp"))
                .map(|at| &name[..at])
                .filter(|base| !base.is_empty())
                .ok_or_else(|| CANNOT_OPEN.to_owned())?;
            Ok(folder.join(format!("{base}.ttl")))
        }
    }
}

/// The `body:` header of a registration's text, read from the lines after its first, blank lines skipped, up to the
/// first line that is no header.
fn body_header(text: &str) -> Option<String> {
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
            return (!value.is_empty()).then(|| value.to_owned());
        }
        if !starts("view:") {
            break;
        }
    }
    None
}

/// `relative` resolved against `base`, or none when it is rooted or lands outside `root`.
fn resolve_within(root: &Path, base: &Path, relative: &str) -> Option<PathBuf> {
    let candidate = Path::new(relative);
    if candidate.is_absolute() || relative.starts_with(['/', '\\']) || candidate.has_root() {
        return None;
    }
    let full = normalise(&base.join(candidate));
    full.starts_with(normalise(root)).then_some(full)
}

/// The path with `.` and `..` resolved without touching the file system.
fn normalise(path: &Path) -> PathBuf {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut out = PathBuf::new();
    for component in absolute.components() {
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
