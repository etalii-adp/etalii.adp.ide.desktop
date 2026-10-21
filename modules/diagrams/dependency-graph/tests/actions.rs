//! The actions, walked along the session's discover-then-execute-then-answer path (the standalone host's
//! `DependencyGraphContextActionProvider.Tests.cs` at `8b6ccfc`).

mod common;

use adp_context::Prompt;
use adp_diagram::{ActionOutcome, ActionTarget};
use common::{GRAPH, canvas, element, open};

#[test]
fn a_node_offers_its_edits_and_its_additions() {
    let graph = open(GRAPH);
    assert_eq!(
        graph.action_ids(&element("aaa")),
        [
            "dependencies.rename",
            "dependencies.remove",
            "dependencies.add-after",
            "dependencies.add-below"
        ]
    );
}

#[test]
fn a_dependency_offers_relabel_and_disconnect() {
    let graph = open(GRAPH);
    assert_eq!(
        graph.action_ids(&element("ccc")),
        ["dependencies.relabel", "dependencies.disconnect"]
    );
}

#[test]
fn empty_canvas_offers_one_add() {
    let graph = open(GRAPH);
    assert_eq!(graph.action_ids(&canvas(700.0, 180.0)), ["dependencies.add-element"]);
}

#[test]
fn adding_on_a_placement_lands_where_it_was_dropped_with_nothing_asked() {
    let mut graph = open(GRAPH);
    assert_eq!(
        graph.execute(&canvas(-180.5, 240.0), "dependencies.add-element", &["new00001"]),
        ActionOutcome::Done
    );
    assert_eq!(graph.attribute("new00001", "x"), "-180.5");
    assert_eq!(graph.attribute("new00001", "row"), "4");
    assert_eq!(graph.attribute("new00001", "label"), "New node");
}

#[test]
fn tab_adds_a_node_a_step_to_the_right_that_the_selected_one_depends_on() {
    let mut graph = open(GRAPH);
    assert_eq!(
        graph.execute(&element("aaa"), "dependencies.add-after", &["new00001", "rel00001"]),
        ActionOutcome::Done
    );
    assert_eq!(graph.attribute("new00001", "x"), "480");
    assert_eq!(graph.attribute("new00001", "row"), "0");
    let relation = graph.find("rel00001");
    assert_eq!(relation.source.as_deref(), Some("aaa"));
    assert_eq!(relation.target.as_deref(), Some("new00001"));
}

#[test]
fn enter_adds_a_node_one_row_below_at_the_same_coordinate() {
    let mut graph = open(GRAPH);
    assert_eq!(
        graph.execute(&element("aaa"), "dependencies.add-below", &["new00001", "rel00001"]),
        ActionOutcome::Done
    );
    assert_eq!(graph.attribute("new00001", "x"), "240");
    assert_eq!(graph.attribute("new00001", "row"), "1");
    let relation = graph.find("rel00001");
    assert_eq!(relation.source.as_deref(), Some("aaa"));
    assert_eq!(relation.target.as_deref(), Some("new00001"));
}

#[test]
fn a_relation_gesture_between_two_nodes_connects_them_in_the_gestures_direction() {
    let mut graph = open(GRAPH);
    let gesture = ActionTarget::Connection {
        source: Some("bbb".to_owned()),
        target: Some("aaa".to_owned()),
    };
    assert_eq!(
        graph.execute(&gesture, "dependencies.connect", &["ddd"]),
        ActionOutcome::Done
    );
    let added = graph.find("ddd");
    assert_eq!(added.source.as_deref(), Some("bbb"));
    assert_eq!(added.target.as_deref(), Some("aaa"));
}

#[test]
fn a_relation_gesture_onto_empty_canvas_creates_the_dependency_and_relates_it_as_one_undo() {
    let mut graph = open(GRAPH);
    let before = graph.text();
    graph
        .connect_to_new("aaa", 900.0, 300.0, &["new00001", "rel00001"])
        .unwrap();
    assert_eq!(graph.attribute("new00001", "x"), "900");
    assert_eq!(graph.attribute("new00001", "row"), "5");
    let relation = graph.find("rel00001");
    assert_eq!(relation.source.as_deref(), Some("aaa"));
    assert_eq!(relation.target.as_deref(), Some("new00001"));
    graph.undo();
    assert_eq!(graph.text(), before);
}

#[test]
fn a_relation_gesture_from_a_vanished_node_fails_with_a_reason() {
    let mut graph = open(GRAPH);
    let refusal = graph
        .connect_to_new("ghost", 0.0, 0.0, &["new00001", "rel00001"])
        .unwrap_err();
    assert!(!refusal.is_empty());
}

#[test]
fn removing_an_unconnected_node_happens_at_once_with_no_confirmation() {
    let mut graph =
        open("dependencies: 1\r\nelements:\r\n  - id: lonely\r\n    label: Alone\r\n    x: 0\r\n    row: 0\r\n");
    assert_eq!(
        graph.execute(&element("lonely"), "dependencies.remove", &[]),
        ActionOutcome::Done
    );
    assert!(graph.elements().is_empty());
}

#[test]
fn removing_a_connected_node_confirms_and_says_how_many_dependencies_go_with_it() {
    let mut graph = open(GRAPH);
    let Prompt::Confirm { message, danger, .. } = graph.prompt(&element("aaa"), "dependencies.remove") else {
        panic!("a remove of a connected node confirms");
    };
    assert_eq!(
        message,
        "Removing this node also removes the 1 dependency attached to it."
    );
    assert!(danger);
    assert!(graph.elements().iter().any(|e| e.id == "aaa"));
}

#[test]
fn renaming_asks_then_commits_and_is_one_undo_away() {
    let mut graph = open(GRAPH);
    let before = graph.text();
    let Prompt::Input {
        label, initial_value, ..
    } = graph.prompt(&element("aaa"), "dependencies.rename")
    else {
        panic!("a rename asks for the label");
    };
    assert_eq!(label, "Label");
    assert_eq!(initial_value, "API gateway");
    assert_eq!(
        graph.answer(&element("aaa"), "dependencies.rename", "Edge router"),
        ActionOutcome::Done
    );
    assert_eq!(graph.attribute("aaa", "label"), "Edge router");
    graph.undo();
    assert_eq!(graph.text(), before);
}

#[test]
fn disconnecting_removes_the_dependency_and_is_one_undo_away() {
    let mut graph = open(GRAPH);
    let before = graph.text();
    assert_eq!(
        graph.execute(&element("ccc"), "dependencies.disconnect", &[]),
        ActionOutcome::Done
    );
    assert!(graph.elements().iter().all(|e| e.source.is_none()));
    graph.undo();
    assert_eq!(graph.text(), before);
}

#[test]
fn a_node_only_action_on_a_dependency_does_not_apply() {
    let mut graph = open(GRAPH);
    let before = graph.text();
    assert!(matches!(
        graph.execute(&element("ccc"), "dependencies.add-after", &["new00001", "rel00001"]),
        ActionOutcome::Refused(_)
    ));
    assert_eq!(graph.text(), before);
}

#[test]
fn no_action_is_about_time() {
    let graph = open(GRAPH);
    let mut ids = graph.action_ids(&element("aaa"));
    ids.extend(graph.action_ids(&element("ccc")));
    ids.extend(graph.action_ids(&canvas(0.0, 0.0)));
    assert!(!ids.is_empty());
    for id in ids {
        let name = id.rsplit('.').next().unwrap();
        for word in ["end", "begin", "moment", "date", "time", "duration"] {
            assert!(!name.split('-').any(|w| w.eq_ignore_ascii_case(word)), "{id}");
        }
    }
}
