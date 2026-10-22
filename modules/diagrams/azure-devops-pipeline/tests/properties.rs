//! What a selected element shows, what of it may be written, and what a written row does (the standalone host's
//! `PipelineContextPropertyProviderTests`). A row is set through the session, as the property grid sets it, so a row
//! the element does not show is refused with the property resolver's sentence (`ContextPropertyResolver.SetAsync`).

mod common;

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use adp_context::{PropertyEditor, PropertyRow};
use adp_diagram::DiagramWorkspace;
use adp_diagram_azure_devops_pipeline::graph;
use adp_diagram_azure_devops_pipeline::mapper::{self, STEP_TYPE};
use adp_diagram_azure_devops_pipeline::properties::*;
use common::{Pipeline, open_text, tools};

const PIPELINE: &str = "pool:
  vmImage: ubuntu-latest
stages:
  - stage: Build
    displayName: Build the solution
    jobs:
      - job: Compile
        steps:
          - script: |
              dotnet build
              dotnet pack
            displayName: Build it
  - stage: Test
    dependsOn: Build
    condition: succeeded()
    trigger: manual
    jobs:
      - job: Verify
        timeoutInMinutes: 30
        continueOnError: true
        strategy:
          matrix:
            linux:
              image: ubuntu-latest
            windows:
              image: windows-latest
        steps:
          - script: dotnet test
            enabled: false";

const THREE_STAGES: &str = "stages:
  - stage: A
    jobs:
      - job: J
        steps:
          - script: x
  - stage: B
    dependsOn: []
    jobs:
      - job: J
        steps:
          - script: x
  - stage: C
    jobs:
      - job: J
        steps:
          - script: x";

fn rows(pipeline: &Pipeline, id: &str) -> Vec<PropertyRow> {
    pipeline.session.properties(pipeline.workspace.documents(), &[id])
}

fn row(pipeline: &Pipeline, id: &str, property: &str) -> Option<PropertyRow> {
    rows(pipeline, id).into_iter().find(|r| r.id == property)
}

fn by_label(rows: &[PropertyRow], label: &str) -> String {
    rows.iter().find(|r| r.label == label).unwrap().value.clone()
}

fn editable(row: &PropertyRow) -> bool {
    row.read_only_reason.is_empty()
}

#[test]
fn a_stage_shows_what_decides_whether_and_how_it_runs() {
    let pipeline = open_text(PIPELINE);
    let labels: Vec<String> = rows(&pipeline, "Test").into_iter().map(|r| r.label).collect();
    for label in ["Name", "Depends on", "Condition", "Trigger", "Pool", "Jobs"] {
        assert!(labels.contains(&label.to_owned()), "{label}: {labels:?}");
    }
}

#[test]
fn a_stages_implicit_ordering_is_shown_as_what_it_resolves_to() {
    let pipeline = open_text(PIPELINE);
    let depends_on = row(&pipeline, "Test", DEPENDS_ON).unwrap();
    let implicit = row(&pipeline, "Build", DEPENDS_ON).unwrap();
    assert_eq!("Build", depends_on.value);
    assert_eq!("Depends on", depends_on.label);
    assert!(implicit.label.contains("by default"));
}

#[test]
fn a_job_with_no_depends_on_says_what_that_means_rather_than_showing_nothing() {
    let pipeline = open_text(PIPELINE);
    let depends_on = row(&pipeline, "Build/Compile", DEPENDS_ON).unwrap();
    assert!(depends_on.value.contains("nothing"));
    assert!(depends_on.label.contains("by default"));
}

#[test]
fn a_job_shows_its_strategy_and_what_it_multiplies_to() {
    let pipeline = open_text(PIPELINE);
    let shown = rows(&pipeline, "Test/Verify");
    assert_eq!("Matrix", by_label(&shown, "Strategy"));
    assert_eq!("2 times", by_label(&shown, "Runs"));
    assert_eq!("30", by_label(&shown, "Timeout (minutes)"));
    assert_eq!("true", by_label(&shown, "Continue on error"));
}

