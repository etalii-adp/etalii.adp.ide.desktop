//! Every edit through the project's history: applied, undone and, where a guard exists, refused, with the file byte
//! for byte as it was after an undo (the standalone host's `DependencyGraphCommands.Tests.cs` at `8b6ccfc`, whose
//! commands are here the session's gestures, actions and property edits).

mod common;

use adp_diagram::ActionOutcome;
use common::{GRAPH, canvas, element, open};

#[test]
fn add_then_undo_is_byte_identical() {
    let mut graph = open(GRAPH);
    let before = graph.text();
    assert_eq!(
        graph.execute(&canvas(720.0, 240.0), "dependencies.add-element", &["new00001"]),
        ActionOutcome::Done
    );
    assert!(graph.text().contains("id: new00001"));
    graph.undo();
    assert_eq!(graph.text(), before);
}

#[test]
fn adding_a_node_at_zero_or_a_negative_coordinate_is_accepted() {
    let mut graph = open(GRAPH);
    assert_eq!(
        graph.execute(&canvas(0.0, 0.0), "dependencies.add-element", &["zero0001"]),
        ActionOutcome::Done
    );
    assert_eq!(
        graph.execute(&canvas(-400.0, -120.0), "dependencies.add-element", &["neg00001"]),
        ActionOutcome::Done
    );
    assert_eq!(graph.attribute("zero0001", "x"), "0");
    assert_eq!(graph.attribute("neg00001", "x"), "-400");
    assert_eq!(graph.attribute("neg00001", "row"), "-2");
}

#[test]
fn adding_a_duplicate_id_is_refused() {
    let mut graph = open(GRAPH);
    let before = graph.text();
    assert!(matches!(
        graph.execute(&canvas(600.0, 0.0), "dependencies.add-element", &["aaa"]),
        ActionOutcome::Refused(_)
    ));
    assert_eq!(graph.text(), before);
}

#[test]
fn remove_takes_the_relations_and_undo_puts_everything_back_byte_for_byte() {
    let mut graph = open(GRAPH);
    let before = graph.text();
    assert!(matches!(
        graph.execute(&element("aaa"), "dependencies.remove", &[]),
        ActionOutcome::Prompt(_)
    ));
    assert_eq!(
        graph.answer(&element("aaa"), "dependencies.remove", ""),
        ActionOutcome::Done
    );
    let ids: Vec<String> = graph.elements().into_iter().map(|e| e.id).collect();
    assert_eq!(ids, ["bbb"]);
    graph.undo();
    assert_eq!(graph.text(), before);
}

#[test]
fn remove_undo_redo_removes_it_again() {
    let mut graph = open(GRAPH);
    graph.execute(&element("aaa"), "dependencies.remove", &[]);
    graph.answer(&element("aaa"), "dependencies.remove", "");
    graph.undo();
    graph.redo();
    assert!(graph.elements().iter().all(|e| e.id != "aaa"));
}

#[test]
fn removing_the_last_node_of_a_file_with_no_trailing_newline_undoes_byte_for_byte() {
    let unterminated = "dependencies: 1\r\nelements:\r\n  - id: last\r\n    x: 100\r\n    row: 0";
    let mut graph = open(unterminated);
    assert_eq!(
        graph.execute(&element("last"), "dependencies.remove", &[]),
        ActionOutcome::Done
    );
    graph.undo();
    assert_eq!(graph.text(), unterminated);
}

#[test]
fn rename_changes_one_line_leaves_the_id_and_undoes() {
    let mut graph = open(GRAPH);
    let before = graph.text();
    assert_eq!(
        graph.ask_and_answer(&element("aaa"), "dependencies.rename", "Renamed"),
        ActionOutcome::Done
    );
    assert_eq!(graph.attribute("aaa", "label"), "Renamed");
    assert_eq!(graph.text(), before.replace("label: API gateway", "label: Renamed"));
    graph.undo();
    assert_eq!(graph.text(), before);
}

#[test]
fn a_placement_moves_the_node_and_undoes_byte_for_byte() {
    let mut graph = open(GRAPH);
    let before = graph.text();
    graph.move_to("aaa", 812.5, 360.0).unwrap();
    assert_eq!(graph.attribute("aaa", "x"), "812.5");
    assert_eq!(graph.attribute("aaa", "row"), "6");
    graph.undo();
    assert_eq!(graph.text(), before);
}

#[test]
fn a_placement_on_an_absent_node_is_refused_and_the_file_untouched() {
    let mut graph = open(GRAPH);
    let before = graph.text();
    assert!(graph.move_to("ghost", 100.0, 0.0).is_err());
    assert_eq!(graph.text(), before);
}

