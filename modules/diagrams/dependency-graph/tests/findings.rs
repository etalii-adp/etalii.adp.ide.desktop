//! The mistakes a dependency graph hides, one test per rule (the standalone host's `DependencyGraphFindings.Tests.cs`
//! and `DependencyGraphReading.Tests.cs` at `8b6ccfc`): the specification's constraints, with the module's
//! `writtenEnd` reading the ends as written.

mod common;

use adp_diagram::{DiagramProblem, DiagramProblemLocation};
use common::open;

fn judge(yaml: &str) -> Vec<DiagramProblem> {
    open(yaml).problems()
}

fn of(yaml: &str, rule: &str) -> Vec<DiagramProblem> {
    judge(yaml).into_iter().filter(|p| p.rule_id == rule).collect()
}

const HEALTHY: &str = "dependencies: 1
elements:
  - id: aaa
    label: API gateway
    x: 240
    row: 0
  - id: bbb
    label: Identity service
    x: 480
    row: 1
relations:
  - id: ccc
    from: aaa
    to: bbb
    label: verifies tokens with
";

#[test]
fn a_correct_graph_has_nothing_wrong_with_it() {
    assert!(judge(HEALTHY).is_empty());
}

#[test]
fn an_empty_graph_is_also_fine() {
    assert!(judge("dependencies: 1\nelements: []\n").is_empty());
}

#[test]
fn the_example_has_nothing_wrong_with_it() {
    assert!(judge(&common::services()).is_empty());
}

#[test]
fn a_dangling_relation_names_the_missing_end() {
    let problems = of(
        "dependencies: 1\nelements:\n  - id: aaa\n    x: 0\n    row: 0\nrelations:\n  - id: ccc\n    from: aaa\n    to: ghost\n",
        "dependencies.dangling-relation",
    );
    assert_eq!(problems.len(), 1);
    assert_eq!(
        problems[0].message,
        "A dependency names 'ghost', and no node with that id is in this graph."
    );
    assert_eq!(
        problems[0].location,
        Some(DiagramProblemLocation::Element("ccc".to_owned()))
    );
}

#[test]
fn a_dangling_relation_is_reported_for_either_end() {
    let problems = of(
        "dependencies: 1\nelements:\n  - id: aaa\n    x: 0\n    row: 0\nrelations:\n  - id: ccc\n    from: ghost\n    to: aaa\n",
        "dependencies.dangling-relation",
    );
    assert_eq!(problems.len(), 1);
    assert!(problems[0].message.contains("'ghost'"));
}

#[test]
fn an_end_written_empty_is_not_reported() {
    assert!(
        of(
            "dependencies: 1\nelements:\n  - id: aaa\n    x: 0\n    row: 0\nrelations:\n  - id: ccc\n    from: aaa\n",
            "dependencies.dangling-relation",
        )
        .is_empty()
    );
}

#[test]
fn a_duplicate_id_is_reported_on_the_second_declaration() {
    let problems = of(
        "dependencies: 1\nelements:\n  - id: aaa\n    x: 0\n    row: 0\n  - id: aaa\n    x: 200\n    row: 1\n",
        "dependencies.duplicate-id",
    );
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].location, Some(DiagramProblemLocation::Line(6)));
}

#[test]
fn a_missing_id_is_reported_with_a_line() {
    let problems = of(
        "dependencies: 1\nelements:\n  - label: Anonymous\n    x: 0\n    row: 0\n",
        "dependencies.missing-id",
    );
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].location, Some(DiagramProblemLocation::Line(3)));
}

#[test]
fn a_node_depending_on_itself_is_reported() {
    let problems = of(
        "dependencies: 1\nelements:\n  - id: aaa\n    x: 0\n    row: 0\nrelations:\n  - id: ccc\n    from: aaa\n    to: aaa\n",
        "dependencies.self-dependency",
    );
    assert_eq!(problems.len(), 1);
    assert_eq!(
        problems[0].message,
        "'aaa' is declared to depend on itself, which says nothing about what needs what."
    );
    assert_eq!(
        problems[0].location,
        Some(DiagramProblemLocation::Element("ccc".to_owned()))
    );
}

#[test]
fn a_self_dependency_between_ids_no_node_holds_is_reported() {
    let problems = of(
        "dependencies: 1\nelements: []\nrelations:\n  - id: ccc\n    from: ghost\n    to: ghost\n",
        "dependencies.self-dependency",
    );
    assert_eq!(problems.len(), 1);
    assert!(problems[0].message.starts_with("'ghost'"));
}

#[test]
fn no_rule_fires_on_a_coordinate_or_a_row_that_will_not_read() {
    assert!(judge("dependencies: 1\nelements:\n  - id: aaa\n    x: sideways\n    row: sideways\n").is_empty());
}

#[test]
fn values_that_will_not_read_are_zero_and_every_node_is_drawn() {
    let graph = open(
        "dependencies: 1\nelements:\n  - id: aaa\n    x: sideways\n    row: 2.5\n  - id: bbb\n    x: 1e3\n    row: 1\n",
    );
    assert_eq!(graph.find("aaa").x, 0.0);
    assert_eq!(graph.find("aaa").y, 0.0);
    assert_eq!(graph.find("bbb").x, 1000.0);
    assert_eq!(graph.find("bbb").y, 60.0);
}

#[test]
fn the_ids_are_reported_before_the_ends_each_in_declaration_order() {
    let problems = judge(
        "dependencies: 1\nelements:\n  - id: aaa\n    x: 0\n    row: 0\nrelations:\n  - id: r1\n    from: aaa\n    to: ghost\n  - label: anonymous\n    from: aaa\n    to: aaa\n  - id: aaa\n    from: ghost\n    to: ghost\n",
    );
    let codes: Vec<(&str, Option<DiagramProblemLocation>)> = problems
        .iter()
        .map(|p| (p.rule_id.as_str(), p.location.clone()))
        .collect();
    assert_eq!(
        codes,
        [
            ("dependencies.missing-id", Some(DiagramProblemLocation::Line(10))),
            ("dependencies.duplicate-id", Some(DiagramProblemLocation::Line(13))),
            (
                "dependencies.dangling-relation",
                Some(DiagramProblemLocation::Element("r1".to_owned()))
            ),
            (
                "dependencies.self-dependency",
                Some(DiagramProblemLocation::Element(String::new()))
            ),
            (
                "dependencies.dangling-relation",
                Some(DiagramProblemLocation::Element("aaa".to_owned()))
            ),
            (
                "dependencies.dangling-relation",
                Some(DiagramProblemLocation::Element("aaa".to_owned()))
            ),
            (
                "dependencies.self-dependency",
                Some(DiagramProblemLocation::Element("aaa".to_owned()))
            ),
        ]
    );
}

#[test]
fn a_file_that_is_not_yaml_is_one_error_naming_its_line() {
    let graph = common::open("elements: [\n  - id: a\n");
    let problems = graph.problems();
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0].rule_id, "dependencies.unparseable");
    assert_eq!(problems[0].severity, adp_diagram::DiagramProblemSeverity::Error);
    assert!(matches!(problems[0].location, Some(DiagramProblemLocation::Line(line)) if line >= 1));
}

#[test]
fn an_empty_file_is_judged_rather_than_crashed() {
    let graph = common::open("");
    assert!(graph.problems().iter().all(|p| p.rule_id != "dependencies.unparseable"));
}
