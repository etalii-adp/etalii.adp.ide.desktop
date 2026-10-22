//! The Sankey diagram's residue (etalii.adp spec 030, FR-018, FR-031 and FR-032, task T077): the `.skv` reader and
//! writer, its layout and arrangement, its actions, property rows and rules, after the standalone host's
//! `src/diagrams/sankey/backend/EtAlii.Adp.Diagram.Sankey` at `8b6ccfc`.
//!
//! The standalone host read and wrote `.skv` with its Sankey module's own code (`SankeyParser`, `SankeyWriter`, the
//! commands under `Commands/`), not through DISL: its `sankey-disl-fbl` specification, which would route it through
//! `sankey.dis` and the persistence plugin `net.etalii.adp.etalii.skv`, waits on FBL gaps and has none of its tasks
//! done at the reference commit. Nor does the vendored `sankey.dis` load, in the standalone host's loader or in this
//! one: `uniqueName` calls `labelOf`, declared after it (DISL section 3.4). So the Sankey diagram is routed through its
//! module here, as C4 is: [`SankeyModule`] [opens its documents itself](adp_diagram::DiagramModule::opens_documents),
//! and each open diagram is a [`view::SankeyDiagramView`], which the session hands every gesture, property edit and
//! action.
//!
//! - [`lines`], [`yaml`], [`model`] and [`parser`]: the document as lines, read through YAML into its model, every
//!   entry knowing its lines.
//! - [`writer`] and [`commands`]: every edit as a splice of the lines it changes, as a command on the project's
//!   history, undone by writing back the text it changed.
//! - [`layout`], [`arrangement`], [`format`] and [`mapper`]: columns, bars and bands; the order Arrange diagram gives
//!   each column; values in the document's words and colours; the elements the shell draws.
//! - [`actions`], [`rules`] and [`store`]: the menus, rows and palette; the findings; one reading per `.skv`, shared
//!   by every view of it.

pub mod actions;
pub mod arrangement;
pub mod commands;
pub mod format;
pub mod layout;
pub mod lines;
pub mod mapper;
pub mod model;
pub mod parser;
pub mod rules;
pub mod store;
pub mod theme;
pub mod view;
pub mod writer;
pub mod yaml;

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;

use adp_diagram::workspace::{CANNOT_OPEN, OpenedDiagram};
use adp_diagram::{DiagramDocuments, DiagramModule, DiagramTools, ModuleView};

use crate::view::SankeyDiagramView;

/// The Sankey diagram's origin, as `docs/tools.md` catalogues it.
pub const ORIGIN: &str = "etalii/sankey";

/// The extension of a Sankey body: ADP's own schema.
pub const EXTENSION: &str = ".skv";

/// The body a new Sankey diagram starts with (`SankeyParser.EmptyDocument`, registered with `AddEmptyDocument`).
pub const EMPTY_DOCUMENT: &str = parser::EMPTY_DOCUMENT;

/// How many lines after its first a registration's headers are looked for in.
const HEADER_SCAN_LIMIT: usize = 8;

/// The Sankey module.
#[derive(Debug, Clone, Copy, Default)]
pub struct SankeyModule;

/// The module, as `adp-diagram` is handed it for [`ORIGIN`].
pub fn module() -> Rc<dyn DiagramModule> {
    Rc::new(SankeyModule)
}

/// `tools` with the Sankey module added for its tool type.
pub fn register(tools: DiagramTools) -> DiagramTools {
    tools.with_module(ORIGIN, module())
}

impl DiagramModule for SankeyModule {
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
        if origin != ORIGIN {
            return Err(CANNOT_OPEN.to_owned());
        }
        let is_registration = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("adp"));
        let (body_path, registration_path) = if is_registration {
            (route_registration(root, path)?, Some(path.to_path_buf()))
        } else {
            // A registration beside the body wins where it exists; this is only for a body on its own.
            let registration = path.with_extension("adp");
            let registered = registration.is_file()
                && fs::read_to_string(&registration).is_ok_and(|text| first_line(&text) == ORIGIN);
            if registered {
                (route_registration(root, &registration)?, Some(registration))
            } else {
                (path.to_path_buf(), None)
            }
        };
        if !body_path.is_file() {
            return Err(format!(
                "The body of '{}' is missing.",
                registration_path
                    .as_deref()
                    .map_or_else(|| file_name(&body_path), file_name)
            ));
        }
        store::get_or_load(documents, &body_path);
        let view: Rc<dyn ModuleView> = Rc::new(SankeyDiagramView::new(&body_path));
        Ok((
            OpenedDiagram {
                origin: ORIGIN.to_owned(),
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

/// The body a registration opens (the standalone host's `DiagramFilePair.BodyOf`): its `body:` header, resolved
/// against its folder and refused outside the project, else the `.skv` beside it named after its subject.
fn route_registration(root: &Path, registration: &Path) -> Result<PathBuf, String> {
    let text = fs::read_to_string(registration).map_err(|_| "The registration file could not be read.".to_owned())?;
    let folder = registration.parent().unwrap_or(root);
    match body_header(&text) {
        Some(body) => resolve_within(root, folder, &body).ok_or_else(|| {
            "Its body: header names a document outside the project, which is refused rather than followed.".to_owned()
        }),
        None => sibling(registration).ok_or_else(|| CANNOT_OPEN.to_owned()),
    }
}

/// The `body:` header of a registration's text, read from the lines after its first, blank lines and other headers
/// skipped, up to the first line that is neither.
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
        let Some((header, value)) = trimmed.split_once(':') else {
            break;
        };
        if header.contains(char::is_whitespace) {
            break;
        }
        if header.eq_ignore_ascii_case("body") {
            let value = value.trim();
            return (!value.is_empty()).then(|| value.to_owned());
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

/// The body a registration without a `body:` header opens: its subject's `.skv` beside it.
fn sibling(registration: &Path) -> Option<PathBuf> {
    let name = file_name(registration);
    let base = name.get(..name.len().checked_sub(4)?)?;
    if base.is_empty() {
        // A folder-scoped registration derives no body.
        return None;
    }
    let subject = match base.rfind('.') {
        Some(at) if at > 0 => &base[..at],
        _ => base,
    };
    let folder = registration.parent()?;
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
