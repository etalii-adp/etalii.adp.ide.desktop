//! The Databricks family's residue (etalii.adp spec 030, FR-018, FR-031 and FR-032, task T081): the tool types
//! `databricks/bundle`, `databricks/job` and `databricks/pipeline`, after the standalone host's
//! `src/diagrams/databricks/backend/EtAlii.Adp.Diagram.Databricks` at `8b6ccfc`.
//!
//! The standalone host read, drew and edited the family's files with its module's own code: one store and one engine
//! for all three types, its three parsers over one YAML reading (JSON being a subset of YAML 1.2, a pipeline's
//! settings `.json` comes through the same door), its writers that splice only the lines an edit concerns, its three
//! computed layouts with the positions a person dragged to stored in the registration's `layout:` block, and one
//! command per edit. The vendored `databricks-*.dis` specifications name the persistence plugin
//! `net.etalii.adp.databricks.files`, which no binding provides, so the family is routed through its module here too:
//! [`DatabricksModule`] [opens its documents itself](adp_diagram::DiagramModule::opens_documents), and each open view is
//! a [`view::DatabricksView`], which the session hands every action, property edit and drag.
//!
//! `.yml` and `.json` belong to the whole world, so a file is never claimed on sight: it becomes one of these diagrams
//! when a registration names the tool type (`Diagram.cs`, `SharedExtension: true`); the registration's `body:` header,
//! else the file beside it named after it, is the body, and its `resource:` header picks one declaration out of a
//! file that holds several.
//!
//! - [`lines`], [`yaml`], [`model`] and [`parser`]: the file, read with the lines each part occupies.
//! - [`writer`] and [`commands`]: every edit, as a command on the project's history, and [`registration_layout`], the
//!   positions stored in a registration.
//! - [`layout`], [`mapper`] and [`selection`]: where everything is drawn, what the shell draws, and what an id names.
//! - [`actions`], [`properties`], [`toolbox`] and [`rules`]: the menus, the property rows, the palettes and the
//!   findings.
//! - [`store`]: one reading per file, shared by every view of it, kept in the project's documents.

pub mod actions;
pub mod commands;
pub mod layout;
pub mod lines;
pub mod mapper;
pub mod model;
pub mod parser;
pub mod properties;
pub mod registration_layout;
pub mod rules;
pub mod selection;
pub mod store;
pub mod theme;
pub mod toolbox;
pub mod view;
pub mod writer;
pub mod yaml;

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;

use adp_diagram::workspace::{CANNOT_OPEN, OpenedDiagram};
use adp_diagram::{DiagramDocuments, DiagramModule, DiagramTools, ModuleView};

use crate::view::DatabricksView;

/// The bundle: what a `databricks.yml` deploys, where, and with what overrides.
pub const BUNDLE_ORIGIN: &str = "databricks/bundle";
/// The job: a task graph with conditions, outcomes and cluster bindings.
pub const JOB_ORIGIN: &str = "databricks/job";
/// The declarative pipeline: sources through transformation to a catalog target.
pub const PIPELINE_ORIGIN: &str = "databricks/pipeline";
/// The family's three tool types, the bundle first.
pub const ORIGINS: [&str; 3] = [BUNDLE_ORIGIN, JOB_ORIGIN, PIPELINE_ORIGIN];

/// How many lines after its first a registration's headers are looked for in.
const HEADER_SCAN_LIMIT: usize = 8;

/// What opening a `.yml` or `.json` no registration names answers.
pub const NOT_REGISTERED: &str = "A .yml or .json file is a Databricks diagram only once it is registered as one: many \
                                  configuration files are not.";

/// The reading a tool type shows: `bundle`, `job` or `pipeline`.
pub fn reading_of(origin: &str) -> &str {
    origin.strip_prefix("databricks/").unwrap_or("bundle")
}

/// The extension of a body of `origin` (`Diagram.cs`): `.json` for a pipeline's settings file, `.yml` otherwise.
pub fn extension_of(origin: &str) -> &'static str {
    if origin == PIPELINE_ORIGIN { ".json" } else { ".yml" }
}

/// The base name as a Databricks key (`KeyOf`): lower case, anything a key cannot carry folded to underscores, so
/// "Nightly Ingest.job" starts life as `nightly_ingest_job`.
pub fn key_of(base_name: &str) -> String {
    let folded: String = base_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    let key = folded.trim_matches('_');
    if key.is_empty() {
        "untitled".to_owned()
    } else {
        key.to_owned()
    }
}

