//! A timeline opened in a project of its own, through the vendored catalogue with the timeline's module registered,
//! as the standalone module's tests opened one through its store and its history.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use adp_core::Catalogue;
use adp_diagram::{ActionOutcome, ActionTarget, DiagramSession, DiagramTools, DiagramWorkspace};
use adp_disl::DislPluginFunctions;
use adp_disl::behavior::ids;

/// The module's own roadmap (`src/diagrams/timeline/examples/example-2/roadmap.tml` of the standalone host at
/// `8b6ccfc`): thirty elements scattered over seventeen rows.
pub fn roadmap() -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/roadmap.tml")).unwrap()
}

/// The tool types of the vendored catalogue, the timeline's with its module.
pub fn tools() -> DiagramTools {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    DiagramTools::new(Catalogue::load(&vendor), |_| DislPluginFunctions::none())
        .with_module(adp_diagram_timeline::ORIGIN, adp_diagram_timeline::module())
}

pub struct Timeline {
    _folder: tempfile::TempDir,
    pub body: PathBuf,
    pub workspace: DiagramWorkspace,
    pub session: DiagramSession,
}

/// `text` as the body of a registered timeline, opened.
pub fn open(text: &str) -> Timeline {
    let folder = tempfile::tempdir().unwrap();
    fs::write(folder.path().join("plan.tml"), text).unwrap();
    fs::write(folder.path().join("plan.adp"), "generic/timeline\r\n").unwrap();
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (_, mut session) = workspace.open(&mut tools(), Path::new("plan.adp")).unwrap();
    session.baseline(workspace.documents()).unwrap();
    Timeline {
        body: folder.path().join("plan.tml"),
        _folder: folder,
        workspace,
        session,
    }
}

pub fn element(id: &str) -> ActionTarget {
    ActionTarget::Element(id.to_owned())
}

impl Timeline {
    pub fn text(&self) -> String {
        fs::read_to_string(&self.body).unwrap()
    }

    pub fn execute(&mut self, target: &ActionTarget, action: &str, new_ids: &[&str]) -> ActionOutcome {
        self.session
            .execute(&mut self.workspace, target, action, &ids::fixed(new_ids))
    }

    pub fn answer(&mut self, target: &ActionTarget, action: &str, value: &str) -> ActionOutcome {
        self.session.answer(&mut self.workspace, target, action, value)
    }

    pub fn set(&mut self, id: &str, row: &str, value: &str) -> Result<(), String> {
        self.session.set_property(&mut self.workspace, &[id], row, value)
    }

    pub fn move_to(&mut self, id: &str, x: f64, y: f64) -> Result<(), String> {
        self.session
            .move_elements(&mut self.workspace, &[(id.to_owned(), x, y)])
    }

    /// Where `id` is drawn.
    pub fn position(&self, id: &str) -> (f64, f64) {
        let elements = self.session.render(self.workspace.documents()).unwrap();
        let element = elements.iter().find(|e| e.id == id).unwrap();
        (element.x, element.y)
    }

    pub fn undo(&mut self) {
        assert!(self.workspace.undo().is_done());
    }

    pub fn redo(&mut self) {
        assert!(self.workspace.redo().is_done());
    }
}
