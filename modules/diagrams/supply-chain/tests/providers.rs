//! The menus, the property grid and the palette, each editing through the commands (the standalone host's
//! `SupplyChainProvidersTests`), asked through the diagram's session as the shell asks them.

mod common;

use adp_context::{Action, Prompt};
use adp_diagram::{ActionOutcome, ActionTarget};
use adp_diagram_supply_chain::actions::{self, ADD_GROUP, ARRANGE, CONNECT, DECREASE, GROUP, INCREASE, REMOVE};
use adp_diagram_supply_chain::model::stages;
use adp_diagram_supply_chain::properties::{self, NO_GROUP};
use adp_diagram_supply_chain::{store, toolbox};
use common::{Diagram, open_bytes, open_fixture};

fn small() -> Diagram {
    open_fixture("lf-line-endings.supply")
}

fn element(id: &str) -> ActionTarget {
    ActionTarget::Element(id.to_owned())
}

fn drop_at(x: f64, y: f64) -> ActionTarget {
    ActionTarget::Canvas { x, y }
}

impl Diagram {
    fn actions_of(&self, target: &ActionTarget) -> Vec<Action> {
        self.session
            .actions(self.workspace.documents(), target)
            .into_iter()
            .flat_map(|g| g.actions)
            .collect()
    }

    fn rows(&self, id: &str) -> Vec<adp_context::PropertyRow> {
        self.session.properties(self.workspace.documents(), &[id])
    }

    fn set(&mut self, id: &str, row: &str, value: &str) -> Result<(), String> {
        self.session.set_property(&mut self.workspace, &[id], row, value)
    }
}

#[test]
fn a_node_has_its_identity_and_amount_rows_with_the_group_by_name() {
    let rows = small().rows("mine");
    assert_eq!(
        vec![
            properties::NAME,
            properties::STAGE,
            properties::GROUP,
            properties::DESCRIPTION,
            properties::QUANTITY,
            properties::UNIT,
            properties::STEP
        ],
        rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>()
    );
    let group = rows.iter().find(|r| r.id == properties::GROUP).unwrap();
    assert_eq!("North", group.value);
    assert_eq!(vec![NO_GROUP.to_owned(), "North".to_owned()], group.candidates);
    assert_eq!("10", rows.iter().find(|r| r.id == properties::QUANTITY).unwrap().value);
    assert_eq!("1", rows.iter().find(|r| r.id == properties::STEP).unwrap().value);
}

#[test]
fn a_flow_and_a_group_have_their_own_rows() {
    let diagram = small();
    let flow = diagram.rows("ore");
    let group = diagram.rows("north");
    assert!(flow.iter().any(|r| r.id == properties::VOLUME && r.value == "6"));
    assert!(flow.iter().any(|r| r.id == properties::PRODUCT && r.value == "Ore"));
    assert_eq!(
        vec![properties::NAME, properties::DESCRIPTION],
        group.iter().map(|r| r.id.as_str()).collect::<Vec<_>>()
    );
}

#[test]
fn choosing_a_group_by_name_writes_its_id_and_none_ungroups() {
    let mut diagram = small();
    diagram.set("shop", properties::GROUP, "North").unwrap();
    diagram.set("mine", properties::GROUP, NO_GROUP).unwrap();
    let model = diagram.model();
    assert_eq!("north", model.node_of("shop").unwrap().group);
    assert_eq!("", model.node_of("mine").unwrap().group);
}

#[test]
fn a_negative_quantity_in_the_grid_is_refused_and_writes_nothing() {
    let mut diagram = small();
    let before = diagram.bytes();
    assert!(diagram.set("mine", properties::QUANTITY, "-1").is_err());
    assert_eq!(before, diagram.bytes());
}

#[test]
fn increase_and_decrease_are_on_a_nodes_menu_with_their_shortcuts_and_step_it() {
    let mut diagram = small();
    let actions = diagram.actions_of(&element("plant"));
    let shortcut = |id: &str| {
        actions
            .iter()
            .find(|a| a.id == id)
            .and_then(|a| a.shortcut.clone())
            .map(|s| s.key)
    };
    assert_eq!(Some("+".to_owned()), shortcut(INCREASE));
    assert_eq!(Some("-".to_owned()), shortcut(DECREASE));

    assert_eq!(ActionOutcome::Done, diagram.execute(&element("plant"), INCREASE));
    assert_eq!(Some(4.5), diagram.model().node_of("plant").unwrap().quantity);
}

