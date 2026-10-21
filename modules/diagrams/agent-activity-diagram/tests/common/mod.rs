//! An activity file opened in a project of its own, through the vendored catalogue with the module registered, as the
//! standalone module's tests opened one through its store, its history and its providers (`AadEntries`,
//! `AadCommandsTests.Services`, `AadDocumentTests.Fixture`).

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use adp_core::Catalogue;
use adp_diagram::{ActionOutcome, ActionTarget, DiagramSession, DiagramTools, DiagramWorkspace};
use adp_diagram_agent_activity::body::AadBody;
use adp_diagram_agent_activity::commands::{AadCommand, AadEdit};
use adp_diagram_agent_activity::model::AadModel;
use adp_diagram_agent_activity::store::{self, AadDocument};
use adp_disl::DislPluginFunctions;
use adp_disl::behavior::ids;
use adp_history::Outcome;

/// The tool types of the vendored catalogue, the agent activity diagram's with its module.
pub fn tools() -> DiagramTools {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    adp_diagram_agent_activity::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
        DislPluginFunctions::none()
    }))
}

/// The folder of the module's test fixtures: the standalone module's `Fixtures/` and its three examples.
pub fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// The text of a fixture.
pub fn fixture(name: &str) -> String {
    fs::read_to_string(fixtures().join(name)).unwrap()
}

/// The one-day fixture's text.
pub fn one_day() -> String {
    fixture("one-day.aad")
}

/// A document made from a text, as the store makes one, for tests that need one without a file.
pub fn read(text: &str) -> AadDocument {
    AadDocument::of(Path::new("body.aad"), text)
}

/// A text's lines as the file splits them.
pub fn lines(text: &str) -> Vec<String> {
    text.split("\r\n").map(str::to_owned).collect()
}

pub struct Diagram {
    pub folder: tempfile::TempDir,
    pub body: PathBuf,
    pub workspace: DiagramWorkspace,
    pub session: DiagramSession,
}

/// `text` as `work.aad`, opened as an agent activity diagram without a registration.
pub fn open(text: &str) -> Diagram {
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join("work.aad");
    fs::write(&body, text).unwrap();
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (opened, mut session) = workspace.open(&mut tools(), Path::new("work.aad")).unwrap();
    assert_eq!("etalii/agent-activity-diagram", opened.origin);
    session.baseline(workspace.documents()).unwrap();
    Diagram {
        folder,
        body,
        workspace,
        session,
    }
}

/// The one-day fixture, opened.
pub fn open_one_day() -> Diagram {
    open(&one_day())
}

impl Diagram {
    pub fn text(&self) -> String {
        fs::read_to_string(&self.body).unwrap()
    }

    /// The body as it is on disk, read.
    pub fn model(&self) -> AadModel {
        AadModel::read(&AadBody::parse(&self.text()))
    }

    /// The document as the store holds it.
    pub fn document(&self) -> &AadDocument {
        store::get(self.workspace.documents(), &self.body).unwrap()
    }

    /// Runs `edit` through the project's history.
    pub fn run(&mut self, edit: AadEdit) -> Result<(), String> {
        let command = AadCommand::new(&self.body, edit);
        match self.workspace.execute_command(&command) {
            Outcome::Done { .. } => Ok(()),
            Outcome::Refused(refusal) => Err(refusal.0),
        }
    }

    pub fn undo(&mut self) -> Result<(), String> {
        match self.workspace.undo() {
            Outcome::Done { .. } => Ok(()),
            Outcome::Refused(refusal) => Err(refusal.0),
        }
    }

    pub fn redo(&mut self) -> Result<(), String> {
        match self.workspace.redo() {
            Outcome::Done { .. } => Ok(()),
            Outcome::Refused(refusal) => Err(refusal.0),
        }
    }

    /// Runs `action` on `target`, what it creates given ShortGuids.
    pub fn execute(&mut self, target: &ActionTarget, action: &str) -> ActionOutcome {
        self.session
            .execute(&mut self.workspace, target, action, &ids::short_guids())
    }

    pub fn answer(&mut self, target: &ActionTarget, action: &str, value: &str) -> ActionOutcome {
        self.session.answer(&mut self.workspace, target, action, value)
    }

    /// Writes `text` as an agent would, and lets the project see it.
    pub fn agent_writes(&mut self, text: &str) {
        fs::write(&self.body, text).unwrap();
        let body = self.body.clone();
        self.workspace.reconcile(&body);
    }
}

pub fn element(id: &str) -> ActionTarget {
    ActionTarget::Element(id.to_owned())
}

/// The ids of a menu's actions, in order.
pub fn ids_of(groups: &[adp_context::ActionGroup]) -> Vec<String> {
    groups.iter().flat_map(|g| &g.actions).map(|a| a.id.clone()).collect()
}
