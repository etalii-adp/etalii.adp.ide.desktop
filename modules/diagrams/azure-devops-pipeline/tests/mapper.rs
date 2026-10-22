//! What the canvas is handed: stages, the jobs of open stages, the steps of open jobs, the arrows with what they wait
//! on, and the templates that could not be followed (the standalone host's `PipelineElementMapperTests`). The
//! standalone payload's fields are the element's attributes here, and an element is placed at its centre where the
//! standalone host sent its top-left.

mod common;

use std::collections::HashSet;

use adp_diagram::{DiagramElement, ElementKind};
use adp_diagram_azure_devops_pipeline::layout::{self, PipelineMetrics};
use adp_diagram_azure_devops_pipeline::mapper::{self, EDGE_TYPE, JOB_TYPE, STAGE_TYPE, STEP_TYPE, TEMPLATE_TYPE};
use adp_diagram_azure_devops_pipeline::model::{EditTarget, PipelineModel, UnresolvedReason, UnresolvedTemplate};
use adp_diagram_azure_devops_pipeline::parser;
use adp_diagram_azure_devops_pipeline::writer::PipelineWriter;
use common::{all_fixtures, document, parse, parse_text};

fn map(model: &PipelineModel, expanded: &[&str]) -> Vec<DiagramElement> {
    let expanded: HashSet<String> = expanded.iter().map(|s| (*s).to_owned()).collect();
    mapper::visible(model, &expanded)
}

fn by_id<'a>(elements: &'a [DiagramElement], id: &str) -> &'a DiagramElement {
    elements
        .iter()
        .find(|e| e.id == id)
        .unwrap_or_else(|| panic!("{id} is mapped"))
}

#[test]
fn a_stage_travels_as_one_element_with_its_own_type() {
    let elements = map(&parse("multi-stage.yml"), &[]);
    let build = by_id(&elements, "Build");
    assert_eq!(STAGE_TYPE, build.element_type);
    assert!(build.element_type.starts_with("azure-devops/pipeline"));
    assert_eq!("Build the solution", build.attribute("displayName"));
}

#[test]
fn an_element_id_is_its_path_within_the_document() {
    let elements = map(&parse("multi-stage.yml"), &["Test"]);
    assert!(elements.iter().any(|e| e.id == "Test"));
    assert!(elements.iter().any(|e| e.id == "Test/Unit"));
}

#[test]
fn an_edit_elsewhere_in_the_file_does_not_change_an_elements_id() {
    let mut document = document("multi-stage.yml");
    fn ids(document: &adp_diagram_azure_devops_pipeline::lines::LineDocument) -> Vec<String> {
        map(&parser::parse(document).unwrap(), &[])
            .into_iter()
            .map(|e| e.id)
            .collect()
    }
    let before = ids(&document);
    let step = parser::parse(&document)
        .unwrap()
        .jobs()
        .find(|j| j.name == "Compile")
        .unwrap()
        .steps[1]
        .clone();
    PipelineWriter::new(&mut document)
        .set_display_name(&EditTarget::of_step(&step), "Something else entirely")
        .unwrap();
    assert_eq!(before, ids(&document));
}

#[test]
fn every_element_id_is_unique() {
    let mut mapped = 0;
    for name in all_fixtures() {
        let model = parse(&name);
        let stages: Vec<&str> = model.stages.iter().map(|s| s.id.as_str()).collect();
        let ids: Vec<String> = map(&model, &stages).into_iter().map(|e| e.id).collect();
        let distinct: HashSet<&String> = ids.iter().collect();
        assert_eq!(ids.len(), distinct.len(), "{name}");
        mapped += ids.len();
    }
    assert!(mapped >= 40, "only {mapped} elements were mapped");
}

#[test]
fn an_elements_position_comes_from_the_layout() {
    let model = parse("multi-stage.yml");
    let arrangement = layout::arrange_pipeline(&model, &PipelineMetrics::default(), &HashSet::new());
    let placed = arrangement.of("Build").unwrap();
    let elements = map(&model, &[]);
    let build = by_id(&elements, "Build");
    assert_eq!(placed.x + placed.size.width / 2.0, build.x);
    assert_eq!(placed.y + placed.size.height / 2.0, build.y);
}

#[test]
fn a_dependency_edge_is_an_element_naming_its_two_ends() {
    let elements = map(&parse("multi-stage.yml"), &[]);
    let edge = elements
        .iter()
        .find(|e| e.element_type == EDGE_TYPE && e.target.as_deref() == Some("Test"))
        .unwrap();
    assert_eq!(ElementKind::Relation, edge.kind);
    assert_eq!(Some("Build"), edge.source.as_deref());
}

#[test]
fn an_edge_id_is_built_from_its_ends_rather_than_a_counter() {
    let elements = map(&parse("multi-stage.yml"), &[]);
    assert!(elements.iter().any(|e| e.id == "edge:Build->Test"));
}

