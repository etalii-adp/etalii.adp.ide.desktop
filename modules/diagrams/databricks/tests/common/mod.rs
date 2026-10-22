//! What the Databricks module's tests share: the round-trip corpus the standalone host's tests read, copied from
//! `src/diagrams/databricks/backend/EtAlii.Adp.Diagram.Databricks.Tests/Fixtures/` at `8b6ccfc`.

#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;

use adp_diagram_databricks::lines::LineDocument;
use adp_diagram_databricks::model::DatabricksModel;
use adp_diagram_databricks::parser;

/// The folder of the corpus.
pub fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// The lakehouse example the parity scripts open, as the standalone host's module ships it.
pub fn lakehouse() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../parity/fixtures/databricks/lakehouse")
}

/// The text of the corpus file `name`, byte for byte.
pub fn text(name: &str) -> String {
    String::from_utf8(fs::read(fixtures().join(name)).expect("a fixture")).expect("UTF-8")
}

/// The corpus file `name` as its lines.
pub fn document(name: &str) -> LineDocument {
    LineDocument::parse(&text(name))
}

/// The three readings of `document`.
pub fn model(document: &LineDocument) -> DatabricksModel {
    parser::read(document).expect("it reads")
}

use std::path::Path;

use adp_core::Catalogue;
use adp_diagram::{DiagramSession, DiagramTools, DiagramWorkspace};
use adp_disl::DislPluginFunctions;

/// The tool types of the vendored catalogue, the family's three with their module.
pub fn tools() -> DiagramTools {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    adp_diagram_databricks::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
        DislPluginFunctions::none()
    }))
}

/// Copies `from` into `to`, folders and all.
pub fn copy_folder(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            copy_folder(&path, &to.join(entry.file_name()));
        } else {
            fs::copy(&path, to.join(entry.file_name())).unwrap();
        }
    }
}

/// A copy of the lakehouse example in a project of its own.
pub struct Lakehouse {
    pub folder: tempfile::TempDir,
    pub tools: DiagramTools,
    pub workspace: DiagramWorkspace,
}

impl Lakehouse {
    pub fn new() -> Self {
        let folder = tempfile::tempdir().unwrap();
        copy_folder(&lakehouse(), folder.path());
        let workspace = DiagramWorkspace::new(folder.path());
        Lakehouse {
            folder,
            tools: tools(),
            workspace,
        }
    }

    /// Opens the registration at `relative` in the project.
    pub fn open(&mut self, relative: &str) -> DiagramSession {
        let (_, session) = self
            .workspace
            .open(&mut self.tools, Path::new(relative))
            .expect("it opens");
        session
    }

    /// The bytes of the project's file at `relative`, as text.
    pub fn read(&self, relative: &str) -> String {
        String::from_utf8(fs::read(self.folder.path().join(relative)).unwrap()).unwrap()
    }
}
