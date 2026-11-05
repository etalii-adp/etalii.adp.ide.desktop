//! The changes the model makes itself forget the groupings made of what they change (DISL 0.9, section 12.4; spec
//! 075, T016): a node removed, a node moved, a node retyped, a relation's end resolved and a diagram attribute set
//! each make a grouping of the typed lists again, which the public operations cannot all reach.

use std::collections::HashMap;

use adp_cel::{Map, Value, groupings_made};

use crate::model::diagram::DislDiagram;
use crate::model::element::DislElement;
use crate::{DislLoader, DislSpecification, contexts};

fn specification() -> DislSpecification {
    DislLoader::load(
        r#"{
          "disl": "0.9",
          "language": { "id": "org.example.grouping", "version": "1.0.0" },
          "metamodel": {
            "diagram": { "attributes": { "title": { "type": "string" } } },
            "types": {
              "Thing": { "attributes": { "family": { "type": "string" } }, "children": { "allowed": ["Thing"] } },
              "Gadget": { "attributes": { "family": { "type": "string" } } } },
            "relations": { "Link": { "source": "Thing", "target": "Thing" } } } }"#,
    )
    .specification
    .expect("the specification loads")
}

fn family(value: &str) -> Map {
    [("family", Value::from(value))].into_iter().collect()
}

/// Three things, a and b of family x and c of y, and a link from a with no target yet.
fn diagram() -> (DislDiagram, Vec<DislElement>) {
    let diagram = DislDiagram::new(&specification());
    let things: Vec<DislElement> = [("a", "x"), ("b", "x"), ("c", "y")]
        .into_iter()
        .map(|(id, f)| diagram.add_node("Thing", id, family(f)).unwrap())
        .collect();
    diagram
        .add_relation("Link", "l", Some(&things[0]), None, Map::default())
        .unwrap();
    (diagram, things)
}

/// How many groupings evaluating the grouping of things and of links made.
fn made(diagram: &DislDiagram) -> u64 {
    let specification = diagram.specification();
    let program = specification
        .environment(contexts::ELEMENT, &[])
        .compile(
            "diagram.nodesOfType('Thing').groupBy(t, t.family).keys().size() + \
             diagram.relationsOfType('Link').groupBy(r, r.type).keys().size()",
        )
        .unwrap();
    let variables: HashMap<String, Value> = [("diagram".to_owned(), diagram.to_value())].into_iter().collect();
    let before = groupings_made();
    program.evaluate(&variables).unwrap();
    groupings_made() - before
}

#[test]
fn unchanged_lists_keep_their_groupings() {
    let (diagram, _) = diagram();
    assert_eq!(made(&diagram), 2);
    assert_eq!(made(&diagram), 0);
}

#[test]
fn a_node_removed_makes_its_list_grouped_again() {
    let (diagram, things) = diagram();
    made(&diagram);
    diagram.remove(&things[2]);
    assert_eq!(made(&diagram), 1, "only the list of things changed");
}

#[test]
fn a_node_moved_makes_the_groupings_again() {
    let (diagram, things) = diagram();
    made(&diagram);
    diagram.reparent(&things[2], Some(&things[0]), None, -1).unwrap();
    assert_eq!(made(&diagram), 2);
}

#[test]
fn a_node_retyped_makes_the_groupings_again() {
    let (diagram, things) = diagram();
    made(&diagram);
    let gadget = diagram.specification().metamodel().type_of("Gadget").cloned().unwrap();
    things[1].retype(gadget, &family("x"));
    assert_eq!(made(&diagram), 2);
}

#[test]
fn an_end_resolved_makes_the_groupings_again() {
    let (diagram, things) = diagram();
    made(&diagram);
    let link = diagram.element_by_id("l").unwrap();
    diagram.resolve_ends(&link, None, Some(things[1].clone()));
    assert_eq!(made(&diagram), 2);
}

#[test]
fn a_diagram_attribute_set_makes_the_groupings_again() {
    let (diagram, _) = diagram();
    made(&diagram);
    diagram.set_attribute("title", Value::from("t"));
    assert_eq!(made(&diagram), 2);
}