#[test]
fn a_deployment_job_says_so_and_shows_its_environment() {
    let pipeline = open_text(
        "stages:
  - stage: Deploy
    jobs:
      - deployment: Ship
        environment: production
        strategy:
          runOnce:
            deploy:
              steps:
                - script: ./deploy.sh",
    );
    let shown = rows(&pipeline, "Deploy/Ship");
    assert_eq!("Deployment job", by_label(&shown, "Kind"));
    assert_eq!("production", by_label(&shown, "Environment"));
    assert_eq!("RunOnce", by_label(&shown, "Strategy"));
}

#[test]
fn a_step_shows_its_kind_and_the_value_that_identifies_it() {
    let pipeline = open_text(PIPELINE);
    let shown = rows(&pipeline, "Build/Compile/step-0");
    assert_eq!("Script", by_label(&shown, "Kind"));
    let identifier = shown.iter().find(|r| r.id == "azure-pipeline.identifier").unwrap();
    assert!(identifier.value.contains("dotnet build"));
    assert_eq!(PropertyEditor::Text, identifier.editor);
}

#[test]
fn exactly_the_three_editable_properties_are_editable() {
    let pipeline = open_text(PIPELINE);
    let mut found: Vec<String> = ["Test", "Test/Verify", "Test/Verify/step-0"]
        .iter()
        .flat_map(|id| rows(&pipeline, id))
        .filter(editable)
        .map(|r| r.id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    found.sort();
    assert_eq!(vec![DEPENDS_ON, DISPLAY_NAME, ENABLED], found);
}

#[test]
fn a_read_only_property_says_why_rather_than_just_being_grey() {
    let pipeline = open_text(PIPELINE);
    let condition = row(&pipeline, "Test", "azure-pipeline.condition").unwrap();
    assert!(!editable(&condition));
    assert!(condition.read_only_reason.contains("edited in the pipeline file"));
}

#[test]
fn an_inherited_pool_says_where_it_was_set() {
    let pipeline = open_text(PIPELINE);
    let pool = row(&pipeline, "Build/Compile", "azure-pipeline.pool").unwrap();
    assert_eq!("ubuntu-latest", pool.value);
    assert!(pool.read_only_reason.contains("whole pipeline"));
}

#[test]
fn an_absent_property_is_not_contributed_at_all() {
    let pipeline = open_text(PIPELINE);
    assert!(row(&pipeline, "Build", "azure-pipeline.condition").is_none());
    assert!(row(&pipeline, "Test", "azure-pipeline.condition").is_some());
}

#[test]
fn an_empty_value_is_a_contributed_row_not_a_missing_one() {
    let pipeline = open_text(
        "stages:\n  - stage: Build\n    condition: ''\n    jobs:\n      - job: A\n        steps:\n          - script: x\n",
    );
    let condition = row(&pipeline, "Build", "azure-pipeline.condition").expect("a row");
    assert_eq!("", condition.value);
}

#[test]
fn an_expression_valued_property_shows_the_expression_rather_than_a_guess() {
    let pipeline = open_text(
        "stages:
  - stage: Build
    condition: eq(variables['Build.Reason'], 'Manual')
    jobs:
      - job: A
        steps:
          - script: x",
    );
    assert_eq!(
        "eq(variables['Build.Reason'], 'Manual')",
        row(&pipeline, "Build", "azure-pipeline.condition").unwrap().value
    );
}

#[test]
fn an_enabled_decided_by_an_expression_is_not_offered_as_a_toggle() {
    let pipeline = open_text("steps:\n  - script: x\n    enabled: $(runIt)\n");
    let enabled = row(&pipeline, "stage-0/job-0/step-0", ENABLED).unwrap();
    assert!(!editable(&enabled));
    assert!(enabled.read_only_reason.contains("when the pipeline runs"));
}

#[test]
fn every_property_of_a_template_sourced_element_names_the_file_it_lives_in() {
    let folder = tempfile::tempdir().unwrap();
    fs::create_dir_all(folder.path().join("templates")).unwrap();
    fs::write(
        folder.path().join("templates/jobs.yml"),
        "jobs:\n  - job: FromTemplate\n    displayName: From a template\n    steps:\n      - script: x\n",
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
    let shown = session.properties(workspace.documents(), &["Build/FromTemplate"]);
    assert!(!shown.is_empty());
    for row in shown {
        assert!(!editable(&row));
        assert!(row.read_only_reason.contains("templates/jobs.yml"), "{row:?}");
    }
}

#[test]
fn properties_are_grouped_so_a_stage_does_not_read_as_one_flat_list() {
    let pipeline = open_text(PIPELINE);
    let groups: HashSet<String> = rows(&pipeline, "Test").into_iter().map(|r| r.group).collect();
    for group in [IDENTITY_GROUP, ORDERING_GROUP, EXECUTION_GROUP] {
        assert!(groups.contains(group), "{group}");
    }
}

#[test]
fn every_property_has_its_own_id_within_an_element() {
    let pipeline = open_text(PIPELINE);
    for id in ["Test", "Test/Verify", "Test/Verify/step-0"] {
        let ids: Vec<String> = rows(&pipeline, id).into_iter().map(|r| r.id).collect();
        let distinct: HashSet<&String> = ids.iter().collect();
        assert_eq!(ids.len(), distinct.len(), "{id}");
    }
}

#[test]
fn setting_the_display_name_goes_through_the_same_command_the_canvas_uses() {
    let mut pipeline = open_text(PIPELINE);
    let before = pipeline.text();
    pipeline.set("Build", DISPLAY_NAME, "Built").unwrap();
    assert!(pipeline.text().contains("displayName: Built"));
    assert!(pipeline.undo());
    assert_eq!(before, pipeline.text());
}

#[test]
fn setting_enabled_switches_the_element() {
    let mut pipeline = open_text(PIPELINE);
    pipeline.set("Test/Verify/step-0", ENABLED, "true").unwrap();
    assert!(!pipeline.model().steps().last().unwrap().execution.is_disabled());
}

#[test]
fn depends_on_is_offered_as_a_list_of_the_names_it_could_wait_for() {
    let pipeline = open_text(THREE_STAGES);
    let depends_on = row(&pipeline, "C", DEPENDS_ON).unwrap();
    assert_eq!(PropertyEditor::Choice, depends_on.editor);
    assert!(depends_on.candidates.contains(&"A".to_owned()));
    assert!(depends_on.candidates.contains(&"B".to_owned()));
}

#[test]
fn the_candidates_leave_out_stages_that_would_close_a_loop() {
    let pipeline = open_text(THREE_STAGES);
    let depends_on = row(&pipeline, "A", DEPENDS_ON).unwrap();
    assert!(!depends_on.candidates.contains(&"B".to_owned()));
    assert!(!depends_on.candidates.contains(&"C".to_owned()));
}

#[test]
fn the_first_stage_has_nothing_to_choose_and_says_so() {
    let pipeline = open_text(THREE_STAGES);
    let depends_on = row(&pipeline, "A", DEPENDS_ON).unwrap();
    assert!(!editable(&depends_on));
    assert!(depends_on.read_only_reason.contains("nothing before this"));
}

#[test]
fn the_list_separates_waiting_for_nothing_from_the_default_order() {
    let pipeline = open_text(THREE_STAGES);
    let depends_on = row(&pipeline, "C", DEPENDS_ON).unwrap();
    assert!(depends_on.candidates.contains(&NOTHING_CANDIDATE.to_owned()));
    assert!(depends_on.candidates.contains(&DEFAULT_CANDIDATE.to_owned()));
}

#[test]
fn picking_a_name_makes_the_element_wait_for_it() {
    let mut pipeline = open_text(THREE_STAGES);
    pipeline.set("C", DEPENDS_ON, "A").unwrap();
    assert_eq!(
        vec!["A".to_owned()],
        pipeline
            .model()
            .stages
            .iter()
            .find(|s| s.name == "C")
            .unwrap()
            .depends_on
    );
}

#[test]
fn picking_nothing_writes_an_empty_depends_on() {
    let mut pipeline = open_text(THREE_STAGES);
    pipeline.set("C", DEPENDS_ON, NOTHING_CANDIDATE).unwrap();
    let model = pipeline.model();
    let stage = model.stages.iter().find(|s| s.name == "C").unwrap();
    assert!(stage.depends_on_declared);
    assert!(stage.depends_on.is_empty());
}

#[test]
fn picking_the_default_order_removes_the_key_entirely() {
    let mut pipeline = open_text(THREE_STAGES);
    pipeline.set("C", DEPENDS_ON, "A").unwrap();
    pipeline.set("C", DEPENDS_ON, DEFAULT_CANDIDATE).unwrap();
    assert!(
        !pipeline
            .model()
            .stages
            .iter()
            .find(|s| s.name == "C")
            .unwrap()
            .depends_on_declared
    );
}

#[test]
fn an_element_waiting_for_several_things_is_shown_but_not_offered_as_a_picker() {
    let pipeline = open_text(
        "stages:
  - stage: A
    jobs:
      - job: J
        steps:
          - script: x
  - stage: B
    dependsOn: []
    jobs:
      - job: J
        steps:
          - script: x
  - stage: C
    dependsOn:
      - A
      - B
    jobs:
      - job: J
        steps:
          - script: x",
    );
    let depends_on = row(&pipeline, "C", DEPENDS_ON).unwrap();
    assert_eq!("A, B", depends_on.value);
    assert!(!editable(&depends_on));
    assert!(depends_on.read_only_reason.contains("several things"));
}

#[test]
fn a_jobs_candidates_are_its_siblings_and_not_itself() {
    let pipeline = open_text(
        "stages:
  - stage: Build
    jobs:
      - job: First
        steps:
          - script: x
      - job: Second
        steps:
          - script: x",
    );
    let depends_on = row(&pipeline, "Build/First", DEPENDS_ON).unwrap();
    assert!(depends_on.candidates.contains(&"Second".to_owned()));
    assert!(!depends_on.candidates.contains(&"First".to_owned()));
}

#[test]
fn only_a_choice_carries_candidates() {
    let pipeline = open_text(THREE_STAGES);
    for row in rows(&pipeline, "C")
        .into_iter()
        .filter(|r| r.editor != PropertyEditor::Choice)
    {
        assert!(row.candidates.is_empty(), "{row:?}");
    }
}

#[test]
fn a_depends_on_naming_something_unknown_is_refused_and_the_file_is_unchanged() {
    let mut pipeline = open_text(PIPELINE);
    let before = pipeline.text();
    let error = pipeline.set("Test", DEPENDS_ON, "Imaginary").unwrap_err();
    assert!(error.contains("Imaginary"), "{error}");
    assert_eq!(before, pipeline.text());
}

#[test]
fn a_depends_on_that_would_create_a_cycle_is_refused() {
    let mut pipeline = open_text(THREE_STAGES);
    let error = pipeline.set("C", DEPENDS_ON, "C").unwrap_err();
    assert!(error.contains("already waiting for it"), "{error}");
}

#[test]
fn setting_a_property_this_element_does_not_have_is_refused() {
    let mut pipeline = open_text(PIPELINE);
    let error = pipeline.set("Build/Compile/step-0", DEPENDS_ON, "Test").unwrap_err();
    assert!(error.contains("is not a property of what is selected"), "{error}");
}

#[test]
fn an_element_that_is_gone_has_no_properties() {
    assert!(rows(&open_text(PIPELINE), "NoSuchStage").is_empty());
}

#[test]
fn a_file_that_does_not_parse_has_no_properties() {
    assert!(rows(&open_text("stages:\n  - stage: Build\n   jobs: [\n"), "Build").is_empty());
}

#[test]
fn a_selected_arrow_describes_nothing() {
    let pipeline = open_text(PIPELINE);
    let model = pipeline.model();
    let arrow = graph::of_stages(&model)
        .edges
        .into_iter()
        .find(|e| e.from_id == "Build" && e.to_id == "Test")
        .unwrap();
    assert!(rows(&pipeline, &mapper::edge_id(&arrow)).is_empty());
}

#[test]
fn a_step_the_canvas_drew_can_be_selected_and_described() {
    let pipeline = open_text(PIPELINE);
    let opened: HashSet<String> = ["Build", "Build/Compile"].iter().map(|s| (*s).to_owned()).collect();
    let drawn = mapper::visible(&pipeline.model(), &opened);
    let steps: Vec<_> = drawn.iter().filter(|e| e.element_type == STEP_TYPE).collect();
    assert_eq!(1, steps.len());
    let shown = rows(&pipeline, &steps[0].id);
    assert_eq!("Script", by_label(&shown, "Kind"));
    assert!(
        shown
            .iter()
            .find(|r| r.id == "azure-pipeline.identifier")
            .unwrap()
            .value
            .contains("dotnet build")
    );
}