#[test]
fn an_implicit_edge_arrives_marked_as_implicit() {
    let model = parse_text(
        "stages:\n  - stage: One\n    jobs:\n      - job: J\n        steps:\n          - script: x\n  - stage: Two\n    jobs:\n      - job: J\n        steps:\n          - script: x",
    );
    let elements = map(&model, &[]);
    let edge = elements.iter().find(|e| e.element_type == EDGE_TYPE).unwrap();
    assert_eq!("true", edge.attribute("implicitDependency"));
}

#[test]
fn an_edge_somebody_wrote_down_is_not_marked_implicit() {
    let elements = map(&parse("multi-stage.yml"), &[]);
    assert_eq!(
        "false",
        by_id(&elements, "edge:Build->Test").attribute("implicitDependency")
    );
}

#[test]
fn a_broken_edge_arrives_marked_broken_rather_than_missing() {
    let elements = map(&parse("edge-broken-graph.yml"), &[]);
    let broken: Vec<_> = elements
        .iter()
        .filter(|e| e.element_type == EDGE_TYPE && e.attribute("broken") == "true")
        .collect();
    assert_eq!(1, broken.len());
    assert_eq!("DoesNotExist", broken[0].attribute("name"));
    assert_eq!(Some("Ghost"), broken[0].target.as_deref());
}

#[test]
fn two_dangling_names_into_one_stage_are_two_arrows() {
    let model = parse_text(
        "stages:\n  - stage: Ghost\n    dependsOn: [Nope, AlsoNope]\n    jobs:\n      - job: J\n        steps:\n          - script: x\n",
    );
    let edges: Vec<String> = map(&model, &[])
        .into_iter()
        .filter(|e| e.element_type == EDGE_TYPE)
        .map(|e| e.id)
        .collect();
    assert_eq!(2, edges.len());
    assert_ne!(edges[0], edges[1]);
}

#[test]
fn an_edges_condition_says_which_outcome_it_waits_for() {
    let elements = map(&parse("multi-stage.yml"), &[]);
    let edge = elements
        .iter()
        .find(|e| e.element_type == EDGE_TYPE && e.target.as_deref() == Some("Notify"))
        .unwrap();
    assert_eq!("always", edge.attribute("edgeCondition"));
    assert_eq!("always()", edge.attribute("condition"));
}

#[test]
fn an_unconditional_edge_waits_on_success() {
    let elements = map(&parse("multi-stage.yml"), &[]);
    assert_eq!(
        "onSuccess",
        by_id(&elements, "edge:Build->Test").attribute("edgeCondition")
    );
}

#[test]
fn a_collapsed_stage_sends_no_jobs() {
    let elements = map(&parse("multi-stage.yml"), &[]);
    assert!(!elements.iter().any(|e| e.element_type == JOB_TYPE));
}

#[test]
fn an_expanded_stage_sends_its_jobs_and_their_edges() {
    let elements = map(&parse("jobs-only.yml"), &["stage-0"]);
    assert_eq!(2, elements.iter().filter(|e| e.element_type == JOB_TYPE).count());
    assert!(elements.iter().any(|e| e.id == "edge:stage-0/Build->stage-0/Test"));
}

#[test]
fn a_job_knows_which_stage_it_is_in() {
    let elements = map(&parse("multi-stage.yml"), &["Test"]);
    assert_eq!(Some("Test"), by_id(&elements, "Test/Unit").parent.as_deref());
}

#[test]
fn a_deployment_job_is_a_different_kind_and_carries_its_environment() {
    let elements = map(&parse("multi-stage.yml"), &["DeployStaging"]);
    let staging = by_id(&elements, "DeployStaging/Staging");
    assert_eq!("deploymentJob", staging.attribute("kind"));
    assert_eq!("staging", staging.attribute("environment"));
    assert_eq!("RunOnce", staging.attribute("strategy"));
}

#[test]
fn a_plain_job_is_the_plain_kind() {
    let elements = map(&parse("multi-stage.yml"), &["Test"]);
    assert_eq!("job", by_id(&elements, "Test/Unit").attribute("kind"));
}

#[test]
fn a_matrix_job_carries_its_multiplicity() {
    let model = parse_text(
        "jobs:\n  - job: Test\n    strategy:\n      matrix:\n        linux:\n          image: ubuntu-latest\n        windows:\n          image: windows-latest\n    steps:\n      - script: x",
    );
    let elements = map(&model, &["stage-0"]);
    let job = by_id(&elements, "stage-0/Test");
    assert_eq!("2", job.attribute("multiplicity"));
    assert_eq!("false", job.attribute("indeterminate"));
    assert!(job.attribute("indicators").contains("×2|Runs 2 times"));
}

#[test]
fn a_job_multiplied_by_an_expression_is_marked_indeterminate() {
    let model =
        parse_text("jobs:\n  - job: Test\n    strategy:\n      parallel: $(slices)\n    steps:\n      - script: x\n");
    let elements = map(&model, &["stage-0"]);
    let job = by_id(&elements, "stage-0/Test");
    assert_eq!("true", job.attribute("indeterminate"));
    assert!(job.attribute("indicators").contains("×?|"));
}