#[test]
fn decrease_at_zero_is_offered_but_unavailable_saying_why() {
    // The shop states no quantity.
    let actions = small().actions_of(&element("shop"));
    let decrease = actions.iter().find(|a| a.id == DECREASE).unwrap();
    assert!(!decrease.is_available());
    assert!(decrease.unavailable_reason.contains("zero"));
}

#[test]
fn removing_a_node_with_flows_asks_first_then_removes_its_flows_too() {
    let mut diagram = small();
    let ActionOutcome::Prompt(Prompt::Confirm { message, .. }) = diagram.execute(&element("plant"), REMOVE) else {
        panic!("a confirmation");
    };
    assert!(message.contains("2 flows"), "{message}");
    assert_eq!(ActionOutcome::Done, diagram.answer(&element("plant"), REMOVE, ""));
    assert!(diagram.model().flows.is_empty());
}

#[test]
fn a_drop_and_a_gesture_discover_the_adds_and_the_connect_and_execute_them() {
    let mut diagram = small();
    let adds: Vec<String> = diagram
        .actions_of(&drop_at(300.0, 700.0))
        .into_iter()
        .map(|a| a.id)
        .collect();
    let mut expected: Vec<String> = stages::ALL.iter().map(|s| actions::add_action_id(s)).collect();
    expected.push(ADD_GROUP.to_owned());
    expected.push(ARRANGE.to_owned());
    assert_eq!(expected, adds);

    assert_eq!(
        ActionOutcome::Done,
        diagram.execute(&drop_at(300.0, 700.0), &actions::add_action_id(stages::CONSUMER))
    );
    let gesture = ActionTarget::Connection {
        source: Some("mine".into()),
        target: Some("shop".into()),
    };
    assert_eq!(
        ActionOutcome::Done,
        diagram.execute_minting(&gesture, CONNECT, &["minted-flow"])
    );

    let model = diagram.model();
    assert!(
        model
            .nodes
            .iter()
            .any(|n| n.node_type == stages::CONSUMER && n.id == "minted-1")
    );
    assert!(model.flows.iter().any(|f| f.from == "mine" && f.to == "shop"));
}

#[test]
fn a_group_needs_a_name() {
    assert!(actions::validate(GROUP, " ").is_err());
    assert!(actions::validate(GROUP, "High street").is_ok());
    let mut diagram = small();
    assert!(matches!(
        diagram.execute(&element("shop"), GROUP),
        ActionOutcome::Prompt(_)
    ));
    assert!(matches!(
        diagram.answer(&element("shop"), GROUP, " "),
        ActionOutcome::Refused(r) if r == "A group needs a name."
    ));
    assert_eq!(
        ActionOutcome::Done,
        diagram.answer(&element("shop"), GROUP, "High street")
    );
    assert_eq!("minted-group", diagram.model().node_of("shop").unwrap().group);
}

#[test]
fn a_body_that_is_not_yaml_is_refused_up_front_and_writes_nothing() {
    let broken = "supply-chain: 1\nnodes: []\nflows: \"never closed\n";
    let mut diagram = open_bytes(broken.as_bytes());
    let outcome = diagram.execute(&drop_at(300.0, 700.0), &actions::add_action_id(stages::CONSUMER));
    assert!(
        matches!(&outcome, ActionOutcome::Refused(r) if r.contains("could not be read")),
        "{outcome:?}"
    );
    assert_eq!(broken, diagram.text());
    let document = store::get(diagram.workspace.documents(), &diagram.body).unwrap();
    assert!(
        document
            .unreadable
            .as_deref()
            .is_some_and(|u| u.starts_with("The document could not be read as YAML: "))
    );
}

#[test]
fn the_palette_offers_every_stage_and_a_group_added_by_its_own_action() {
    let mut expected: Vec<String> = stages::ALL.iter().map(|s| actions::add_action_id(s)).collect();
    expected.push(ADD_GROUP.to_owned());
    assert_eq!(
        expected,
        toolbox::items()
            .into_iter()
            .map(|i| i.drop_action_id)
            .collect::<Vec<_>>()
    );
}

#[test]
fn every_add_on_empty_canvas_carries_the_icon_its_palette_entry_has() {
    let adds: Vec<(String, String)> = small()
        .actions_of(&drop_at(300.0, 700.0))
        .into_iter()
        .filter(|a| a.id != ARRANGE)
        .map(|a| (a.id, a.icon))
        .collect();
    let palette: Vec<(String, String)> = toolbox::items()
        .into_iter()
        .map(|i| (i.drop_action_id, i.icon))
        .collect();
    assert_eq!(palette, adds);
    let mut icons: Vec<&String> = palette.iter().map(|(_, icon)| icon).collect();
    icons.sort();
    icons.dedup();
    assert_eq!(palette.len(), icons.len());
}
