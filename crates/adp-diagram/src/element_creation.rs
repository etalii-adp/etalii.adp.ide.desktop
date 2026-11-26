//! An element added to a diagram by another tool, by name alone (etalii.adp spec 092, FR-025; `knowledge.md`, "A
//! relation to a diagram"): a knowledge relation's *Create* adds an element with the typed name to its target diagram,
//! of a kind the diagram's module names, placed where that tool places an element added without a position. What is
//! planned here is the diagram body's new bytes, never written: the knowledge designer writes them with its own file
//! in one step, whose undo takes both back.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use adp_disl::IdSource;
use adp_disl::behavior::ids;
use adp_fbl::history::apply_in_order;
use adp_links::IndexedDocument;

use crate::document::{DiagramChange, DiagramDocument, DiagramDocuments, EditedFile};
use crate::module::CreatedKind;
use crate::tool::DiagramTools;

/// Why a diagram is given no element by name: its module names no kind of element another tool may add.
pub const NOTHING_TO_CREATE: &str = "This diagram is given no element from another document.";

/// What adding an element by name gives: the id it was given and the body that holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedElement {
    pub id: String,
    pub body: Vec<u8>,
}

/// The kinds of element another tool may add to `document`, in its module's order: none for a tool type without a
/// module or whose module names none.
pub fn created_kinds(document: &DiagramDocument) -> Vec<CreatedKind> {
    document
        .tool()
        .module
        .as_ref()
        .map(|module| module.created_kinds())
        .unwrap_or_default()
}

/// The body of `document` with an element of the type `kind` names added under a new id of the specification's
/// strategy and named `name`, where its tool places an element added without a position; or why it is not.
pub fn with_element(document: &DiagramDocument, kind: &str, name: &str) -> Result<CreatedElement, String> {
    let module = document.tool().module.clone().ok_or(NOTHING_TO_CREATE)?;
    if !module.created_kinds().iter().any(|k| k.type_name == kind) {
        return Err(NOTHING_TO_CREATE.to_owned());
    }
    let id = ids::of(document.tool().definition.specification())?.next(kind)?;
    let change = module.creation(document, kind, &id, name).ok_or(NOTHING_TO_CREATE)?;
    let edit = document.plan("Create", &[DiagramChange::Disl(change)])?;
    let mut body = document.body_bytes().to_vec();
    for step in &edit.steps {
        if step.file != EditedFile::Body {
            // An element added by name writes the body alone: one that would need its registration is not added.
            return Err(NOTHING_TO_CREATE.to_owned());
        }
        body = apply_in_order(&body, &step.splices)?;
    }
    Ok(CreatedElement { id, body })
}

/// A diagram of the workspace that another tool's link may point at: its path, its elements as the links index holds
/// them but for those that hold a link of their own - a linked element stands for another document's element - and
/// the kinds of element another tool may add to it.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkableDiagram {
    pub path: PathBuf,
    pub indexed: IndexedDocument,
    pub kinds: Vec<CreatedKind>,
}

/// The diagrams under `root` whose tool type takes part in links (FBL section 17.3), read from disk, in path order.
pub fn diagrams_under(tools: &mut DiagramTools, root: &Path) -> Vec<LinkableDiagram> {
    let mut found = Vec::new();
    let mut scratch = DiagramDocuments::default();
    for path in crate::workspace::files_under(&crate::workspace::normalise(root)) {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        for origin in tools.for_file(&name) {
            let Ok(tool) = tools.get(&origin) else { continue };
            if crate::links::document_id_rule(tool.definition.specification()).is_none() {
                continue;
            }
            let Ok(document) = scratch.open(tool, &path, None) else {
                continue;
            };
            let read = crate::links::indexed(document).map(|mut indexed| {
                let holders: HashSet<String> = indexed.links.iter().map(|l| l.holder.clone()).collect();
                indexed.elements.retain(|e| !holders.contains(&e.id));
                LinkableDiagram {
                    path: path.clone(),
                    indexed,
                    kinds: created_kinds(document),
                }
            });
            scratch.close(&path);
            if let Some(diagram) = read {
                found.push(diagram);
                break;
            }
        }
    }
    found
}

/// The body of the diagram at `path`, read from disk, with an element of `kind` named `name` added
/// ([`with_element`]); or why not.
pub fn create_in(tools: &mut DiagramTools, path: &Path, kind: &str, name: &str) -> Result<CreatedElement, String> {
    let file = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut scratch = DiagramDocuments::default();
    for origin in tools.for_file(&file) {
        let Ok(tool) = tools.get(&origin) else { continue };
        let Ok(document) = scratch.open(tool, path, None) else {
            continue;
        };
        if crate::links::takes_part(document) {
            return with_element(document, kind, name);
        }
    }
    Err(NOTHING_TO_CREATE.to_owned())
}
