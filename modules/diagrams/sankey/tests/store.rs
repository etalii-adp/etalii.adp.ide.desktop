//! The `.skv` store and what opens a Sankey diagram: one reading per body, a change from outside read again, a delete
//! ending the document, the new body the add dialog writes, and the routes a registration and a bare body take.
//! Ported from the standalone host's `EtAlii.Adp.Diagram.Sankey.Tests/SankeyDocumentStore.Tests.cs`
//! (`DocumentStoreContract`), `SankeyDocumentReloader.Tests.cs` (`DocumentReloaderContract`) and
//! `ServiceCollectionAddSankey.Tests.cs` at `8b6ccfc`, the parts of those contracts a store kept in the project's
//! documents has.

mod common;

use std::fs;
use std::path::Path;

use adp_diagram::{DiagramDocuments, DiagramWorkspace, DiskChange, ModuleDocuments};
use adp_diagram_sankey::model::CURRENT_VERSION;
use adp_diagram_sankey::store::{self, STATE_KEY, SankeyStore};
use adp_diagram_sankey::{EMPTY_DOCUMENT, ORIGIN};
use common::{TestFolder, brightwater_coffee, parse, tools};

#[test]
fn the_empty_document_reads_as_an_empty_diagram_of_the_current_version_with_no_problem() {
    let model = parse(EMPTY_DOCUMENT);
    assert!(model.problems.is_empty());
    assert!(model.nodes.is_empty());
    assert!(model.flows.is_empty());
    assert_eq!(model.version, Some(CURRENT_VERSION));
    assert!(EMPTY_DOCUMENT.ends_with("\r\n"));
}

#[test]
fn a_document_is_read_once_and_shared() {
    let folder = TestFolder::new(&brightwater_coffee(), "brightwater-coffee.skv");
    let mut documents = DiagramDocuments::default();
    let first = store::get_or_load(&mut documents, &folder.body).model.clone();
    let second = store::get_or_load(&mut documents, &folder.body).model.clone();
    assert!(std::rc::Rc::ptr_eq(&first, &second));
    assert_eq!(first.nodes.len(), 14);
}

#[test]
fn its_own_write_is_unchanged_a_change_from_outside_is_read_again_and_a_delete_ends_it() {
    let folder = TestFolder::new(&brightwater_coffee(), "brightwater-coffee.skv");
    let mut documents = DiagramDocuments::default();
    let text = store::get_or_load(&mut documents, &folder.body).text.clone();
    let marked = format!("# store contract marker\r\n{text}");
    store::save(&mut documents, &folder.body, &marked).unwrap();
    let state = documents.module_state::<SankeyStore>(STATE_KEY);
    assert_eq!(state.reconcile(&folder.body), DiskChange::Unchanged);

    fs::write(&folder.body, "sankey: 1\r\nnodes:\r\n  - id: only\r\n").unwrap();
    assert_eq!(state.reconcile(&folder.body), DiskChange::Reloaded(folder.body.clone()));
    assert_eq!(state.get(&folder.body).unwrap().model.nodes.len(), 1);

    fs::remove_file(&folder.body).unwrap();
    assert_eq!(state.reconcile(&folder.body), DiskChange::Deleted(folder.body.clone()));
    let ended = state.get(&folder.body).unwrap();
    assert!(ended.model.nodes.is_empty() && ended.model.flows.is_empty());
    assert_eq!(state.reconcile(Path::new("elsewhere.skv")), DiskChange::NotTracked);
}

#[test]
fn a_body_with_a_byte_order_mark_keeps_it_when_written() {
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join("marked.skv");
    fs::write(&body, "\u{feff}sankey: 1\nnodes: []\nflows: []\n").unwrap();
    let mut documents = DiagramDocuments::default();
    let text = store::get_or_load(&mut documents, &body).text.clone();
    assert!(text.starts_with("sankey"));
    store::save(&mut documents, &body, &text.replace("nodes: []", "nodes:\n  - id: a")).unwrap();
    assert!(
        fs::read_to_string(&body)
            .unwrap()
            .starts_with("\u{feff}sankey: 1\nnodes:\n  - id: a\n")
    );
}

#[test]
fn a_registration_opens_its_body_and_a_bare_body_opens_on_its_own() {
    let folder = TestFolder::new(&brightwater_coffee(), "brightwater-coffee.skv");
    fs::write(
        folder.folder().join("company.adp"),
        "etalii/sankey\r\nbody: brightwater-coffee.skv\r\n",
    )
    .unwrap();
    let mut workspace = DiagramWorkspace::new(folder.folder());
    let (opened, _) = workspace.open(&mut tools(), Path::new("company.adp")).unwrap();
    assert_eq!(opened.origin, ORIGIN);
    assert_eq!(opened.body_path.file_name().unwrap(), "brightwater-coffee.skv");
    assert!(opened.registration_path.is_some());

    let bare = TestFolder::new(&brightwater_coffee(), "flows.skv");
    let mut workspace = DiagramWorkspace::new(bare.folder());
    let (opened, mut session) = workspace.open(&mut tools(), Path::new("flows.skv")).unwrap();
    assert_eq!(opened.origin, ORIGIN);
    assert_eq!(opened.registration_path, None);
    assert_eq!(session.baseline(workspace.documents()).unwrap().len(), 1);
}

#[test]
fn a_registration_whose_body_is_missing_or_outside_the_project_is_refused() {
    let folder = tempfile::tempdir().unwrap();
    fs::write(folder.path().join("gone.adp"), "etalii/sankey\r\n").unwrap();
    fs::write(folder.path().join("away.adp"), "etalii/sankey\r\nbody: ../away.skv\r\n").unwrap();
    let mut workspace = DiagramWorkspace::new(folder.path());
    let missing = workspace.open(&mut tools(), Path::new("gone.adp")).err().unwrap();
    assert!(missing.contains("is missing"), "{missing}");
    let away = workspace.open(&mut tools(), Path::new("away.adp")).err().unwrap();
    assert!(away.contains("outside the project"), "{away}");
}
