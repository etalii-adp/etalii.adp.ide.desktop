//! The menus, the property grid and the palette, each editing through the commands, asked of an open diagram's
//! session as the shell asks them. Ported from the standalone host's
//! `EtAlii.Adp.Diagram.Sankey.Tests/SankeyProviders.Tests.cs` at `8b6ccfc`. Not ported:
//! `ADocumentThatCouldNotBeRead_IsRefusedByTheMenusAndTheGrid_AndWritesNothing`, whose store answers every document
//! as unreadable, a stand-in this store has no seam for; `ABodyThatIsNotYaml_IsRefusedUpFront_AndWritesNothing` holds
//! the same refusal on a real file.

mod common;

use std::fs;

use adp_context::{Prompt, PropertyRow};
use adp_diagram::{ActionOutcome, ActionTarget};
use adp_diagram_sankey::actions::{self, *};
use adp_diagram_sankey::store;
use adp_disl::behavior::ids;
use common::{Diagram, fixture, open};

fn small() -> Diagram {
    open(&fixture("lf-line-endings.skv"), "small")
}

fn element(id: &str) -> ActionTarget {
    ActionTarget::Element(id.to_owned())
}

fn rows(diagram: &Diagram, id: &str) -> Vec<PropertyRow> {
    diagram.session.properties(diagram.workspace.documents(), &[id])
}

fn row<'a>(rows: &'a [PropertyRow], id: &str) -> &'a PropertyRow {
    rows.iter().find(|r| r.id == id).unwrap()
}

fn set(diagram: &mut Diagram, id: &str, property: &str, value: &str) -> Result<(), String> {
    diagram
        .session
        .set_property(&mut diagram.workspace, &[id], property, value)
}

fn execute(diagram: &mut Diagram, target: &ActionTarget, action: &str) -> ActionOutcome {
    diagram
        .session
        .execute(&mut diagram.workspace, target, action, &ids::fixed(&["n"]))
}

fn answer(diagram: &mut Diagram, target: &ActionTarget, action: &str, value: &str) -> ActionOutcome {
    diagram.session.answer(&mut diagram.workspace, target, action, value)
}

#[test]
fn a_node_has_its_rows_with_its_value_shown_but_not_editable() {
    let diagram = small();
    let rows = rows(&diagram, "m");
    let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            NAME_PROPERTY,
            DESCRIPTION_PROPERTY,
            COLOR_PROPERTY,
            CUSTOM_COLOR_PROPERTY,
            COLUMN_PROPERTY,
            VALUE_PROPERTY,
            NOTE_PROPERTY,
            FORMAT_PROPERTY
        ]
    );
    let value = row(&rows, VALUE_PROPERTY);
    assert_eq!(value.value, "10");
    assert!(!value.read_only_reason.is_empty());
    let column = row(&rows, COLUMN_PROPERTY);
    assert_eq!(column.value, AUTO_COLUMN);
    assert_eq!(column.candidates, [AUTO_COLUMN, "1", "2", "3", "4"]);
    assert_eq!(row(&rows, COLOR_PROPERTY).value, DEFAULT_COLOR);
}

#[test]
fn a_custom_colour_shows_as_custom_with_its_hex_in_its_own_row() {
    let diagram = small();
    let rows = rows(&diagram, "x");
    let color = row(&rows, COLOR_PROPERTY);
    assert_eq!(color.value, CUSTOM_COLOR);
    assert!(color.candidates.iter().any(|c| c == CUSTOM_COLOR));
    assert_eq!(row(&rows, CUSTOM_COLOR_PROPERTY).value, "#336699");
}

#[test]
fn a_flow_has_its_value_and_step_editable_in_the_grid() {
    let mut diagram = small();
    let rows = rows(&diagram, "m->x");
    set(&mut diagram, "m->x", VALUE_PROPERTY, "8.5").unwrap();
    assert_eq!(row(&rows, VALUE_PROPERTY).value, "7");
    assert_eq!(row(&rows, STEP_PROPERTY).value, "0.5");
    assert_eq!(diagram.folder.model().flow_of("m->x").unwrap().value, Some(8.5));
}

#[test]
fn the_palette_choice_writes_the_word_default_clears_it_and_custom_takes_a_hex() {
    let mut diagram = small();
    set(&mut diagram, "b", COLOR_PROPERTY, "teal").unwrap();
    set(&mut diagram, "a", COLOR_PROPERTY, DEFAULT_COLOR).unwrap();
    set(&mut diagram, "y", CUSTOM_COLOR_PROPERTY, "#aa3300").unwrap();
    assert!(set(&mut diagram, "y", CUSTOM_COLOR_PROPERTY, "#zz").is_err());
    let model = diagram.folder.model();
    let colour = |id: &str| model.node_of(id).unwrap().color.clone();
    assert_eq!(
        (colour("b"), colour("a"), colour("y")),
        ("teal".into(), "".into(), "#aa3300".into())
    );
}

#[test]
fn auto_column_removes_the_stated_column() {
    let mut diagram = small();
    set(&mut diagram, "y", COLUMN_PROPERTY, "4").unwrap();
    let stated = diagram.folder.model().node_of("y").unwrap().column;
    set(&mut diagram, "y", COLUMN_PROPERTY, AUTO_COLUMN).unwrap();
    assert_eq!(stated, Some(4));
    assert_eq!(diagram.folder.model().node_of("y").unwrap().column, None);
}

