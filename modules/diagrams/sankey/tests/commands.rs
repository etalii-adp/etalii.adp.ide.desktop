//! Every command edits, every undo gives back the original bytes, and every refusal writes nothing, through the store
//! on a real file. Ported from the standalone host's `EtAlii.Adp.Diagram.Sankey.Tests/SankeyCommands.Tests.cs` and
//! the one edit and one missing entry of `SankeyCommandHandlers.Tests.cs` (`EditHandlerContract`) at `8b6ccfc`. A new
//! node's id is given to its command here, where the standalone handler minted one when it was given none.

mod common;

use adp_diagram_sankey::commands::{self, SankeyCommand, SankeyEdit};
use adp_diagram_sankey::layout::{self, SankeyLayout};
use adp_diagram_sankey::model::{geometry, keys};
use adp_diagram_sankey::store;
use common::{Dispatcher, TestFolder, fixture};

fn small() -> TestFolder {
    TestFolder::new(&fixture("crlf-line-endings.skv"), "small.skv")
}

fn set(entry: &str, key: &str, value: &str) -> SankeyEdit {
    SankeyEdit::SetProperty {
        entry_id: entry.to_owned(),
        key: key.to_owned(),
        value: value.to_owned(),
    }
}

fn add(x: f64, y: f64, id: &str) -> SankeyEdit {
    SankeyEdit::AddNode {
        x,
        y,
        node_id: id.to_owned(),
    }
}

fn connect(from: &str, to: &str) -> SankeyEdit {
    SankeyEdit::Connect {
        from: from.to_owned(),
        to: to.to_owned(),
    }
}

fn remove(id: &str) -> SankeyEdit {
    SankeyEdit::Remove {
        entry_id: id.to_owned(),
    }
}

fn move_node(node: &str, anchor: &str, after: bool) -> SankeyEdit {
    SankeyEdit::MoveNode {
        node_id: node.to_owned(),
        anchor_id: anchor.to_owned(),
        after,
    }
}

fn step(flow: &str, steps: i32) -> SankeyEdit {
    SankeyEdit::StepValue {
        flow_id: flow.to_owned(),
        steps,
    }
}

fn every_edit() -> Vec<(&'static str, SankeyEdit)> {
    vec![
        ("add a node", add(0.0, 900.0, "n")),
        ("add a node in a column", add(geometry::COLUMN_PITCH * 3.0, 0.0, "n")),
        ("connect", connect("a", "x")),
        ("remove a node", remove("m")),
        ("remove a flow", remove("m->y")),
        ("move a node up", move_node("y", "x", false)),
        ("move a node to the end", move_node("a", "y", true)),
        ("rename", set("a", keys::NAME, "Source: A")),
        ("colour a node", set("b", keys::COLOR, "teal")),
        ("custom colour", set("a", keys::COLOR, "#c0ffee")),
        ("clear a colour", set("x", keys::COLOR, "")),
        ("note", set("x", keys::NOTE, "+3% Y/Y")),
        ("format", set("y", keys::FORMAT, "({value} t)")),
        ("column", set("y", keys::COLUMN, "4")),
        ("describe a flow", set("a->m", keys::DESCRIPTION, "Most of it.")),
        ("set a value", set("a->m", keys::VALUE, "7.25")),
        ("set a step", set("a->m", keys::STEP, "0.25")),
        ("increase", step("m->x", 1)),
        ("decrease", step("a->m", -1)),
        ("thicker", SankeyEdit::ScaleThickness { steps: 1 }),
        ("thinner", SankeyEdit::ScaleThickness { steps: -1 }),
    ]
}

#[test]
fn every_edit_then_its_undo_gives_the_original_bytes() {
    for (name, edit) in every_edit() {
        let folder = small();
        let original = folder.bytes();
        let mut dispatcher = Dispatcher::default();

        let undo = dispatcher
            .dispatch(&SankeyCommand::new(&folder.body, edit))
            .unwrap_or_else(|refusal| panic!("{name}: {refusal}"));

        // It edited, otherwise the undo proves nothing, and left a document that reads cleanly.
        assert_ne!(folder.bytes(), original, "{name}");
        assert!(folder.model().problems.is_empty(), "{name}");

        dispatcher.dispatch(undo.as_ref()).unwrap();
        assert_eq!(folder.bytes(), original, "{name}");
    }
}

