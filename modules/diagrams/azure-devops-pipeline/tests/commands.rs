//! Every edit as a command: what it writes, what it refuses, and the inverse it names, which put back where it was
//! leaves the file byte for byte (the standalone host's `PipelineCommandsTests` and `PipelineCommandHandlersTests`;
//! the latter's save warning is the core store's, which this module's store has no seam for).

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use adp_diagram::DiagramDocuments;
use adp_diagram_azure_devops_pipeline::blocks::PLACEHOLDER_SCRIPT;
use adp_diagram_azure_devops_pipeline::commands::{AddKind, PipelineCommand, PipelineEdit};
use adp_diagram_azure_devops_pipeline::model::{PipelineModel, StrategyKind};
use adp_diagram_azure_devops_pipeline::store;
use common::{fixture, parse_text};

const PIPELINE: &str = "stages:
  - stage: Build
    jobs:
      - job: Compile
        steps:
          - script: dotnet build
            displayName: Build
          - script: dotnet pack
            displayName: Pack
      - job: Lint
        steps:
          - script: dotnet format
  - stage: Test
    dependsOn: Build
    jobs:
      - job: Verify
        steps:
          - script: dotnet test";

const SEQUENTIAL: &str = "stages:
  - stage: A
    jobs:
      - job: J
        steps:
          - script: x
  - stage: B
    jobs:
      - job: J
        steps:
          - script: x
  - stage: C
    jobs:
      - job: J
        steps:
          - script: x";

/// A workspace holding one pipeline, `azure-pipelines.yml`, and the project's documents.
struct Workspace {
    folder: tempfile::TempDir,
    path: PathBuf,
    documents: DiagramDocuments,
}

impl Workspace {
    fn with(content: &str) -> Workspace {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("azure-pipelines.yml");
        fs::write(&path, content).unwrap();
        Workspace {
            folder,
            path,
            documents: DiagramDocuments::default(),
        }
    }

    fn root(&self) -> &Path {
        self.folder.path()
    }

    fn text(&self) -> String {
        fs::read_to_string(&self.path).unwrap()
    }

    fn model(&mut self) -> PipelineModel {
        let root = self.root().to_path_buf();
        (*store::get_or_load(&mut self.documents, &root, &self.path).model).clone()
    }

    /// Runs `edit`: the inverse it names, none when it changed nothing, or its refusal.
    fn run(&mut self, edit: PipelineEdit) -> Result<Option<PipelineCommand>, String> {
        PipelineCommand::new(self.folder.path(), &self.path, edit).run(&mut self.documents)
    }

    fn undo(&mut self, inverse: &PipelineCommand) -> Result<Option<PipelineCommand>, String> {
        inverse.run(&mut self.documents)
    }
}

fn rename(id: &str, name: &str) -> PipelineEdit {
    PipelineEdit::Rename {
        element_id: id.to_owned(),
        display_name: name.to_owned(),
    }
}

fn depends(id: &str, names: &[&str], declared: bool) -> PipelineEdit {
    PipelineEdit::SetDependencies {
        element_id: id.to_owned(),
        depends_on: names.iter().map(|n| (*n).to_owned()).collect(),
        declared,
    }
}

fn enabled(id: &str, enabled: bool) -> PipelineEdit {
    PipelineEdit::SetEnabled {
        element_id: id.to_owned(),
        enabled,
    }
}

fn add(kind: AddKind, parent: &str, name: &str) -> PipelineEdit {
    PipelineEdit::Add {
        kind,
        parent_id: parent.to_owned(),
        name: name.to_owned(),
    }
}

fn remove(id: &str) -> PipelineEdit {
    PipelineEdit::Remove {
        element_id: id.to_owned(),
    }
}

fn move_step(id: &str, to: i64) -> PipelineEdit {
    PipelineEdit::MoveStep {
        step_id: id.to_owned(),
        to_index: to,
    }
}

#[test]
fn a_rename_sets_the_display_name_and_is_undoable() {
    let mut w = Workspace::with(PIPELINE);
    let before = w.text();
    let inverse = w.run(rename("Build", "Build the solution")).unwrap().unwrap();
    assert!(w.text().contains("displayName: Build the solution"));
    w.undo(&inverse).unwrap();
    assert_eq!(before, w.text());
}

#[test]
fn a_rename_of_something_gone_says_so_rather_than_failing() {
    let mut w = Workspace::with(PIPELINE);
    let error = w.run(rename("NoSuchStage", "Anything")).unwrap_err();
    assert!(error.contains("no longer in this pipeline"), "{error}");
}

