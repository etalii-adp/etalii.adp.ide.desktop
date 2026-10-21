//! A dependency graph opened in a project of its own, through the vendored catalogue with the dependency graph's
//! module registered, as the standalone module's tests opened one through its store and the project's history
//! (`DependencyGraphTestDispatcher.cs` at `8b6ccfc`).

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use adp_context::{Prompt, PropertyRow};
use adp_core::Catalogue;
use adp_diagram::{
    ActionOutcome, ActionTarget, DiagramElement, DiagramProblem, DiagramSession, DiagramTools, DiagramWorkspace,
};
use adp_disl::DislPluginFunctions;
use adp_disl::behavior::ids;

/// The graph most of the standalone module's tests start from: two nodes, one dependency, a comment.
pub const GRAPH: &str = "dependencies: 1
# A comment the commands must never disturb.
elements:
  - id: aaa
    label: API gateway
    x: 240
    row: 0
  - id: bbb
    label: Identity service
    x: 480
    row: 2
relations:
  - id: ccc
    from: aaa
    to: bbb
    label: verifies tokens with
";

/// The module's own example (`src/diagrams/dependency-graph/examples/example-1/services.dgr` of the standalone host
/// at `8b6ccfc`): ten services and eleven dependencies.
pub fn services() -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/services.dgr")).unwrap()
}

/// The tool types of the vendored catalogue, the dependency graph's with its module.
pub fn tools() -> DiagramTools {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    adp_diagram_dependency_graph::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
        DislPluginFunctions::none()
    }))
}

pub struct Graph {
    _folder: tempfile::TempDir,
    pub body: PathBuf,
    pub workspace: DiagramWorkspace,
    pub session: DiagramSession,
}

/// `text` as the body of a registered dependency graph, opened.
pub fn open(text: &str) -> Graph {
    try_open(text).unwrap()
}

/// `text` as the body of a registered dependency graph, opened, or why it does not open.
pub fn try_open(text: &str) -> Result<Graph, String> {
    let folder = tempfile::tempdir().unwrap();
    fs::write(folder.path().join("services.dgr"), text).unwrap();
    fs::write(folder.path().join("services.adp"), "generic/dependencies\n").unwrap();
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (_, mut session) = workspace.open(&mut tools(), Path::new("services.adp"))?;
    session.baseline(workspace.documents())?;
    Ok(Graph {
        body: folder.path().join("services.dgr"),
        _folder: folder,
        workspace,
        session,
    })
}

pub fn element(id: &str) -> ActionTarget {
    ActionTarget::Element(id.to_owned())
}

pub fn canvas(x: f64, y: f64) -> ActionTarget {
    ActionTarget::Canvas { x, y }
}

impl Graph {
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

    /// Runs `action` on `target` and answers the prompt it raises with `value`.
    pub fn ask_and_answer(&mut self, target: &ActionTarget, action: &str, value: &str) -> ActionOutcome {
        match self.execute(target, action, &[]) {
            ActionOutcome::Prompt(_) => self.answer(target, action, value),
            other => other,
        }
    }

    /// The prompt `action` on `target` raises.
    pub fn prompt(&mut self, target: &ActionTarget, action: &str) -> Prompt {
        match self.execute(target, action, &[]) {
            ActionOutcome::Prompt(prompt) => prompt,
            other => panic!("{action} did not ask: {other:?}"),
        }
    }

    pub fn set(&mut self, id: &str, row: &str, value: &str) -> Result<(), String> {
        self.session.set_property(&mut self.workspace, &[id], row, value)
    }

    pub fn rows(&self, id: &str) -> Vec<PropertyRow> {
        self.session.properties(self.workspace.documents(), &[id])
    }

    pub fn row(&self, id: &str, row: &str) -> PropertyRow {
        self.rows(id).into_iter().find(|r| r.id == row).unwrap()
    }

    pub fn action_ids(&self, target: &ActionTarget) -> Vec<String> {
        self.session
            .actions(self.workspace.documents(), target)
            .into_iter()
            .flat_map(|group| group.actions)
            .map(|action| action.id)
            .collect()
    }

    pub fn move_to(&mut self, id: &str, x: f64, y: f64) -> Result<(), String> {
        self.session
            .move_elements(&mut self.workspace, &[(id.to_owned(), x, y)])
    }

    pub fn connect(&mut self, source: &str, target: &str, new_id: &str) -> Result<(), String> {
        self.session
            .connect(&mut self.workspace, "DependsOn", source, target, &ids::fixed(&[new_id]))
    }

    pub fn connect_to_new(&mut self, source: &str, x: f64, y: f64, new_ids: &[&str]) -> Result<(), String> {
        self.session
            .connect_to_new(&mut self.workspace, "DependsOn", source, x, y, &ids::fixed(new_ids))
    }

    pub fn elements(&self) -> Vec<DiagramElement> {
        self.session.render(self.workspace.documents()).unwrap()
    }

    pub fn find(&self, id: &str) -> DiagramElement {
        self.elements().into_iter().find(|e| e.id == id).unwrap()
    }

    /// `id`'s attribute `name` as text.
    pub fn attribute(&self, id: &str, name: &str) -> String {
        self.find(id)
            .attributes
            .into_iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v)
            .unwrap_or_default()
    }

    pub fn problems(&self) -> Vec<DiagramProblem> {
        self.session.problems(self.workspace.documents())
    }

    pub fn undo(&mut self) {
        assert!(self.workspace.undo().is_done());
    }

    pub fn redo(&mut self) {
        assert!(self.workspace.redo().is_done());
    }
}
