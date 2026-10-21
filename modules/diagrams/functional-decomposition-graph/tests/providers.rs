//! The palette, the property grid and the menus answer the client's ids, and every edit they make is one of the
//! module's commands, one undo away - on the field-service example (ported from the standalone host's
//! `FdgProviders.Tests.cs` at `8b6ccfc`).

mod common;

use adp_context::Prompt;
use adp_diagram::ActionOutcome;
use adp_diagram_functional_decomposition_graph::actions;
use adp_diagram_functional_decomposition_graph::properties::{CONNECTION_NAME, DESCRIPTION, HEIGHT, NAME, TEXT, WIDTH};
use common::{Graph, gesture, item, placement};

fn row_ids(graph: &Graph, id: &str) -> Vec<String> {
    graph
        .session
        .properties(graph.workspace.documents(), &[id])
        .into_iter()
        .map(|r| r.id)
        .collect()
}

#[test]
fn every_type_and_a_connection_offers_a_description() {
    let graph = Graph::example();
    for (id, what) in [
        ("planning", "UI element"),
        ("open-task", "action"),
        ("planning-day", "data element"),
        ("sync-queue", "function"),
        ("note-offline", "comment"),
        ("c-open-shows-detail", "connection"),
    ] {
        assert!(
            row_ids(&graph, id).contains(&DESCRIPTION.to_owned()),
            "A {what} offers no Description row."
        );
    }
}

#[test]
fn the_four_named_types_offer_a_name_and_no_text() {
    let graph = Graph::example();
    for id in ["planning", "open-task", "planning-day", "sync-queue"] {
        let rows = row_ids(&graph, id);
        assert!(rows.contains(&NAME.to_owned()), "{id}");
        assert!(!rows.contains(&TEXT.to_owned()), "{id}");
        assert!(!rows.contains(&HEIGHT.to_owned()), "{id}");
    }
}

#[test]
fn a_comment_offers_no_name_but_its_text_and_its_height() {
    let graph = Graph::example();
    let rows = row_ids(&graph, "note-offline");
    assert!(!rows.contains(&NAME.to_owned()));
    assert!(rows.contains(&TEXT.to_owned()));
    assert!(rows.contains(&HEIGHT.to_owned()));
}

#[test]
fn a_connection_offers_its_name() {
    let graph = Graph::example();
    assert_eq!(row_ids(&graph, "c-open-shows-detail"), [CONNECTION_NAME, DESCRIPTION]);
}

#[test]
fn a_property_set_through_the_grid_reaches_the_document_and_undoes() {
    for (id, property, value) in [
        ("planning", NAME, "Plan"),
        ("note-offline", TEXT, "Offline first."),
        ("step-list", DESCRIPTION, "Every step of the open task."),
        ("planning", WIDTH, "220"),
        ("note-offline", HEIGHT, "140"),
        ("c-sync-upload", CONNECTION_NAME, "invokes"),
        ("c-open-shows-detail", DESCRIPTION, ""),
    ] {
        let mut graph = Graph::example();
        graph.set(id, property, value).unwrap();
        assert_ne!(graph.original, graph.bytes(), "{id} {property}");
        graph.undo();
        assert_eq!(graph.original, graph.bytes(), "{id} {property}");
    }
}

/// A resize goes through the rows, so the width row, and a Comment's height row, are what make it possible; a
/// height on a type that shares one is not a row, and is refused.
#[test]
fn a_canvas_resize_reaches_a_width_and_a_comments_height_only() {
    let mut graph = Graph::example();
    graph.set("planning", WIDTH, "220").unwrap();
    graph.set("note-offline", HEIGHT, "140").unwrap();
    assert!(graph.set("planning", HEIGHT, "140").is_err());
    let model = graph.model();
    assert_eq!(model.element_of("planning").unwrap().width, 220.0);
    assert_eq!(model.element_of("note-offline").unwrap().height, Some(140.0));
}

