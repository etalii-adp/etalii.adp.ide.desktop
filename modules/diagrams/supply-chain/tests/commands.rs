//! Every command edits, every undo gives back the original bytes, and every refusal writes nothing, through the
//! project's history on a real file (the standalone host's `SupplyChainCommandsTests`).

mod common;

use adp_diagram_supply_chain::commands::{SupplyChainCommand, SupplyChainEdit, keys, stepped};
use adp_diagram_supply_chain::layout::SupplyChainLayout;
use adp_diagram_supply_chain::model::{geometry, stages};
use adp_diagram_supply_chain::{store, validator};
use adp_history::Outcome;
use common::{Diagram, open_fixture};

fn small() -> Diagram {
    open_fixture("crlf-line-endings.supply")
}

fn set(entry: &str, key: &str, value: &str) -> SupplyChainEdit {
    SupplyChainEdit::SetProperty {
        entry_id: entry.into(),
        key: key.into(),
        value: value.into(),
    }
}

fn step(entry: &str, steps: i32) -> SupplyChainEdit {
    SupplyChainEdit::Step {
        entry_id: entry.into(),
        steps,
    }
}

fn moved(entry: &str, x: f64, y: f64) -> SupplyChainEdit {
    SupplyChainEdit::Move {
        entry_id: entry.into(),
        x,
        y,
    }
}

fn add_group(x: f64, y: f64) -> SupplyChainEdit {
    SupplyChainEdit::AddGroup {
        x,
        y,
        id: "added-group".into(),
    }
}

fn every_edit() -> Vec<(&'static str, SupplyChainEdit)> {
    vec![
        (
            "add a node",
            SupplyChainEdit::AddNode {
                stage: stages::HUB.into(),
                x: 500.0,
                y: 500.0,
                id: "added-node".into(),
            },
        ),
        (
            "connect",
            SupplyChainEdit::Connect {
                from: "mine".into(),
                to: "shop".into(),
                id: "added-flow".into(),
            },
        ),
        (
            "remove a node",
            SupplyChainEdit::Remove {
                entry_id: "plant".into(),
            },
        ),
        ("remove a flow", SupplyChainEdit::Remove { entry_id: "ore".into() }),
        (
            "remove a group",
            SupplyChainEdit::Remove {
                entry_id: "north".into(),
            },
        ),
        (
            "place",
            SupplyChainEdit::Place {
                places: vec![("mine".into(), (10.0, 20.0))],
            },
        ),
        (
            "move a group",
            SupplyChainEdit::Place {
                places: vec![("mine".into(), (10.0, 20.0)), ("plant".into(), (10.0, 160.0))],
            },
        ),
        ("arrange", SupplyChainEdit::Arrange),
        ("rename", set("mine", keys::NAME, "Open-pit mine")),
        ("set a quantity", set("plant", keys::QUANTITY, "7.25")),
        ("clear a quantity", set("plant", keys::QUANTITY, "")),
        ("set a stage", set("plant", keys::TYPE, stages::INTEGRATOR)),
        ("ungroup", set("plant", keys::GROUP, "")),
        ("describe a flow", set("goods", keys::DESCRIPTION, "Boxed.")),
        ("increase", step("mine", 1)),
        ("decrease", step("plant", -1)),
        ("increase a flow", step("goods", 1)),
        (
            "put in a new group",
            SupplyChainEdit::Group {
                node_id: "shop".into(),
                name: "High street".into(),
                id: "added-group".into(),
            },
        ),
        ("add a group", add_group(2000.0, 2000.0)),
        ("move a node", moved("shop", 900.0, 60.0)),
        ("move a node into a group", moved("shop", 0.0, 40.0)),
        ("move a group's frame", moved("north", 300.0, 300.0)),
    ]
}

impl Diagram {
    fn run(&mut self, edit: SupplyChainEdit) -> Result<(), String> {
        let command = SupplyChainCommand::new(&self.body, edit);
        match self.workspace.execute_command(&command) {
            Outcome::Done { .. } => Ok(()),
            Outcome::Refused(refusal) => Err(refusal.0),
        }
    }

    fn dispatch(&mut self, edit: SupplyChainEdit) {
        self.run(edit).unwrap();
    }
}

