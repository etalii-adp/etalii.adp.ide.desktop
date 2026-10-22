//! One reading per pipeline in the project's documents: a file not there yet, one that does not parse, saves that are
//! read again, changes from outside and templates read once per workspace (the standalone host's
//! `PipelineDocumentStoreTests`, `PipelineDocumentStoreLostEditTests`, `PipelineDocumentReloaderTests` and
//! `PipelineSharedReadTests`).

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use adp_diagram::{DiagramDocuments, DiskChange, ModuleDocuments};
use adp_diagram_azure_devops_pipeline::lines::LineDocument;
use adp_diagram_azure_devops_pipeline::model::{EditTarget, UnresolvedReason};
use adp_diagram_azure_devops_pipeline::store::{self, PipelineStore};
use adp_diagram_azure_devops_pipeline::templates::PipelineTemplates;
use adp_diagram_azure_devops_pipeline::writer::PipelineWriter;
use common::fixture;

const SIMPLE: &str =
    "stages:\n  - stage: Build\n    jobs:\n      - job: Compile\n        steps:\n          - script: x\n";
const BROKEN: &str = "stages:\n  - stage: Build\n   jobs: [\n";

struct Workspace {
    folder: tempfile::TempDir,
    documents: DiagramDocuments,
}

impl Workspace {
    fn new() -> Workspace {
        Workspace {
            folder: tempfile::tempdir().unwrap(),
            documents: DiagramDocuments::default(),
        }
    }

    fn root(&self) -> PathBuf {
        self.folder.path().to_path_buf()
    }

    fn write(&self, relative: &str, content: &str) -> PathBuf {
        let full = self.folder.path().join(relative);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(&full, content).unwrap();
        full
    }

    fn load(&mut self, path: &Path) -> store::PipelineDocument {
        let root = self.root();
        store::get_or_load(&mut self.documents, &root, path).clone()
    }

    fn save(&mut self, path: &Path, text: &str) -> Result<(), String> {
        let root = self.root();
        store::save(&mut self.documents, &root, path, text)
    }

    fn reconcile(&mut self, path: &Path) -> DiskChange {
        self.documents
            .module_state::<PipelineStore>(store::STATE_KEY)
            .reconcile(path)
    }
}

#[test]
fn two_opens_of_the_same_file_share_one_document() {
    let mut w = Workspace::new();
    let path = w.write("azure-pipelines.yml", SIMPLE);
    let first = w.load(&path);
    let second = w.load(&path);
    assert!(std::rc::Rc::ptr_eq(&first.model, &second.model));
}

#[test]
fn a_file_that_is_not_there_yet_opens_as_empty_rather_than_failing() {
    let mut w = Workspace::new();
    let document = w.load(&w.root().join("not-written-yet.yml"));
    assert!(document.is_usable());
    assert!(document.model.stages.is_empty());
    assert_eq!("", document.text);
}

#[test]
fn a_file_that_does_not_parse_opens_carrying_the_reason_and_the_line() {
    let mut w = Workspace::new();
    let path = w.write("broken.yml", BROKEN);
    let document = w.load(&path);
    assert!(!document.is_usable());
    let reason = document.unreadable.clone().unwrap();
    assert!(reason.starts_with("Line 3: "), "{reason}");
    assert!(document.model.stages.is_empty());
    let outcome = adp_diagram_azure_devops_pipeline::parser::outcome(&LineDocument::parse(BROKEN), None, &path);
    assert_eq!(1, outcome.problems.len());
    assert_eq!(2, outcome.problems[0].line);
    assert_eq!(reason, format!("Line 3: {}", outcome.problems[0].message));
}

#[test]
fn a_file_that_does_not_parse_is_not_written_back() {
    let mut w = Workspace::new();
    let path = w.write("broken.yml", BROKEN);
    w.load(&path);
    assert!(w.save(&path, BROKEN).is_err());
    assert_eq!(BROKEN, fs::read_to_string(&path).unwrap());
}

#[test]
fn the_store_resolves_templates_from_the_root_it_was_opened_in() {
    let mut w = Workspace::new();
    w.write(
        "templates/shared.yml",
        "jobs:\n  - job: Shared\n    steps:\n      - script: x\n",
    );
    let path = w.write(
        "pipelines/azure-pipelines.yml",
        "jobs:\n  - template: /templates/shared.yml\n",
    );
    let document = w.load(&path);
    assert_eq!("Shared", document.model.jobs().next().unwrap().name);
    let edited = format!("{}# edited\n", document.text);
    w.save(&path, &edited).unwrap();
    assert_eq!(edited, fs::read_to_string(&path).unwrap());
}

#[test]
fn saving_an_edited_document_writes_it_byte_for_byte() {
    let mut w = Workspace::new();
    let path = w.write("azure-pipelines.yml", SIMPLE);
    let document = w.load(&path);
    let mut lines = document.lines();
    PipelineWriter::new(&mut lines)
        .set_display_name(&EditTarget::of_stage(&document.model.stages[0]), "Build it")
        .unwrap();
    w.save(&path, &lines.text()).unwrap();
    assert_eq!(lines.text(), fs::read_to_string(&path).unwrap());
    assert!(fs::read_to_string(&path).unwrap().contains("displayName: Build it"));
}