fn every_refusal() -> Vec<(&'static str, &'static str, SankeyEdit)> {
    vec![
        ("connect a node to itself", "itself", connect("a", "a")),
        ("connect twice", "already", connect("a", "m")),
        ("connect to nothing", "one node to another", connect("a", "nowhere")),
        ("decrease below zero", "already zero", step("m->y", -1)),
        ("step a node", "change a flow's value", step("m", 1)),
        ("a negative value", "zero or more", set("a->m", keys::VALUE, "-3")),
        (
            "a value that is not a number",
            "zero or more",
            set("a->m", keys::VALUE, "lots"),
        ),
        ("a step of zero", "step of zero", set("a->m", keys::STEP, "0")),
        ("an unknown colour", "not a colour", set("a", keys::COLOR, "chartreuse")),
        ("a column of zero", "not a column", set("a", keys::COLUMN, "0")),
        ("an empty name", "needs a name", set("a", keys::NAME, " ")),
        ("remove what is not there", "no longer", remove("nothing-here")),
        ("a value on a node", "cannot be set", set("a", keys::VALUE, "3")),
        ("move next to itself", "next to itself", move_node("a", "a", true)),
        (
            "thinner than thinnest",
            "already as thin",
            SankeyEdit::ScaleThickness { steps: -100 },
        ),
    ]
}

#[test]
fn every_refusal_writes_nothing_and_says_why() {
    for (name, saying, edit) in every_refusal() {
        let folder = small();
        let mut dispatcher = Dispatcher::default();
        // Two refusals need the document at an end of its range first; that state is then the original. The cache is
        // loaded, so an in-place edit would have somewhere to hide.
        if name == "decrease below zero" {
            dispatcher.run(&folder.body, set("m->y", keys::VALUE, "0"));
        } else if name == "thinner than thinnest" {
            dispatcher.run(&folder.body, SankeyEdit::ScaleThickness { steps: -100 });
        }
        let bytes_before = folder.bytes();
        let cached_before = store::get_or_load(&mut dispatcher.documents, &folder.body).text.clone();

        let refusal = dispatcher
            .dispatch(&SankeyCommand::new(&folder.body, edit))
            .err()
            .unwrap_or_else(|| panic!("{name} was accepted"));

        assert!(
            refusal.to_lowercase().contains(&saying.to_lowercase()),
            "{name}: {refusal}"
        );
        assert_eq!(folder.bytes(), bytes_before, "{name}");
        assert_eq!(
            store::get_or_load(&mut dispatcher.documents, &folder.body).text,
            cached_before,
            "{name}"
        );
    }
}

#[test]
fn increasing_adds_the_flows_own_step_and_otherwise_a_tenth_of_its_magnitude() {
    let folder = small();
    let mut dispatcher = Dispatcher::default();
    // m->x steps by its own 0.5, and a->m, which states none, by a tenth of its magnitude: 0.1.
    dispatcher.run(&folder.body, step("m->x", 2));
    dispatcher.run(&folder.body, step("a->m", -3));
    let model = folder.model();
    assert_eq!(model.flow_of("m->x").unwrap().value, Some(8.0));
    assert_eq!(model.flow_of("a->m").unwrap().value, Some(5.7));
}

#[test]
fn the_default_step_is_a_tenth_of_the_values_order_of_magnitude() {
    for (value, expected) in [
        (None, 0.1),
        (Some(0.5), 0.1),
        (Some(6.0), 0.1),
        (Some(42.0), 1.0),
        (Some(1302.0), 100.0),
    ] {
        assert!((commands::default_step(value) - expected).abs() < 1e-6, "{value:?}");
    }
}

#[test]
fn decreasing_stops_at_zero_rather_than_going_negative() {
    let folder = small();
    // 3 in steps of 0.1 is thirty steps; a hundred are asked for.
    Dispatcher::default().run(&folder.body, step("m->y", -100));
    assert_eq!(folder.model().flow_of("m->y").unwrap().value, Some(0.0));
}