#[test]
fn a_size_with_three_decimals_is_shown_as_the_file_holds_it() {
    let mut graph = Graph::example();
    graph.set("planning", WIDTH, "220.125").unwrap();
    let rows = graph.session.properties(graph.workspace.documents(), &["planning"]);
    assert_eq!(rows.iter().find(|r| r.id == WIDTH).unwrap().value, "220.125");
    assert!(String::from_utf8(graph.bytes()).unwrap().contains("width: 220.125"));
}

#[test]
fn a_size_that_is_not_one_is_refused_and_nothing_is_written() {
    for (id, property, value) in [
        ("planning", WIDTH, "wide"),
        ("planning", WIDTH, "0"),
        ("note-offline", HEIGHT, "-5"),
    ] {
        let mut graph = Graph::example();
        let refusal = graph.set(id, property, value).unwrap_err();
        assert_eq!(refusal, format!("'{value}' is not a size this graph can draw."));
        assert_eq!(graph.original, graph.bytes());
    }
}

#[test]
fn a_drop_adds_its_type_centred_where_it_landed() {
    let mut graph = Graph::example();
    assert_eq!(
        graph.execute(placement(300.0, 700.0), "fdg.add.action"),
        ActionOutcome::Done
    );
    let model = graph.model();
    let added: Vec<_> = model.elements.iter().filter(|e| e.name == "New action").collect();
    assert_eq!(added.len(), 1);
    // The new Action's top-left is the centre less half its size (160 x 48).
    assert_eq!((added[0].x, added[0].y), (220.0, 676.0));
    // Its id is a ShortGuid.
    assert_eq!(added[0].id.len(), 25);
    assert!(
        added[0]
            .id
            .chars()
            .all(|c| c.is_ascii_digit() || c.is_ascii_lowercase())
    );
}

#[test]
fn a_second_drop_of_a_type_is_named_after_the_first() {
    let mut graph = Graph::example();
    graph.execute(placement(0.0, 0.0), "fdg.add.function");
    graph.execute(placement(0.0, 100.0), "fdg.add.function");
    graph.execute(placement(0.0, 200.0), "fdg.add.comment");
    let model = graph.model();
    let names: Vec<&str> = model.elements.iter().rev().take(3).map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["", "New function 2", "New function"]);
    assert_eq!(model.elements.last().unwrap().text, "New comment");
}

#[test]
fn a_body_that_is_not_yaml_refuses_an_add_up_front_and_writes_nothing() {
    let broken = "functional-decomposition-graph: 1\nelements: []\nconnections: \"never closed\n";
    let mut graph = Graph::of(broken);
    let ActionOutcome::Refused(refusal) = graph.execute(placement(300.0, 700.0), "fdg.add.action") else {
        panic!("an add on a body that is not YAML was made");
    };
    assert!(refusal.contains("could not be read"), "{refusal}");
    assert_eq!(String::from_utf8(graph.bytes()).unwrap(), broken);
}

#[test]
fn a_finished_connect_gesture_connects_and_a_refused_one_says_why() {
    let mut graph = Graph::example();
    // Tick step shows nothing yet, so it may show the Step list; Open task already shows one.
    assert_eq!(
        graph.execute(gesture("tick-step", "step-list"), "fdg.connect.shows"),
        ActionOutcome::Done
    );
    assert!(
        graph
            .model()
            .connections
            .iter()
            .any(|c| c.from == "tick-step" && c.to == "step-list")
    );
    let ActionOutcome::Refused(refusal) = graph.execute(gesture("open-task", "task-list"), "fdg.connect.shows") else {
        panic!("a second Shows was drawn");
    };
    assert!(refusal.contains("cardinality check"), "{refusal}");
}

#[test]
fn renaming_a_comment_opens_its_text() {
    let mut graph = Graph::example();
    let Prompt::Input { initial_value, .. } = graph.prompt(item("note-offline"), actions::RENAME) else {
        panic!("a rename asks for text");
    };
    assert!(initial_value.starts_with("Everything on these screens must work offline."));
}

