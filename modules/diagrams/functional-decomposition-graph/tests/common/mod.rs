//! What the tests share: the fixtures, copied from the standalone host's
//! `backend/EtAlii.Adp.Diagram.FunctionalDecompositionGraph.Tests/Fixtures/` at `8b6ccfc`, and the module's own
//! field-service example (`examples/field-service/`), byte for byte.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use adp_diagram_functional_decomposition_graph::lines::LineDocument;
use adp_diagram_functional_decomposition_graph::model::FdgModel;
use adp_diagram_functional_decomposition_graph::parser;

/// The fixture `name`.
pub fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

pub fn bytes(name: &str) -> Vec<u8> {
    fs::read(fixture_path(name)).unwrap()
}

pub fn load(name: &str) -> LineDocument {
    LineDocument::parse(&String::from_utf8(bytes(name)).unwrap())
}

pub fn model(name: &str) -> FdgModel {
    parser::parse(&load(name))
}

/// The field-service example's body; a missing example fails rather than skips.
pub fn example_path() -> PathBuf {
    let path = fixture_path("field-service/field-service.fdg");
    assert!(
        path.is_file(),
        "The field-service example is missing: {}",
        path.display()
    );
    path
}

/// A project folder holding a copy of the field-service example, and the copy's body.
pub fn example_project() -> (tempfile::TempDir, PathBuf) {
    let folder = tempfile::tempdir().unwrap();
    for name in ["field-service.fdg", "field-service.adp"] {
        fs::copy(fixture_path(&format!("field-service/{name}")), folder.path().join(name)).unwrap();
    }
    let body = folder.path().join("field-service.fdg");
    (folder, body)
}

/// The example as it reads now.
pub fn parse_file(path: &Path) -> FdgModel {
    parser::parse(&LineDocument::parse(&fs::read_to_string(path).unwrap()))
}

use adp_context::Prompt;
use adp_core::Catalogue;
use adp_diagram::{ActionOutcome, ActionTarget, DiagramSession, DiagramTools, DiagramWorkspace};
use adp_disl::DislPluginFunctions;

/// The tool types of the vendored catalogue, the functional decomposition graph's with its module.
pub fn tools() -> DiagramTools {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    adp_diagram_functional_decomposition_graph::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
        DislPluginFunctions::none()
    }))
}

/// The field-service example, copied into a project of its own and opened through its registration.
pub struct Graph {
    pub folder: tempfile::TempDir,
    pub body: PathBuf,
    pub original: Vec<u8>,
    pub workspace: DiagramWorkspace,
    pub session: DiagramSession,
}

impl Graph {
    pub fn example() -> Graph {
        let (folder, body) = example_project();
        Graph::open(folder, body, "field-service.adp")
    }

    /// `text` as the body of a registered graph, opened.
    pub fn of(text: &str) -> Graph {
        let folder = tempfile::tempdir().unwrap();
        let body = folder.path().join("graph.fdg");
        fs::write(&body, text).unwrap();
        fs::write(
            folder.path().join("graph.adp"),
            "etalii/functional-decomposition-graph\nbody: graph.fdg\n",
        )
        .unwrap();
        Graph::open(folder, body, "graph.adp")
    }

    fn open(folder: tempfile::TempDir, body: PathBuf, registration: &str) -> Graph {
        let mut workspace = DiagramWorkspace::new(folder.path());
        let (_, mut session) = workspace.open(&mut tools(), Path::new(registration)).unwrap();
        session.baseline(workspace.documents()).unwrap();
        Graph {
            original: fs::read(&body).unwrap(),
            folder,
            body,
            workspace,
            session,
        }
    }

    pub fn bytes(&self) -> Vec<u8> {
        fs::read(&self.body).unwrap()
    }

    pub fn model(&self) -> FdgModel {
        parse_file(&self.body)
    }

    /// Runs `action_id` on `target`, minting ids as the shell does.
    pub fn execute(&mut self, target: ActionTarget, action_id: &str) -> ActionOutcome {
        let view = self.session.module_view().expect("a module's view").clone();
        view.execute(&mut self.workspace, &target, action_id)
    }

    pub fn answer(&mut self, target: ActionTarget, action_id: &str, value: &str) -> ActionOutcome {
        self.session.answer(&mut self.workspace, &target, action_id, value)
    }

    pub fn prompt(&mut self, target: ActionTarget, action_id: &str) -> Prompt {
        match self.execute(target, action_id) {
            ActionOutcome::Prompt(prompt) => prompt,
            other => panic!("{action_id} did not ask: {other:?}"),
        }
    }

    pub fn set(&mut self, id: &str, property: &str, value: &str) -> Result<(), String> {
        self.session.set_property(&mut self.workspace, &[id], property, value)
    }

    pub fn undo(&mut self) {
        assert!(self.workspace.undo().is_done());
    }
}

pub fn item(id: &str) -> ActionTarget {
    ActionTarget::Element(id.to_owned())
}

pub fn placement(x: f64, y: f64) -> ActionTarget {
    ActionTarget::Canvas { x, y }
}

pub fn gesture(from: &str, to: &str) -> ActionTarget {
    ActionTarget::Connection {
        source: Some(from.to_owned()),
        target: Some(to.to_owned()),
    }
}
