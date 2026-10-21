//! The property rows of a node and of a dependency, and what setting one writes (the standalone host's
//! `DependencyGraphContextPropertyProvider.Tests.cs` at `8b6ccfc`).

mod common;

use common::open;

const GRAPH: &str = "dependencies: 1
elements:
  - id: aaa
    label: API gateway
    x: 240
    row: 0
  - id: bbb
    label: Identity service
    x: 480.5
    row: 2
relations:
  - id: ccc
    from: aaa
    to: bbb
    label: verifies tokens with
";

#[test]
fn a_node_contributes_label_x_and_row_and_nothing_else() {
    let graph = open(GRAPH);
    let rows = graph.rows("aaa");
    let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, ["dependencies.label", "dependencies.x", "dependencies.row"]);
    assert!(rows.iter().all(|r| r.read_only_reason.is_empty()));
}

#[test]
fn no_time_derived_row_exists() {
    let graph = open(GRAPH);
    for row in graph.rows("aaa") {
        for label in ["Begin", "End", "Duration"] {
            assert!(!row.label.eq_ignore_ascii_case(label));
        }
    }
}

#[test]
fn a_nodes_coordinate_is_shown_as_the_document_wrote_it() {
    let graph = open(GRAPH);
    assert_eq!(graph.row("aaa", "dependencies.x").value, "240");
    assert_eq!(graph.row("bbb", "dependencies.x").value, "480.5");
}

#[test]
fn a_dependency_shows_its_ends_read_only_with_the_reason() {
    let graph = open(GRAPH);
    let from = graph.row("ccc", "dependencies.from");
    let to = graph.row("ccc", "dependencies.to");
    assert_eq!(from.value, "aaa");
    assert_eq!(to.value, "bbb");
    assert!(from.read_only_reason.contains("canvas"));
    assert!(to.read_only_reason.contains("canvas"));
    assert!(graph.row("ccc", "dependencies.label").read_only_reason.is_empty());
}

#[test]
fn an_edit_is_one_undo_away() {
    let mut graph = open(GRAPH);
    let before = graph.text();
    graph.set("aaa", "dependencies.x", "612").unwrap();
    assert!(graph.text().contains("x: 612\n"));
    graph.undo();
    assert_eq!(graph.text(), before);
}

#[test]
fn a_fractional_coordinate_is_accepted() {
    let mut graph = open(GRAPH);
    graph.set("aaa", "dependencies.x", "612.75").unwrap();
    assert_eq!(graph.attribute("aaa", "x"), "612.75");
}

#[test]
fn a_coordinate_that_is_not_a_number_is_refused_naming_the_value() {
    let mut graph = open(GRAPH);
    let refusal = graph.set("aaa", "dependencies.x", "sideways").unwrap_err();
    assert_eq!(refusal, "'sideways' is not a number this graph can place a node at.");
    assert!(graph.text().contains("x: 240"));
}

#[test]
fn a_fractional_row_is_refused_rather_than_truncated() {
    let mut graph = open(GRAPH);
    let refusal = graph.set("aaa", "dependencies.row", "2.6").unwrap_err();
    assert_eq!(refusal, "'2.6' is not a number this graph can place a node at.");
    assert_eq!(graph.attribute("aaa", "row"), "0");
}

#[test]
fn editing_the_row_moves_the_node() {
    let mut graph = open(GRAPH);
    graph.set("aaa", "dependencies.row", "7").unwrap();
    assert_eq!(graph.attribute("aaa", "row"), "7");
    assert_eq!(graph.find("aaa").y, 420.0);
}

#[test]
fn editing_a_label_renames() {
    let mut graph = open(GRAPH);
    graph.set("aaa", "dependencies.label", "Edge router").unwrap();
    assert_eq!(graph.attribute("aaa", "label"), "Edge router");
}

#[test]
fn a_dependencys_label_is_editable_but_its_ends_are_not() {
    let mut graph = open(GRAPH);
    graph.set("ccc", "dependencies.label", "authenticates against").unwrap();
    assert!(graph.set("ccc", "dependencies.from", "bbb").is_err());
    assert_eq!(graph.attribute("ccc", "label"), "authenticates against");
    assert_eq!(graph.find("ccc").source.as_deref(), Some("aaa"));
}

#[test]
fn an_unknown_property_is_refused() {
    let mut graph = open(GRAPH);
    assert!(graph.set("aaa", "dependencies.nonsense", "x").is_err());
}
