//! A causal loop diagram opened in a project of its own, through the vendored catalogue with the module registered,
//! as the standalone module's tests read a body through `CausalLoopBody.Parse` and edited it through its store and
//! its history.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use adp_core::Catalogue;
use adp_diagram::{ActionOutcome, ActionTarget, DiagramProblem, DiagramSession, DiagramTools, DiagramWorkspace};
use adp_diagram_causal_loop::reading::Model;
use adp_disl::DislPluginFunctions;
use adp_disl::behavior::ids;

/// The tool types of the vendored catalogue, the causal loop diagram's with its module.
pub fn tools() -> DiagramTools {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    DiagramTools::new(Catalogue::load(&vendor), |_| DislPluginFunctions::none())
        .with_module(adp_diagram_causal_loop::ORIGIN, adp_diagram_causal_loop::module())
}

/// The folder of the module's test fixtures: the standalone module's `Fixtures/*.cld` and its three examples.
pub fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// The text of a fixture.
pub fn fixture(name: &str) -> String {
    fs::read_to_string(fixtures().join(name)).unwrap()
}

/// The standalone module's corpus: its fixtures, then its examples, each by its path under `tests/fixtures`.
pub fn corpus() -> Vec<String> {
    let mut fixtures: Vec<String> = fs::read_dir(fixtures())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".cld"))
        .collect();
    fixtures.sort();
    let mut examples: Vec<String> = ["business-philosophy", "on-call", "reference"]
        .iter()
        .map(|e| format!("examples/{e}/{e}.cld"))
        .collect();
    fixtures.append(&mut examples);
    fixtures
}

pub struct Diagram {
    _folder: tempfile::TempDir,
    pub body: PathBuf,
    pub registration: PathBuf,
    pub workspace: DiagramWorkspace,
    pub session: DiagramSession,
}

/// `text` as the body of a causal loop diagram registered with `registration`, opened.
pub fn open_with(text: &str, registration: &str) -> Diagram {
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join("feedback.cld");
    let adp = folder.path().join("feedback.adp");
    fs::write(&body, text).unwrap();
    fs::write(&adp, registration).unwrap();
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (_, mut session) = workspace.open(&mut tools(), Path::new("feedback.adp")).unwrap();
    session.baseline(workspace.documents()).unwrap();
    Diagram {
        body,
        registration: adp,
        _folder: folder,
        workspace,
        session,
    }
}

/// `text` as the body of a registered causal loop diagram, opened.
pub fn open(text: &str) -> Diagram {
    open_with(text, "systems/causal-loop-diagram\r\n")
}

/// The statements as a body, each line ended with CRLF, as the standalone tests joined them.
pub fn body(statements: &[&str]) -> String {
    statements.iter().map(|s| format!("{s}\r\n")).collect()
}

/// The model the module's layouts and cycle finder read from `statements`, through the binding.
pub fn model(statements: &[&str]) -> Model {
    open(&body(statements)).model()
}

/// A ring of `count` variables `v0`, `v1`, ... labelled `Variable n`, without links.
pub fn ring(count: usize) -> Vec<String> {
    (0..count).map(|i| format!("variable v{i} \"Variable {i}\"")).collect()
}

/// `statements` as a model.
pub fn model_of(statements: &[String]) -> Model {
    let statements: Vec<&str> = statements.iter().map(String::as_str).collect();
    model(&statements)
}

impl Diagram {
    pub fn text(&self) -> String {
        fs::read_to_string(&self.body).unwrap()
    }

    pub fn registration_text(&self) -> String {
        fs::read_to_string(&self.registration).unwrap()
    }

    pub fn model(&self) -> Model {
        Model::of(&self.workspace.documents().get(&self.body).unwrap().model().diagram)
    }

    /// Runs `action` on `target`, what it creates given stand-in ids, as the shell gives them: under the `natural`
    /// strategy an element is read back with its keys' id.
    pub fn execute(&mut self, target: &ActionTarget, action: &str) -> ActionOutcome {
        let ids = ids::fixed(&["stand-in-1", "stand-in-2", "stand-in-3", "stand-in-4"]);
        self.session.execute(&mut self.workspace, target, action, &ids)
    }

    pub fn answer(&mut self, target: &ActionTarget, action: &str, value: &str) -> ActionOutcome {
        self.session.answer(&mut self.workspace, target, action, value)
    }

    pub fn arrange(&mut self) -> ActionOutcome {
        self.execute(
            &ActionTarget::Canvas { x: 0.0, y: 0.0 },
            adp_diagram_causal_loop::ARRANGE,
        )
    }

    pub fn problems(&self) -> Vec<DiagramProblem> {
        self.session.problems(self.workspace.documents())
    }

    /// Where `id` is drawn: the centre of its box.
    pub fn position(&self, id: &str) -> (f64, f64) {
        let elements = self.session.render(self.workspace.documents()).unwrap();
        let element = elements.iter().find(|e| e.id == id).unwrap();
        (element.x, element.y)
    }
}
