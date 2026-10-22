//! The Azure DevOps pipeline's residue (etalii.adp spec 030, FR-018, FR-031 and FR-032, task T078): the tool type
//! `azure-devops/pipeline` and its `.yml` documents, after the standalone host's
//! `src/diagrams/azure-devops-pipeline/backend/EtAlii.Adp.Diagram.AzureDevOpsPipeline` at `8b6ccfc`.
//!
//! The standalone host read, drew and edited a pipeline with its module's own code: its YAML reader that follows
//! templates, its graph of what waits for what, its layered layout, its writer that splices only the lines of
//! `displayName`, `dependsOn` and `enabled`, and one command per edit. The vendored `azure-devops-pipeline.dis` names a
//! binding, `azure-devops-pipeline.fbl`, that is not at the reference commit (only its draft is), so the pipeline is
//! routed through its module here too: [`PipelineModule`] [opens its documents itself](adp_diagram::DiagramModule::opens_documents),
//! and each open view is a [`view::PipelineView`], which the session hands every action and property edit.
//!
//! `.yml` is shared with every other YAML file a repository holds, so a pipeline is never claimed on sight: a body is
//! a pipeline when a registration names this tool type (`Diagram.cs`, `SharedExtension: true`), the registration's
//! `body:` header, else the `.yml` beside it named after it.
//!
//! - [`lines`], [`yaml`], [`model`], [`parser`] and [`templates`]: the file, read with the lines each element occupies
//!   and the templates it follows.
//! - [`writer`], [`blocks`], [`edits`] and [`commands`]: every edit, as a command on the project's history.
//! - [`graph`], [`layout`] and [`mapper`]: what waits for what, where everything is drawn, and what the shell draws.
//! - [`actions`], [`properties`], [`toolbox`] and [`rules`]: the menus, the property rows, the palette and the
//!   findings.
//! - [`store`]: one reading per pipeline, shared by every view of it, kept in the project's documents.

pub mod actions;
pub mod blocks;
pub mod commands;
pub mod edits;
pub mod graph;
pub mod layout;
pub mod lines;
pub mod mapper;
pub mod model;
pub mod parser;
pub mod properties;
pub mod rules;
pub mod store;
pub mod templates;
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

use crate::view::PipelineView;

/// The tool type this module serves.
pub const ORIGIN: &str = "azure-devops/pipeline";

/// The extension of a pipeline's body.
pub const EXTENSION: &str = ".yml";

/// How many lines after its first a registration's headers are looked for in.
const HEADER_SCAN_LIMIT: usize = 8;

/// What opening a `.yml` no registration names answers: a YAML file is a pipeline only when registered as one.
pub const NOT_REGISTERED: &str =
    "A .yml file is a pipeline diagram only once it is registered as one: many YAML files are not pipelines.";

/// The body a new pipeline starts from (`ServiceCollection.AddAzurePipeline.cs`, `NewPipeline`): a trigger, a pool
/// and one job with one step, the smallest pipeline that runs, with the file's name as its first comment. Written
/// with CRLF, as the C# raw string literal is in the standalone host's checkout.
pub fn new_pipeline(base_name: &str) -> String {
    [
        format!("# {base_name}"),
        String::new(),
        "trigger:".to_owned(),
        "  - main".to_owned(),
        String::new(),
        "pool:".to_owned(),
        "  vmImage: ubuntu-latest".to_owned(),
        String::new(),
        "jobs:".to_owned(),
        "  - job: Build".to_owned(),
        "    displayName: Build".to_owned(),
        "    steps:".to_owned(),
        "      - script: echo \"Replace this with your build\"".to_owned(),
        "        displayName: Build".to_owned(),
    ]
    .iter()
    .map(|line| format!("{line}\r\n"))
    .collect()
}

/// The pipeline module.
#[derive(Debug, Clone, Copy, Default)]
pub struct PipelineModule;

/// `tools` with the pipeline's module added to its tool type.
pub fn register(tools: DiagramTools) -> DiagramTools {
    tools.with_module(ORIGIN, Rc::new(PipelineModule))
}

impl DiagramModule for PipelineModule {
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
                beside.is_file() && fs::read_to_string(&beside).is_ok_and(|text| first_line(&text) == ORIGIN);
            if !registered {
                return Err(NOT_REGISTERED.to_owned());
            }
            beside
        };
        let body_path = route_registration(root, &registration)?;
        if !body_path.is_file() {
            return Err(format!("The body of '{}' is missing.", file_name(&registration)));
        }
        store::get_or_load(documents, root, &body_path);
        let view: Rc<dyn ModuleView> = Rc::new(PipelineView::new(root, &body_path));
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

/// The body a registration opens (the standalone host's `DiagramFilePair.BodyOf`): its `body:` header, resolved
/// against its folder and refused outside the project, else the `.yml` beside it named after it.
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