#[test]
fn a_rename_that_changes_nothing_is_not_recorded_as_undoable() {
    let mut w = Workspace::with(PIPELINE);
    assert_eq!(None, w.run(rename("Build", "")).unwrap());
}

#[test]
fn drawing_an_edge_writes_the_dependency() {
    let mut w = Workspace::with(SEQUENTIAL);
    w.run(depends("C", &["A"], true)).unwrap();
    let model = w.model();
    assert_eq!(
        vec!["A".to_owned()],
        model.stages.iter().find(|s| s.name == "C").unwrap().depends_on
    );
}

#[test]
fn drawing_an_edge_that_closes_a_loop_is_refused() {
    let mut w = Workspace::with(PIPELINE);
    let error = w.run(depends("Build", &["Test"], true)).unwrap_err();
    assert!(error.contains("already waiting for it"), "{error}");
    assert!(!w.text().contains("dependsOn: Test"));
}

#[test]
fn removing_an_edge_puts_the_element_back_on_the_default() {
    let mut w = Workspace::with(PIPELINE);
    w.run(depends("Test", &[], false)).unwrap();
    assert!(
        !w.model()
            .stages
            .iter()
            .find(|s| s.name == "Test")
            .unwrap()
            .depends_on_declared
    );
}

#[test]
fn making_the_implicit_explicit_writes_a_depends_on_that_was_not_there() {
    let mut w = Workspace::with(SEQUENTIAL);
    assert!(
        !w.model()
            .stages
            .iter()
            .find(|s| s.name == "C")
            .unwrap()
            .depends_on_declared
    );
    w.run(depends("C", &["A"], true)).unwrap();
    assert!(w.text().contains("dependsOn: A"));
    let model = w.model();
    assert!(model.stages.iter().find(|s| s.name == "C").unwrap().depends_on_declared);
    assert!(!model.stages.iter().find(|s| s.name == "B").unwrap().depends_on_declared);
}

#[test]
fn a_dependency_on_something_that_does_not_exist_is_refused() {
    let mut w = Workspace::with(PIPELINE);
    let error = w.run(depends("Test", &["Imaginary"], true)).unwrap_err();
    assert!(error.contains("nothing called 'Imaginary'"), "{error}");
}

#[test]
fn taking_the_last_starting_stages_freedom_is_refused() {
    let mut w = Workspace::with(PIPELINE);
    assert!(w.run(depends("Build", &["Build"], true)).is_err());
}

#[test]
fn a_step_has_nothing_to_wait_for() {
    let mut w = Workspace::with(PIPELINE);
    let error = w.run(depends("Build/Compile/step-0", &["Lint"], true)).unwrap_err();
    assert!(error.contains("run in order"), "{error}");
}

#[test]
fn disabling_a_step_writes_enabled_false_and_is_undoable() {
    let mut w = Workspace::with(PIPELINE);
    let before = w.text();
    let inverse = w.run(enabled("Build/Compile/step-0", false)).unwrap().unwrap();
    assert!(w.model().steps().next().unwrap().execution.is_disabled());
    w.undo(&inverse).unwrap();
    assert_eq!(before, w.text());
}

#[test]
fn enabling_something_removes_the_key_rather_than_writing_true() {
    let mut w = Workspace::with(PIPELINE);
    w.run(enabled("Build/Compile/step-0", false)).unwrap();
    w.run(enabled("Build/Compile/step-0", true)).unwrap();
    assert!(!w.text().contains("enabled:"));
}

#[test]
fn adding_a_stage_produces_something_that_would_actually_run() {
    let mut w = Workspace::with(PIPELINE);
    w.run(add(AddKind::Stage, "", "Ship")).unwrap();
    let model = w.model();
    let stage = model.stages.iter().find(|s| s.name == "Ship").unwrap();
    assert!(!stage.jobs.is_empty());
    assert!(!stage.jobs[0].steps.is_empty());
}

#[test]
fn adding_a_stage_is_undone_by_removing_it() {
    let mut w = Workspace::with(PIPELINE);
    let before = w.text();
    let inverse = w.run(add(AddKind::Stage, "", "Ship")).unwrap().unwrap();
    w.undo(&inverse).unwrap();
    assert_eq!(before, w.text());
}

#[test]
fn adding_a_stage_with_a_taken_name_picks_a_free_one() {
    let mut w = Workspace::with(PIPELINE);
    w.run(add(AddKind::Stage, "", "Build")).unwrap();
    assert!(w.model().stages.iter().any(|s| s.name == "Build2"));
}

