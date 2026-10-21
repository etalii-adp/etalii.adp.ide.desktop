//! The supply chain diagram's residue (etalii.adp spec 030, FR-018, FR-031 and FR-032, task T076): the tool type
//! `etalii/supply-chain` and its `.supply` documents, after the standalone host's
//! `src/diagrams/supply-chain/backend/EtAlii.Adp.Diagram.SupplyChain` at `8b6ccfc`.
//!
//! The standalone host read, drew and edited a supply chain with its module's own code: its YAML parser, its line
//! splices, its layered layout, its trace and one command per edit. The vendored `supply-chain.dis` describes that
//! tool in DISL as far as DISL reaches, and its persistence is the plugin `net.etalii.adp.etalii.supply`, the module's
//! own reader and writer; the standalone host's loader did not load it (`formatNumber` and `l.max()` were missing from
//! its evaluator). So the supply chain is routed through its module here too: [`SupplyChainModule`]
//! [opens its documents itself](adp_diagram::DiagramModule::opens_documents), and each open view is a
//! [`view::SupplyChainView`], which the session hands every gesture, property edit and action.
//!
//! - [`lines`], [`yaml`], [`model`] and [`parser`]: the `.supply` format, read for its entries and the lines each one
//!   occupies.
//! - [`writer`] and [`commands`]: every edit, as a command on the project's history, splicing only the lines it
//!   changes; its undo puts the body's text back.
//! - [`layout`], [`trace`] and [`mapper`]: where everything is drawn, the chain through the selection, and what the
//!   shell draws.
//! - [`actions`], [`properties`], [`toolbox`] and [`validator`]: the menus, the property rows, the palette and the
//!   findings.
//! - [`store`]: one reading per `.supply`, shared by every view of it, kept in the project's documents.

pub mod actions;
pub mod commands;
pub mod layout;
pub mod lines;
pub mod mapper;
pub mod model;
pub mod parser;
pub mod properties;
pub mod store;
pub mod toolbox;
pub mod trace;
pub mod validator;
pub mod view;
pub mod writer;
pub mod yaml;

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;

use adp_diagram::workspace::{CANNOT_OPEN, OpenedDiagram};
use adp_diagram::{DiagramDocuments, DiagramModule, DiagramTools, ModuleView};
use adp_disl::IdSource;

use crate::view::SupplyChainView;

/// The tool type this module serves.
pub const ORIGIN: &str = "etalii/supply-chain";

/// The extension of a supply chain's body.
pub const EXTENSION: &str = ".supply";

/// The body a new supply chain diagram starts from (`SupplyChainParser.EmptyDocument`, registered with
/// `AddEmptyDocument`).
pub const TEMPLATE: &str = parser::EMPTY_DOCUMENT;

/// How many lines after its first a registration's headers are looked for in.
const HEADER_SCAN_LIMIT: usize = 8;

/// The ids the module mints itself: ShortGuids, a version 4 GUID in 25 characters of base 36, as the standalone
/// host's commands minted them (`supply-chain.dis`, `persistence.ids`).
#[derive(Debug, Clone, Copy, Default)]
pub struct ShortGuids;

impl IdSource for ShortGuids {
    fn next(&self, type_name: &str) -> Result<String, String> {
        adp_disl::behavior::ids::short_guids().next(type_name)
    }
}

/// The supply chain module.
#[derive(Debug, Clone, Copy, Default)]
pub struct SupplyChainModule;

/// `tools` with the supply chain's module added to its tool type.
pub fn register(tools: DiagramTools) -> DiagramTools {
    tools.with_module(ORIGIN, Rc::new(SupplyChainModule))
}

impl DiagramModule for SupplyChainModule {
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
            (route_registration(root, path)?, Some(path.to_path_buf()))
        } else {
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
        let view: Rc<dyn ModuleView> = Rc::new(SupplyChainView::new(&body_path));
        Ok((
            OpenedDiagram {
                origin: origin.to_owned(),
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
/// against its folder and refused outside the project, else the `.supply` beside it named after it.
fn route_registration(root: &Path, registration: &Path) -> Result<PathBuf, String> {
    let text = fs::read_to_string(registration).map_err(|_| "The registration file could not be read.".to_owned())?;
    let folder = registration.parent().unwrap_or(root);
    match body_header(&text) {
        Some(body) => resolve_within(root, folder, &body).ok_or_else(|| {
            "Its body: header names a document outside the project, which is refused rather than followed.".to_owned()
        }),
        None => {
            let name = file_name(registration);
            let base = name
                .len()
                .checked_sub(4)
                .filter(|at| name[*at..].eq_ignore_ascii_case(".adp"))
                .map(|at| &name[..at])
                .filter(|base| !base.is_empty())
                .ok_or_else(|| CANNOT_OPEN.to_owned())?;
            Ok(folder.join(format!("{base}{EXTENSION}")))
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