#[test]
fn every_edit_then_its_undo_gives_the_original_bytes() {
    for (name, edit) in every_edit() {
        let mut diagram = small();
        let original = diagram.bytes();

        diagram.run(edit).unwrap_or_else(|e| panic!("{name}: {e}"));

        // It edited - otherwise the undo proves nothing - and left a document that reads cleanly.
        assert_ne!(original, diagram.bytes(), "{name}");
        assert!(
            diagram.model().problems.is_empty(),
            "{name}: {:?}",
            diagram.model().problems
        );

        assert!(diagram.workspace.undo().is_done(), "{name}");
        assert_eq!(original, diagram.bytes(), "{name}");
    }
}

#[test]
fn every_refusal_writes_nothing_and_says_why() {
    let connect = |from: &str, to: &str| SupplyChainEdit::Connect {
        from: from.into(),
        to: to.into(),
        id: "f".into(),
    };
    for (name, edit, saying) in [
        ("connect a node to itself", connect("mine", "mine"), "itself"),
        ("connect twice", connect("mine", "plant"), "already"),
        ("connect to nothing", connect("mine", "nowhere"), "one node to another"),
        ("decrease below zero", step("shop", -1), "already zero"),
        ("a negative quantity", set("mine", keys::QUANTITY, "-3"), "zero or more"),
        (
            "a quantity that is not a number",
            set("mine", keys::QUANTITY, "lots"),
            "zero or more",
        ),
        ("a step of zero", set("mine", keys::STEP, "0"), "step of zero"),
        ("an unknown stage", set("mine", keys::TYPE, "quarry"), "not a stage"),
        ("an unknown group", set("mine", keys::GROUP, "south"), "no group"),
        ("step a group", step("north", 1), "no value of its own"),
        (
            "remove what is not there",
            SupplyChainEdit::Remove {
                entry_id: "nothing-here".into(),
            },
            "no longer",
        ),
        (
            "a product on a node",
            set("mine", keys::PRODUCT, "Ore"),
            "cannot be set",
        ),
    ] {
        let mut diagram = small();
        let original = diagram.bytes();
        let cached = store::get(diagram.workspace.documents(), &diagram.body)
            .unwrap()
            .text
            .clone();

        let refusal = diagram.run(edit).expect_err(name);

        assert!(refusal.to_lowercase().contains(saying), "{name}: {refusal}");
        assert_eq!(original, diagram.bytes(), "{name}");
        assert_eq!(
            cached,
            store::get(diagram.workspace.documents(), &diagram.body).unwrap().text,
            "{name}"
        );
    }
}

#[test]
fn increasing_adds_the_nodes_own_step_and_decreasing_takes_it_away() {
    let mut diagram = small();
    // The plant steps by 0.5, the mine by the default of 1.
    diagram.dispatch(step("plant", 1));
    diagram.dispatch(step("mine", -3));
    let model = diagram.model();
    assert_eq!(Some(4.5), model.node_of("plant").unwrap().quantity);
    assert_eq!(Some(7.0), model.node_of("mine").unwrap().quantity);
}

#[test]
fn decreasing_stops_at_zero_rather_than_going_negative() {
    let mut diagram = small();
    // 4 in steps of 0.5 is eight steps; ten are asked for.
    diagram.dispatch(step("plant", -10));
    assert_eq!(Some(0.0), diagram.model().node_of("plant").unwrap().quantity);
}

#[test]
fn ten_steps_of_a_fraction_come_back_to_a_whole_number() {
    let mut value = Some(0.0);
    for _ in 0..10 {
        value = stepped(value, Some(0.1), 1);
    }
    assert_eq!(Some(1.0), value);
}

#[test]
fn an_added_node_is_centred_on_the_drop_with_a_name() {
    let mut diagram = small();
    diagram.dispatch(SupplyChainEdit::AddNode {
        stage: stages::HUB.into(),
        x: 500.0,
        y: 500.0,
        id: "added".into(),
    });
    let model = diagram.model();
    let added = model.nodes.last().unwrap();
    assert_eq!(stages::HUB, added.node_type);
    assert_eq!("New hub", added.name);
    assert_eq!(Some(500.0 - geometry::NODE_WIDTH / 2.0), added.x);
    assert_eq!(Some(500.0 - geometry::NODE_HEIGHT / 2.0), added.y);
}

