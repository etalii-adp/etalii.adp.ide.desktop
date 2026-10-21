//! The store that lets several C4 diagrams share one document, which is the mechanism behind "model once, view many"
//! (etalii.adp spec 030, task T061): one reading per `.dsl`, a save that changes only the edited line, a reload that
//! cannot read keeping the last good model, and a deleted body no longer drawn.
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/`
//! `C4DocumentStore.Tests.cs` (with the shared `EtAlii.Adp.TestSupport/DocumentStoreContract.cs`),
//! `C4DocumentStore.Sharing.Tests.cs`, `C4DocumentStore.LostEdit.Tests.cs`, `C4DocumentStore.FailedReload.Tests.cs`
//! and `C4DocumentReloader.Tests.cs` (with `DocumentReloaderContract.cs`). The store keeps its documents in the
//! project's `DiagramDocuments`; "the sessions are told" is the body in `take_changed`. A read that fails is made
//! deterministic with a handle that shares nothing, which Windows enforces, so those tests run there.
//!
//! The watcher's side, a change on disk reported to the workspace, is `DiagramWorkspace::reconcile` and `close`,
//! which reach the store as they reach every other document.

use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_core::Catalogue;
use adp_diagram::document::DiskChange;
use adp_diagram::{DiagramDocuments, DiagramSession, DiagramTools, DiagramWorkspace};
use adp_diagram_c4::lines::LineDocument;
use adp_diagram_c4::store;
use adp_disl::DislPluginFunctions;
use adp_history::{Applied, Command, Refusal};

const SAMPLE: &str = r#"workspace "Sample" {
    model {
        s = softwareSystem "System" "Does the thing."
    }
    views {
        systemContext s "context" {
            include *
        }
    }
}"#;

const BANK: &str = r#"workspace "Bank" {
    model {
        u = person "Customer" "A customer."
        s = softwareSystem "Banking" "Does banking."
        u -> s "Uses"
    }
    views {
        systemContext s "context" {
            include *
        }
    }
}"#;

const MARKER: &str = "Store contract marker";

fn courier() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/examples/reference/architecture/courier.dsl")
}

fn write(folder: &tempfile::TempDir, name: &str, text: &str) -> PathBuf {
    let path = folder.path().join(name);
    fs::write(&path, text).unwrap();
    path
}

fn copy_courier(folder: &tempfile::TempDir) -> PathBuf {
    let path = folder.path().join("courier.dsl");
    fs::copy(courier(), &path).unwrap();
    path
}

/// `document` with the first software system's line renamed from `from` to `to`.
fn rename(document: &mut LineDocument, from: &str, to: &str) {
    let line = document
        .code_lines()
        .into_iter()
        .find(|l| l.code().contains("softwareSystem"))
        .unwrap();
    document
        .replace_line(line.number, &line.text.replace(from, to))
        .unwrap();
}

/// The contract's edit: the marker put before the first software system's name.
fn mark(document: &mut LineDocument) {
    rename(document, "softwareSystem \"", &format!("softwareSystem \"{MARKER} "));
}

fn elements(documents: &mut DiagramDocuments, path: &Path) -> usize {
    store::get_or_load(documents, path).workspace.elements.len()
}

/// Holds `path` open sharing nothing, so a read of it fails while the guard lives.
#[cfg(windows)]
fn lock(path: &Path) -> fs::File {
    use std::os::windows::fs::OpenOptionsExt;
    fs::OpenOptions::new().read(true).share_mode(0).open(path).unwrap()
}

// ---- C4DocumentStore.Sharing.Tests.cs --------------------------------------------------------------------------------

#[test]
fn two_opens_of_one_path_share_one_document() {
    let folder = tempfile::tempdir().unwrap();
    let path = write(&folder, "model.dsl", SAMPLE);
    let mut documents = DiagramDocuments::default();
    let first = store::get_or_load(&mut documents, &path).workspace.clone();
    let second = store::get_or_load(&mut documents, &path).workspace.clone();
    assert!(Rc::ptr_eq(&first, &second));
}

#[test]
fn a_missing_file_opens_as_an_empty_document_rather_than_throwing() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("not-written-yet.dsl");
    let mut documents = DiagramDocuments::default();
    let document = store::get_or_load(&mut documents, &path);
    assert!(document.lines().lines().iter().all(|line| line.text.is_empty()));
    assert!(document.workspace.elements.is_empty());
}

#[test]
fn save_writes_the_document_back_byte_for_byte() {
    let folder = tempfile::tempdir().unwrap();
    let path = write(&folder, "model.dsl", SAMPLE);
    let mut documents = DiagramDocuments::default();
    let lines = store::get_or_load(&mut documents, &path).lines();
    store::save(&mut documents, &path, &lines).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), SAMPLE);
}

#[test]
fn save_of_an_edited_document_changes_only_the_edited_line() {
    let folder = tempfile::tempdir().unwrap();
    let path = write(&folder, "model.dsl", SAMPLE);
    let mut documents = DiagramDocuments::default();
    let mut lines = store::get_or_load(&mut documents, &path).lines();
    rename(&mut lines, "\"System\"", "\"Renamed\"");
    store::save(&mut documents, &path, &lines).unwrap();
    let after = fs::read_to_string(&path).unwrap();
    assert!(after.contains("Renamed"));
    assert_eq!(SAMPLE.split('\n').count(), after.split('\n').count());
}

#[test]
fn save_raises_changed_once_the_workspace_is_reparsed() {
    let folder = tempfile::tempdir().unwrap();
    let path = write(&folder, "model.dsl", SAMPLE);
    let mut documents = DiagramDocuments::default();
    let mut lines = store::get_or_load(&mut documents, &path).lines();
    rename(&mut lines, "\"System\"", "\"Renamed\"");
    documents.take_changed();
    store::save(&mut documents, &path, &lines).unwrap();
    assert_eq!(documents.take_changed(), vec![path.clone()]);
    let workspace = &store::get(&documents, &path).unwrap().workspace;
    assert_eq!(workspace.find("s").unwrap().name, "Renamed");
}

#[test]
fn the_model_reflects_a_save_so_the_next_reader_sees_the_edit() {
    let folder = tempfile::tempdir().unwrap();
    let path = write(&folder, "model.dsl", SAMPLE);
    let mut documents = DiagramDocuments::default();
    let mut lines = store::get_or_load(&mut documents, &path).lines();
    rename(&mut lines, "\"System\"", "\"Renamed\"");
    store::save(&mut documents, &path, &lines).unwrap();
    let name = store::get_or_load(&mut documents, &path)
        .workspace
        .find("s")
        .unwrap()
        .name
        .clone();
    assert_eq!(name, "Renamed");
}

#[test]
fn save_creates_the_folder_when_the_body_lives_beside_nothing_yet() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("nested").join("model.dsl");
    let mut documents = DiagramDocuments::default();
    let lines = store::get_or_load(&mut documents, &path).lines();
    store::save(&mut documents, &path, &lines).unwrap();
    assert!(path.is_file());
}

// ---- C4DocumentStore.LostEdit.Tests.cs and the contract's ASaveAfterAReload --------------------------------------------

#[test]
fn a_save_after_a_reload_writes_the_edit_rather_than_the_file_it_just_reread() {
    let folder = tempfile::tempdir().unwrap();
    let path = write(&folder, "sample.dsl", SAMPLE);
    let mut documents = DiagramDocuments::default();
    let mut lines = store::get_or_load(&mut documents, &path).lines();
    lines
        .replace_line(3, "        s = softwareSystem \"System\" \"Renamed by the command.\"")
        .unwrap();
    assert!(lines.text().contains("Renamed by the command."));
    // The notification of an earlier write of this same file lands between the edit and the save.
    store::reload(&mut documents, &path);
    store::save(&mut documents, &path, &lines).unwrap();
    assert!(fs::read_to_string(&path).unwrap().contains("Renamed by the command."));
}

#[test]
fn the_contracts_save_after_a_reload_writes_the_edit_on_the_courier_example() {
    let folder = tempfile::tempdir().unwrap();
    let path = copy_courier(&folder);
    let mut documents = DiagramDocuments::default();
    let mut lines = store::get_or_load(&mut documents, &path).lines();
    mark(&mut lines);
    assert!(lines.text().contains(MARKER));
    store::reload(&mut documents, &path);
    store::save(&mut documents, &path, &lines).unwrap();
    assert!(fs::read_to_string(&path).unwrap().contains(MARKER));
}

// ---- DocumentStoreContract.cs ----------------------------------------------------------------------------------------

#[test]
fn an_external_write_is_read_and_tells_the_sessions() {
    let folder = tempfile::tempdir().unwrap();
    let path = copy_courier(&folder);
    let mut documents = DiagramDocuments::default();
    let mut lines = store::get_or_load(&mut documents, &path).lines();
    assert!(!lines.text().contains(MARKER));
    // Another program writes the file.
    mark(&mut lines);
    fs::write(&path, lines.text()).unwrap();
    documents.take_changed();
    store::reload(&mut documents, &path);
    assert!(store::get(&documents, &path).unwrap().text.contains(MARKER));
    assert_eq!(documents.take_changed(), vec![path]);
}

#[cfg(windows)]
#[test]
fn a_read_that_fails_keeps_the_last_good_document() {
    let folder = tempfile::tempdir().unwrap();
    let path = copy_courier(&folder);
    let mut documents = DiagramDocuments::default();
    let loaded = store::get_or_load(&mut documents, &path).workspace.clone();
    assert!(!loaded.elements.is_empty());
    documents.take_changed();
    let guard = lock(&path);
    store::reload(&mut documents, &path);
    drop(guard);
    assert!(Rc::ptr_eq(&loaded, &store::get(&documents, &path).unwrap().workspace));
    assert!(
        documents.take_changed().is_empty(),
        "the sessions have nothing to redraw"
    );
}

#[test]
fn a_deleted_body_ends_as_a_new_document_and_tells_the_sessions() {
    let folder = tempfile::tempdir().unwrap();
    let path = copy_courier(&folder);
    let mut documents = DiagramDocuments::default();
    assert!(elements(&mut documents, &path) > 0);
    documents.take_changed();
    fs::remove_file(&path).unwrap();
    store::body_deleted(&mut documents, &path);
    let document = store::get(&documents, &path).unwrap();
    assert!(document.workspace.elements.is_empty());
    assert!(document.is_usable(), "a new element can be saved into it");
    assert_eq!(documents.take_changed(), vec![path]);
}

// ---- DocumentReloaderContract.cs -------------------------------------------------------------------------------------

#[test]
fn a_missing_body_is_kept_on_reload_and_cleared_only_by_the_watchers_delete() {
    let folder = tempfile::tempdir().unwrap();
    let path = copy_courier(&folder);
    let mut documents = DiagramDocuments::default();
    assert!(elements(&mut documents, &path) > 0);
    fs::remove_file(&path).unwrap();
    // A body missing on a reload is far more often a publish in flight than a delete.
    store::reload(&mut documents, &path);
    assert!(elements(&mut documents, &path) > 0);
    store::body_deleted(&mut documents, &path);
    assert_eq!(elements(&mut documents, &path), 0);
}

#[test]
fn a_deletion_tells_the_sessions() {
    let folder = tempfile::tempdir().unwrap();
    let path = copy_courier(&folder);
    let mut documents = DiagramDocuments::default();
    assert!(elements(&mut documents, &path) > 0);
    documents.take_changed();
    fs::remove_file(&path).unwrap();
    store::body_deleted(&mut documents, &path);
    assert_eq!(documents.take_changed(), vec![path]);
}

// ---- C4DocumentStore.FailedReload.Tests.cs ---------------------------------------------------------------------------

#[cfg(windows)]
#[test]
fn a_save_after_a_reload_that_could_not_read_does_not_write_an_empty_model_over_the_real_file() {
    let folder = tempfile::tempdir().unwrap();
    let body = write(&folder, "model.dsl", BANK);
    let mut documents = DiagramDocuments::default();
    assert!(elements(&mut documents, &body) > 0);
    {
        let _guard = lock(&body);
        store::reload(&mut documents, &body);
    }
    // The store still holds the model: the failed read did not replace it.
    assert!(elements(&mut documents, &body) > 0);
    let lines = store::get_or_load(&mut documents, &body).lines();
    store::save(&mut documents, &body, &lines).unwrap();
    let on_disk = fs::read_to_string(&body).unwrap();
    assert!(on_disk.contains("Customer"));
    assert!(on_disk.contains("Banking"));
}

#[cfg(windows)]
#[test]
fn a_save_of_a_model_that_could_not_be_read_even_once_refuses_and_leaves_the_file_alone() {
    let folder = tempfile::tempdir().unwrap();
    let body = write(&folder, "model.dsl", BANK);
    let mut documents = DiagramDocuments::default();
    {
        let _guard = lock(&body);
        assert_eq!(elements(&mut documents, &body), 0, "the first read failed");
    }
    let lines = store::get_or_load(&mut documents, &body).lines();
    assert!(store::save(&mut documents, &body, &lines).is_err());
    assert_eq!(fs::read_to_string(&body).unwrap(), BANK);
}

#[test]
fn a_body_that_is_really_deleted_ends_as_an_empty_model_not_the_last_one_kept_alive() {
    let folder = tempfile::tempdir().unwrap();
    let body = write(&folder, "model.dsl", BANK);
    let mut documents = DiagramDocuments::default();
    assert!(elements(&mut documents, &body) > 0);
    fs::remove_file(&body).unwrap();
    store::body_deleted(&mut documents, &body);
    assert_eq!(elements(&mut documents, &body), 0);
}

#[test]
fn a_new_body_that_does_not_exist_yet_still_opens_empty_and_saves() {
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join("new.dsl");
    let mut documents = DiagramDocuments::default();
    assert_eq!(elements(&mut documents, &body), 0);
    let lines = store::get_or_load(&mut documents, &body).lines();
    store::save(&mut documents, &body, &lines).unwrap();
    assert!(body.is_file(), "a new, empty model was saved");
}

// ---- the watcher: DiagramWorkspace::reconcile and close --------------------------------------------------------------

/// Opens a body in the project's documents, as a view's open does.
struct Load(PathBuf);

impl Command<DiagramDocuments> for Load {
    fn label(&self) -> String {
        "Load".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        store::get_or_load(documents, &self.0);
        Ok(Applied::final_change())
    }
}

/// A project with `body` open through the C4 module, and the view of it.
fn opened(folder: &tempfile::TempDir, body: &Path) -> (DiagramWorkspace, DiagramSession) {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    let mut tools = adp_diagram_c4::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
        DislPluginFunctions::none()
    }));
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (_, session) = workspace.open(&mut tools, body).unwrap();
    workspace.take_changed();
    (workspace, session)
}

fn opened_elements(workspace: &DiagramWorkspace, body: &Path) -> usize {
    store::get(workspace.documents(), body)
        .unwrap()
        .workspace
        .elements
        .len()
}

#[test]
fn the_stores_own_save_does_not_come_back_as_a_reload() {
    let folder = tempfile::tempdir().unwrap();
    let body = copy_courier(&folder);
    let (mut workspace, session) = opened(&folder, &body);
    session
        .set_property(&mut workspace, &["tracking"], "c4.name", MARKER)
        .unwrap();
    assert_eq!(workspace.take_changed(), vec![body.clone()]);
    let saved = store::get(workspace.documents(), &body).unwrap().workspace.clone();

    // The watcher reports the store's own write.
    assert!(matches!(workspace.reconcile(&body), DiskChange::Unchanged));

    // Not an external change: nothing is read again and nobody is told twice.
    assert!(workspace.take_changed().is_empty());
    assert!(Rc::ptr_eq(
        &saved,
        &store::get(workspace.documents(), &body).unwrap().workspace
    ));
    assert!(
        workspace.history().availability().can_undo,
        "no barrier at the store's own write"
    );
}

#[test]
fn a_forgotten_document_is_read_afresh_on_the_next_open() {
    let folder = tempfile::tempdir().unwrap();
    let body = copy_courier(&folder);
    let (mut workspace, _) = opened(&folder, &body);
    assert!(opened_elements(&workspace, &body) > 0);
    workspace.close(&body);
    assert!(store::get(workspace.documents(), &body).is_none());
    fs::remove_file(&body).unwrap();
    // The next open reads the file, which is gone, so the document is a new one.
    assert!(workspace.execute_command(&Load(body.clone())).is_done());
    assert_eq!(opened_elements(&workspace, &body), 0);
}

#[test]
fn a_body_deleted_under_an_open_model_ends_empty_through_the_watcher() {
    let folder = tempfile::tempdir().unwrap();
    let body = write(&folder, "model.dsl", BANK);
    let (mut workspace, _) = opened(&folder, &body);
    assert!(opened_elements(&workspace, &body) > 0);
    fs::remove_file(&body).unwrap();
    assert!(matches!(workspace.reconcile(&body), DiskChange::Deleted(_)));
    assert_eq!(opened_elements(&workspace, &body), 0);
    assert_eq!(workspace.take_changed(), vec![body]);
}

#[test]
fn external_saves_of_an_unchanged_model_never_empty_it_through_the_watcher() {
    let folder = tempfile::tempdir().unwrap();
    let body = write(&folder, "model.dsl", BANK);
    let (mut workspace, _) = opened(&folder, &body);
    assert!(opened_elements(&workspace, &body) > 0);
    for _ in 0..50 {
        // Every external save replaces the body through a temporary file and a rename.
        adp_documents::text::write_atomically(&body, BANK.as_bytes()).unwrap();
        let change = workspace.reconcile(&body);
        assert!(!matches!(change, DiskChange::Deleted(_)), "{change:?}");
        assert!(opened_elements(&workspace, &body) > 0);
    }
}