/// The body a new diagram of `origin` starts as (`ServiceCollection.AddDatabricks.cs`, `EmptyDocument`): complete
/// enough to open, read and judge clean, in CRLF with a trailing newline.
pub fn empty_document(origin: &str, base_name: &str) -> String {
    let key = key_of(base_name);
    match reading_of(origin) {
        "job" => format!(
            "resources:\r\n  jobs:\r\n    {key}:\r\n      name: {base_name}\r\n      tasks:\r\n        - task_key: main\r\n          \
             notebook_task:\r\n            notebook_path: notebooks/{key}\r\n"
        ),
        "pipeline" => format!(
            "{{\r\n  \"name\": \"{key}\",\r\n  \"libraries\": [\r\n    {{ \"notebook\": {{ \"path\": \"transformations/{key}\" \
             }} }}\r\n  ]\r\n}}\r\n"
        ),
        _ => {
            format!("bundle:\r\n  name: {key}\r\ntargets:\r\n  dev:\r\n    mode: development\r\n    default: true\r\n")
        }
    }
}

/// The empty bundle named after `base_name`.
pub fn new_bundle(base_name: &str) -> String {
    empty_document(BUNDLE_ORIGIN, base_name)
}

/// The empty job named after `base_name`.
pub fn new_job(base_name: &str) -> String {
    empty_document(JOB_ORIGIN, base_name)
}

/// The empty pipeline settings file named after `base_name`.
pub fn new_pipeline(base_name: &str) -> String {
    empty_document(PIPELINE_ORIGIN, base_name)
}

/// The Databricks module, one for the family's three tool types.
#[derive(Debug, Clone, Copy, Default)]
pub struct DatabricksModule;

/// `tools` with the family's module added to its three tool types.
pub fn register(tools: DiagramTools) -> DiagramTools {
    let module = Rc::new(DatabricksModule);
    ORIGINS
        .iter()
        .fold(tools, |tools, origin| tools.with_module(origin, module.clone()))
}

impl DiagramModule for DatabricksModule {
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
        let is_registration = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("adp"));
        let registration = if is_registration {
            path.to_path_buf()
        } else {
            let beside = path.with_extension("adp");
            let registered =
                beside.is_file() && fs::read_to_string(&beside).is_ok_and(|text| first_line(&text) == origin);
            if !registered {
                return Err(NOT_REGISTERED.to_owned());
            }
            beside
        };
        let text =
            fs::read_to_string(&registration).map_err(|_| "The registration file could not be read.".to_owned())?;
        let body_path = route_registration(root, &registration, &text, origin)?;
        if !body_path.is_file() {
            return Err(format!("The body of '{}' is missing.", file_name(&registration)));
        }
        store::get_or_load(documents, &body_path);
        store::store(documents).track_registration(&registration, &body_path);
        let view: Rc<dyn ModuleView> = Rc::new(DatabricksView::new(
            &body_path,
            Some(&registration),
            reading_of(origin),
            resource_header(&text),
        ));
        Ok((
            OpenedDiagram {
                origin: origin.to_owned(),
                body_path,
                registration_path: Some(registration),
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

/// The lines after a registration's first that its headers are looked for in.
fn header_lines(text: &str) -> impl Iterator<Item = &str> {
    text.trim_start_matches('\u{feff}')
        .lines()
        .skip(1)
        .take(HEADER_SCAN_LIMIT)
}

fn starts_with_ignoring_case(text: &str, prefix: &str) -> bool {
    text.get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}

/// The `resource:` header's value (`DatabricksHeaders.ReadResourceKey`): looked for in the eight lines after the first,
/// blank lines skipped, up to the `layout:` block; none when there is none.
pub fn resource_header(text: &str) -> Option<String> {
    for line in header_lines(text) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if starts_with_ignoring_case(trimmed, "resource:") {
            let value = trimmed["resource:".len()..].trim();
            return (!value.is_empty()).then(|| value.to_owned());
        }
        if starts_with_ignoring_case(trimmed, "layout:") {
            return None;
        }
    }
    None
}

/// The `body:` header of a registration's text, read from the lines after its first, blank lines skipped, up to the
/// first line that is no header.
fn body_header(text: &str) -> Option<String> {
    for line in header_lines(text) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if starts_with_ignoring_case(trimmed, "body:") {
            let value = trimmed["body:".len()..].trim();
            return (!value.is_empty()).then(|| value.to_owned());
        }
        if !starts_with_ignoring_case(trimmed, "view:") && !starts_with_ignoring_case(trimmed, "resource:") {
            break;
        }
    }
    None
}

/// The body a registration opens (the standalone host's `DiagramFilePair.BodyOf`): its `body:` header, resolved
/// against its folder and refused outside the project, else the file beside it named after it with its tool type's
/// extension.
fn route_registration(root: &Path, registration: &Path, text: &str, origin: &str) -> Result<PathBuf, String> {
    let folder = registration.parent().unwrap_or(root);
    match body_header(text) {
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
            Ok(folder.join(format!("{base}{}", extension_of(origin))))
        }
    }
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
