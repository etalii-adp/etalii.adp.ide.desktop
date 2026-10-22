//! What each element offers and what each offer does, through an open view as the canvas reaches it (the standalone
//! host's `PipelineContextActionProviderTests`).

mod common;

use std::fs;

use adp_context::{Prompt, Shortcut};
use adp_diagram::{ActionOutcome, ActionTarget};
use adp_diagram_azure_devops_pipeline::actions::*;
use adp_diagram_azure_devops_pipeline::graph;
use adp_diagram_azure_devops_pipeline::mapper::edge_id;
use common::{Pipeline, open_text};

const PIPELINE: &str = "stages:
  - stage: Build
    jobs:
      - job: Compile
        steps:
          - script: dotnet build
          - script: dotnet pack
      - job: Lint
        steps:
          - script: dotnet format
  - stage: Test
    dependsOn: Build
    jobs:
      - job: Verify
        steps:
          - script: dotnet test";

const ONLY: &str = "stages:\n  - stage: Only\n    jobs:\n      - job: A\n        steps:\n          - script: x\n";

fn open() -> Pipeline {
    open_text(PIPELINE)
}

fn label_of(pipeline: &Pipeline, id: &str, action: &str) -> String {
    pipeline
        .session
        .actions(pipeline.workspace.documents(), &ActionTarget::Element(id.to_owned()))
        .into_iter()
        .flat_map(|g| g.actions)
        .find(|a| a.id == action)
        .unwrap()
        .label
}

fn shortcut_of(pipeline: &Pipeline, id: &str, action: &str) -> Option<Shortcut> {
    pipeline
        .session
        .actions(pipeline.workspace.documents(), &ActionTarget::Element(id.to_owned()))
        .into_iter()
        .flat_map(|g| g.actions)
        .find(|a| a.id == action)
        .unwrap()
        .shortcut
}

#[test]
fn the_rename_is_the_only_action_that_asks() {
    let mut pipeline = open();
    assert!(matches!(
        pipeline.execute("Build", RENAME),
        ActionOutcome::Prompt(Prompt::Input { .. })
    ));
}

#[test]
fn a_stage_is_offered_the_edits_that_apply_to_it() {
    let actions = open().action_ids("Build");
    for id in [RENAME, TOGGLE_ENABLED, ADD_JOB, ADD_DEPLOYMENT_JOB, ADD_STAGE] {
        assert!(actions.contains(&id.to_owned()), "{id}: {actions:?}");
    }
}

#[test]
fn a_stage_is_not_offered_add_step() {
    assert!(!open().action_ids("Build").contains(&ADD_STEP.to_owned()));
}

#[test]
fn a_job_is_offered_add_step_and_not_add_job() {
    let actions = open().action_ids("Build/Compile");
    assert!(actions.contains(&ADD_STEP.to_owned()));
    assert!(!actions.contains(&ADD_JOB.to_owned()));
}

#[test]
fn actions_carry_their_shortcuts_as_data() {
    assert_eq!(Some(Shortcut::key("F2")), shortcut_of(&open(), "Build", RENAME));
}

#[test]
fn a_step_move_answers_to_alt_and_the_arrow_key_as_the_browser_reports_it() {
    let pipeline = open();
    for (action, step, key) in [
        (MOVE_STEP_UP, "Build/Compile/step-1", "ArrowUp"),
        (MOVE_STEP_DOWN, "Build/Compile/step-0", "ArrowDown"),
    ] {
        let shortcut = shortcut_of(&pipeline, step, action).expect("a shortcut");
        assert_eq!(Shortcut::key(key).with_alt(), shortcut);
        assert_ne!(Shortcut::key(key), shortcut);
    }
}

#[test]
fn the_only_stage_is_not_offered_remove() {
    assert!(!open_text(ONLY).action_ids("Only").contains(&REMOVE.to_owned()));
}

#[test]
fn one_of_several_stages_is_offered_remove() {
    assert!(open().action_ids("Test").contains(&REMOVE.to_owned()));
}

#[test]
fn a_stages_only_job_is_not_offered_remove() {
    assert!(!open().action_ids("Test/Verify").contains(&REMOVE.to_owned()));
}

#[test]
fn a_jobs_only_step_is_not_offered_remove() {
    assert!(!open().action_ids("Build/Lint/step-0").contains(&REMOVE.to_owned()));
}

#[test]
fn a_first_step_is_not_offered_move_up() {
    let actions = open().action_ids("Build/Compile/step-0");
    assert!(!actions.contains(&MOVE_STEP_UP.to_owned()));
    assert!(actions.contains(&MOVE_STEP_DOWN.to_owned()));
}

#[test]
fn a_last_step_is_not_offered_move_down() {
    let actions = open().action_ids("Build/Compile/step-1");
    assert!(actions.contains(&MOVE_STEP_UP.to_owned()));
    assert!(!actions.contains(&MOVE_STEP_DOWN.to_owned()));
}

#[test]
fn a_stage_with_no_depends_on_is_not_offered_to_clear_it() {
    assert!(!open().action_ids("Build").contains(&CLEAR_DEPENDENCIES.to_owned()));
}

