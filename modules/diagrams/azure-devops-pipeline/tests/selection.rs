//! What a selected stage, job, step or arrow reads as (the standalone host's `PipelineContextSourceResolverTests`):
//! its chain of names and its text, an arrow as the element that waits, and nothing for what is not there. The
//! standalone resolver's tracking of a selection across edits is the session's here: a selection is described again
//! from the document as it is now.

mod common;

use std::fs;
use std::path::Path;

use adp_diagram::{ActionOutcome, DiagramWorkspace};
use adp_diagram_azure_devops_pipeline::graph::{self, PipelineEdge};
use adp_diagram_azure_devops_pipeline::mapper::edge_id;
use adp_diagram_azure_devops_pipeline::model::PipelineModel;
use adp_diagram_azure_devops_pipeline::view::find;
use common::{Pipeline, open_text, tools};

const PIPELINE: &str = "stages:
  - stage: Build
    displayName: Build the solution
    jobs:
      - job: Compile
        steps:
          - script: dotnet build
            displayName: Compile it
  - stage: Test
    jobs:
      - job: Verify
        steps:
          - script: dotnet test";

const ARROWS: &str = "stages:
  - stage: Build
    jobs:
      - job: Compile
        steps:
          - script: dotnet build
      - job: Pack
        dependsOn: Compile
        steps:
          - script: dotnet pack
  - stage: Test
    jobs:
      - job: Verify
        steps:
          - script: dotnet test
  - stage: Deploy
    dependsOn:
      - Test
      - Nowhere
    jobs:
      - job: Ship
        steps:
          - script: ./ship";

fn describe(pipeline: &Pipeline, id: &str) -> Option<(Vec<String>, String)> {
    pipeline
        .session
        .select(pipeline.workspace.documents(), id)
        .map(|selected| (selected.path, selected.text))
}

fn drawn_edges(model: &PipelineModel) -> Vec<PipelineEdge> {
    let mut edges = graph::of_stages(model).edges;
    for stage in &model.stages {
        edges.extend(graph::of_jobs(stage).edges);
    }
    edges
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn a_stage_resolves_to_its_own_name() {
    let (path, text) = describe(&open_text(PIPELINE), "Build").unwrap();
    assert_eq!(strings(&["Build the solution"]), path);
    assert_eq!("Build the solution", text);
}

#[test]
fn a_job_resolves_to_its_stage_and_itself() {
    let (path, _) = describe(&open_text(PIPELINE), "Build/Compile").unwrap();
    assert_eq!(strings(&["Build the solution", "Compile"]), path);
}

#[test]
fn a_step_resolves_through_its_job() {
    let (path, _) = describe(&open_text(PIPELINE), "Build/Compile/step-0").unwrap();
    assert_eq!(strings(&["Build the solution", "Compile", "Compile it"]), path);
}

#[test]
fn an_unknown_element_is_rejected() {
    assert!(describe(&open_text(PIPELINE), "NoSuchStage").is_none());
}

#[test]
fn a_file_that_does_not_parse_selects_nothing() {
    assert!(describe(&open_text("stages:\n  - stage: Build\n   jobs: [\n"), "Build").is_none());
}

#[test]
fn an_element_from_a_template_is_found_with_its_chain() {
    let folder = tempfile::tempdir().unwrap();
    fs::create_dir_all(folder.path().join("templates")).unwrap();
    fs::write(
        folder.path().join("templates/build.yml"),
        "jobs:\n  - job: Compile\n    steps:\n      - script: x\n",
    )
    .unwrap();
    fs::write(
        folder.path().join("pipeline.yml"),
        "stages:\n  - stage: Build\n    jobs:\n      - template: templates/build.yml\n",
    )
    .unwrap();
    fs::write(folder.path().join("pipeline.adp"), "azure-devops/pipeline\r\n").unwrap();
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (_, session) = workspace.open(&mut tools(), Path::new("pipeline.adp")).unwrap();
    let selected = session.select(workspace.documents(), "Build/Compile").expect("found");
    assert_eq!(strings(&["Build", "Compile"]), selected.path);
}

#[test]
fn a_renamed_element_is_described_by_its_new_name() {
    let mut pipeline = open_text(PIPELINE);
    assert_eq!(
        ActionOutcome::Done,
        pipeline.answer("Build", adp_diagram_azure_devops_pipeline::actions::RENAME, "Built")
    );
    assert_eq!(strings(&["Built"]), describe(&pipeline, "Build").unwrap().0);
}

#[test]
fn a_deleted_element_selects_nothing_any_more() {
    let mut pipeline = open_text(PIPELINE);
    assert_eq!(
        ActionOutcome::Done,
        pipeline.execute("Test", adp_diagram_azure_devops_pipeline::actions::REMOVE)
    );
    assert!(describe(&pipeline, "Test").is_none());
}

#[test]
fn every_arrow_the_canvas_draws_resolves_implicit_explicit_broken_and_job_level() {
    let pipeline = open_text(ARROWS);
    let edges = drawn_edges(&pipeline.model());
    assert!(edges.iter().any(|e| e.is_implicit));
    assert!(edges.iter().any(|e| e.is_broken));
    assert!(
        edges
            .iter()
            .any(|e| !e.is_implicit && !e.is_broken && e.to_id == "Deploy")
    );
    assert!(edges.iter().any(|e| e.to_id == "Build/Pack"));
    for edge in edges {
        assert!(describe(&pipeline, &edge_id(&edge)).is_some(), "{}", edge_id(&edge));
    }
}

#[test]
fn an_arrow_answers_as_the_element_that_waits() {
    let pipeline = open_text(ARROWS);
    let model = pipeline.model();
    let edges = drawn_edges(&model);
    for (from, to, path, text) in [
        ("Test", "Deploy", vec!["Deploy"], "Deploy waits for Test"),
        ("Build", "Test", vec!["Test"], "Test waits for Build"),
        ("", "Deploy", vec!["Deploy"], "Deploy waits for Nowhere"),
        (
            "Build/Compile",
            "Build/Pack",
            vec!["Build", "Pack"],
            "Pack waits for Compile",
        ),
    ] {
        let edge = edges.iter().find(|e| e.from_id == from && e.to_id == to).unwrap();
        let (found_path, found_text) = find(&model, &edge_id(edge)).unwrap();
        assert_eq!(strings(&path), found_path);
        assert_eq!(text, found_text);
    }
}

#[test]
fn an_arrow_that_is_not_drawn_is_still_rejected() {
    assert!(describe(&open_text(ARROWS), "edge:Deploy->Build").is_none());
}

#[test]
fn a_selected_arrow_survives_an_unrelated_edit_and_clears_when_it_goes() {
    let mut pipeline = open_text(ARROWS);
    let id = "edge:Test->Deploy";
    assert!(describe(&pipeline, id).is_some());
    pipeline
        .answer("Build", adp_diagram_azure_devops_pipeline::actions::RENAME, "Built")
        .eq(&ActionOutcome::Done)
        .then_some(())
        .unwrap();
    assert_eq!(strings(&["Deploy"]), describe(&pipeline, id).unwrap().0);
    // Another program takes Test out of what Deploy waits for.
    let changed = pipeline.text().replace("      - Test\n", "");
    fs::write(&pipeline.body, changed).unwrap();
    let body = pipeline.body.clone();
    pipeline.workspace.reconcile(&body);
    assert!(describe(&pipeline, id).is_none());
}
