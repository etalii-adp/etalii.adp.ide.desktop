//! The pipeline module's test fixtures, read and opened as the standalone module's tests read and opened them
//! (`Fixtures/`, `PipelineTestDispatcher`, `ViewedPipelineDocumentStore`): a fixture parsed alone or with its
//! templates, and a copy of one opened through the vendored catalogue with the module registered.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use adp_core::Catalogue;
use adp_diagram::{ActionOutcome, ActionTarget, DiagramSession, DiagramTools, DiagramWorkspace};
use adp_diagram_azure_devops_pipeline::lines::LineDocument;
use adp_diagram_azure_devops_pipeline::model::PipelineModel;
use adp_diagram_azure_devops_pipeline::parser;
use adp_diagram_azure_devops_pipeline::templates::PipelineTemplates;
use adp_disl::DislPluginFunctions;

/// The tool types of the vendored catalogue, the pipeline's with its module.
pub fn tools() -> DiagramTools {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    adp_diagram_azure_devops_pipeline::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
        DislPluginFunctions::none()
    }))
}

/// The folder of the module's test fixtures: the standalone module's `Fixtures/`.
pub fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// The text of a fixture.
pub fn fixture(name: &str) -> String {
    fs::read_to_string(fixtures().join(name)).unwrap()
}

/// A fixture's lines.
pub fn document(name: &str) -> LineDocument {
    LineDocument::parse(&fixture(name))
}

/// A fixture read, no template followed.
pub fn parse(name: &str) -> PipelineModel {
    parser::parse(&document(name)).unwrap()
}

/// A text read, no template followed.
pub fn parse_text(text: &str) -> PipelineModel {
    parser::parse(&LineDocument::parse(text)).unwrap()
}

/// A fixture read with the templates the fixtures folder holds.
pub fn parse_with_templates(name: &str) -> PipelineModel {
    let root = fixtures();
    let path = root.join(name);
    parser::parse_with(&document(name), &PipelineTemplates::new(&root), &path).unwrap()
}

/// Every `.yml` fixture, by its path relative to the fixtures folder.
pub fn all_fixtures() -> Vec<String> {
    let mut found = Vec::new();
    let mut pending = vec![fixtures()];
    while let Some(folder) = pending.pop() {
        for entry in fs::read_dir(&folder).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|e| e == "yml") {
                found.push(
                    path.strip_prefix(fixtures())
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    found.sort();
    found
}

/// A pipeline opened in a project of its own, through its registration.
pub struct Pipeline {
    pub folder: tempfile::TempDir,
    pub body: PathBuf,
    pub workspace: DiagramWorkspace,
    pub session: DiagramSession,
}

/// A copy of the fixtures folder as a project, with fixture `name` registered as a pipeline and opened.
pub fn open_fixture(name: &str) -> Pipeline {
    let folder = tempfile::tempdir().unwrap();
    copy_folder(&fixtures(), folder.path());
    open_in(folder, name)
}

/// `text` as the body `pipeline.yml` of a project of its own, registered and opened.
pub fn open_text(text: &str) -> Pipeline {
    let folder = tempfile::tempdir().unwrap();
    fs::write(folder.path().join("pipeline.yml"), text).unwrap();
    open_in(folder, "pipeline.yml")
}

fn open_in(folder: tempfile::TempDir, name: &str) -> Pipeline {
    let body = folder.path().join(name);
    let registration = body.with_extension("adp");
    fs::write(&registration, "azure-devops/pipeline\r\n").unwrap();
    let mut workspace = DiagramWorkspace::new(folder.path());
    let relative = registration.strip_prefix(folder.path()).unwrap().to_path_buf();
    let (_, mut session) = workspace.open(&mut tools(), &relative).unwrap();
    session.baseline(workspace.documents()).unwrap();
    Pipeline {
        folder,
        body,
        workspace,
        session,
    }
}

fn copy_folder(from: &Path, to: &Path) {
    for entry in fs::read_dir(from).unwrap().flatten() {
        let path = entry.path();
        let target = to.join(entry.file_name());
        if path.is_dir() {
            fs::create_dir_all(&target).unwrap();
            copy_folder(&path, &target);
        } else {
            fs::copy(&path, &target).unwrap();
        }
    }
}

impl Pipeline {
    pub fn text(&self) -> String {
        fs::read_to_string(&self.body).unwrap()
    }

    /// The body as it is on disk, read.
    pub fn model(&self) -> PipelineModel {
        parse_text(&self.text())
    }

    pub fn execute(&mut self, id: &str, action: &str) -> ActionOutcome {
        let target = ActionTarget::Element(id.to_owned());
        let ids = adp_disl::behavior::ids::fixed(&[]);
        self.session.execute(&mut self.workspace, &target, action, &ids)
    }

    pub fn answer(&mut self, id: &str, action: &str, value: &str) -> ActionOutcome {
        let target = ActionTarget::Element(id.to_owned());
        self.session.answer(&mut self.workspace, &target, action, value)
    }

    pub fn set(&mut self, id: &str, property: &str, value: &str) -> Result<(), String> {
        self.session.set_property(&mut self.workspace, &[id], property, value)
    }

    pub fn undo(&mut self) -> bool {
        self.workspace.undo().is_done()
    }

    pub fn redo(&mut self) -> bool {
        self.workspace.redo().is_done()
    }

    /// The ids of every action the element offers.
    pub fn action_ids(&self, id: &str) -> Vec<String> {
        let target = ActionTarget::Element(id.to_owned());
        self.session
            .actions(self.workspace.documents(), &target)
            .into_iter()
            .flat_map(|g| g.actions)
            .map(|a| a.id)
            .collect()
    }
}

/// Whether a line carries a comment and nothing before it (`Line.IsComment`).
pub fn is_comment(text: &str) -> bool {
    text.trim_start().starts_with('#')
}
