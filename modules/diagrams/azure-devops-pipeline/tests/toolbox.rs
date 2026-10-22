//! The palette, and a drop as the menu's own action (the standalone host's `PipelineToolboxProviderTests`), and the
//! tool type's identity: one origin whose `.yml` is shared, so a bare body is never claimed (`DiagramTests`).

mod common;

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use adp_diagram::{ActionOutcome, ActionTarget, DiagramWorkspace};
use adp_diagram_azure_devops_pipeline::actions;
use adp_diagram_azure_devops_pipeline::toolbox::items;
use common::{Pipeline, open_text, tools};

const PIPELINE: &str = "stages:
  - stage: Build
    jobs:
      - job: Compile
        steps:
          - script: dotnet build";

fn drop(pipeline: &mut Pipeline, entry: &str, onto: &str) -> ActionOutcome {
    let item = items().into_iter().find(|i| i.id == entry).unwrap();
    pipeline.answer(onto, &item.drop_action_id, "")
}

#[test]
fn it_describes_the_four_things_that_can_be_added() {
    let labels: Vec<String> = items().into_iter().map(|i| i.label).collect();
    assert_eq!(vec!["Stage", "Job", "Deployment job", "Script step"], labels);
}

#[test]
fn every_entry_carries_only_data() {
    for item in items() {
        assert!(!item.id.is_empty());
        assert!(!item.label.is_empty());
        assert!(!item.icon.is_empty());
        assert!(!item.description.is_empty());
        assert!(!item.drop_action_id.is_empty());
    }
}

#[test]
fn every_entry_names_an_action_that_exists() {
    let known = [
        actions::ADD_STAGE,
        actions::ADD_JOB,
        actions::ADD_DEPLOYMENT_JOB,
        actions::ADD_STEP,
    ];
    for item in items() {
        assert!(known.contains(&item.drop_action_id.as_str()), "{}", item.drop_action_id);
    }
}

#[test]
fn every_entry_says_where_it_goes() {
    for item in items() {
        assert!(item.description.contains("Drop on a"), "{}", item.description);
    }
}

#[test]
fn every_entry_has_its_own_id() {
    let ids: HashSet<String> = items().into_iter().map(|i| i.id).collect();
    assert_eq!(items().len(), ids.len());
}

#[test]
fn the_view_offers_the_palette() {
    let pipeline = open_text(PIPELINE);
    assert_eq!(items(), pipeline.session.toolbox(pipeline.workspace.documents()));
}

#[test]
fn dropping_a_job_on_a_stage_adds_one() {
    let mut pipeline = open_text(PIPELINE);
    assert_eq!(
        ActionOutcome::Done,
        drop(&mut pipeline, "azure-pipeline.toolbox.job", "Build")
    );
    assert_eq!(2, pipeline.model().stages[0].jobs.len());
}

#[test]
fn dropping_a_step_on_a_job_adds_one() {
    let mut pipeline = open_text(PIPELINE);
    assert_eq!(
        ActionOutcome::Done,
        drop(&mut pipeline, "azure-pipeline.toolbox.step", "Build/Compile")
    );
    assert_eq!(2, pipeline.model().jobs().next().unwrap().steps.len());
}

#[test]
fn dropping_a_stage_on_a_stage_adds_another_after_it() {
    let mut pipeline = open_text(PIPELINE);
    assert_eq!(
        ActionOutcome::Done,
        drop(&mut pipeline, "azure-pipeline.toolbox.stage", "Build")
    );
    assert_eq!(2, pipeline.model().stages.len());
}

#[test]
fn dropping_a_deployment_job_on_a_stage_adds_one_that_would_run() {
    let mut pipeline = open_text(PIPELINE);
    assert_eq!(
        ActionOutcome::Done,
        drop(&mut pipeline, "azure-pipeline.toolbox.deployment-job", "Build")
    );
    let model = pipeline.model();
    let job = model.jobs().find(|j| j.is_deployment).unwrap();
    assert!(!job.environment.is_empty());
    assert!(!job.steps.is_empty());
}

#[test]
fn dropping_a_step_on_a_stage_is_refused_by_the_same_rule_that_keeps_it_out_of_the_menu() {
    let mut pipeline = open_text(PIPELINE);
    match drop(&mut pipeline, "azure-pipeline.toolbox.step", "Build") {
        ActionOutcome::Refused(reason) => assert!(reason.contains("goes in a job"), "{reason}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn dropping_on_a_template_sourced_element_is_refused() {
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
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (_, session) = workspace.open(&mut tools(), Path::new("pipeline.adp")).unwrap();
    let outcome = session.answer(
        &mut workspace,
        &ActionTarget::Element("Build/FromTemplate".to_owned()),
        actions::ADD_STEP,
        "",
    );
    match outcome {
        ActionOutcome::Refused(reason) => assert!(reason.contains("templates/jobs.yml"), "{reason}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_drop_is_undone_like_any_other_edit() {
    let mut pipeline = open_text(PIPELINE);
    let before = pipeline.text();
    drop(&mut pipeline, "azure-pipeline.toolbox.job", "Build");
    assert!(pipeline.undo());
    assert_eq!(before, pipeline.text());
}

#[test]
fn the_module_serves_one_pipeline_type() {
    assert_eq!("azure-devops/pipeline", adp_diagram_azure_devops_pipeline::ORIGIN);
    assert_eq!(".yml", adp_diagram_azure_devops_pipeline::EXTENSION);
}

#[test]
fn its_extension_is_shared_and_so_never_routes_a_bare_body() {
    let folder = tempfile::tempdir().unwrap();
    fs::write(folder.path().join("compose.yml"), "services: {}\n").unwrap();
    let mut workspace = DiagramWorkspace::new(folder.path());
    let opened = workspace.open(&mut tools(), Path::new("compose.yml"));
    assert!(opened.is_err());
    // Registered, the same body is a pipeline.
    fs::write(folder.path().join("compose.adp"), "azure-devops/pipeline\r\n").unwrap();
    let (opened, _) = workspace.open(&mut tools(), Path::new("compose.yml")).unwrap();
    assert_eq!("azure-devops/pipeline", opened.origin);
}