#[test]
fn increase_and_decrease_are_on_a_flows_menu_with_their_shortcuts_and_step_it() {
    let mut diagram = small();
    let groups = diagram.session.actions(diagram.workspace.documents(), &element("m->x"));
    let increased = execute(&mut diagram, &element("m->x"), INCREASE);
    let actions: Vec<&adp_context::Action> = groups.iter().flat_map(|g| &g.actions).collect();
    let shortcut = |id: &str| {
        actions
            .iter()
            .find(|a| a.id == id)
            .and_then(|a| a.shortcut.as_ref())
            .map(|s| s.key.clone())
    };
    assert_eq!(shortcut(INCREASE).as_deref(), Some("+"));
    assert_eq!(shortcut(DECREASE).as_deref(), Some("-"));
    assert!(actions.iter().any(|a| a.id == THICKER));
    assert_eq!(increased, ActionOutcome::Done);
    assert_eq!(diagram.folder.model().flow_of("m->x").unwrap().value, Some(7.5));
}

#[test]
fn renaming_a_node_asks_for_the_name_then_writes_it() {
    let mut diagram = small();
    let asked = execute(&mut diagram, &element("a"), RENAME);
    let empty = answer(&mut diagram, &element("a"), RENAME, " ");
    let committed = answer(&mut diagram, &element("a"), RENAME, "Origin");
    let ActionOutcome::Prompt(Prompt::Input { initial_value, .. }) = asked else {
        panic!("{asked:?}");
    };
    assert_eq!(initial_value, "Source A");
    assert_eq!(empty, ActionOutcome::Refused(NAME_REQUIRED.to_owned()));
    assert_eq!(committed, ActionOutcome::Done);
    assert_eq!(diagram.folder.model().nodes[0].name, "Origin");
}

#[test]
fn removing_a_node_with_flows_asks_first_then_removes_its_flows_too() {
    let mut diagram = small();
    let asked = execute(&mut diagram, &element("m"), REMOVE);
    let committed = answer(&mut diagram, &element("m"), REMOVE, "");
    let ActionOutcome::Prompt(Prompt::Confirm { message, .. }) = asked else {
        panic!("{asked:?}");
    };
    assert!(message.contains("4 flows"), "{message}");
    assert_eq!(committed, ActionOutcome::Done);
    assert!(diagram.folder.model().flows.is_empty());
}

#[test]
fn a_drop_and_a_gesture_discover_the_add_and_the_connect_and_execute_them() {
    let mut diagram = small();
    let canvas = ActionTarget::Canvas { x: 700.0, y: 900.0 };
    let offered: Vec<String> = diagram
        .session
        .actions(diagram.workspace.documents(), &canvas)
        .iter()
        .flat_map(|g| g.actions.iter().map(|a| a.id.clone()))
        .collect();
    let added = execute(&mut diagram, &canvas, ADD);
    let gesture = ActionTarget::Connection {
        source: Some("a".to_owned()),
        target: Some("x".to_owned()),
    };
    let connected = execute(&mut diagram, &gesture, CONNECT);
    assert_eq!(offered, [ADD, ARRANGE, THICKER, THINNER]);
    assert_eq!(added, ActionOutcome::Done);
    assert_eq!(connected, ActionOutcome::Done);
    let model = diagram.folder.model();
    assert_eq!(
        model.nodes.iter().find(|n| n.name == "New node").unwrap().column,
        Some(3)
    );
    assert!(model.flows.iter().any(|f| f.from == "a" && f.to == "x"));
}

#[test]
fn thicker_and_thinner_rescale_the_whole_diagram() {
    let mut diagram = small();
    let thicker = execute(&mut diagram, &ActionTarget::Canvas { x: 0.0, y: 0.0 }, THICKER);
    assert_eq!(thicker, ActionOutcome::Done);
    assert_eq!(diagram.folder.model().settings.thickness, 1.25);
}

#[test]
fn a_body_that_is_not_yaml_is_refused_up_front_and_writes_nothing() {
    let broken = "sankey: 1\nnodes: []\nflows: \"never closed\n";
    let folder = common::TestFolder::new(&fixture("lf-line-endings.skv"), "small.skv");
    fs::write(&folder.body, broken).unwrap();
    let mut diagram = open(&folder.body, "broken");
    let added = execute(&mut diagram, &ActionTarget::Canvas { x: 0.0, y: 0.0 }, ADD);
    let renamed = set(&mut diagram, "m", NAME_PROPERTY, "Renamed");
    let ActionOutcome::Refused(refusal) = added else {
        panic!("{added:?}");
    };
    assert!(refusal.contains("could not be read"), "{refusal}");
    assert!(renamed.unwrap_err().contains("could not be read"));
    assert_eq!(fs::read_to_string(&diagram.folder.body).unwrap(), broken);
    let document = store::get(diagram.workspace.documents(), &diagram.folder.body).unwrap();
    assert!(
        document
            .unreadable
            .as_deref()
            .unwrap()
            .starts_with("The document could not be read as YAML: ")
    );
}

#[test]
fn the_palette_offers_a_node_added_by_the_add_action() {
    let diagram = small();
    let toolbox = diagram.session.toolbox(diagram.workspace.documents());
    assert_eq!(toolbox.len(), 1);
    assert_eq!(toolbox[0].drop_action_id, actions::ADD);
}

/// The context service's rule: an action not offered, or offered disabled, is not run.
#[test]
fn an_action_not_offered_or_disabled_is_not_run() {
    let mut diagram = small();
    diagram
        .session
        .set_property(&mut diagram.workspace, &["m->y"], VALUE_PROPERTY, "0")
        .unwrap();
    for (target, action) in [(element("m->y"), DECREASE), (element("m"), INCREASE)] {
        assert_eq!(
            execute(&mut diagram, &target, action),
            ActionOutcome::Refused(NOT_AVAILABLE.to_owned()),
            "{action}"
        );
    }
}
