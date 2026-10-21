//! The module's two reads of a file against the world's write side (etalii.adp spec 030, task T061): a document and a
//! registration's `view:` header are both readable while a writer holds the file, as Windows enforces sharing.
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/C4SharedRead.Tests.cs` at
//! `8b6ccfc`. The writer holds the file as `File.WriteAllText` does: write access, sharing reads only.
#![cfg(windows)]

use std::fs;
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;

use adp_core::Catalogue;
use adp_diagram::{DiagramDocuments, DiagramTools, DiagramWorkspace};
use adp_diagram_c4::mapper::VIEW_TYPE;
use adp_diagram_c4::store;
use adp_disl::DislPluginFunctions;

const MODEL: &str = r#"workspace "Bank" {
    model {
        s = softwareSystem "Banking" "Does banking." {
            web = container "Web" "Serves." "React"
        }
    }
    views {
        systemContext s "context" {
            include *
        }
        container s "containers" {
            include *
        }
    }
}"#;

/// FILE_SHARE_READ: the handle every save holds.
const SHARE_READ: u32 = 1;

fn writer(path: &Path) -> fs::File {
    fs::OpenOptions::new()
        .write(true)
        .share_mode(SHARE_READ)
        .open(path)
        .unwrap()
}

#[test]
fn get_or_load_reads_a_document_an_editor_is_still_writing() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("model.dsl");
    fs::write(&path, MODEL).unwrap();
    let _editor = writer(&path);
    let mut documents = DiagramDocuments::default();
    // The model itself, not the empty fallback.
    assert_eq!(store::get_or_load(&mut documents, &path).lines().text(), MODEL);
}

#[test]
fn open_reads_a_registration_an_editor_is_still_writing_and_still_names_its_view() {
    let folder = tempfile::tempdir().unwrap();
    let root = folder.path().to_path_buf();
    fs::write(root.join("model.dsl"), MODEL).unwrap();
    let adp = root.join("containers.adp");
    fs::write(&adp, "c4/container\nbody: model.dsl\nview: containers\n").unwrap();
    let _editor = writer(&adp);
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    let mut tools = adp_diagram_c4::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
        DislPluginFunctions::none()
    }));
    let mut workspace = DiagramWorkspace::new(&root);
    let (_, session) = workspace.open(&mut tools, &adp).unwrap();
    let view = session
        .render(workspace.documents())
        .unwrap()
        .into_iter()
        .find(|e| e.element_type == VIEW_TYPE)
        .unwrap();
    assert_eq!(view.attribute("viewKey"), "containers");
}
