//! The C4 diagram types' residue (etalii.adp spec 030, FR-018, FR-031 and FR-032, task T061): the Structurizr DSL
//! reader and writer, and the six views over one workspace, after the standalone host's
//! `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4` at `8b6ccfc`.
//!
//! The standalone host read and wrote Structurizr DSL with its C4 module's own code: its parser, its line edits, its
//! layout and its layout sidecar. No DISL specification of C4 loads there: the vendored `c4-*.dis` describe the
//! conversion's later stage, name a binding (`structurizr.fbl`) the reference commit does not have, and refuse to load
//! (`'includes' is not a variable here`, as the standalone host's own loader says). So C4 is routed through its
//! module here too: [`C4Module`] [opens its documents itself](adp_diagram::DiagramModule::opens_documents), and each
//! open view is a [`view::C4DiagramView`], which the session hands every gesture, property edit and action.
//!
//! - [`model`], [`tokens`], [`lines`] and [`parser`]: the workspace a `.dsl` declares, read line by line.
//! - [`commands`] and [`placement`]: every edit, as a command on the project's history, rewriting only the lines it
//!   changes; a move written to the layout sidecar ([`sidecar`]), never to the body.
//! - [`rules`]: what each view shows and the C4 rules, as the problems panel shows them.
//! - [`metrics`], [`layout`], [`theme`] and [`mapper`]: where a view's elements go and what the shell draws.
//! - [`actions`], [`properties`] and [`toolbox`]: the menus, the property rows, the palette and the empty documents.
//! - [`store`]: one reading per `.dsl`, shared by every view of it, kept in the project's documents.
//!
//! The six origins each bind one view kind (`c4/context`, `c4/container`, `c4/component`, `c4/system-landscape`,
//! `c4/dynamic`, `c4/deployment`); which view a diagram shows comes from its registration's `view:` header, not from
//! its origin. `c4/code` keeps its registration and opens nothing ([`toolbox::CODE_UNAVAILABLE`]).

pub mod actions;
pub mod commands;
pub mod layout;
pub mod lines;
pub mod mapper;
pub mod metrics;
pub mod model;
pub mod parser;
pub mod placement;
pub mod properties;
pub mod rules;
pub mod sidecar;
pub mod store;
pub mod theme;
pub mod tokens;
pub mod toolbox;
pub mod view;

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;

use adp_diagram::workspace::{CANNOT_OPEN, OpenedDiagram};
use adp_diagram::{DiagramDocuments, DiagramModule, DiagramTools, ModuleView};

use crate::view::C4DiagramView;

/// The extension of a C4 body: a Structurizr DSL workspace.
pub const EXTENSION: &str = ".dsl";

/// How many lines after its first a registration's headers are looked for in.
const HEADER_SCAN_LIMIT: usize = 8;

/// The C4 module, registered for every C4 origin.
#[derive(Debug, Default, Clone, Copy)]
pub struct C4Module;

/// `tools` with the C4 module added to every C4 tool type, `c4/code` included, so that its registration is refused
/// with the sentence that says why.
pub fn register(tools: DiagramTools) -> DiagramTools {
    let module: Rc<dyn DiagramModule> = Rc::new(C4Module);
    toolbox::TYPES
        .iter()
        .fold(tools, |tools, (origin, _)| tools.with_module(origin, module.clone()))
}

impl DiagramModule for C4Module {
    fn opens_documents(&self) -> bool {
        true
    }

    fn extensions(&self) -> Vec<String> {
        vec![EXTENSION.to_owned()]
    }

