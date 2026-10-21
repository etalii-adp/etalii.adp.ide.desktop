//! The open graph: what it draws and where, and the two move legs, one implemented and one refusing (the standalone
//! host's `DependencyGraphSession.Tests.cs`, `DependencyGraphElementMapper.Tests.cs` and `Examples.Tests.cs` at
//! `8b6ccfc`).

mod common;

use adp_diagram::ElementKind;
use common::{GRAPH, open};

#[test]
fn the_baseline_carries_every_node_and_dependency() {
    let graph = open(GRAPH);
    let ids: Vec<String> = graph.elements().into_iter().map(|e| e.id).collect();
    assert_eq!(ids, ["aaa", "bbb", "ccc"]);
}

#[test]
fn opening_the_same_document_twice_produces_identical_output() {
    let one = open(GRAPH).elements();
    let two = open(GRAPH).elements();
    assert_eq!(one, two);
}

#[test]
fn a_node_is_drawn_at_its_x_and_its_rows_y() {
    let graph = open(GRAPH);
    let aaa = graph.find("aaa");
    let bbb = graph.find("bbb");
    assert_eq!((aaa.x, aaa.y), (240.0, 0.0));
    assert_eq!((bbb.x, bbb.y), (480.0, 120.0));
    assert!(aaa.placed);
}

#[test]
fn a_dependency_carries_its_ends_in_order() {
    let graph = open(GRAPH);
    let ccc = graph.find("ccc");
    assert_eq!(ccc.kind, ElementKind::Relation);
    assert_eq!(ccc.source.as_deref(), Some("aaa"));
    assert_eq!(ccc.target.as_deref(), Some("bbb"));
}

#[test]
fn the_reparenting_move_is_refused_with_a_reason() {
    let mut graph = open(GRAPH);
    let before = graph.text();
    let refusal = graph
        .session
        .move_element(&mut graph.workspace, "aaa", Some("bbb"), 0)
        .unwrap_err();
    assert!(!refusal.is_empty());
    assert_eq!(graph.text(), before);
}

#[test]
fn a_positional_move_rewrites_the_nodes_coordinate_and_row() {
    let mut graph = open(GRAPH);
    graph.move_to("aaa", 812.5, 240.0).unwrap();
    let text = graph.text();
    assert!(text.contains("x: 812.5"));
    assert!(text.contains("row: 4"));
    assert_eq!((graph.find("aaa").x, graph.find("aaa").y), (812.5, 240.0));
}

#[test]
fn a_move_snaps_to_the_nearest_row() {
    let mut graph = open(GRAPH);
    graph.move_to("aaa", 240.0, 170.0).unwrap();
    assert_eq!(graph.attribute("aaa", "row"), "3");
    graph.move_to("aaa", 240.0, -30.0).unwrap();
    assert_eq!(graph.attribute("aaa", "row"), "-1");
}

#[test]
fn a_move_is_one_undo_away() {
    let mut graph = open(GRAPH);
    let before = graph.text();
    graph.move_to("aaa", 812.5, 240.0).unwrap();
    graph.undo();
    assert_eq!(graph.text(), before);
}

#[test]
fn moving_a_relation_is_refused() {
    let mut graph = open(GRAPH);
    let before = graph.text();
    assert!(graph.move_to("ccc", 0.0, 0.0).is_err());
    assert_eq!(graph.text(), before);
}

#[test]
fn the_example_opens_with_every_service_and_dependency() {
    let graph = open(&common::services());
    let elements = graph.elements();
    assert_eq!(elements.iter().filter(|e| e.kind == ElementKind::Node).count(), 10);
    assert_eq!(elements.iter().filter(|e| e.kind == ElementKind::Relation).count(), 11);
    let broker = graph.find("message-broker");
    assert_eq!((broker.x, broker.y), (807.8672330646216, -240.0));
}

#[test]
fn the_palette_offers_one_node() {
    let graph = open(GRAPH);
    let toolbox = graph.session.toolbox(graph.workspace.documents());
    assert_eq!(toolbox.len(), 1);
    assert_eq!(toolbox[0].id, "dependencies.toolbox.node");
    assert_eq!(toolbox[0].drop_action_id, "dependencies.add-element");
}
