//! The findings of a file (etalii.adp spec 030, task T081), as the standalone host's `DatabricksValidator.Tests.cs`
//! checks them at `8b6ccfc`.

mod common;

use adp_diagram::{DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity};
use adp_diagram_databricks::rules;
use common::text;

fn count(problems: &[DiagramProblem], rule: &str) -> usize {
    problems.iter().filter(|p| p.rule_id == rule).count()
}

fn only<'a>(problems: &'a [DiagramProblem], rule: &str) -> &'a DiagramProblem {
    assert_eq!(count(problems, rule), 1, "{problems:?}");
    problems.iter().find(|p| p.rule_id == rule).unwrap()
}

#[test]
fn a_clean_file_reports_nothing() {
    assert!(rules::validate(&text("job.yml")).is_empty());
    assert!(rules::validate(&text("pipeline.json")).is_empty());
}

#[test]
fn a_file_that_does_not_parse_reports_that_and_only_that() {
    let problems = rules::validate(&text("broken.yml"));
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].rule_id, rules::UNPARSEABLE);
    assert_eq!(problems[0].severity, DiagramProblemSeverity::Error);
    assert!(matches!(problems[0].location, Some(DiagramProblemLocation::Line(_))));
}

#[test]
fn the_job_findings_each_fire_with_their_lines() {
    let problems = rules::validate(&text("findings-job.yml"));
    assert_eq!(count(&problems, rules::CYCLE), 2);
    assert!(only(&problems, rules::MISSING_TASK).message.contains("'nowhere'"));
    assert!(only(&problems, rules::DANGLING_CLUSTER).message.contains("'warp_core'"));
    only(&problems, rules::DUPLICATE_TASK_KEY);
    only(&problems, rules::TASK_KEY_MISSING);
    assert_eq!(problems.len(), 6);
    for problem in &problems {
        assert!(matches!(problem.location, Some(DiagramProblemLocation::Line(n)) if n >= 1));
    }
}

#[test]
fn the_bundle_findings_default_target_and_stray_override() {
    let problems = rules::validate(&text("findings-bundle.yml"));
    assert_eq!(
        only(&problems, rules::NO_DEFAULT_TARGET).severity,
        DiagramProblemSeverity::Warning
    );
    assert!(
        only(&problems, rules::OVERRIDE_OF_UNDECLARED)
            .message
            .contains("'phantom'")
    );
    assert_eq!(problems.len(), 2);
}

#[test]
fn a_second_default_target_is_an_error() {
    let problems = rules::validate(
        "bundle:\r\n  name: greedy\r\ntargets:\r\n  dev:\r\n    default: true\r\n  prod:\r\n    default: true\r\n",
    );
    assert!(
        only(&problems, rules::MULTIPLE_DEFAULT_TARGETS)
            .message
            .contains("'prod'")
    );
}

#[test]
fn an_override_with_includes_in_play_is_not_accused() {
    let problems = rules::validate(
        "bundle:\r\n  name: split\r\ninclude:\r\n  - resources/*.yml\r\ntargets:\r\n  dev:\r\n    default: \
         true\r\n    resources:\r\n      jobs:\r\n        elsewhere:\r\n          name: x\r\n",
    );
    assert_eq!(count(&problems, rules::OVERRIDE_OF_UNDECLARED), 0);
}

#[test]
fn the_pipeline_findings_empty_libraries_and_schema_without_catalog() {
    let problems = rules::validate(&text("findings-pipeline.json"));
    assert_eq!(
        only(&problems, rules::NO_LIBRARIES).severity,
        DiagramProblemSeverity::Error
    );
    assert_eq!(
        only(&problems, rules::SCHEMA_WITHOUT_CATALOG).severity,
        DiagramProblemSeverity::Warning
    );
}