#[test]
fn adding_a_job_joins_the_stage_it_was_dropped_on() {
    let mut w = Workspace::with(PIPELINE);
    w.run(add(AddKind::Job, "Test", "Smoke")).unwrap();
    let model = w.model();
    assert!(
        model
            .stages
            .iter()
            .find(|s| s.name == "Test")
            .unwrap()
            .jobs
            .iter()
            .any(|j| j.name == "Smoke")
    );
}

#[test]
fn adding_a_deployment_job_writes_the_environment_and_strategy_it_needs() {
    let mut w = Workspace::with(PIPELINE);
    w.run(add(AddKind::DeploymentJob, "Test", "Release")).unwrap();
    let model = w.model();
    let job = model.jobs().find(|j| j.name == "Release").unwrap();
    assert!(job.is_deployment);
    assert!(!job.environment.is_empty());
    assert_eq!(StrategyKind::RunOnce, job.strategy.kind);
    assert!(!job.steps.is_empty());
}

#[test]
fn adding_a_job_to_a_step_is_refused() {
    let mut w = Workspace::with(PIPELINE);
    let error = w.run(add(AddKind::Job, "Build/Compile/step-0", "")).unwrap_err();
    assert!(error.contains("goes in a stage"), "{error}");
}

#[test]
fn adding_a_step_to_a_stage_is_refused() {
    let mut w = Workspace::with(PIPELINE);
    let error = w.run(add(AddKind::Step, "Build", "")).unwrap_err();
    assert!(error.contains("goes in a job"), "{error}");
}

#[test]
fn adding_a_step_joins_the_end_of_its_job() {
    let mut w = Workspace::with(PIPELINE);
    w.run(add(AddKind::Step, "Build/Compile", "")).unwrap();
    let model = w.model();
    let steps = &model.jobs().find(|j| j.name == "Compile").unwrap().steps;
    assert_eq!(3, steps.len());
    assert!(steps[2].identifier.contains(PLACEHOLDER_SCRIPT));
}

#[test]
fn removing_a_stage_puts_back_exactly_what_was_there() {
    let mut w = Workspace::with(
        "stages:
  - stage: Build
    jobs:
      - job: Compile
        steps:
          - script: x
  # This comment belongs to Ship, and an add would not bring it back.
  - stage: Ship
    dependsOn: Build
    condition: always()
    jobs:
      - job: Push
        steps:
          - script: y",
    );
    let before = w.text();
    let inverse = w.run(remove("Ship")).unwrap().unwrap();
    w.undo(&inverse).unwrap();
    assert_eq!(before, w.text());
}

#[test]
fn a_removal_followed_by_blank_lines_is_undone_byte_for_byte_and_redone_on_the_same_element() {
    for (id, what) in [
        ("Test", "the stage"),
        ("Build/Lint", "the job"),
        ("Build/Compile/step-1", "the step"),
    ] {
        let mut w = Workspace::with(&fixture("blank-lines-after.yml"));
        let original = w.text();
        let inverse = w.run(remove(id)).unwrap().unwrap();
        let without = w.text();
        let redo = w.undo(&inverse).unwrap();
        assert_eq!(original, w.text(), "{what}");
        let redo = redo.unwrap_or_else(|| panic!("the redo of removing {what} was lost"));
        w.undo(&redo)
            .unwrap_or_else(|e| panic!("the redo of removing {what} was refused: {e}"));
        assert_eq!(without, w.text(), "{what}");
    }
}

#[test]
fn removing_the_last_stage_is_refused() {
    let mut w =
        Workspace::with("stages:\n  - stage: Only\n    jobs:\n      - job: A\n        steps:\n          - script: x\n");
    let error = w.run(remove("Only")).unwrap_err();
    assert!(error.contains("at least one stage"), "{error}");
}

#[test]
fn removing_a_jobs_last_step_is_refused() {
    let mut w = Workspace::with(PIPELINE);
    let error = w.run(remove("Build/Lint/step-0")).unwrap_err();
    assert!(error.contains("at least one step"), "{error}");
}

#[test]
fn removing_a_stages_last_job_is_refused() {
    let mut w = Workspace::with(PIPELINE);
    let error = w.run(remove("Test/Verify")).unwrap_err();
    assert!(error.contains("at least one job"), "{error}");
}

#[test]
fn removing_a_step_leaves_the_rest_of_the_job_alone() {
    let mut w = Workspace::with(PIPELINE);
    w.run(remove("Build/Compile/step-0")).unwrap();
    let model = w.model();
    let steps = &model.jobs().find(|j| j.name == "Compile").unwrap().steps;
    assert_eq!(1, steps.len());
    assert_eq!("Pack", steps[0].label());
}

