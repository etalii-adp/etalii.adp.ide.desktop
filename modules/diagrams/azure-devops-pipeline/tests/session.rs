//! An open pipeline: what it delivers, what opening and closing a stage or a job pushes, edits from this view and from
//! outside reaching it, and drags refused (the standalone host's `PipelineSessionTests`, with the part of
//! `ServiceCollectionAddAzurePipelineTests` that is not the standalone host's dependency injection: a view opened on
//! its body reads its templates from the project's root, and two views of one body share its document). Which stages
//! a view has open is that view's own, as it was a connection's in the standalone host.

mod common;

use std::fs;
use std::path::Path;

use adp_diagram::{ActionOutcome, DiagramDelta, DiagramViewport, DiagramWorkspace, DocumentChange};
use adp_diagram_azure_devops_pipeline::actions::{TOGGLE_JOB, TOGGLE_STAGE};
use adp_diagram_azure_devops_pipeline::view::{PARENT_REFUSAL, PLACEMENT_REFUSAL};
use common::{Pipeline, open_text, tools};

const TWO_STAGES: &str = "stages:
  - stage: Build
    jobs:
      - job: Compile
        steps:
          - script: x
      - job: Lint
        steps:
          - script: y
  - stage: Test
    jobs:
      - job: Verify
        steps:
          - script: z";

const STEPS_OUT_OF_ALPHABETICAL_ORDER: &str = "stages:
  - stage: Build
    jobs:
      - job: Compile
        steps:
          - script: zebra
            displayName: Zebra
          - script: apple
            displayName: Apple
          - script: mango
            displayName: Mango
      - job: Empty";

fn added(deltas: &[DiagramDelta]) -> Vec<String> {
    deltas
        .iter()
        .filter_map(|d| match d {
            DiagramDelta::Add(elements) => Some(elements.iter().map(|e| e.id.clone()).collect::<Vec<_>>()),
            _ => None,
        })
        .flatten()
        .collect()
}

