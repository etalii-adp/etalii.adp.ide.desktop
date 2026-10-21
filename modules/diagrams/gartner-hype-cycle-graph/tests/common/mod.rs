//! A hype cycle graph opened in a project of its own, through the vendored catalogue with the module registered, as
//! the standalone module's tests opened one through its store and its history (`GhgTestDispatcher.cs`,
//! `GhgModuleFiles.cs` at `8b6ccfc`).
//!
//! The fixtures are the standalone host's own, copied unchanged: its nine examples
//! (`src/diagrams/gartner-hype-cycle-graph/examples/<name>/<name>.ghg`), its scale fixture
//! (`src/diagrams/gartner-hype-cycle-graph/scale-fixture.json`) and one of its test fixtures
//! (`backend/EtAlii.Adp.Diagram.GartnerHypeCycleGraph.Tests/Fixtures/triggers-and-notes.ghg`).

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use adp_context::Prompt;
use adp_core::Catalogue;
use adp_diagram::{ActionOutcome, ActionTarget, DiagramSession, DiagramTools, DiagramWorkspace, Resize, ResizedEdge};
use adp_diagram_hype_cycle::scale::{self, TimeUnit};
use adp_disl::DislPluginFunctions;
use adp_disl::behavior::ids;

pub fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// The text of the standalone host's example `name`.
pub fn example(name: &str) -> String {
    fs::read_to_string(fixtures().join("examples").join(format!("{name}.ghg"))).unwrap()
}

/// The standalone host's nine examples, by name.
pub const EXAMPLES: [&str; 9] = [
    "coal-technologies",
    "digital-trends",
    "electric-vehicles",
    "energy-breakthroughs",
    "eras-of-innovation",
    "internet-evolution",
    "llms-and-agents",
    "technology-trends",
    "warfare-in-ukraine",
];

/// The text of the standalone host's test fixture `name`.
pub fn fixture(name: &str) -> String {
    fs::read_to_string(fixtures().join(format!("{name}.ghg"))).unwrap()
}

/// The month index of `year` and `month`.
pub fn m(year: i64, month: i64) -> i64 {
    scale::month_index(year, month)
}

/// The x of the start of `year`, in a diagram drawn in `unit`.
pub fn x(year: i64, unit: TimeUnit) -> f64 {
    scale::x_of(m(year, 1), unit)
}

/// The tool types of the vendored catalogue, the hype cycle graph's with its module.
pub fn tools() -> DiagramTools {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    DiagramTools::new(Catalogue::load(&vendor), |_| DislPluginFunctions::none())
        .with_module(adp_diagram_hype_cycle::ORIGIN, adp_diagram_hype_cycle::module())
}

pub struct Graph {
    _folder: tempfile::TempDir,
    pub body: PathBuf,
    pub workspace: DiagramWorkspace,
    pub session: DiagramSession,
}

/// `text` as the body `graph.ghg` of a project, opened.
pub fn open(text: &str) -> Graph {
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join("graph.ghg");
    fs::write(&body, text).unwrap();
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (_, mut session) = workspace.open(&mut tools(), Path::new("graph.ghg")).unwrap();
    session.baseline(workspace.documents()).unwrap();
    Graph {
        _folder: folder,
        body,
        workspace,
        session,
    }
}

pub fn element(id: &str) -> ActionTarget {
    ActionTarget::Element(id.to_owned())
}

pub fn canvas(x: f64, y: f64) -> ActionTarget {
    ActionTarget::Canvas { x, y }
}

pub fn connection(source: &str, target: &str) -> ActionTarget {
    ActionTarget::Connection {
        source: Some(source.to_owned()),
        target: Some(target.to_owned()),
    }
}

impl Graph {
    pub fn text(&self) -> String {
        fs::read_to_string(&self.body).unwrap()
    }

    pub fn bytes(&self) -> Vec<u8> {
        fs::read(&self.body).unwrap()
    }

    /// Runs `action` on `target`, confirming what it asks to confirm; the refusal when it is refused.
    pub fn run(&mut self, target: &ActionTarget, action: &str, new_ids: &[&str]) -> Result<(), String> {
        match self
            .session
            .execute(&mut self.workspace, target, action, &ids::fixed(new_ids))
        {
            ActionOutcome::Done => Ok(()),
            ActionOutcome::Refused(refusal) => Err(refusal),
            ActionOutcome::Prompt(Prompt::Confirm { .. }) => self.answer(target, action, ""),
            ActionOutcome::Prompt(prompt) => Err(format!("it asked: {prompt:?}")),
        }
    }

    /// Runs `action` on `target`, answering its prompt with `value`.
    pub fn run_answered(&mut self, target: &ActionTarget, action: &str, value: &str) -> Result<(), String> {
        match self
            .session
            .execute(&mut self.workspace, target, action, &ids::fixed(&[]))
        {
            ActionOutcome::Prompt(_) => self.answer(target, action, value),
            ActionOutcome::Refused(refusal) => Err(refusal),
            ActionOutcome::Done => Err("it asked nothing".to_owned()),
        }
    }

    pub fn answer(&mut self, target: &ActionTarget, action: &str, value: &str) -> Result<(), String> {
        match self.session.answer(&mut self.workspace, target, action, value) {
            ActionOutcome::Done => Ok(()),
            ActionOutcome::Refused(refusal) => Err(refusal),
            ActionOutcome::Prompt(prompt) => Err(format!("it asked again: {prompt:?}")),
        }
    }

    pub fn set(&mut self, id: &str, row: &str, value: &str) -> Result<(), String> {
        self.session.set_property(&mut self.workspace, &[id], row, value)
    }

    pub fn move_to(&mut self, id: &str, x: f64, y: f64) -> Result<(), String> {
        self.session
            .move_elements(&mut self.workspace, &[(id.to_owned(), x, y)])
    }

    /// A drag of `id`'s `edge` that leaves its box at (`x`, `y`) and `width` by `height` canvas units.
    pub fn resize(
        &mut self,
        id: &str,
        edge: ResizedEdge,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Result<(), String> {
        let resize = Resize {
            edge,
            x,
            y,
            right: x + width,
            bottom: y + height,
            width,
            height,
        };
        self.session.resize_element(&mut self.workspace, id, &resize)
    }

    /// Where `id` is drawn: a trend's and a note's top-left corner, a trigger's centre.
    pub fn position(&self, id: &str) -> (f64, f64) {
        let elements = self.session.render(self.workspace.documents()).unwrap();
        let element = elements.iter().find(|e| e.id == id).unwrap();
        (element.x, element.y)
    }

    /// The document's DISL model as it is now.
    pub fn diagram(&self) -> adp_disl::DislDiagram {
        self.workspace
            .documents()
            .get(&self.body)
            .unwrap()
            .model()
            .diagram
            .clone()
    }

    /// The value of the property row `row` of `id`, as the grid shows it.
    pub fn row(&self, id: &str, row: &str) -> Option<String> {
        self.session
            .properties(self.workspace.documents(), &[id])
            .into_iter()
            .find(|r| r.id == row)
            .map(|r| r.value)
    }

    pub fn undo(&mut self) {
        assert!(self.workspace.undo().is_done());
    }

    pub fn redo(&mut self) {
        assert!(self.workspace.redo().is_done());
    }
}