#[test]
fn a_stage_with_a_depends_on_is_offered_to_clear_it() {
    assert!(open().action_ids("Test").contains(&CLEAR_DEPENDENCIES.to_owned()));
}

#[test]
fn an_element_from_a_template_is_offered_nothing() {
    let folder = tempfile::tempdir().unwrap();
    fs::create_dir_all(folder.path().join("templates")).unwrap();
    fs::write(
        folder.path().join("templates/jobs.yml"),
        "jobs:\n  - job: FromTemplate\n    steps:\n      - script: x\n",
    )
    .unwrap();
    fs::write(
        folder.path().join("pipeline.yml"),
        "stages:\n  - stage: Build\n    jobs:\n      - template: templates/jobs.yml\n",
    )
    .unwrap();
    fs::write(folder.path().join("pipeline.adp"), "azure-devops/pipeline\r\n").unwrap();
    let mut workspace = adp_diagram::DiagramWorkspace::new(folder.path());
    let (_, session) = workspace
        .open(&mut common::tools(), std::path::Path::new("pipeline.adp"))
        .unwrap();
    let actions = session.actions(
        workspace.documents(),
        &ActionTarget::Element("Build/FromTemplate".to_owned()),
    );
    assert!(actions.is_empty(), "{actions:?}");
}

#[test]
fn a_file_that_does_not_parse_offers_nothing() {
    assert!(
        open_text("stages:\n  - stage: Build\n   jobs: [\n")
            .action_ids("Build")
            .is_empty()
    );
}

#[test]
fn an_element_that_is_gone_offers_nothing() {
    assert!(open().action_ids("NoSuchStage").is_empty());
}

#[test]
fn a_rename_asks_for_the_new_name_starting_from_the_current_one() {
    let mut pipeline = open_text(
        "stages:\n  - stage: Build\n    displayName: Build it\n    jobs:\n      - job: A\n        steps:\n          - script: x\n",
    );
    match pipeline.execute("Build", RENAME) {
        ActionOutcome::Prompt(Prompt::Input { initial_value, .. }) => assert_eq!("Build it", initial_value),
        other => panic!("{other:?}"),
    }
}

#[test]
fn an_action_with_everything_it_needs_does_not_put_a_dialog_in_the_way() {
    assert_eq!(ActionOutcome::Done, open().execute("Build", TOGGLE_ENABLED));
}

#[test]
fn committing_a_rename_writes_it_through_the_history() {
    let mut pipeline = open();
    assert_eq!(ActionOutcome::Done, pipeline.answer("Build", RENAME, "Build it"));
    assert!(pipeline.text().contains("displayName: Build it"));
    assert!(pipeline.workspace.history().availability().can_undo);
}

#[test]
fn committing_a_rename_is_undoable() {
    let mut pipeline = open();
    let before = pipeline.text();
    pipeline.answer("Build", RENAME, "Build it");
    assert!(pipeline.undo());
    assert_eq!(before, pipeline.text());
}

#[test]
fn toggling_enabled_switches_the_element_off_and_back_on() {
    let mut pipeline = open();
    pipeline.answer("Build/Lint", TOGGLE_ENABLED, "");
    assert!(
        pipeline
            .model()
            .jobs()
            .find(|j| j.name == "Lint")
            .unwrap()
            .execution
            .is_disabled()
    );
    pipeline.answer("Build/Lint", TOGGLE_ENABLED, "");
    assert!(
        !pipeline
            .model()
            .jobs()
            .find(|j| j.name == "Lint")
            .unwrap()
            .execution
            .is_disabled()
    );
}

#[test]
fn the_toggle_is_labelled_for_what_it_will_do() {
    let mut pipeline = open();
    let before = label_of(&pipeline, "Build/Lint", TOGGLE_ENABLED);
    pipeline.answer("Build/Lint", TOGGLE_ENABLED, "");
    let after = label_of(&pipeline, "Build/Lint", TOGGLE_ENABLED);
    assert_eq!("Disable", before);
    assert_eq!("Enable", after);
}

#[test]
fn adding_a_job_through_the_menu_uses_the_same_command_the_toolbox_will() {
    let mut pipeline = open();
    assert_eq!(ActionOutcome::Done, pipeline.answer("Test", ADD_JOB, ""));
    assert_eq!(
        2,
        pipeline
            .model()
            .stages
            .iter()
            .find(|s| s.name == "Test")
            .unwrap()
            .jobs
            .len()
    );
}

#[test]
fn moving_a_step_down_reorders_it() {
    let mut pipeline = open();
    assert_eq!(
        ActionOutcome::Done,
        pipeline.answer("Build/Compile/step-0", MOVE_STEP_DOWN, "")
    );
    let identifiers: Vec<String> = pipeline
        .model()
        .jobs()
        .find(|j| j.name == "Compile")
        .unwrap()
        .steps
        .iter()
        .map(|s| s.identifier.clone())
        .collect();
    assert_eq!(vec!["dotnet pack", "dotnet build"], identifiers);
}

