//! A supply chain diagram opened in a project of its own, through the vendored catalogue with the module registered,
//! as the standalone module's tests opened a copy of one document through its store, its history and its providers
//! (`SupplyChainTestFolder`, `SupplyChainTestDispatcher`, `SupplyChainExamples`).

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use adp_core::Catalogue;
use adp_diagram::{ActionOutcome, ActionTarget, DiagramSession, DiagramTools, DiagramWorkspace};
use adp_diagram_supply_chain::lines::LineDocument;
use adp_diagram_supply_chain::model::SupplyChainModel;
use adp_diagram_supply_chain::parser;
use adp_disl::DislPluginFunctions;
use adp_disl::behavior::ids;

/// The tool types of the vendored catalogue, the supply chain's with its module.
pub fn tools() -> DiagramTools {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    adp_diagram_supply_chain::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
        DislPluginFunctions::none()
    }))
}

/// The folder of the module's test fixtures: the standalone module's `Fixtures/*.supply` and its two examples.
pub fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

pub fn fixture_path(name: &str) -> PathBuf {
    fixtures().join(name)
}

/// The text of a fixture.
pub fn fixture(name: &str) -> String {
    fs::read_to_string(fixture_path(name)).unwrap()
}

/// The automotive example's body.
pub const AUTOMOTIVE: &str = "examples/automotive/automotive.supply";

/// The GPU and memory example's body.
pub const GPU_MEMORY: &str = "examples/gpu-memory/gpu-memory.supply";

/// A fixture's lines.
pub fn load(name: &str) -> LineDocument {
    LineDocument::parse(&fixture(name))
}

/// A fixture read.
pub fn read(name: &str) -> SupplyChainModel {
    parser::parse(&load(name))
}

/// A text read.
pub fn read_text(text: &str) -> SupplyChainModel {
    parser::parse(&LineDocument::parse(text))
}

pub struct Diagram {
    _folder: tempfile::TempDir,
    pub body: PathBuf,
    pub workspace: DiagramWorkspace,
    pub session: DiagramSession,
}

/// A copy of fixture `name` as `small.supply`, opened as a supply chain without a registration.
pub fn open_fixture(name: &str) -> Diagram {
    open_bytes(&fs::read(fixture_path(name)).unwrap())
}

/// `bytes` as the body of a supply chain, opened.
pub fn open_bytes(bytes: &[u8]) -> Diagram {
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join("small.supply");
    fs::write(&body, bytes).unwrap();
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (_, mut session) = workspace.open(&mut tools(), Path::new("small.supply")).unwrap();
    session.baseline(workspace.documents()).unwrap();
    Diagram {
        body,
        _folder: folder,
        workspace,
        session,
    }
}

impl Diagram {
    pub fn bytes(&self) -> Vec<u8> {
        fs::read(&self.body).unwrap()
    }

    pub fn text(&self) -> String {
        fs::read_to_string(&self.body).unwrap()
    }

    /// The body as it is on disk, read.
    pub fn model(&self) -> SupplyChainModel {
        read_text(&self.text())
    }

    /// Runs `action` on `target`, what it creates given the ids `minted`.
    pub fn execute_minting(&mut self, target: &ActionTarget, action: &str, minted: &[&str]) -> ActionOutcome {
        let ids = ids::fixed(minted);
        self.session.execute(&mut self.workspace, target, action, &ids)
    }

    pub fn execute(&mut self, target: &ActionTarget, action: &str) -> ActionOutcome {
        self.execute_minting(target, action, &["minted-1", "minted-2"])
    }

    pub fn answer(&mut self, target: &ActionTarget, action: &str, value: &str) -> ActionOutcome {
        let ids = ids::fixed(&["minted-group"]);
        self.session
            .answer_with_ids(&mut self.workspace, target, action, value, &ids)
    }

    pub fn element(id: &str) -> ActionTarget {
        ActionTarget::Element(id.to_owned())
    }
}
