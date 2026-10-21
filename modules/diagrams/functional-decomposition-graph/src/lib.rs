//! The functional decomposition graph's residue (etalii.adp spec 030, task T072): the `.fdg` reader and line-splicing
//! writer, the rules, the menus, the rows and the palette, after the standalone host's
//! `src/diagrams/functional-decomposition-graph` at `8b6ccfc`.
//!
//! The standalone host read and wrote `.fdg` with this module's own code, its own YAML subset whose every edit is a
//! line splice, and drew it through its own commands and providers; no DISL specification of it loaded there (the
//! vendored `functional-decomposition-graph.dis` calls `inCycle` and `reachable`, which its evaluator lacked, and
//! names the persistence plugin `net.etalii.adp.etalii.fdg` for the format DISL's persistence cannot describe). So
//! the tool type is routed through its module here as C4 is: [`FdgModule`]
//! [opens its documents itself](adp_diagram::DiagramModule::opens_documents), and each open view is a
//! [`view::FdgView`], which the session hands every gesture, property edit and action.
//!
//! - [`lines`] and [`yaml`]: the document as its lines, the splicing helpers, and the YAML it is read with.
//! - [`model`] and [`parser`]: what a document declares, read without ever failing (`FdgParser`).
//! - [`writer`] and [`commands`]: every edit as a splice of its own lines, as a command on the project's history
//!   whose undo restores the text the body held (`FdgWriter`, `Commands/`).
//! - [`relations`] and [`rules`]: the five relations' table, the ownership cycles, the connect verdict and the eight
//!   rule ids (`FdgRelations`, `FdgOwnership`, `FdgConnectVerdict`, `FdgRuleSet`, `FdgValidator`).
//! - [`actions`], [`properties`] and [`mapper`]: the menus, the property rows, what is drawn and the palette
//!   (`FdgContextActionProvider`, `FdgContextPropertyProvider`, `FdgElementMapper`, `FdgToolboxProvider`).
//! - [`store`] and [`view`]: one reading per body in the project's documents, and one open view of it (`FdgSession`,
//!   `FdgDocumentStore`, `FdgContextSourceResolver`).
//!
//! The client's behaviour is here too where it decides something: a drag from one element's body to another's draws
//! the one relation whose ends admit the pair (`relationOnto`, [`relations::onto`]), and a drop adds its type centred
//! where it landed.

pub mod actions;
pub mod commands;
pub mod lines;
pub mod mapper;
pub mod model;
pub mod parser;
pub mod properties;
pub mod relations;
pub mod rules;
pub mod store;
pub mod view;
pub mod writer;
pub mod yaml;

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;

use adp_diagram::workspace::{CANNOT_OPEN, OpenedDiagram};
use adp_diagram::{DiagramDocuments, DiagramModule, DiagramTools, ModuleView};
use adp_fbl::RegistrationDocument;

use crate::view::FdgView;

/// The tool type this module serves.
pub const ORIGIN: &str = "etalii/functional-decomposition-graph";

/// The extension of a body.
pub const EXTENSION: &str = ".fdg";

/// What a new `.fdg` is written with when one is added: the header and two empty sections in CRLF, the house style a
/// new file has no style of its own to override (`ServiceCollection.AddFunctionalDecompositionGraph.cs`,
/// `AddEmptyDocument(..., FdgParser.EmptyDocument("\r\n"))`).
pub const EMPTY_DOCUMENT: &str = "functional-decomposition-graph: 1\r\nelements: []\r\nconnections: []\r\n";

/// The functional decomposition graph's module.
#[derive(Debug, Default, Clone, Copy)]
pub struct FdgModule;

/// The module, as a host registers it for [`ORIGIN`].
pub fn module() -> Rc<dyn DiagramModule> {
    Rc::new(FdgModule)
}

/// `tools` with the module added for [`ORIGIN`].
pub fn register(tools: DiagramTools) -> DiagramTools {
    tools.with_module(ORIGIN, module())
}

impl DiagramModule for FdgModule {
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
        let (body_path, registration_path) = if is_registration {
            (body_of(root, path)?, Some(path.to_path_buf()))
        } else {
            // A registration beside the body, of this tool type, is the one it was opened through.
            let beside = path.with_extension("adp");
            let registered = beside.is_file()
                && fs::read(&beside)
                    .map(|bytes| RegistrationDocument::read(bytes).origin == ORIGIN)
                    .unwrap_or(false);
            (path.to_path_buf(), registered.then_some(beside))
        };
        if origin != ORIGIN {
            return Err(CANNOT_OPEN.to_owned());
        }
        if !body_path.is_file() {
            return Err(format!(
                "The body of '{}' is missing.",
                file_name(registration_path.as_deref().unwrap_or(&body_path))
            ));
        }
        store::get_or_load(documents, &body_path);
        let view: Rc<dyn ModuleView> = Rc::new(FdgView::new(&body_path));
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

/// The body a registration opens (the standalone host's `DiagramFilePair.BodyOf`): its `body:` header, resolved
/// against its folder and refused outside the project, else the `.fdg` beside it named after it.
fn body_of(root: &Path, registration: &Path) -> Result<PathBuf, String> {
    let bytes = fs::read(registration).map_err(|_| "The registration file could not be read.".to_owned())?;
    let document = RegistrationDocument::read(bytes);
    let folder = registration.parent().unwrap_or(root);
    match document.body() {
        Some(body) => {
            let candidate = Path::new(body);
            if candidate.is_absolute() || body.starts_with(['/', '\\']) || candidate.has_root() {
                return Err("The registration's body is an absolute path, which is never followed.".to_owned());
            }
            let full = normalise(&folder.join(candidate));
            if !full.starts_with(normalise(root)) {
                return Err(
                    "Its body: header names a document outside the project, which is refused rather than followed."
                        .to_owned(),
                );
            }
            Ok(full)
        }
        None => {
            let stem = registration
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            if stem.is_empty() {
                return Err(CANNOT_OPEN.to_owned());
            }
            Ok(folder.join(format!("{stem}{EXTENSION}")))
        }
    }
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