fn removed(deltas: &[DiagramDelta]) -> Vec<String> {
    deltas
        .iter()
        .filter_map(|d| match d {
            DiagramDelta::Remove(ids) => Some(ids.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

/// A fresh baseline of the view, after it was opened.
fn baseline(pipeline: &mut Pipeline) -> Vec<DiagramDelta> {
    pipeline.session.baseline(pipeline.workspace.documents()).unwrap()
}

/// What the view's next rendering pushes against what it last delivered.
fn refresh(pipeline: &mut Pipeline) -> Vec<DiagramDelta> {
    pipeline
        .session
        .update_view(pipeline.workspace.documents(), DiagramViewport::UNBOUNDED)
        .unwrap()
}

/// What a change of the body pushes.
fn changed(pipeline: &mut Pipeline) -> Vec<DiagramDelta> {
    let body = pipeline.body.clone();
    match pipeline
        .session
        .on_document_changed(pipeline.workspace.documents(), &body)
    {
        DocumentChange::Changed(deltas) => deltas,
        other => panic!("{other:?}"),
    }
}

#[test]
fn opening_a_pipeline_streams_a_baseline() {
    let mut pipeline = open_text(TWO_STAGES);
    let ids = added(&baseline(&mut pipeline));
    for id in ["Build", "Test", "edge:Build->Test"] {
        assert!(ids.contains(&id.to_owned()), "{id}: {ids:?}");
    }
}

#[test]
fn a_baseline_of_a_collapsed_pipeline_carries_no_jobs() {
    let mut pipeline = open_text(TWO_STAGES);
    assert!(!added(&baseline(&mut pipeline)).contains(&"Build/Compile".to_owned()));
}

#[test]
fn expanding_a_stage_pushes_its_jobs_without_removing_anything() {
    let mut pipeline = open_text(TWO_STAGES);
    baseline(&mut pipeline);
    assert_eq!(ActionOutcome::Done, pipeline.execute("Build", TOGGLE_STAGE));
    let deltas = refresh(&mut pipeline);
    let ids = added(&deltas);
    assert!(ids.contains(&"Build/Compile".to_owned()));
    assert!(ids.contains(&"Build/Lint".to_owned()));
    assert!(removed(&deltas).is_empty());
}

#[test]
fn collapsing_a_stage_takes_its_jobs_back_off() {
    let mut pipeline = open_text(TWO_STAGES);
    baseline(&mut pipeline);
    pipeline.execute("Build", TOGGLE_STAGE);
    refresh(&mut pipeline);
    pipeline.execute("Build", TOGGLE_STAGE);
    assert!(removed(&refresh(&mut pipeline)).contains(&"Build/Compile".to_owned()));
}

#[test]
fn a_toggle_in_another_view_is_not_this_ones_business() {
    let mut mine = open_text(TWO_STAGES);
    let (_, mut yours) = mine.workspace.open(&mut tools(), Path::new("pipeline.adp")).unwrap();
    yours.baseline(mine.workspace.documents()).unwrap();
    baseline(&mut mine);
    mine.execute("Build", TOGGLE_STAGE);
    assert!(added(&refresh(&mut mine)).contains(&"Build/Compile".to_owned()));
    let theirs = yours
        .update_view(mine.workspace.documents(), DiagramViewport::UNBOUNDED)
        .unwrap();
    assert!(added(&theirs).is_empty() && removed(&theirs).is_empty(), "{theirs:?}");
    assert_eq!(TWO_STAGES, mine.text());
}

#[test]
fn an_edit_through_another_view_reaches_this_one() {
    let mut mine = open_text(TWO_STAGES);
    let (_, yours) = mine.workspace.open(&mut tools(), Path::new("pipeline.adp")).unwrap();
    baseline(&mut mine);
    let outcome = yours.answer(
        &mut mine.workspace,
        &adp_diagram::ActionTarget::Element("Build".to_owned()),
        adp_diagram_azure_devops_pipeline::actions::RENAME,
        "Build it all",
    );
    assert_eq!(ActionOutcome::Done, outcome);
    let deltas = changed(&mut mine);
    let build = deltas
        .iter()
        .filter_map(|d| match d {
            DiagramDelta::Add(elements) => elements.iter().find(|e| e.id == "Build"),
            _ => None,
        })
        .next()
        .expect("the renamed stage is pushed");
    assert_eq!("Build it all", build.attribute("displayName"));
}

#[test]
fn a_stage_that_disappears_is_removed_rather_than_left_on_the_canvas() {
    let mut pipeline = open_text(TWO_STAGES);
    baseline(&mut pipeline);
    fs::write(
        &pipeline.body,
        "stages:\n  - stage: Build\n    jobs:\n      - job: Compile\n        steps:\n          - script: x\n",
    )
    .unwrap();
    let body = pipeline.body.clone();
    pipeline.workspace.reconcile(&body);
    let gone = removed(&changed(&mut pipeline));
    assert!(gone.contains(&"Test".to_owned()));
    assert!(gone.contains(&"edge:Build->Test".to_owned()));
}

#[test]
fn an_external_edit_pushes_the_difference() {
    let mut pipeline = open_text(TWO_STAGES);
    baseline(&mut pipeline);
    fs::write(
        &pipeline.body,
        format!(
            "{TWO_STAGES}\n  - stage: Ship\n    jobs:\n      - job: Deploy\n        steps:\n          - script: q\n"
        ),
    )
    .unwrap();
    let body = pipeline.body.clone();
    pipeline.workspace.reconcile(&body);
    assert!(added(&changed(&mut pipeline)).contains(&"Ship".to_owned()));
}

#[test]
fn a_change_to_a_different_file_is_not_this_sessions_business() {
    let mut pipeline = open_text(TWO_STAGES);
    baseline(&mut pipeline);
    let other = pipeline.folder.path().join("other.yml");
    assert!(matches!(
        pipeline
            .session
            .on_document_changed(pipeline.workspace.documents(), &other),
        DocumentChange::OtherDocument
    ));
}

#[test]
fn a_file_that_does_not_parse_shows_nothing_rather_than_half_drawn() {
    let mut pipeline = open_text("stages:\n  - stage: Build\n   jobs: [\n");
    assert!(added(&baseline(&mut pipeline)).is_empty());
    let problems = pipeline.session.problems(pipeline.workspace.documents());
    assert_eq!(1, problems.len());
    assert!(problems[0].message.contains("not YAML"), "{problems:?}");
}

#[test]
fn a_working_file_reports_nothing_wrong() {
    let pipeline = open_text(TWO_STAGES);
    assert!(pipeline.session.problems(pipeline.workspace.documents()).is_empty());
}

#[test]
fn reporting_a_viewport_delivers_the_whole_pipeline() {
    let mut pipeline = open_text(TWO_STAGES);
    baseline(&mut pipeline);
    let deltas = pipeline
        .session
        .update_view(
            pipeline.workspace.documents(),
            DiagramViewport {
                min_x: 0.0,
                min_y: 0.0,
                max_x: 10.0,
                max_y: 10.0,
            },
        )
        .unwrap();
    assert!(removed(&deltas).is_empty());
}

#[test]
fn dragging_a_stage_onto_another_is_refused_with_a_reason_rather_than_ignored() {
    let mut pipeline = open_text(TWO_STAGES);
    let refusal = pipeline
        .session
        .move_element(&mut pipeline.workspace, "Build", Some("Test"), 0)
        .unwrap_err();
    assert_eq!(PARENT_REFUSAL, refusal);
    assert!(refusal.contains("dependencies"));
    assert_eq!(TWO_STAGES, pipeline.text());
}

#[test]
fn dragging_to_a_point_is_refused_with_a_reason() {
    let mut pipeline = open_text(TWO_STAGES);
    let refusal = pipeline
        .session
        .move_elements(&mut pipeline.workspace, &[("Build".to_owned(), 100.0, 200.0)])
        .unwrap_err();
    assert_eq!(PLACEMENT_REFUSAL, refusal);
}

#[test]
fn expanding_a_job_pushes_its_steps_in_declared_order() {
    let mut pipeline = open_text(STEPS_OUT_OF_ALPHABETICAL_ORDER);
    baseline(&mut pipeline);
    pipeline.execute("Build", TOGGLE_STAGE);
    refresh(&mut pipeline);
    assert_eq!(ActionOutcome::Done, pipeline.execute("Build/Compile", TOGGLE_JOB));
    let deltas = refresh(&mut pipeline);
    let steps: Vec<String> = added(&deltas)
        .into_iter()
        .filter(|id| id.starts_with("Build/Compile/"))
        .collect();
    assert_eq!(
        vec!["Build/Compile/step-0", "Build/Compile/step-1", "Build/Compile/step-2"],
        steps
    );
    assert!(removed(&deltas).is_empty());
}

#[test]
fn collapsing_a_job_takes_exactly_its_steps_back_off() {
    let mut pipeline = open_text(STEPS_OUT_OF_ALPHABETICAL_ORDER);
    baseline(&mut pipeline);
    pipeline.execute("Build", TOGGLE_STAGE);
    pipeline.execute("Build/Compile", TOGGLE_JOB);
    refresh(&mut pipeline);
    pipeline.execute("Build/Compile", TOGGLE_JOB);
    let gone = removed(&refresh(&mut pipeline));
    let mut steps: Vec<&String> = gone.iter().filter(|id| id.starts_with("Build/Compile/")).collect();
    steps.sort();
    assert_eq!(
        vec!["Build/Compile/step-0", "Build/Compile/step-1", "Build/Compile/step-2"],
        steps
    );
    assert!(!gone.contains(&"Build/Compile".to_owned()));
    assert!(!gone.contains(&"Build".to_owned()));
}

#[test]
fn an_unopened_job_carries_no_steps_on_the_wire() {
    let mut pipeline = open_text(STEPS_OUT_OF_ALPHABETICAL_ORDER);
    baseline(&mut pipeline);
    pipeline.execute("Build", TOGGLE_STAGE);
    assert!(!added(&refresh(&mut pipeline)).iter().any(|id| id.contains("/step-")));
}

#[test]
fn a_view_opened_on_its_body_reads_its_templates_from_the_projects_root() {
    let folder = tempfile::tempdir().unwrap();
    fs::create_dir_all(folder.path().join("templates")).unwrap();
    fs::create_dir_all(folder.path().join("pipelines")).unwrap();
    fs::write(
        folder.path().join("templates/shared.yml"),
        "jobs:\n  - job: Shared\n    steps:\n      - script: x\n",
    )
    .unwrap();
    fs::write(
        folder.path().join("pipelines/build.yml"),
        "jobs:\n  - template: /templates/shared.yml\n",
    )
    .unwrap();
    fs::write(folder.path().join("pipelines/build.adp"), "azure-devops/pipeline\r\n").unwrap();
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (opened, mut session) = workspace.open(&mut tools(), Path::new("pipelines/build.adp")).unwrap();
    assert_eq!("azure-devops/pipeline", opened.origin);
    assert!(!added(&session.baseline(workspace.documents()).unwrap()).is_empty());
    let selected = session
        .select(workspace.documents(), "stage-0/Shared")
        .expect("the template's job");
    assert_eq!("Shared", selected.text);
}

#[test]
fn a_registration_names_its_body_with_a_body_header() {
    let folder = tempfile::tempdir().unwrap();
    fs::write(
        folder.path().join("azure-pipelines.yml"),
        "stages:\n  - stage: Build\n    jobs:\n      - job: A\n        steps:\n          - script: x\n",
    )
    .unwrap();
    fs::write(
        folder.path().join("ci.adp"),
        "azure-devops/pipeline\r\nbody: azure-pipelines.yml\r\n",
    )
    .unwrap();
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (opened, _) = workspace.open(&mut tools(), Path::new("ci.adp")).unwrap();
    assert_eq!(folder.path().join("azure-pipelines.yml"), opened.body_path);
}
