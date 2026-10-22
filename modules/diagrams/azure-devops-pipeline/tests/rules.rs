//! What is wrong with a pipeline: dangling `dependsOn`, cycles, no stage to start with, stages nothing reaches,
//! unnamed elements, templates that cannot be followed, and a text that is not YAML (the standalone host's
//! `PipelineRuleSetTests` and `PipelineValidatorTests`).

mod common;

use adp_diagram::{DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity};
use adp_diagram_azure_devops_pipeline::rules::{self, *};
use common::{all_fixtures, fixture, parse_text};

fn judge(yaml: &str) -> Vec<DiagramProblem> {
    rules::judge(&parse_text(yaml))
}

fn of(yaml: &str, rule: &str) -> Vec<DiagramProblem> {
    judge(yaml).into_iter().filter(|p| p.rule_id == rule).collect()
}

fn single(yaml: &str, rule: &str) -> DiagramProblem {
    let found = of(yaml, rule);
    assert_eq!(1, found.len(), "{found:?}");
    found.into_iter().next().unwrap()
}

const HEALTHY: &str = "stages:
  - stage: Build
    jobs:
      - job: Compile
        steps:
          - script: dotnet build
  - stage: Test
    dependsOn: Build
    jobs:
      - job: Verify
        steps:
          - script: dotnet test";

const DANGLING: &str = "stages:
  - stage: Build
    jobs:
      - job: J
        steps:
          - script: x
  - stage: Ship
    dependsOn: Bild
    jobs:
      - job: J
        steps:
          - script: x";

#[test]
fn a_correct_pipeline_has_nothing_wrong_with_it() {
    assert!(judge(HEALTHY).is_empty());
}

#[test]
fn every_fixture_that_is_meant_to_be_valid_reports_nothing() {
    for name in all_fixtures() {
        if name == "edge-broken-graph.yml" {
            continue;
        }
        let errors: Vec<String> = judge(&fixture(&name))
            .into_iter()
            .filter(|p| p.severity == DiagramProblemSeverity::Error)
            .map(|p| p.message)
            .collect();
        assert!(errors.is_empty(), "{name}: {errors:?}");
    }
}

#[test]
fn a_depends_on_naming_nothing_is_reported_with_both_names() {
    let problem = single(DANGLING, DANGLING_DEPENDENCY);
    assert_eq!(DiagramProblemSeverity::Error, problem.severity);
    assert!(problem.message.contains("Ship"));
    assert!(problem.message.contains("Bild"));
}

#[test]
fn a_dangling_job_dependency_is_reported_too() {
    let problem = single(
        "jobs:
  - job: Build
    steps:
      - script: x
  - job: Test
    dependsOn: Compile
    steps:
      - script: x",
        DANGLING_DEPENDENCY,
    );
    assert!(problem.message.contains("job called 'Compile'"), "{}", problem.message);
}

#[test]
fn a_cycle_is_reported_naming_the_whole_loop() {
    let problem = single(
        "stages:
  - stage: Left
    dependsOn: Right
    jobs:
      - job: J
        steps:
          - script: x
  - stage: Right
    dependsOn: Left
    jobs:
      - job: J
        steps:
          - script: x",
        CYCLE,
    );
    assert_eq!(DiagramProblemSeverity::Error, problem.severity);
    assert!(problem.message.contains("Left"));
    assert!(problem.message.contains("Right"));
}

#[test]
fn a_stage_waiting_for_itself_says_exactly_that() {
    let problem = single(
        "stages:\n  - stage: Loop\n    dependsOn: Loop\n    jobs:\n      - job: J\n        steps:\n          - script: x",
        CYCLE,
    );
    assert!(problem.message.contains("waits for itself"));
}

#[test]
fn a_pipeline_that_cannot_start_is_reported() {
    let problem = single(
        "stages:
  - stage: A
    dependsOn: B
    jobs:
      - job: J
        steps:
          - script: x
  - stage: B
    dependsOn: A
    jobs:
      - job: J
        steps:
          - script: x",
        NO_STARTING_STAGE,
    );
    assert_eq!(DiagramProblemSeverity::Error, problem.severity);
    assert!(problem.location.is_none());
}

#[test]
fn an_ordinary_pipeline_is_not_accused_of_having_no_start() {
    assert!(of(HEALTHY, NO_STARTING_STAGE).is_empty());
}

#[test]
fn a_stage_nothing_can_reach_is_reported() {
    let problem = single(
        "stages:
  - stage: Build
    dependsOn: []
    jobs:
      - job: J
        steps:
          - script: x
  - stage: Ghost
    dependsOn: Missing
    jobs:
      - job: J
        steps:
          - script: x
  - stage: After
    dependsOn: Ghost
    jobs:
      - job: J
        steps:
          - script: x",
        UNREACHABLE,
    );
    assert!(problem.message.contains("After"));
    assert_eq!(DiagramProblemSeverity::Warning, problem.severity);
}

#[test]
fn a_stage_with_a_dangling_dependency_is_not_also_reported_as_unreachable() {
    assert_eq!(1, of(DANGLING, DANGLING_DEPENDENCY).len());
    assert!(of(DANGLING, UNREACHABLE).is_empty());
}

#[test]
fn the_last_stage_of_a_pipeline_is_not_called_unreachable() {
    assert!(of(HEALTHY, UNREACHABLE).is_empty());
}