#[test]
fn reordering_steps_changes_the_order_they_run_in() {
    let mut w = Workspace::with(PIPELINE);
    w.run(move_step("Build/Compile/step-1", 0)).unwrap();
    let model = w.model();
    let labels: Vec<String> = model
        .jobs()
        .find(|j| j.name == "Compile")
        .unwrap()
        .steps
        .iter()
        .map(|s| s.label())
        .collect();
    assert_eq!(vec!["Pack", "Build"], labels);
}

#[test]
fn reordering_steps_is_undoable() {
    let mut w = Workspace::with(PIPELINE);
    let before = w.text();
    let inverse = w.run(move_step("Build/Compile/step-1", 0)).unwrap().unwrap();
    w.undo(&inverse).unwrap();
    assert_eq!(before, w.text());
}

#[test]
fn moving_a_step_nowhere_does_nothing() {
    let mut w = Workspace::with(PIPELINE);
    assert_eq!(None, w.run(move_step("Build/Compile/step-0", 0)).unwrap());
}

#[test]
fn an_edit_to_a_template_sourced_element_is_refused_naming_the_template() {
    let mut w = Workspace::with("stages:\n  - stage: Build\n    jobs:\n      - template: templates/jobs.yml\n");
    fs::create_dir_all(w.root().join("templates")).unwrap();
    fs::write(
        w.root().join("templates").join("jobs.yml"),
        "jobs:\n  - job: FromTemplate\n    steps:\n      - script: x\n",
    )
    .unwrap();
    let error = w.run(rename("Build/FromTemplate", "Renamed")).unwrap_err();
    assert!(error.contains("templates/jobs.yml"), "{error}");
}

#[test]
fn an_edit_to_a_file_that_does_not_parse_is_refused_rather_than_making_it_worse() {
    let mut w = Workspace::with("stages:\n  - stage: Build\n   jobs: [\n");
    let error = w.run(rename("Build", "Anything")).unwrap_err();
    assert!(error.contains("Line "), "{error}");
}

#[test]
fn every_edit_leaves_a_pipeline_that_still_parses() {
    let mut w = Workspace::with(PIPELINE);
    w.run(rename("Build", "Build it")).unwrap();
    w.run(enabled("Build/Lint", false)).unwrap();
    w.run(add(AddKind::Stage, "", "Ship")).unwrap();
    w.run(add(AddKind::Job, "Ship", "Push")).unwrap();
    w.run(add(AddKind::Step, "Ship/Push", "")).unwrap();
    w.run(move_step("Build/Compile/step-1", 0)).unwrap();
    w.run(depends("Ship", &["Test"], true)).unwrap();
    let root = w.root().to_path_buf();
    let path = w.path.clone();
    let document = store::get_or_load(&mut w.documents, &root, &path);
    assert!(document.is_usable(), "{:?}", document.unreadable);
    let names: Vec<&str> = document.model.stages.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(vec!["Build", "Test", "Ship"], names);
    assert_eq!(
        vec!["Test".to_owned()],
        document
            .model
            .stages
            .iter()
            .find(|s| s.name == "Ship")
            .unwrap()
            .depends_on
    );
    assert_eq!(parse_text(&w.text()).stages.len(), 3);
}

#[test]
fn every_handler_can_be_undone() {
    for edit in [
        rename("Build", "Build it all"),
        enabled("Test", false),
        depends("DeployProduction", &["Build"], true),
        add(AddKind::Stage, "", ""),
        add(AddKind::Job, "Test", ""),
        add(AddKind::Step, "Build/Compile", ""),
        remove("Test/Integration"),
        remove("Test"),
        move_step("Build/Compile/step-0", 2),
    ] {
        let mut w = Workspace::with(&fixture("multi-stage.yml"));
        let before = w.text();
        let inverse = w.run(edit.clone()).unwrap().unwrap();
        w.undo(&inverse).unwrap_or_else(|e| panic!("{edit:?}: {e}"));
        assert_eq!(before, w.text(), "{edit:?}");
    }
}

#[test]
fn a_removed_step_is_removed_again_by_the_redo_of_its_undo() {
    let mut w = Workspace::with(&fixture("multi-stage.yml"));
    let inverse = w.run(remove("Build/Compile/step-1")).unwrap().unwrap();
    let without = w.text();
    let redo = w.undo(&inverse).unwrap().expect("a redo");
    w.undo(&redo).unwrap();
    assert_eq!(without, w.text());
}
