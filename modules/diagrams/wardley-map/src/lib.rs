//! The Wardley map's residue (etalii.adp spec 030, FR-018, FR-031 and FR-032, task T070): the `.owm` reader and
//! writer, the identity sidecar, the map's axes and evolution stages, and what its menus, rows and gestures do, after
//! the standalone host's `src/diagrams/wardley-map` at `8b6ccfc`.
//!
//! The standalone host read and wrote the Online Wardley Maps text format with its Wardley module's own code. The
//! vendored `wardley-map.dis` does not load there (`'inertia' is not a variable here`, `The function 'max' is not
//! supported`, as `adp-disl`'s `standalone-diagnostics.txt` records), and no binding for it exists; it names the
//! persistence plugin `net.etalii.adp.wardley.owm` as its format, which is this module: [`WardleyModule`]
//! [opens its documents itself](adp_diagram::DiagramModule::opens_documents), and each open map is a
//! [`view::WardleyView`], which the session hands every gesture, property edit and action.
//!
//! - [`lines`], [`model`] and [`parser`]: an `.owm` as its lines, read into a map; every line keeps its terminator.
//! - [`writer`] and [`commands`]: every edit, as a command on the project's history, splicing only the span it changes.
//! - [`identities`] and [`store`]: the ShortGuids the format does not provide, kept in `{name}.identities.json`, and one
//!   reading per `.owm` shared by every view of it.
//! - [`axis`]: the one conversion from `[visibility, maturity]` to the canvas, and the four evolution stages.
//! - [`mapper`], [`descriptions`], [`actions`], [`properties`], [`rules`] and [`toolbox`]: what is drawn, what a
//!   selection reads as, the menus, the rows, the problems and the palette.

pub mod actions;
pub mod axis;
pub mod commands;
pub mod descriptions;
pub mod identities;
pub mod lines;
pub mod mapper;
pub mod model;
pub mod parser;
pub mod properties;
pub mod rules;
pub mod store;
pub mod toolbox;
pub mod view;
pub mod writer;

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;

use adp_diagram::workspace::{CANNOT_OPEN, OpenedDiagram};
use adp_diagram::{DiagramDocuments, DiagramModule, DiagramTools, ModuleView};
use adp_disl::IdSource;

use crate::view::WardleyView;

/// The tool type this module serves.
pub const ORIGIN: &str = "wardley/map";

/// The persistence plugin `wardley-map.dis` names as its format: this module.
pub const PLUGIN_ID: &str = "net.etalii.adp.wardley.owm";

/// The extension of a Wardley map's body.
pub const EXTENSION: &str = ".owm";

/// Every extension the DSL is written with: the declared one and the `.wm` the ecosystem's own tooling also reads.
pub const EXTENSIONS: [&str; 2] = [EXTENSION, ".wm"];

/// How many lines after its first a registration's headers are looked for in.
const HEADER_SCAN_LIMIT: usize = 8;

/// The Wardley map module, and where the ids of the elements its edits create come from.
#[derive(Default, Clone)]
pub struct WardleyModule {
    ids: Option<Rc<dyn IdSource>>,
}

impl std::fmt::Debug for WardleyModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WardleyModule").finish_non_exhaustive()
    }
}

impl WardleyModule {
    pub fn new() -> Self {
        Self::default()
    }

    /// The module whose new elements take their ids from `ids` rather than fresh ShortGuids: a replay hands it the
    /// ids the standalone host generated.
    pub fn with_ids(ids: Rc<dyn IdSource>) -> Self {
        WardleyModule { ids: Some(ids) }
    }
}

/// `tools` with the Wardley map's module added to its tool type.
pub fn register(tools: DiagramTools) -> DiagramTools {
    tools.with_module(ORIGIN, Rc::new(WardleyModule::new()))
}

/// The body a new map named `base_name` starts as (`ServiceCollection.AddWardleyMap.EmptyDocument`).
pub fn empty_document(base_name: &str) -> String {
    toolbox::empty_document(base_name)
}

impl DiagramModule for WardleyModule {
    fn opens_documents(&self) -> bool {
        true
    }

    fn extensions(&self) -> Vec<String> {
        EXTENSIONS.iter().map(|e| (*e).to_owned()).collect()
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
            (route_registration(root, path)?, Some(path.to_path_buf()))
        } else {
            let registration = path.with_extension("adp");
            let registered = registration.is_file()
                && fs::read_to_string(&registration).is_ok_and(|text| first_line(&text) == ORIGIN);
            (path.to_path_buf(), registered.then_some(registration))
        };
        if origin != ORIGIN {
            return Err(CANNOT_OPEN.to_owned());
        }
        store::set_ids(documents, self.ids.clone());
        // A body that is missing opens as an empty map, and is created on the first save.
        store::get_or_load(documents, &body_path);
        let view: Rc<dyn ModuleView> = Rc::new(WardleyView::new(root, &body_path));
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

fn first_line(text: &str) -> String {
    text.trim_start_matches('\u{feff}')
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_owned()
}

/// The body a registration opens (the standalone host's `DiagramFilePair.BodyOf`): its `body:` header, resolved
/// against its folder and refused outside the project, else the `.owm` beside it of its name.
fn route_registration(root: &Path, registration: &Path) -> Result<PathBuf, String> {
    let text = fs::read_to_string(registration).map_err(|_| "The registration file could not be read.".to_owned())?;
    let folder = registration.parent().unwrap_or(root);
    match body_header(&text) {
        Some(body) => resolve_within(root, folder, &body).ok_or_else(|| {
            "Its body: header names a document outside the project, which is refused rather than followed.".to_owned()
        }),
        None => {
            let stem = registration
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| CANNOT_OPEN.to_owned())?;
            Ok(folder.join(format!("{stem}{EXTENSION}")))
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
        if trimmed.get(..5).is_some_and(|h| h.eq_ignore_ascii_case("body:")) {
            let value = trimmed[5..].trim();
            return (!value.is_empty()).then(|| value.to_owned());
        }
        if !trimmed.get(..5).is_some_and(|h| h.eq_ignore_ascii_case("view:")) {
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
