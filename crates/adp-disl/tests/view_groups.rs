//! `self.view.groups` (DISL 0.11, section 6.9): a map from each group key of a node's compartments to whether that
//! group is folded, its stored state where the host keeps one, else its default; a compartment with `groupBy` gives
//! a key per value of its enumeration, a collapsible one without gives its own id, and a node without a collapsible
//! compartment has an empty map.

mod common;

use adp_cel::{Map, Value};
use adp_disl::expressions::contexts;
use adp_disl::{DislDiagram, DislElement, DislSpecification};
use common::{attrs, loaded, with};

fn specification() -> DislSpecification {
    loaded(&with(
        r#""disl": "0.11",
        "metamodel": {
          "enums": { "Status": { "values": { "open": {}, "waiting": {}, "done": {} } } },
          "types": {
            "Card": { "attributes": { "name": { "type": "string" } },
              "children": { "tasks": { "types": ["Task"], "many": true }, "notes": { "types": ["Note"], "many": true } } },
            "Task": { "attributes": { "status": { "type": "Status" } } },
            "Note": { "attributes": { "text": { "type": "string" } } },
            "Chip": { "attributes": { "name": { "type": "string" } } } } },
        "notation": { "nodes": {
          "Card": { "shape": "rect", "compartments": [
            { "id": "tasks", "items": { "children": ["Task"], "slot": "tasks" }, "itemText": { "cel": "item.status" },
              "groupBy": { "attribute": "status", "collapsed": { "waiting": true, "done": true } } },
            { "id": "notes", "items": { "children": ["Note"], "slot": "notes" }, "itemText": { "cel": "item.text" },
              "collapsible": true, "collapsed": true } ] },
          "Chip": { "shape": "rect" } } }"#,
    ))
}

fn evaluate(specification: &DislSpecification, element: &DislElement, expression: &str) -> Result<Value, String> {
    let variables: std::collections::HashMap<String, Value> = [
        ("self".to_owned(), element.to_value()),
        ("diagram".to_owned(), element.diagram().to_value()),
        ("env".to_owned(), Value::map(Map::default())),
    ]
    .into_iter()
    .collect();
    specification
        .environment(contexts::ELEMENT, &[])
        .compile(expression)?
        .evaluate(&variables)
}

fn groups(specification: &DislSpecification, element: &DislElement) -> Vec<(String, bool)> {
    let Ok(Value::Map(map)) = evaluate(specification, element, "self.view.groups") else {
        panic!("self.view.groups is a map");
    };
    let mut groups: Vec<(String, bool)> = map
        .iter()
        .map(|(k, v)| (k.to_string(), v == &Value::Bool(true)))
        .collect();
    groups.sort();
    groups
}

#[test]
fn a_group_nobody_folded_reads_its_default() {
    let specification = specification();
    let diagram = DislDiagram::new(&specification);
    let card = diagram.add_node("Card", "c1", attrs([("name", "a".into())])).unwrap();
    assert_eq!(
        vec![
            ("done".to_owned(), true),
            ("notes".to_owned(), true),
            ("open".to_owned(), false),
            ("waiting".to_owned(), true)
        ],
        groups(&specification, &card)
    );
    assert_eq!(
        Ok(Value::Bool(true)),
        evaluate(&specification, &card, "self.view.groups['waiting']")
    );
}

#[test]
fn a_stored_state_wins_over_the_default() {
    let specification = specification();
    let diagram = DislDiagram::new(&specification);
    let card = diagram.add_node("Card", "c1", attrs([("name", "a".into())])).unwrap();
    let stored: Map = [
        ("waiting", Value::Bool(false)),
        ("open", Value::Bool(true)),
        ("gone", Value::Bool(true)),
    ]
    .into_iter()
    .collect();
    let mut view = Map::default();
    view.insert("pinned".to_owned(), Value::Bool(true));
    view.insert("groups".to_owned(), Value::map(stored));
    card.set_view(Some(view));
    assert_eq!(
        vec![
            ("done".to_owned(), true),
            ("notes".to_owned(), true),
            ("open".to_owned(), true),
            ("waiting".to_owned(), false)
        ],
        groups(&specification, &card)
    );
    assert_eq!(
        Ok(Value::Bool(true)),
        evaluate(&specification, &card, "self.view.pinned")
    );
}

#[test]
fn a_node_without_a_collapsible_compartment_has_an_empty_map() {
    let specification = specification();
    let diagram = DislDiagram::new(&specification);
    let chip = diagram.add_node("Chip", "x1", attrs([("name", "a".into())])).unwrap();
    chip.set_view(Some(Map::default()));
    assert!(groups(&specification, &chip).is_empty());
}