#[test]
fn connect_then_undo_is_byte_identical() {
    let mut graph = open(GRAPH);
    let before = graph.text();
    graph.connect("bbb", "aaa", "ddd").unwrap();
    graph.undo();
    assert_eq!(graph.text(), before);
}

#[test]
fn connect_writes_the_dependency_in_the_direction_it_was_given() {
    let mut graph = open(GRAPH);
    graph.connect("bbb", "aaa", "ddd").unwrap();
    let added = graph.find("ddd");
    assert_eq!(added.source.as_deref(), Some("bbb"));
    assert_eq!(added.target.as_deref(), Some("aaa"));
    assert!(graph.text().ends_with("  - id: ddd\n    from: bbb\n    to: aaa\n"));
}

#[test]
fn a_self_dependency_is_refused() {
    let mut graph = open(GRAPH);
    let before = graph.text();
    assert_eq!(
        graph.connect("aaa", "aaa", "ddd"),
        Err(adp_diagram_dependency_graph::SELF_DEPENDENCY.to_owned())
    );
    assert_eq!(graph.text(), before);
}

#[test]
fn a_second_dependency_between_the_same_pair_is_permitted() {
    let mut graph = open(GRAPH);
    graph.connect("aaa", "bbb", "ddd").unwrap();
    assert_eq!(graph.elements().iter().filter(|e| e.source.is_some()).count(), 2);
}

#[test]
fn a_dependency_on_an_absent_node_is_refused() {
    let mut graph = open(GRAPH);
    let before = graph.text();
    assert!(graph.connect("aaa", "ghost", "ddd").is_err());
    assert_eq!(graph.text(), before);
}

#[test]
fn disconnect_then_undo_brings_the_dependency_back_the_right_way_round() {
    let mut graph = open(GRAPH);
    assert_eq!(
        graph.execute(&element("ccc"), "dependencies.disconnect", &[]),
        ActionOutcome::Done
    );
    assert!(graph.elements().iter().all(|e| e.source.is_none()));
    graph.undo();
    let restored = graph.find("ccc");
    assert_eq!(restored.source.as_deref(), Some("aaa"));
    assert_eq!(restored.target.as_deref(), Some("bbb"));
    assert_eq!(graph.attribute("ccc", "label"), "verifies tokens with");
}

#[test]
fn relabel_then_undo_is_byte_identical() {
    let mut graph = open(GRAPH);
    let before = graph.text();
    assert_eq!(
        graph.ask_and_answer(&element("ccc"), "dependencies.relabel", "renamed"),
        ActionOutcome::Done
    );
    assert!(graph.text().contains("label: renamed"));
    graph.undo();
    assert_eq!(graph.text(), before);
}

#[test]
fn add_connected_creates_both_as_one_history_entry() {
    let mut graph = open(GRAPH);
    let before = graph.text();
    graph
        .connect_to_new("aaa", 900.0, 180.0, &["new00001", "rel00001"])
        .unwrap();
    assert_eq!(graph.elements().iter().filter(|e| e.source.is_none()).count(), 3);
    let relation = graph.find("rel00001");
    assert_eq!(relation.source.as_deref(), Some("aaa"));
    assert_eq!(relation.target.as_deref(), Some("new00001"));
    assert_eq!(graph.attribute("new00001", "row"), "3");
    graph.undo();
    assert_eq!(graph.text(), before);
}

#[test]
fn add_connected_into_a_graph_with_no_relations_section_undoes_byte_for_byte() {
    let bare = "dependencies: 1\r\nelements:\r\n  - id: only\r\n    label: Only node\r\n    x: 0\r\n    row: 0\r\n";
    let mut graph = open(bare);
    graph
        .connect_to_new("only", 240.0, 0.0, &["new00003", "rel00003"])
        .unwrap();
    assert!(graph.text().contains("relations:\r\n  - id: rel00003\r\n"));
    graph.undo();
    assert_eq!(graph.text(), bare);
}

#[test]
fn no_edit_touches_the_comment() {
    let mut graph = open(GRAPH);
    graph.execute(&canvas(720.0, 240.0), "dependencies.add-element", &["new00001"]);
    graph.ask_and_answer(&element("aaa"), "dependencies.rename", "Renamed");
    graph.connect("new00001", "aaa", "ddd").unwrap();
    graph.execute(&element("new00001"), "dependencies.remove", &[]);
    graph.answer(&element("new00001"), "dependencies.remove", "");
    assert!(graph.text().contains("# A comment the commands must never disturb."));
}

#[test]
fn an_edit_on_an_unparseable_file_is_refused_and_the_file_untouched() {
    let broken = "elements: [\n  - id: a\n";
    let mut graph = common::open(broken);
    assert!(!matches!(
        graph.ask_and_answer(&element("a"), "dependencies.rename", "Renamed"),
        ActionOutcome::Done
    ));
    assert_eq!(graph.text(), broken);
}
