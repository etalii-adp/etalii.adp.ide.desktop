//! What is wrong with a map (etalii.adp spec 030, task T070), ported from the standalone host's
//! `WardleyRuleSet.Tests.cs` (`WardleyRuleSetTests` and `WardleyValidatorTests`) at `8b6ccfc`. `ItIsKeyedByThisModulesOrigin`
//! has no counterpart: the module's view answers its problems itself.

mod common;

use std::fs;
use std::time::{Duration, Instant};

use adp_diagram::{DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity};
use adp_diagram_wardley_map::rules;
use common::parse_text;

fn validate(text: &str) -> Vec<DiagramProblem> {
    rules::validate(&parse_text(text))
}

fn single<'a>(problems: &'a [DiagramProblem], rule_id: &str) -> &'a DiagramProblem {
    let found: Vec<_> = problems.iter().filter(|p| p.rule_id == rule_id).collect();
    assert_eq!(found.len(), 1, "{rule_id} in {problems:?}");
    found[0]
}

#[test]
fn a_clean_map_has_nothing_to_say() {
    let problems =
        validate("anchor Business [0.95, 0.63]\ncomponent Kettle [0.43, 0.35]\nBusiness->Kettle\nevolve Kettle 0.62");
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn a_link_to_something_that_is_not_there_is_an_error_at_the_line_that_says_it() {
    let problems = validate("anchor Business [0.95, 0.63]\ncomponent Kettle [0.43, 0.35]\nKettle->Electricity");
    let problem = single(&problems, rules::LINK_TARGET_MISSING);
    assert_eq!(problem.severity, DiagramProblemSeverity::Error);
    assert!(problem.message.contains("Electricity"));
    assert!(problem.message.contains("Kettle->Electricity"));
    assert_eq!(problem.location, Some(DiagramProblemLocation::Line(3)));
}

#[test]
fn a_link_with_both_ends_missing_is_reported_twice() {
    let problems = validate("anchor Business [0.95, 0.63]\nAlpha->Beta\n");
    assert_eq!(
        problems
            .iter()
            .filter(|p| p.rule_id == rules::LINK_TARGET_MISSING)
            .count(),
        2
    );
}

#[test]
fn an_evolve_on_something_that_is_not_there_is_an_error() {
    let problems = validate("anchor Business [0.95, 0.63]\nevolve Kettle 0.62\n");
    let problem = single(&problems, rules::EVOLVE_TARGET_MISSING);
    assert_eq!(problem.severity, DiagramProblemSeverity::Error);
    assert_eq!(problem.location, Some(DiagramProblemLocation::Line(2)));
}

#[test]
fn a_pipeline_on_something_that_is_not_there_is_an_error() {
    let problems = validate("anchor Business [0.95, 0.63]\npipeline Kettle\n{\n  component Electric Kettle [0.63]\n}");
    let problem = single(&problems, rules::PIPELINE_PARENT_MISSING);
    assert_eq!(problem.severity, DiagramProblemSeverity::Error);
    assert!(problem.message.contains("Kettle"));
}

#[test]
fn a_coordinate_off_the_scale_is_an_error() {
    for (statement, axis) in [
        ("component Kettle [1.4, 0.35]", "visibility"),
        ("component Kettle [0.4, -0.2]", "maturity"),
    ] {
        let problems = validate(&format!("anchor Business [0.95, 0.63]\n{statement}\n"));
        let problem = single(&problems, rules::COORDINATE_OUT_OF_RANGE);
        assert_eq!(problem.severity, DiagramProblemSeverity::Error);
        assert!(problem.message.contains(axis), "{}", problem.message);
        assert_eq!(problem.location, Some(DiagramProblemLocation::Line(2)));
    }
}

#[test]
fn a_coordinate_off_the_scale_is_caught_on_everything_the_format_positions() {
    let problems = validate(
        "anchor Business [0.95, 0.63]\ncomponent Kettle [0.43, 0.35]\npipeline Kettle\n{\n  component Electric Kettle [1.63]\n}\nevolve Kettle 1.2\nnote Off the edge [0.2, 2.0]\npioneers [0.30, 0.20, 1.55, 0.45]\naccelerator Regulation [0.5, -0.1]\nannotation 1 [0.48, 1.85] Too far",
    );
    assert_eq!(
        problems
            .iter()
            .filter(|p| p.rule_id == rules::COORDINATE_OUT_OF_RANGE)
            .count(),
        6
    );
}

#[test]
fn two_components_sharing_a_name_is_an_error_at_the_element_they_both_claim() {
    let problems =
        validate("anchor Business [0.95, 0.63]\ncomponent Kettle [0.43, 0.35]\ncomponent Kettle [0.10, 0.70]");
    let problem = single(&problems, rules::DUPLICATE_NAME);
    assert_eq!(problem.severity, DiagramProblemSeverity::Error);
    assert_eq!(
        problem.location,
        Some(DiagramProblemLocation::Element("Kettle".to_owned()))
    );
    assert!(problem.message.contains("lines 2, 3"), "{}", problem.message);
}

#[test]
fn a_map_with_no_anchor_is_a_warning_against_the_file() {
    let problems = validate("component Kettle [0.43, 0.35]\n");
    let problem = single(&problems, rules::ANCHOR_MISSING);
    assert_eq!(problem.severity, DiagramProblemSeverity::Warning);
    assert_eq!(problem.location, None);
}

#[test]
fn an_empty_map_is_not_told_off_for_having_no_anchor() {
    assert!(validate("title Nothing yet\n").is_empty());
}

#[test]
fn a_url_nothing_defines_is_a_warning() {
    let problems = validate("anchor Business [0.95, 0.63]\ncomponent Kettle [0.43, 0.35] url(handbook)");
    let problem = single(&problems, rules::URL_UNDEFINED);
    assert_eq!(problem.severity, DiagramProblemSeverity::Warning);
    assert!(problem.message.contains("handbook"));
}

#[test]
fn a_semantically_broken_map_still_parses_and_keeps_everything_else() {
    let map = parse_text("anchor Business [0.95, 0.63]\ncomponent Kettle [0.43, 0.35]\nKettle->Nothing");
    assert!(!rules::validate(&map).is_empty());
    assert_eq!(map.components.len(), 2);
    assert_eq!(map.links.len(), 1);
}

#[test]
fn every_rule_id_is_prefixed_with_this_modules_name() {
    let problems = validate(
        "component Kettle [1.43, 0.35]\ncomponent Kettle [0.10, 0.70]\nKettle->Nothing\nevolve Nothing 0.5\ncomponent Handbook [0.2, 0.2] url(missing)",
    );
    assert!(!problems.is_empty());
    assert!(
        problems.iter().all(|p| p.rule_id.starts_with("wardley.")),
        "{problems:?}"
    );
}

#[test]
fn validation_is_bounded_even_on_a_large_map() {
    let mut lines = vec!["anchor Business [0.95, 0.63]".to_owned()];
    lines.extend((0..1000).map(|i| format!("component C{i} [0.5, 0.5]")));
    lines.extend((0..999).map(|i| format!("C{i}->C{}", i + 1)));
    let map = parse_text(&(lines.join("\n") + "\n"));
    let started = Instant::now();
    let problems = rules::validate(&map);
    assert!(problems.is_empty());
    assert!(started.elapsed() < Duration::from_secs(2), "{:?}", started.elapsed());
}

// ---- WardleyValidatorTests ------------------------------------------------------------------------------------------

fn validate_in_project(text: &str, setup: impl FnOnce(&std::path::Path)) -> Vec<DiagramProblem> {
    let root = tempfile::tempdir().unwrap();
    setup(root.path());
    fs::write(root.path().join("map.owm"), text).unwrap();
    rules::validate_document(text, root.path())
}

#[test]
fn a_submap_naming_a_map_that_is_not_in_the_project_is_a_warning() {
    let problems = validate_in_project(
        "anchor Business [0.95, 0.63]\nsubmap Supply [0.5, 0.5] url(supply)\nurl supply [supply.owm]",
        |_| {},
    );
    let problem = single(&problems, rules::SUBMAP_MISSING);
    assert_eq!(problem.severity, DiagramProblemSeverity::Warning);
    assert!(problem.message.contains("supply.owm"));
    assert!(problem.message.contains("Supply"));
}

#[test]
fn a_submap_naming_a_map_that_is_there_is_silent() {
    let problems = validate_in_project(
        "anchor Business [0.95, 0.63]\nsubmap Supply [0.5, 0.5] url(supply)\nurl supply [supply.owm]",
        |root| fs::write(root.join("supply.owm"), "title Supply\n").unwrap(),
    );
    assert!(!problems.iter().any(|p| p.rule_id == rules::SUBMAP_MISSING));
}

#[test]
fn an_address_on_the_internet_is_left_alone() {
    let problems = validate_in_project(
        "anchor Business [0.95, 0.63]\ncomponent Handbook [0.5, 0.5] url(handbook)\nurl handbook [https://example.com/handbook]",
        |_| {},
    );
    assert!(!problems.iter().any(|p| p.rule_id == rules::SUBMAP_MISSING));
}

#[test]
fn an_address_walking_out_of_the_project_is_not_followed() {
    let problems = validate_in_project(
        "anchor Business [0.95, 0.63]\ncomponent Secrets [0.5, 0.5] url(secrets)\nurl secrets [../../elsewhere/secrets.owm]",
        |_| {},
    );
    assert!(!problems.iter().any(|p| p.rule_id == rules::SUBMAP_MISSING));
}

#[test]
fn it_carries_the_rule_sets_problems_through_unchanged() {
    let problems = validate_in_project("component Kettle [0.43, 0.35]\nKettle->Nothing\n", |_| {});
    assert!(problems.iter().any(|p| p.rule_id == rules::LINK_TARGET_MISSING));
    assert!(problems.iter().any(|p| p.rule_id == rules::ANCHOR_MISSING));
}

#[test]
fn a_line_the_parser_passes_over_is_a_warning_at_its_line() {
    let problems = validate_in_project("anchor Business [0.95, 0.63]\nevolve\n", |_| {});
    let problem = single(&problems, rules::LINE_PASSED_OVER);
    assert_eq!(problem.severity, DiagramProblemSeverity::Warning);
    assert_eq!(
        problem.message,
        "This evolve statement could not be read, so it was passed over."
    );
    assert_eq!(problem.location, Some(DiagramProblemLocation::Line(2)));
}

#[test]
fn a_statement_this_module_does_not_model_is_silent() {
    let problems = validate_in_project(
        "anchor Business [0.95, 0.63]\ny_axis Value chain->Invisible->Visible\n",
        |_| {},
    );
    assert!(!problems.iter().any(|p| p.rule_id == rules::LINE_PASSED_OVER));
}
