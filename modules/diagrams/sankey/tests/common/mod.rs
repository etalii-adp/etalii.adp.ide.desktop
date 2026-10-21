//! What the Sankey module's tests share, after the standalone module's `SankeyTestFolder.cs`, `SankeyTestDispatcher.cs`
//! and `SankeyExamples.cs` at `8b6ccfc`: the fixtures and the three examples, a copy of a document in a folder of its
//! own, and the commands run against the project's documents as its dispatcher ran them against its store.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use adp_core::Catalogue;
use adp_diagram::{DiagramDocuments, DiagramSession, DiagramTools, DiagramWorkspace};
use adp_diagram_sankey::commands::{SankeyCommand, SankeyEdit};
use adp_diagram_sankey::lines::LineDocument;
use adp_diagram_sankey::model::SankeyModel;
use adp_diagram_sankey::parser;
use adp_disl::DislPluginFunctions;
use adp_history::{Applied, Command};

/// The folder of the module's test fixtures: the standalone module's `Fixtures/*.skv` and its three examples.
pub fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// The path of a fixture.
pub fn fixture(name: &str) -> PathBuf {
    fixtures().join(name)
}

/// The fictional company's income statement.
pub fn brightwater_coffee() -> PathBuf {
    example("brightwater-coffee")
}

/// The UK energy flows.
pub fn uk_energy() -> PathBuf {
    example("uk-energy")
}

/// The recent US graduates.
pub fn recent_graduates() -> PathBuf {
    example("recent-graduates")
}

fn example(name: &str) -> PathBuf {
    fixtures().join(format!("examples/{name}/{name}.skv"))
}

/// The three examples.
pub fn every_example() -> [PathBuf; 3] {
    [brightwater_coffee(), uk_energy(), recent_graduates()]
}

/// The document `text` as read.
pub fn parse(text: &str) -> SankeyModel {
    parser::parse(&LineDocument::parse(text))
}

/// The document at `path` as read.
pub fn load(path: &Path) -> SankeyModel {
    parse(&fs::read_to_string(path).unwrap())
}

/// A copy of one document in a folder of its own, removed afterwards (`SankeyTestFolder`).
pub struct TestFolder {
    folder: tempfile::TempDir,
    pub body: PathBuf,
}

impl TestFolder {
    pub fn new(source: &Path, name: &str) -> Self {
        let folder = tempfile::tempdir().unwrap();
        let body = folder.path().join(name);
        fs::copy(source, &body).unwrap();
        TestFolder { folder, body }
    }

    pub fn folder(&self) -> &Path {
        self.folder.path()
    }

    pub fn bytes(&self) -> Vec<u8> {
        fs::read(&self.body).unwrap()
    }

    pub fn model(&self) -> SankeyModel {
        load(&self.body)
    }
}

/// Runs commands against the project's documents, as the standalone module's dispatcher ran them against its store.
#[derive(Default)]
pub struct Dispatcher {
    pub documents: DiagramDocuments,
}

impl Dispatcher {
    /// Runs `command`: its undo, or why it was refused.
    pub fn dispatch(
        &mut self,
        command: &dyn Command<DiagramDocuments>,
    ) -> Result<Box<dyn Command<DiagramDocuments>>, String> {
        command
            .apply(&mut self.documents)
            .map(|Applied { inverse, .. }| inverse.expect("every edit has its undo"))
            .map_err(|refusal| refusal.0)
    }

    /// Runs `edit` on the body at `body`, which must be accepted.
    pub fn run(&mut self, body: &Path, edit: SankeyEdit) {
        let command = SankeyCommand::new(body, edit);
        if let Err(refusal) = self.dispatch(&command) {
            panic!("{refusal}");
        }
    }
}

/// The tool types of the vendored catalogue, the Sankey diagram's with its module.
pub fn tools() -> DiagramTools {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    adp_diagram_sankey::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
        DislPluginFunctions::none()
    }))
}

/// A Sankey diagram opened in a project of its own through its registration.
pub struct Diagram {
    pub folder: TestFolder,
    pub workspace: DiagramWorkspace,
    pub session: DiagramSession,
}

/// The document at `source` registered as `name` (`name.skv` and `name.adp`) and opened.
pub fn open(source: &Path, name: &str) -> Diagram {
    let folder = TestFolder::new(source, &format!("{name}.skv"));
    fs::write(
        folder.folder().join(format!("{name}.adp")),
        format!("etalii/sankey\nbody: {name}.skv\n"),
    )
    .unwrap();
    let mut workspace = DiagramWorkspace::new(folder.folder());
    let (_, mut session) = workspace.open(&mut tools(), Path::new(&format!("{name}.adp"))).unwrap();
    session.baseline(workspace.documents()).unwrap();
    Diagram {
        folder,
        workspace,
        session,
    }
}
