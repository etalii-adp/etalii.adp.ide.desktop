//! The agent activity diagram's residue (etalii.adp spec 030, FR-018, FR-031 and FR-032, task T075): the tool type
//! `etalii/agent-activity-diagram` and its `.aad` documents, after the standalone host's
//! `src/diagrams/agent-activity-diagram/backend/EtAlii.Adp.Diagram.AgentActivityDiagram` at `8b6ccfc`.
//!
//! The standalone host's module derived what it could from its bundled specification (`agent-activity-diagram.dis`)
//! and read and wrote the file only through its bundled binding (`agent-activity-diagram.fbl#aad`), but it was a module
//! of its own: its session, its document store, its menus, rows and commands, and the reader's own part of the file
//! (DISL 0.4 `persistence.view.bind`: the locked positions, the folded groups and the archived switch), which the
//! generic DISL path does not keep. So the agent activity diagram is routed through its module here too:
//! [`AgentActivityModule`] [opens its documents itself](adp_diagram::DiagramModule::opens_documents), and each open
//! view is a [`view::AgentActivityView`], which the session hands every gesture, property edit and action. Its
//! definition is the one etalii.adp publishes, vendored under `vendor/etalii.adp/definitions/diagrams/` and embedded as
//! the standalone host embedded its `definition/` folder.
//!
//! - [`definition`]: the specification and binding, and what the module reads off them by name.
//! - [`body`] and [`model`]: one file read through the binding, and what the diagram reads of it.
//! - [`commands`]: every edit, as a command on the project's history, spliced by the binding.
//! - [`mapper`]: what the shell draws, each element where it is locked or where the radiating layout puts it.
//! - [`actions`], [`properties`] and [`validator`]: the menus, the property rows and the findings.
//! - [`store`]: one reading per `.aad`, shared by every view of it, kept in the project's documents.

pub mod actions;
pub mod body;
pub mod commands;
pub mod definition;
pub mod mapper;
pub mod model;
pub mod properties;
pub mod store;
pub mod theme;
pub mod validator;
pub mod view;

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;

use adp_diagram::workspace::{CANNOT_OPEN, OpenedDiagram};
use adp_diagram::{DiagramDocuments, DiagramModule, DiagramTools, ModuleView};

use crate::view::AgentActivityView;

/// The tool type this module serves.
pub const ORIGIN: &str = "etalii/agent-activity-diagram";

/// The extension of an activity file.
pub const EXTENSION: &str = ".aad";

/// The body a new agent activity diagram starts from: the header alone, in CRLF, the house style, since a new file
/// has no style of its own to keep (`ServiceCollection.AddAgentActivityDiagram.cs`, `AadBody.EmptyDocument("\r\n")`).
pub const TEMPLATE: &str = "agent-activity-diagram: 1\r\n";

/// How many lines after its first a registration's headers are looked for in.
const HEADER_SCAN_LIMIT: usize = 8;

/// The moment of a gesture as the file writes one: local time with its offset, to the second (`AadEdits.Now`). Where
/// the platform cannot say the local offset, the moment is written in UTC.
pub fn now() -> String {
    let moment = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    let offset = moment.offset();
    let (hours, minutes) = (offset.whole_hours(), offset.minutes_past_hour());
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{}{:02}:{:02}",
        moment.year(),
        u8::from(moment.month()),
        moment.day(),
        moment.hour(),
        moment.minute(),
        moment.second(),
        if hours < 0 || minutes < 0 { '-' } else { '+' },
        hours.unsigned_abs(),
        minutes.unsigned_abs()
    )
}

/// The agent activity diagram's module.
#[derive(Debug, Clone, Copy, Default)]
pub struct AgentActivityModule;

/// `tools` with the agent activity diagram's module added to its tool type.
pub fn register(tools: DiagramTools) -> DiagramTools {
    tools.with_module(ORIGIN, Rc::new(AgentActivityModule))
}

impl DiagramModule for AgentActivityModule {
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
        let view: Rc<dyn ModuleView> = Rc::new(AgentActivityView::new(&body_path));
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
/// against its folder and refused outside the project, else the `.aad` beside it named after it.
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