#[test]
fn saving_rebuilds_the_model_so_the_next_reader_sees_the_edit() {
    let mut w = Workspace::new();
    let path = w.write("azure-pipelines.yml", SIMPLE);
    let document = w.load(&path);
    let mut lines = document.lines();
    PipelineWriter::new(&mut lines)
        .set_display_name(&EditTarget::of_stage(&document.model.stages[0]), "Build it")
        .unwrap();
    w.save(&path, &lines.text()).unwrap();
    assert_eq!("Build it", w.load(&path).model.stages[0].display_name);
}

#[test]
fn saving_tells_the_views_on_that_document() {
    let mut w = Workspace::new();
    let path = w.write("azure-pipelines.yml", SIMPLE);
    w.load(&path);
    w.documents.take_changed();
    w.save(&path, &SIMPLE.replace("stage: Build", "stage: Built")).unwrap();
    assert_eq!(vec![path.clone()], w.documents.take_changed());
}

#[test]
fn its_own_write_is_not_a_change_from_outside() {
    let mut w = Workspace::new();
    let path = w.write("azure-pipelines.yml", SIMPLE);
    w.load(&path);
    w.save(&path, &SIMPLE.replace("stage: Build", "stage: Built")).unwrap();
    assert_eq!(DiskChange::Unchanged, w.reconcile(&path));
}

#[test]
fn a_reload_picks_up_an_external_edit() {
    let mut w = Workspace::new();
    let path = w.write("azure-pipelines.yml", SIMPLE);
    w.load(&path);
    fs::write(&path, SIMPLE.replace("stage: Build", "stage: Rebuilt")).unwrap();
    assert_eq!(DiskChange::Reloaded(path.clone()), w.reconcile(&path));
    assert_eq!("Rebuilt", w.load(&path).model.stages[0].name);
}

#[test]
fn a_reload_picks_up_an_edit_to_a_template_too() {
    let mut w = Workspace::new();
    let path = w.write("azure-pipelines.yml", "jobs:\n  - template: templates/shared.yml\n");
    let template = w.write(
        "templates/shared.yml",
        "jobs:\n  - job: Before\n    steps:\n      - script: x\n",
    );
    assert_eq!("Before", w.load(&path).model.jobs().next().unwrap().name);
    fs::write(&template, "jobs:\n  - job: After\n    steps:\n      - script: x\n").unwrap();
    // The body changed on disk too, as an editor saving both would leave it.
    fs::write(&path, "jobs:\n  - template: templates/shared.yml\n# touched\n").unwrap();
    w.reconcile(&path);
    assert_eq!("After", w.load(&path).model.jobs().next().unwrap().name);
}

#[test]
fn closing_a_document_makes_the_next_open_read_it_afresh() {
    let mut w = Workspace::new();
    let path = w.write("azure-pipelines.yml", SIMPLE);
    let first = w.load(&path);
    w.documents.module_state::<PipelineStore>(store::STATE_KEY).close(&path);
    let second = w.load(&path);
    assert!(!std::rc::Rc::ptr_eq(&first.model, &second.model));
}

#[test]
fn two_pipelines_sharing_a_template_share_one_resolver() {
    let mut w = Workspace::new();
    let one = w.write("one.yml", "jobs:\n  - template: templates/shared.yml\n");
    let two = w.write("two.yml", "jobs:\n  - template: templates/shared.yml\n");
    let template = w.write(
        "templates/shared.yml",
        "jobs:\n  - job: Before\n    steps:\n      - script: x\n",
    );
    w.load(&one);
    fs::write(&template, "jobs:\n  - job: After\n    steps:\n      - script: x\n").unwrap();
    assert_eq!("Before", w.load(&two).model.jobs().next().unwrap().name);
}

#[test]
fn a_template_that_cannot_be_followed_is_carried_on_the_model_rather_than_failing_the_load() {
    let mut w = Workspace::new();
    let path = w.write("azure-pipelines.yml", "stages:\n  - template: elsewhere.yml@other\n");
    let document = w.load(&path);
    assert!(document.is_usable());
    assert_eq!(1, document.model.unresolved.len());
    assert_eq!(UnresolvedReason::OtherRepository, document.model.unresolved[0].reason);
}

#[test]
fn a_reload_of_a_pipeline_never_opened_reads_nothing_and_tells_nobody() {
    let mut w = Workspace::new();
    let path = w.write("azure-pipelines.yml", &fixture("jobs-only.yml"));
    assert_eq!(DiskChange::NotTracked, w.reconcile(&path));
    assert!(w.documents.take_changed().is_empty());
    assert!(w.load(&path).model.jobs().next().is_some());
}

#[test]
fn a_deleted_body_ends_its_document() {
    let mut w = Workspace::new();
    let path = w.write("azure-pipelines.yml", SIMPLE);
    w.load(&path);
    fs::remove_file(&path).unwrap();
    assert_eq!(DiskChange::Deleted(path.clone()), w.reconcile(&path));
    assert!(w.load(&path).model.stages.is_empty());
}

#[test]
fn a_document_an_editor_is_still_writing_is_read() {
    let mut w = Workspace::new();
    let path = w.write("azure-pipelines.yml", SIMPLE);
    let document = w.load(&path);
    assert!(document.is_usable());
    assert_eq!(SIMPLE, document.text);
}

#[test]
fn a_template_an_editor_is_still_writing_is_read() {
    let w = Workspace::new();
    let path = w.write("steps.yml", "steps:\n  - script: x\n");
    let document = PipelineTemplates::new(&w.root()).read(&path).expect("read");
    assert_eq!("steps:\n  - script: x\n", document.text());
}
