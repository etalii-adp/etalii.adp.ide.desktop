//! What the mind map's tests share: the corpus, and a temporary project holding it as a registered mind map, opened
//! through `adp-diagram` with this module added, as the shell opens it (the standalone host's
//! `Support/MindmapTestProject.cs` at `8b6ccfc`).

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use adp_core::Catalogue;
use adp_diagram::DiagramSession;
use adp_diagram::session::{ActionOutcome, ActionTarget};
use adp_diagram::tool::DiagramTools;
use adp_diagram::workspace::DiagramWorkspace;
use adp_diagram_mindmap::document::MindmapDocument;
use adp_disl::{DislPluginFunctions, IdSource};

/// `tests/fixtures/architecture.mm`: a genuine Freeplane 1.12.15 save, copied unchanged from the standalone host's
/// `EtAlii.Adp.Diagram.Mindmap.Tests/Fixtures/`.
pub fn corpus_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/architecture.mm")
}

pub fn corpus() -> String {
    fs::read_to_string(corpus_path()).expect("the corpus")
}

pub fn load(assign_missing_ids: bool) -> MindmapDocument {
    MindmapDocument::parse(&corpus(), assign_missing_ids).expect("the corpus parses")
}

/// The catalogue's tool types with the mind map's module.
pub fn tools() -> DiagramTools {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    adp_diagram_mindmap::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
        DislPluginFunctions::none()
    }))
}

/// Ids handed out in order, `ID_new1`, `ID_new2`, and so on.
#[derive(Default)]
pub struct CountingIds(std::cell::Cell<usize>);

impl IdSource for CountingIds {
    fn next(&self, _type_name: &str) -> Result<String, String> {
        self.0.set(self.0.get() + 1);
        Ok(format!("ID_new{}", self.0.get()))
    }
}

/// A temporary project with the corpus as `docs/architecture.mm`, registered by `docs/architecture.adp`, and open.
pub struct TestProject {
    pub folder: tempfile::TempDir,
    pub workspace: DiagramWorkspace,
    pub session: DiagramSession,
    pub ids: CountingIds,
}

impl TestProject {
    pub fn new() -> Self {
        Self::with_body(&corpus())
    }

    pub fn with_body(body: &str) -> Self {
        let folder = tempfile::tempdir().unwrap();
        fs::create_dir_all(folder.path().join("docs")).unwrap();
        fs::write(folder.path().join("docs/architecture.adp"), "freeplane/mindmap\n").unwrap();
        fs::write(folder.path().join("docs/architecture.mm"), body).unwrap();
        let mut tools = tools();
        let mut workspace = DiagramWorkspace::new(folder.path());
        let (_, mut session) = workspace
            .open(&mut tools, Path::new("docs/architecture.adp"))
            .expect("the map opens");
        session.baseline(workspace.documents()).unwrap();
        TestProject {
            folder,
            workspace,
            session,
            ids: CountingIds::default(),
        }
    }

    pub fn root(&self) -> &Path {
        self.folder.path()
    }

    pub fn body_path(&self) -> PathBuf {
        self.folder.path().join("docs/architecture.mm")
    }

    pub fn body(&self) -> String {
        fs::read_to_string(self.body_path()).unwrap()
    }

    /// The map as it is on disk now.
    pub fn document(&self) -> MindmapDocument {
        MindmapDocument::parse(&self.body(), false).unwrap()
    }

    pub fn execute(&mut self, node: &str, action: &str) -> ActionOutcome {
        self.session.execute(
            &mut self.workspace,
            &ActionTarget::Element(node.to_owned()),
            action,
            &self.ids,
        )
    }

    pub fn answer(&mut self, node: &str, action: &str, value: &str) -> ActionOutcome {
        self.session.answer(
            &mut self.workspace,
            &ActionTarget::Element(node.to_owned()),
            action,
            value,
        )
    }

    /// The ids of the actions `node`'s menu offers, in order.
    pub fn offered(&self, node: &str) -> Vec<String> {
        self.session
            .actions(self.workspace.documents(), &ActionTarget::Element(node.to_owned()))
            .into_iter()
            .flat_map(|g| g.actions)
            .map(|a| a.id)
            .collect()
    }

    pub fn can_undo(&self) -> bool {
        self.workspace.history().availability().can_undo
    }
}