#[test]
fn ten_steps_of_a_fraction_come_back_to_a_whole_number() {
    let mut value = Some(0.0);
    for _ in 0..10 {
        value = commands::stepped(value, Some(0.1), 1);
    }
    assert_eq!(value, Some(1.0));
}

#[test]
fn an_added_node_lands_in_the_column_of_the_drop_above_the_first_node_below_it() {
    let folder = small();
    // Between x and y in the third column.
    let layout = SankeyLayout::of(&folder.model());
    let between = (layout.boxes["x"].bottom() + layout.boxes["y"].y) / 2.0;

    Dispatcher::default().run(&folder.body, add(layout::left_of(2) + 10.0, between, "n"));

    let model = folder.model();
    let ids: Vec<&str> = model.nodes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, ["a", "b", "m", "x", "n", "y"]);
    let added = model.node_of("n").unwrap();
    assert_eq!((added.name.as_str(), added.column), ("New node", Some(3)));
    assert_eq!(SankeyLayout::of(&model).column_order[2], ["x", "n", "y"]);
}

#[test]
fn a_node_dropped_in_the_first_column_states_no_column_and_a_further_one_gets_a_fresh_name() {
    let folder = small();
    let mut dispatcher = Dispatcher::default();
    dispatcher.run(&folder.body, add(0.0, 2000.0, "n1"));
    dispatcher.run(&folder.body, add(0.0, 2000.0, "n2"));
    let model = folder.model();
    let added = &model.nodes[5..];
    let names: Vec<&str> = added.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(names, ["New node", "New node 2"]);
    assert!(added.iter().all(|n| n.column.is_none()));
}

#[test]
fn a_new_flow_carries_on_what_the_source_has_not_passed_on() {
    let folder = small();
    let mut dispatcher = Dispatcher::default();
    // The middle takes in 10 and passes on 10, so first take 3 away from what it passes on.
    dispatcher.run(&folder.body, set("m->y", keys::VALUE, "0"));
    dispatcher.run(&folder.body, connect("m", "b"));
    dispatcher.run(&folder.body, connect("x", "y"));
    let model = folder.model();
    assert_eq!(model.flow_of("m->b").unwrap().value, Some(3.0));
    assert_eq!(model.flow_of("x->y").unwrap().value, Some(7.0));
}

#[test]
fn removing_a_node_removes_every_flow_to_or_from_it() {
    let folder = small();
    Dispatcher::default().run(&folder.body, remove("m"));
    let model = folder.model();
    assert!(model.node_of("m").is_none());
    assert!(model.flows.is_empty());
}

#[test]
fn thicker_writes_the_thickness_under_the_header_and_thinner_changes_that_line() {
    let folder = small();
    let mut dispatcher = Dispatcher::default();
    dispatcher.run(&folder.body, SankeyEdit::ScaleThickness { steps: 1 });
    dispatcher.run(&folder.body, SankeyEdit::ScaleThickness { steps: -2 });
    let text = std::fs::read_to_string(&folder.body).unwrap();
    assert!(text.starts_with("sankey: 1\r\nthickness: 0.8\r\n"), "{text}");
    assert_eq!(folder.model().settings.thickness, 0.8);
}

/// `EditHandlerContract`: an edit is written and undone to the original bytes, and an edit of an entry that is not
/// there is refused and writes nothing.
#[test]
fn an_edit_of_the_company_is_written_and_one_of_a_missing_node_is_refused() {
    let folder = TestFolder::new(&common::brightwater_coffee(), "brightwater-coffee.skv");
    let original = folder.bytes();
    let mut dispatcher = Dispatcher::default();
    let undo = dispatcher
        .dispatch(&SankeyCommand::new(
            &folder.body,
            set("cafes", keys::NAME, "Coffee bars"),
        ))
        .unwrap();
    assert_eq!(folder.model().node_of("cafes").unwrap().name, "Coffee bars");
    dispatcher.dispatch(undo.as_ref()).unwrap();
    assert_eq!(folder.bytes(), original);

    let refusal = dispatcher
        .dispatch(&SankeyCommand::new(
            &folder.body,
            set("no-such-node", keys::NAME, "Coffee bars"),
        ))
        .err()
        .unwrap();
    assert_eq!(refusal, commands::GONE);
    assert_eq!(folder.bytes(), original);
}