    fn open(
        &self,
        root: &Path,
        path: &Path,
        origin: &str,
        documents: &mut DiagramDocuments,
    ) -> Result<(OpenedDiagram, Rc<dyn ModuleView>), String> {
        let is_registration = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("adp"));
        let (origin, body_path, registration_path, view_key) = if is_registration {
            let (body, view) = route_registration(root, path)?;
            (origin.to_owned(), body, Some(path.to_path_buf()), view)
        } else {
            // A registration beside the body wins where it exists; this is only for a body on its own.
            let registration = path.with_extension("adp");
            match registration
                .is_file()
                .then(|| fs::read_to_string(&registration).ok())
                .flatten()
                .map(|text| first_line(&text))
                .filter(|o| toolbox::TYPES.iter().any(|(t, _)| t == o))
            {
                Some(registered) => {
                    let (body, view) = route_registration(root, &registration)?;
                    (registered, body, Some(registration), view)
                }
                None => (origin.to_owned(), path.to_path_buf(), None, None),
            }
        };
        if origin == "c4/code" {
            return Err(toolbox::CODE_UNAVAILABLE.to_owned());
        }
        if toolbox::view_kind_of(&origin).is_none() {
            return Err(CANNOT_OPEN.to_owned());
        }
        if !body_path.is_file() {
            return Err(format!(
                "The body of '{}' is missing.",
                registration_path
                    .as_deref()
                    .map_or_else(|| file_name(&body_path), file_name)
            ));
        }
        store::get_or_load(documents, &body_path);
        let view: Rc<dyn ModuleView> = Rc::new(C4DiagramView::new(&origin, &body_path, view_key.as_deref()));
        Ok((
            OpenedDiagram {
                origin,
                body_path,
                registration_path,
            },
            view,
        ))
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn first_line(text: &str) -> String {
    text.trim_start_matches('\u{feff}')
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_owned()
}

/// The body a registration opens and the view it names (the standalone host's `DiagramFilePair.BodyOf`): its `body:`
/// header, resolved against its folder and refused outside the project, else the body beside it named after its
/// subject.
fn route_registration(root: &Path, registration: &Path) -> Result<(PathBuf, Option<String>), String> {
    let text = fs::read_to_string(registration).map_err(|_| "The registration file could not be read.".to_owned())?;
    let (body, view) = headers(&text);
    let folder = registration.parent().unwrap_or(root);
    let body_path = match body {
        Some(body) => resolve_within(root, folder, &body).ok_or_else(|| {
            "Its body: header names a document outside the project, which is refused rather than followed.".to_owned()
        })?,
        None => sibling(registration).ok_or_else(|| CANNOT_OPEN.to_owned())?,
    };
    Ok((body_path, view))
}

/// The `body:` and `view:` headers of a registration's text, read from the lines after its first, blank lines
/// skipped, up to the first line that is neither.
fn headers(text: &str) -> (Option<String>, Option<String>) {
    let (mut body, mut view) = (None, None);
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
        let value = |header: &str| {
            let value = trimmed[header.len()..].trim();
            (!value.is_empty()).then(|| value.to_owned())
        };
        let starts = |header: &str| {
            trimmed
                .get(..header.len())
                .is_some_and(|h| h.eq_ignore_ascii_case(header))
        };
        if starts("body:") {
            body = value("body:");
        } else if starts("view:") {
            view = value("view:");
        } else {
            break;
        }
    }
    (body, view)
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

/// The body a registration without a `body:` header opens: its subject's `.dsl` beside it, or, for a qualified name
/// (`courier.containers.adp`) whose whole base names a body that exists, that one.
fn sibling(registration: &Path) -> Option<PathBuf> {
    let name = file_name(registration);
    let base = name.strip_suffix(".adp").or_else(|| {
        name.len()
            .checked_sub(4)
            .filter(|at| name[*at..].eq_ignore_ascii_case(".adp"))
            .map(|at| &name[..at])
    })?;
    if base.is_empty() {
        // A folder-scoped registration derives no body.
        return None;
    }
    let folder = registration.parent()?;
    let subject = match base.rfind('.') {
        Some(at) if at > 0 => &base[..at],
        _ => base,
    };
    let whole = folder.join(format!("{base}{EXTENSION}"));
    if subject != base && whole.is_file() {
        return Some(whole);
    }
    Some(folder.join(format!("{subject}{EXTENSION}")))
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