#[test]
fn putting_a_node_in_a_new_group_creates_the_group_and_moves_the_node_into_it() {
    let mut diagram = small();
    diagram.dispatch(SupplyChainEdit::Group {
        node_id: "shop".into(),
        name: "High street".into(),
        id: "high-street".into(),
    });
    let model = diagram.model();
    let group = model.groups.iter().find(|g| g.name == "High street").unwrap();
    assert_eq!(group.id, model.node_of("shop").unwrap().group);
}

#[test]
fn a_legacy_stage_word_reads_as_the_stage_it_became_and_an_edit_writes_the_new_word() {
    let mut diagram = small();
    assert_eq!(stages::PRODUCER, diagram.model().node_of("plant").unwrap().node_type);
    diagram.dispatch(set("plant", keys::TYPE, "assembler"));
    assert_eq!(stages::INTEGRATOR, diagram.model().node_of("plant").unwrap().node_type);
    assert!(diagram.text().contains("type: integrator"));
}

#[test]
fn an_added_group_is_drawn_empty_centred_on_the_drop() {
    let mut diagram = small();
    diagram.dispatch(add_group(2000.0, 2000.0));
    let model = diagram.model();
    let group = model.groups.last().unwrap();
    assert_eq!("New group", group.name);
    let frame = SupplyChainLayout::of(&model).group_boxes[&group.id];
    assert_eq!(2000.0, frame.center_x());
    assert_eq!(2000.0, frame.center_y());
    assert!(validator::validate(&model).is_empty());
}

#[test]
fn dragging_a_node_into_another_groups_frame_moves_it_into_that_group() {
    let mut diagram = small();
    diagram.dispatch(add_group(2000.0, 2000.0));
    // The plant's centre lands in the middle of the new group's frame.
    diagram.dispatch(moved(
        "plant",
        2000.0 - geometry::NODE_WIDTH / 2.0,
        2000.0 - geometry::NODE_HEIGHT / 2.0,
    ));
    let model = diagram.model();
    assert_eq!("added-group", model.node_of("plant").unwrap().group);
    assert_eq!("north", model.node_of("mine").unwrap().group);
}

#[test]
fn dragging_a_node_outside_every_other_frame_keeps_it_in_its_own_group() {
    let mut diagram = small();
    diagram.dispatch(moved("plant", 5000.0, 5000.0));
    let model = diagram.model();
    assert_eq!("north", model.node_of("plant").unwrap().group);
    assert!(SupplyChainLayout::of(&model).group_boxes["north"].right() >= 5000.0 + geometry::NODE_WIDTH);
}

#[test]
fn a_group_its_last_member_leaves_stays_where_its_frame_was() {
    let mut diagram = small();
    diagram.dispatch(add_group(2000.0, 2000.0));
    let (x, y) = (
        2000.0 - geometry::NODE_WIDTH / 2.0,
        2000.0 - geometry::NODE_HEIGHT / 2.0,
    );
    diagram.dispatch(moved("mine", x, y));
    let north = SupplyChainLayout::of(&diagram.model()).group_boxes["north"];

    diagram.dispatch(moved("plant", x, y));

    let model = diagram.model();
    assert!(model.nodes.iter().all(|n| n.group != "north"));
    let frame = SupplyChainLayout::of(&model).group_boxes["north"];
    assert_eq!(
        (north.x.round_ties_even(), north.y.round_ties_even()),
        (frame.x, frame.y)
    );
}

#[test]
fn dragging_an_empty_group_places_its_frame() {
    let mut diagram = small();
    diagram.dispatch(add_group(2000.0, 2000.0));
    diagram.dispatch(moved("added-group", 100.0, 900.0));
    let frame = SupplyChainLayout::of(&diagram.model()).group_boxes["added-group"];
    assert_eq!((100.0, 900.0), (frame.x, frame.y));
}

#[test]
fn arranging_places_every_node() {
    let mut diagram = small();
    diagram.dispatch(SupplyChainEdit::Arrange);
    assert!(diagram.model().nodes.iter().all(|n| n.is_placed()));
}

#[test]
fn an_undone_edit_is_redone_with_the_id_it_was_given() {
    let mut diagram = small();
    diagram.dispatch(add_group(2000.0, 2000.0));
    let added = diagram.bytes();
    assert!(diagram.workspace.undo().is_done());
    assert!(diagram.workspace.redo().is_done());
    assert_eq!(added, diagram.bytes());
}