#[test]
fn removing_an_element_with_connections_asks_first_and_the_commit_removes_them_all() {
    let mut graph = Graph::example();
    let Prompt::Confirm { message, danger, .. } = graph.prompt(item("task-row"), actions::REMOVE) else {
        panic!("a remove of a connected element confirms");
    };
    assert!(message.contains("2 connections"), "{message}");
    assert!(danger);
    assert_eq!(graph.answer(item("task-row"), actions::REMOVE, ""), ActionOutcome::Done);
    assert!(
        !graph
            .model()
            .connections
            .iter()
            .any(|c| c.from == "task-row" || c.to == "task-row")
    );
}

#[test]
fn removing_an_element_without_connections_needs_no_ceremony() {
    let mut graph = Graph::example();
    assert_eq!(
        graph.execute(item("note-offline"), actions::REMOVE),
        ActionOutcome::Done
    );
    assert!(graph.model().element_of("note-offline").is_none());
}

#[test]
fn a_placement_and_a_gesture_discover_exactly_the_clients_ids() {
    let graph = Graph::example();
    let ids = |target| -> Vec<String> {
        graph
            .session
            .actions(graph.workspace.documents(), &target)
            .into_iter()
            .flat_map(|g| g.actions)
            .map(|a| a.id)
            .collect()
    };
    assert_eq!(
        ids(placement(1.0, 2.0)),
        [
            "fdg.add.ui-element",
            "fdg.add.data-element",
            "fdg.add.action",
            "fdg.add.function",
            "fdg.add.comment"
        ]
    );
    assert_eq!(
        ids(gesture("a", "b")),
        [
            "fdg.connect.ui-child",
            "fdg.connect.owns-action",
            "fdg.connect.owns-data",
            "fdg.connect.owns-function",
            "fdg.connect.shows"
        ]
    );
    assert_eq!(ids(item("planning")), ["fdg.rename", "fdg.remove"]);
    assert_eq!(ids(item("c-sync-upload")), ["fdg.rename-connection", "fdg.disconnect"]);
}

/// An action its target does not discover is not run: an add is a drop, a rename is an element's.
#[test]
fn an_action_its_target_does_not_offer_is_refused() {
    let mut graph = Graph::example();
    assert!(matches!(
        graph.execute(item("planning"), "fdg.add.action"),
        ActionOutcome::Refused(_)
    ));
    assert!(matches!(
        graph.execute(placement(0.0, 0.0), actions::RENAME),
        ActionOutcome::Refused(_)
    ));
    assert_eq!(graph.original, graph.bytes());
}

#[test]
fn the_toolbox_offers_the_five_types_each_dropping_its_add_action() {
    let graph = Graph::example();
    let drops: Vec<String> = graph
        .session
        .toolbox(graph.workspace.documents())
        .into_iter()
        .map(|i| i.drop_action_id)
        .collect();
    assert_eq!(
        drops,
        [
            "fdg.add.ui-element",
            "fdg.add.action",
            "fdg.add.data-element",
            "fdg.add.function",
            "fdg.add.comment"
        ]
    );
}

/// A drag from one element's body to another's draws the one relation the pair admits, and nothing for a pair none
/// does (`relationOnto`).
#[test]
fn a_body_to_body_drag_runs_the_connect_of_the_relation_the_pair_admits() {
    let graph = Graph::example();
    let view = graph.session.module_view().unwrap();
    let documents = graph.workspace.documents();
    let action = |from, to| view.connect_action(documents, from, to);
    assert_eq!(
        action("tick-step", "step-list"),
        Some(Ok("fdg.connect.shows".to_owned()))
    );
    assert_eq!(
        action("planning", "planning-day"),
        Some(Ok("fdg.connect.owns-data".to_owned()))
    );
    let refused = Some(Err("Nothing in this diagram connects these two.".to_owned()));
    assert_eq!(action("open-task", "tick-step"), refused);
    assert_eq!(action("note-offline", "planning"), refused);
}
