//! Every command edits, every undo gives back the original bytes, and every refusal writes nothing, on disk or in
//! the store's reading - on the field-service example, through the project's history (ported from the standalone
//! host's `FdgCommands.Tests.cs` and `FdgCommandHandlers.Tests.cs` at `8b6ccfc`).

mod common;

use std::fs;

use adp_diagram_functional_decomposition_graph::commands::{FdgCommand, FdgEdit};
use adp_diagram_functional_decomposition_graph::model::{connection_types, element_types};
use adp_diagram_functional_decomposition_graph::{relations, store};
use adp_history::Outcome;
use common::Graph;

fn add(element_type: &str, x: f64, y: f64) -> FdgEdit {
    FdgEdit::Add {
        element_type: element_type.to_owned(),
        x,
        y,
        element_id: "new-one".to_owned(),
    }
}

fn connect(relation: &str, from: &str, to: &str) -> FdgEdit {
    FdgEdit::Connect {
        relation: relation.to_owned(),
        from: from.to_owned(),
        to: to.to_owned(),
        connection_id: "new-link".to_owned(),
    }
}

fn every_edit() -> Vec<(&'static str, FdgEdit)> {
    vec![
        ("add a UI element", add(element_types::UI_ELEMENT, 300.0, 700.0)),
        ("add a comment", add(element_types::COMMENT, 900.0, 700.0)),
        (
            "remove an element",
            FdgEdit::Remove {
                element_id: "task-row".to_owned(),
            },
        ),
        (
            "move",
            FdgEdit::Place {
                element_id: "planning".to_owned(),
                x: 10.0,
                y: 20.0,
            },
        ),
        (
            "resize",
            FdgEdit::Size {
                element_id: "planning".to_owned(),
                width: 220.0,
                height: None,
            },
        ),
        (
            "resize a comment",
            FdgEdit::Size {
                element_id: "note-offline".to_owned(),
                width: 300.0,
                height: Some(120.0),
            },
        ),
        // Tick step shows nothing yet, Shows has no limit into a page, and Shows is not ownership.
        ("connect", connect(connection_types::SHOWS, "tick-step", "step-list")),
        (
            "disconnect",
            FdgEdit::Disconnect {
                connection_id: "c-back-shows-list".to_owned(),
            },
        ),
        (
            "rename",
            FdgEdit::Rename {
                element_id: "planning".to_owned(),
                value: "Plan".to_owned(),
            },
        ),
        (
            "set a comment's text",
            FdgEdit::Rename {
                element_id: "note-offline".to_owned(),
                value: "Offline first.\nAlways.".to_owned(),
            },
        ),
        (
            "describe an element",
            FdgEdit::Describe {
                id: "step-list".to_owned(),
                description: "Every step of the open task.".to_owned(),
            },
        ),
        (
            "clear a connection's description",
            FdgEdit::Describe {
                id: "c-open-shows-detail".to_owned(),
                description: String::new(),
            },
        ),
        (
            "rename a connection",
            FdgEdit::RenameConnection {
                connection_id: "c-sync-upload".to_owned(),
                name: "invokes".to_owned(),
            },
        ),
    ]
}

fn run(graph: &mut Graph, edit: FdgEdit) -> Outcome {
    let command = FdgCommand::new(&graph.body, edit);
    graph.workspace.execute_command(&command)
}

#[test]
fn every_edit_then_its_undo_gives_the_original_bytes() {
    for (name, edit) in every_edit() {
        let mut graph = Graph::example();
        let outcome = run(&mut graph, edit);
        assert!(outcome.is_done(), "{name}: {outcome:?}");
        assert_ne!(graph.original, graph.bytes(), "{name} edited nothing");

        graph.undo();
        assert_eq!(graph.original, graph.bytes(), "{name}");
        // And the redo makes it again, under the same ids.
        assert!(graph.workspace.redo().is_done(), "{name}");
        assert_ne!(graph.original, graph.bytes(), "{name}");
    }
}

fn every_refusal() -> Vec<(&'static str, FdgEdit, &'static str)> {
    vec![
        (
            "connect an Action to an Action",
            connect(connection_types::OWNS_ACTION, "open-task", "tick-step"),
            "type check",
        ),
        (
            "link an element to itself",
            connect(connection_types::UI_CHILD, "planning", "planning"),
            "type check",
        ),
        // Task row already has Task list as its parent.
        (
            "give a UI element a second parent",
            connect(connection_types::UI_CHILD, "planning", "task-row"),
            "cardinality check",
        ),
        // Open task already shows Task detail.
        (
            "give an Action a second Shows",
            connect(connection_types::SHOWS, "open-task", "task-list"),
            "cardinality check",
        ),
        // Planning owns Task list, which owns Task row: Task row owning Planning closes the loop.
        (
            "close an ownership loop",
            connect(connection_types::UI_CHILD, "task-row", "planning"),
            "cycle check",
        ),
        // The writer sets the width before it refuses the height: the case the copy exists for.
        (
            "give a UI element a height",
            FdgEdit::Size {
                element_id: "planning".to_owned(),
                width: 300.0,
                height: Some(200.0),
            },
            "own height",
        ),
        (
            "remove an element that is not there",
            FdgEdit::Remove {
                element_id: "nothing-here".to_owned(),
            },
            "no longer in this graph",
        ),
        (
            "rename a connection that is not there",
            FdgEdit::RenameConnection {
                connection_id: "nothing-here".to_owned(),
                name: "x".to_owned(),
            },
            "no longer in this graph",
        ),
        (
            "add a type the notation has not",
            add("swimlane", 0.0, 0.0),
            "has no `swimlane` element",
        ),
        (
            "add under an id already used",
            FdgEdit::Add {
                element_type: element_types::ACTION.to_owned(),
                x: 0.0,
                y: 0.0,
                element_id: "planning".to_owned(),
            },
            "already used",
        ),
    ]
}