#[test]
fn a_stage_inside_a_compile_time_conditional_is_marked_indeterminate() {
    let elements = map(&parse("edge-expressions.yml"), &[]);
    assert_eq!("true", by_id(&elements, "Deploy").attribute("indeterminate"));
    assert_eq!("false", by_id(&elements, "Build").attribute("indeterminate"));
}

#[test]
fn a_disabled_step_arrives_disabled() {
    let model = parse_text("steps:\n  - script: x\n    enabled: false\n  - script: y\n");
    let elements = map(&model, &["stage-0", "stage-0/job-0"]);
    assert_eq!("false", by_id(&elements, "stage-0/job-0/step-0").attribute("enabled"));
    assert_eq!("true", by_id(&elements, "stage-0/job-0/step-1").attribute("enabled"));
}

#[test]
fn a_step_gated_on_an_expression_is_not_drawn_as_skipped() {
    let model = parse_text("steps:\n  - script: x\n    enabled: $(runIt)\n");
    let elements = map(&model, &["stage-0", "stage-0/job-0"]);
    assert_eq!("true", by_id(&elements, "stage-0/job-0/step-0").attribute("enabled"));
}

#[test]
fn a_jobs_steps_in_declared_order_keep_it() {
    let elements = map(&parse("multi-stage.yml"), &["Build", "Build/Compile"]);
    let steps: Vec<&DiagramElement> = elements.iter().filter(|e| e.element_type == STEP_TYPE).collect();
    let names: Vec<&str> = steps.iter().map(|s| s.attribute("displayName")).collect();
    assert_eq!(vec!["self", "Build", "$(Build.ArtifactStagingDirectory)"], names);
    assert!(steps.windows(2).all(|pair| pair[0].y < pair[1].y));
}

#[test]
fn an_element_from_a_template_says_so_and_says_which_one() {
    let mut stage = parse("multi-stage.yml").stages[0].clone();
    stage.template = "templates/build.yml".to_owned();
    let model = PipelineModel {
        stages: vec![stage.clone()],
        ..PipelineModel::default()
    };
    let elements = map(&model, &[]);
    let mapped = by_id(&elements, &stage.id);
    assert_eq!("true", mapped.attribute("fromTemplate"));
    assert_eq!("templates/build.yml", mapped.attribute("templatePath"));
}

#[test]
fn a_template_that_could_not_be_followed_gets_an_element_saying_why() {
    let model = parse_text("stages:\n  - template: elsewhere.yml@other\n");
    let unresolved = PipelineModel {
        unresolved: vec![UnresolvedTemplate {
            reference: model.templates[0].clone(),
            reason: UnresolvedReason::OtherRepository,
        }],
        ..model
    };
    let elements = map(&unresolved, &[]);
    let element = elements.iter().find(|e| e.element_type == TEMPLATE_TYPE).unwrap();
    assert_eq!("elsewhere.yml@other", element.attribute("reference"));
    assert!(element.attribute("unresolvedReason").contains("not checked out here"));
    assert_eq!("true", element.attribute("indeterminate"));
}

#[test]
fn a_template_that_was_followed_needs_no_element_of_its_own() {
    let elements = map(&parse("multi-stage.yml"), &[]);
    assert!(!elements.iter().any(|e| e.element_type == TEMPLATE_TYPE));
}

#[test]
fn an_element_carries_the_lines_it_lives_on() {
    let elements = map(&parse("multi-stage.yml"), &[]);
    let build = by_id(&elements, "Build");
    let first: usize = build.attribute("firstLine").parse().unwrap();
    let last: usize = build.attribute("lastLine").parse().unwrap();
    assert!(first >= 1);
    assert!(last >= first);
}

#[test]
fn an_implicit_stage_carries_no_lines_because_it_has_none() {
    let elements = map(&parse("jobs-only.yml"), &[]);
    let stage = by_id(&elements, "stage-0");
    assert_eq!("0", stage.attribute("firstLine"));
    assert_eq!("0", stage.attribute("lastLine"));
}

#[test]
fn an_inherited_pool_says_it_was_inherited() {
    let elements = map(&parse("jobs-only.yml"), &["stage-0"]);
    let build = by_id(&elements, "stage-0/Build");
    assert_eq!("ubuntu-latest", build.attribute("pool"));
    assert_eq!("true", build.attribute("poolInherited"));
}

#[test]
fn every_fixture_maps_without_failing() {
    let mut inspected = 0;
    for name in all_fixtures() {
        let model = parse(&name);
        let stages: Vec<&str> = model.stages.iter().map(|s| s.id.as_str()).collect();
        for element in map(&model, &stages) {
            assert!(!element.id.is_empty());
            assert!(!element.element_type.is_empty());
            assert!(!element.attributes.is_empty(), "{} carries nothing", element.id);
            inspected += 1;
        }
    }
    assert!(inspected >= 40, "only {inspected} elements were mapped");
}
