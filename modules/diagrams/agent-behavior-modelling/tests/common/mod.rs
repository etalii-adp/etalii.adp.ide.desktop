//! What the agent behavior model's tests share: the module's four examples, copied unchanged from the standalone
//! host's `src/diagrams/agent-behavior-modelling/examples/` at `8b6ccfc` (written for ADP, as their `readme.md` says),
//! and a temporary project holding one of them as a registered behavior model, opened through `adp-diagram` with this
//! module added, as the shell opens it (the standalone host's `AbmExamples.cs` and its session and command tests).

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use adp_core::Catalogue;
use adp_diagram::DiagramSession;
use adp_diagram::session::{ActionOutcome, ActionTarget};
use adp_diagram::tool::DiagramTools;
use adp_diagram::workspace::DiagramWorkspace;
use adp_diagram_agent_behavior_modelling::layout::{self, NODE_HEIGHT, NODE_WIDTH, Positions};
use adp_diagram_agent_behavior_modelling::model::Model;
use adp_diagram_agent_behavior_modelling::{plugin, registration};
use adp_disl::{DislPluginFunctions, IdSource};

/// The four example names.
pub const EXAMPLES: [&str; 4] = [
    "pull-request-reviewer",
    "customer-support",
    "bug-fixer",
    "research-assistant",
];

/// The Markdown body of the example `name`.
pub fn example(name: &str) -> String {
    fs::read_to_string(example_path(name, "md")).expect("the example")
}

fn example_path(name: &str, extension: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/examples")
        .join(name)
        .join(format!("{name}.{extension}"))
}

/// The model the parser reads from `text`.
pub fn parse(text: &str) -> Model {
    plugin::read_model(text.as_bytes())
}

/// The catalogue's tool types with this module.
pub fn tools() -> DiagramTools {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    adp_diagram_agent_behavior_modelling::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
        DislPluginFunctions::none()
    }))
}

/// Ids nothing uses: a behavior model's node is named by its place.
pub struct NoIds;

impl IdSource for NoIds {
    fn next(&self, _type_name: &str) -> Result<String, String> {
        Ok("new".to_owned())
    }
}

/// A temporary project with a behavior model as `agent.md`, registered by `agent.adp`, and open.
pub struct TestProject {
    pub folder: tempfile::TempDir,
    pub workspace: DiagramWorkspace,
    pub session: DiagramSession,
}

impl TestProject {
    /// The example `name`, with a registration that stores nothing.
    pub fn example(name: &str) -> Self {
        Self::with(&example(name), "etalii/agent-behavior-modelling\r\n")
    }

    pub fn with(body: &str, registration: &str) -> Self {
        let folder = tempfile::tempdir().unwrap();
        fs::write(folder.path().join("agent.adp"), registration).unwrap();
        fs::write(folder.path().join("agent.md"), body).unwrap();
        let mut tools = tools();
        let mut workspace = DiagramWorkspace::new(folder.path());
        let (_, mut session) = workspace
            .open(&mut tools, Path::new("agent.adp"))
            .expect("the behavior model opens");
        session.baseline(workspace.documents()).unwrap();
        TestProject {
            folder,
            workspace,
            session,
        }
    }

    pub fn body(&self) -> String {
        fs::read_to_string(self.folder.path().join("agent.md")).unwrap()
    }

    pub fn registration(&self) -> String {
        fs::read_to_string(self.folder.path().join("agent.adp")).unwrap()
    }

    pub fn model(&self) -> Model {
        parse(&self.body())
    }

    /// Where each node is drawn, as top-left corners, with the rows the registration stores.
    pub fn drawn(&self) -> Positions {
        layout::arrange(&self.model(), &registration::read(self.registration().as_bytes()))
    }

    pub fn execute(&mut self, target: ActionTarget, action: &str) -> ActionOutcome {
        self.session.execute(&mut self.workspace, &target, action, &NoIds)
    }

    pub fn answer(&mut self, target: ActionTarget, action: &str, value: &str) -> ActionOutcome {
        self.session.answer(&mut self.workspace, &target, action, value)
    }

    pub fn on(&mut self, node: &str, action: &str) -> ActionOutcome {
        self.execute(ActionTarget::Element(node.to_owned()), action)
    }

    pub fn property(&mut self, node: &str, row: &str, value: &str) -> Result<(), String> {
        self.session.set_property(&mut self.workspace, &[node], row, value)
    }

    /// A drag of `node` dropped with its top-left at (`x`, `y`), as the standalone host's canvas sent it; the
    /// session is handed the centre, as the desktop host's canvas sends it.
    pub fn drop_at(&mut self, node: &str, x: f64, y: f64) -> Result<(), String> {
        self.session.move_elements(
            &mut self.workspace,
            &[(node.to_owned(), x + NODE_WIDTH / 2.0, y + NODE_HEIGHT / 2.0)],
        )
    }

    pub fn undo(&mut self) {
        assert!(self.workspace.undo().is_done(), "there is something to undo");
    }

    pub fn redo(&mut self) {
        assert!(self.workspace.redo().is_done(), "there is something to redo");
    }

    pub fn can_undo(&self) -> bool {
        self.workspace.history().availability().can_undo
    }
}