#[test]
fn every_refusal_writes_nothing_and_says_why() {
    for (name, edit, saying) in every_refusal() {
        let mut graph = Graph::example();
        let cached = store::get(graph.workspace.documents(), &graph.body)
            .unwrap()
            .text
            .clone();
        match run(&mut graph, edit) {
            Outcome::Refused(refusal) => assert!(refusal.0.contains(saying), "{name}: {}", refusal.0),
            other => panic!("{name} was not refused: {other:?}"),
        }
        assert_eq!(graph.original, graph.bytes(), "{name}");
        assert_eq!(
            cached,
            store::get(graph.workspace.documents(), &graph.body).unwrap().text,
            "{name}"
        );
    }
}

#[test]
fn deleting_an_element_with_two_connections_restores_all_three_on_undo() {
    let mut graph = Graph::example();
    let touching = |graph: &Graph| {
        graph
            .model()
            .connections
            .iter()
            .filter(|c| c.from == "task-row" || c.to == "task-row")
            .count()
    };
    assert_eq!(touching(&graph), 2);

    assert!(
        run(
            &mut graph,
            FdgEdit::Remove {
                element_id: "task-row".to_owned()
            }
        )
        .is_done()
    );
    assert!(graph.model().element_of("task-row").is_none());
    assert_eq!(touching(&graph), 0);

    graph.undo();
    assert_eq!(graph.original, graph.bytes());
    assert_eq!(touching(&graph), 2);
}

#[test]
fn a_cycle_closing_link_is_refused_by_its_own_check_and_nothing_is_written() {
    let mut graph = Graph::example();
    assert!(relations::would_close(&graph.model(), "task-row", "planning"));

    let Outcome::Refused(refusal) = run(&mut graph, connect(connection_types::UI_CHILD, "task-row", "planning")) else {
        panic!("a cycle-closing link was written");
    };
    assert!(refusal.0.contains("cycle check"), "{}", refusal.0);
    assert!(relations::cycles_in(&graph.model()).is_empty());
    assert_eq!(graph.original, graph.bytes());
}

#[test]
fn a_move_through_the_session_is_written_and_one_undo_away() {
    let mut graph = Graph::example();
    graph
        .session
        .move_elements(&mut graph.workspace, &[("planning".to_owned(), 10.0, 20.0)])
        .unwrap();
    let planning = graph.model().element_of("planning").unwrap().clone();
    assert_eq!((planning.x, planning.y), (10.0, 20.0));

    graph.undo();
    assert_eq!(graph.original, graph.bytes());
}

#[test]
fn a_drag_the_graph_cannot_place_is_refused_in_its_own_words_and_writes_nothing() {
    let mut graph = Graph::example();
    let connection = graph.model().connections[0].id.clone();

    let curve = graph
        .session
        .move_elements(&mut graph.workspace, &[(connection, 10.0, 20.0)]);
    let reparent = graph
        .session
        .move_element(&mut graph.workspace, "planning", Some("dispatch"), 0);

    assert_eq!(curve, Err("That is not something this graph can move.".to_owned()));
    assert_eq!(
        reparent,
        Err(
            "An element here has no parent to move it under; its parent is a connection, drawn from the parent to it."
                .to_owned()
        )
    );
    assert_eq!(graph.original, graph.bytes());
}

/// A body that is not YAML is never written: an edit is refused up front, saying it could not be read, and the
/// reading keeps the parser's reason.
#[test]
fn a_body_that_is_not_yaml_is_refused_up_front_and_never_written() {
    let broken = "functional-decomposition-graph: 1\nelements: []\nconnections: \"never closed\n";
    let mut graph = Graph::of(broken);

    let outcome = run(
        &mut graph,
        FdgEdit::Rename {
            element_id: "planning".to_owned(),
            value: "Plan".to_owned(),
        },
    );

    let Outcome::Refused(refusal) = outcome else {
        panic!("an edit of a body that is not YAML was made");
    };
    assert!(refusal.0.contains("could not be read"), "{}", refusal.0);
    assert_eq!(fs::read_to_string(&graph.body).unwrap(), broken);
    let document = store::get(graph.workspace.documents(), &graph.body).unwrap();
    assert!(
        document
            .unreadable
            .as_deref()
            .is_some_and(|r| r.starts_with("The document could not be read as YAML: "))
    );
    assert_eq!(document.model.problems.len(), 1);
}

/// What every edit does with a save: an edit of an element that is gone is refused and writes nothing, and one that
/// is made is one entry on the project's history (`EditHandlerContract`).
#[test]
fn an_edit_is_one_history_entry_and_an_edit_of_what_is_gone_none() {
    let mut graph = Graph::example();
    let missing = run(
        &mut graph,
        FdgEdit::Rename {
            element_id: "no-such-element".to_owned(),
            value: "Dispatch board".to_owned(),
        },
    );
    assert!(!missing.is_done());
    assert!(!graph.workspace.history().availability().can_undo);

    assert!(
        run(
            &mut graph,
            FdgEdit::Rename {
                element_id: "planning".to_owned(),
                value: "Dispatch board".to_owned()
            }
        )
        .is_done()
    );
    assert!(graph.model().element_of("planning").unwrap().name == "Dispatch board");
    assert!(graph.workspace.history().availability().can_undo);
    graph.undo();
    assert!(!graph.workspace.history().availability().can_undo);
    assert_eq!(graph.original, graph.bytes());
}