#[test]
fn removing_a_stage_takes_it_out() {
    let mut pipeline = open();
    assert_eq!(ActionOutcome::Done, pipeline.answer("Test", REMOVE, ""));
    let names: Vec<String> = pipeline.model().stages.iter().map(|s| s.name.clone()).collect();
    assert_eq!(vec!["Build"], names);
}

#[test]
fn clearing_dependencies_puts_the_stage_back_on_the_default() {
    let mut pipeline = open();
    assert_eq!(ActionOutcome::Done, pipeline.answer("Test", CLEAR_DEPENDENCIES, ""));
    assert!(
        !pipeline
            .model()
            .stages
            .iter()
            .find(|s| s.name == "Test")
            .unwrap()
            .depends_on_declared
    );
}

#[test]
fn an_unknown_action_is_refused_rather_than_ignored() {
    match open().answer("Build", "azure-pipeline.invent-a-stage", "") {
        ActionOutcome::Refused(reason) => assert!(reason.contains("does not apply"), "{reason}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn every_offered_action_actually_does_something() {
    let ids: Vec<String> = {
        let model = open().model();
        model
            .stages
            .iter()
            .flat_map(|s| {
                std::iter::once(s.id.clone()).chain(
                    s.jobs
                        .iter()
                        .flat_map(|j| std::iter::once(j.id.clone()).chain(j.steps.iter().map(|st| st.id.clone()))),
                )
            })
            .collect()
    };
    assert!(ids.len() >= 5);
    let mut offered = 0;
    for id in &ids {
        for action in open().action_ids(id) {
            offered += 1;
            let mut pipeline = open();
            match pipeline.execute(id, &action) {
                ActionOutcome::Refused(reason) => panic!("{action} was offered on {id} but failed: {reason}"),
                ActionOutcome::Prompt(_) => {
                    let committed = pipeline.answer(id, &action, "A new name");
                    assert_eq!(ActionOutcome::Done, committed, "{action} on {id}");
                }
                ActionOutcome::Done => {}
            }
        }
    }
    assert!(offered >= 20, "only {offered} actions were offered");
}

#[test]
fn an_action_that_asks_for_nothing_is_done_by_its_execution() {
    for (action, id) in [
        (TOGGLE_ENABLED, "Build"),
        (CLEAR_DEPENDENCIES, "Test"),
        (REMOVE, "Test"),
        (ADD_STAGE, "Build"),
        (ADD_JOB, "Build"),
        (ADD_DEPLOYMENT_JOB, "Build"),
        (ADD_STEP, "Build/Compile"),
        (MOVE_STEP_DOWN, "Build/Compile/step-0"),
    ] {
        let mut pipeline = open();
        let before = pipeline.text();
        assert_eq!(ActionOutcome::Done, pipeline.execute(id, action), "{action}");
        assert_ne!(before, pipeline.text(), "{action}");
    }
}

#[test]
fn every_action_that_writes_something_commits_successfully() {
    for (action, id) in [
        (TOGGLE_ENABLED, "Build"),
        (CLEAR_DEPENDENCIES, "Test"),
        (REMOVE, "Test"),
        (ADD_STAGE, "Build"),
        (ADD_JOB, "Build"),
        (ADD_DEPLOYMENT_JOB, "Build"),
        (ADD_STEP, "Build/Compile"),
        (MOVE_STEP_DOWN, "Build/Compile/step-0"),
        (RENAME, "Build"),
    ] {
        let mut pipeline = open();
        assert_eq!(
            ActionOutcome::Done,
            pipeline.answer(id, action, "A new name"),
            "{action} on {id}"
        );
    }
}

#[test]
fn a_selected_arrow_offers_nothing() {
    let pipeline = open();
    let model = pipeline.model();
    let arrow = graph::of_stages(&model)
        .edges
        .into_iter()
        .find(|e| e.from_id == "Build" && e.to_id == "Test")
        .unwrap();
    assert!(pipeline.action_ids(&edge_id(&arrow)).is_empty());
}

#[test]
fn a_job_with_steps_is_offered_the_toggle_that_shows_them() {
    assert!(open_text(ONLY).action_ids("Only/A").contains(&TOGGLE_JOB.to_owned()));
}

#[test]
fn a_job_with_no_steps_is_offered_no_toggle() {
    let pipeline = open_text(
        "stages:\n  - stage: Only\n    jobs:\n      - job: A\n        steps:\n          - script: x\n      - job: Empty\n",
    );
    assert!(!pipeline.action_ids("Only/Empty").contains(&TOGGLE_JOB.to_owned()));
}

#[test]
fn opening_a_stage_shows_its_jobs_and_writes_nothing() {
    let mut pipeline = open();
    let before = pipeline.text();
    assert_eq!("Show jobs", label_of(&pipeline, "Build", TOGGLE_STAGE));
    assert_eq!(ActionOutcome::Done, pipeline.execute("Build", TOGGLE_STAGE));
    assert_eq!("Hide jobs", label_of(&pipeline, "Build", TOGGLE_STAGE));
    assert_eq!(before, pipeline.text());
    assert!(!pipeline.workspace.history().availability().can_undo);
}