#[test]
fn a_stage_in_a_cycle_is_not_also_reported_as_unreachable() {
    let yaml = "stages:
  - stage: Start
    dependsOn: []
    jobs:
      - job: J
        steps:
          - script: x
  - stage: Left
    dependsOn: Right
    jobs:
      - job: J
        steps:
          - script: x
  - stage: Right
    dependsOn: Left
    jobs:
      - job: J
        steps:
          - script: x";
    assert_eq!(1, of(yaml, CYCLE).len());
    assert!(of(yaml, UNREACHABLE).is_empty());
}

#[test]
fn an_unnamed_stage_is_reported_gently() {
    let problem = single(
        "stages:\n  - stage:\n    jobs:\n      - job: J\n        steps:\n          - script: x",
        UNNAMED,
    );
    assert_eq!(DiagramProblemSeverity::Warning, problem.severity);
    assert!(problem.message.contains("no name"));
}

#[test]
fn a_stage_with_only_a_display_name_is_not_reported() {
    assert!(
        of(
            "stages:\n  - stage:\n    displayName: The one at the start\n    jobs:\n      - job: J\n        steps:\n          - script: x",
            UNNAMED
        )
        .is_empty()
    );
}

#[test]
fn the_implicit_stage_is_not_reported_as_unnamed() {
    assert!(of("jobs:\n  - job: Build\n    steps:\n      - script: x\n", UNNAMED).is_empty());
}

#[test]
fn a_template_in_another_repository_is_a_warning_rather_than_an_error() {
    let problem = single(
        "stages:\n  - stage: Build\n    jobs:\n      - job: J\n        steps:\n          - script: x\n  - template: deploy.yml@shared",
        TEMPLATE_NOT_FOLLOWED,
    );
    assert_eq!(DiagramProblemSeverity::Warning, problem.severity);
    assert!(problem.message.contains("another repository"));
}

#[test]
fn a_template_path_built_from_an_expression_is_reported_too() {
    let problem = single(
        "stages:\n  - stage: Build\n    jobs:\n      - job: J\n        steps:\n          - script: x\n  - template: templates/${{ parameters.which }}.yml",
        TEMPLATE_NOT_FOLLOWED,
    );
    assert!(problem.message.contains("compile time"));
}

#[test]
fn an_ordinary_relative_template_is_not_reported() {
    assert!(
        of(
            "stages:\n  - stage: Build\n    jobs:\n      - template: templates/build-jobs.yml",
            TEMPLATE_NOT_FOLLOWED
        )
        .is_empty()
    );
}

#[test]
fn every_problem_names_the_rule_that_found_it() {
    let problems = judge(&fixture("edge-broken-graph.yml"));
    assert!(!problems.is_empty());
    for problem in problems {
        assert!(problem.rule_id.starts_with("azure-pipeline."));
        assert!(!problem.message.is_empty());
    }
}

#[test]
fn a_problem_on_an_element_names_the_element_so_the_canvas_can_mark_it() {
    let problems = judge(&fixture("edge-broken-graph.yml"));
    let dangling = problems.iter().find(|p| p.rule_id == DANGLING_DEPENDENCY).unwrap();
    assert_eq!(
        Some(DiagramProblemLocation::Element("Ghost".to_owned())),
        dangling.location
    );
}

#[test]
fn the_deliberately_broken_fixture_reports_everything_it_was_written_to_show() {
    let problems = judge(&fixture("edge-broken-graph.yml"));
    assert!(problems.iter().any(|p| p.rule_id == DANGLING_DEPENDENCY));
    assert!(problems.iter().any(|p| p.rule_id == CYCLE));
}

#[test]
fn a_file_that_declares_nothing_is_not_accused_of_anything() {
    assert!(judge("trigger:\n  - main\n").is_empty());
    assert!(judge(&fixture("extends.yml")).is_empty());
}

#[test]
fn a_correct_pipeline_reaches_the_rules_and_survives_them() {
    assert!(
        rules::validate(
            "stages:\n  - stage: Build\n    jobs:\n      - job: Compile\n        steps:\n          - script: dotnet build"
        )
        .is_empty()
    );
}

#[test]
fn a_flawed_pipeline_reports_the_rule_that_fired() {
    let problems = rules::validate(
        "stages:\n  - stage: Build\n    dependsOn: Missing\n    jobs:\n      - job: Compile\n        steps:\n          - script: dotnet build",
    );
    assert!(problems.iter().any(|p| p.rule_id == DANGLING_DEPENDENCY));
}

#[test]
fn a_file_that_is_not_yaml_is_one_problem_naming_its_line() {
    let problems = rules::validate("stages: [\n  - stage: Build\n");
    assert_eq!(1, problems.len());
    let problem = &problems[0];
    assert_eq!(UNPARSEABLE, problem.rule_id);
    assert_eq!(DiagramProblemSeverity::Error, problem.severity);
    match problem.location {
        Some(DiagramProblemLocation::Line(number)) => assert!(number >= 1),
        ref other => panic!("{other:?}"),
    }
}

#[test]
fn an_empty_file_is_judged_rather_than_crashed() {
    assert!(!rules::validate("").iter().any(|p| p.rule_id == UNPARSEABLE));
}
