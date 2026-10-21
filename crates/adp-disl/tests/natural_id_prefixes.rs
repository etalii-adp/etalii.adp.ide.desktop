//! `persistence.ids.prefix` under the `natural` strategy (DISL sections 11.5 and 11.5.1): the id of an element of a
//! type that declares a prefix is that prefix followed by its natural id - for a node the id it was read with, for a
//! relation read without one its ends as read, joined by `|` - and a type that declares none keeps the id it was read
//! with (the standalone host's `NaturalIdPrefixesTests`).

mod common;

use adp_cel::{Map, Value};
use adp_disl::model::builder;
use adp_disl::{DislDiagram, DislElement, DislSpecification};
use adp_fbl::text::Span;
use adp_fbl::{FblElement, FblModel};
use common::{attrs, loaded, with};

fn specification(ids: &str) -> DislSpecification {
    loaded(&with(&format!(
        r#""metamodel": {{
          "types": {{ "Pump": {{ "attributes": {{ "tag": {{ "type": "string", "key": true }} }} }},
                     "Tank": {{ "attributes": {{ "tag": {{ "type": "string", "key": true }},
                                               "feeds": {{ "type": "Pump", "many": true }},
                                               "spare": {{ "type": "Pump" }} }} }} }},
          "relations": {{ "Pipe": {{ "source": ["Pump", "Tank"], "target": ["Pump", "Tank"] }} }} }},
        "persistence": {{
          "ids": {ids},
          "typeMap": {{ "Line": {{ "as": "Pipe", "attributes": {{ "from": "source", "to": "target" }} }} }} }}"#
    )))
}

fn prefixed() -> DislSpecification {
    specification(r#"{ "strategy": "natural", "prefix": { "Pump": "pump:", "Pipe": "pipe:" } }"#)
}

#[allow(clippy::too_many_arguments)]
fn fbl(
    id: &str,
    stored: bool,
    type_name: &str,
    rule: &str,
    relation: bool,
    attributes: Map,
    source: Option<&str>,
    target: Option<&str>,
    line: usize,
) -> FblElement {
    FblElement {
        id: id.to_owned(),
        id_is_stored: stored,
        type_name: type_name.to_owned(),
        rule: rule.to_owned(),
        is_relation: relation,
        attributes,
        parent_id: None,
        parent_slot: None,
        source: source.map(str::to_owned),
        target: target.map(str::to_owned),
        own_span: Span::new(0, 0),
        line,
        file: None,
    }
}

fn node(id: &str, type_name: &str, line: usize) -> FblElement {
    fbl(
        id,
        true,
        type_name,
        &type_name.to_lowercase(),
        false,
        attrs([("tag", id.into())]),
        None,
        None,
        line,
    )
}

/// A pipe read as an entry whose reference attributes name its ends, as a lines binding reads one.
fn line_of(from: &str, to: &str, line: usize) -> FblElement {
    fbl(
        &format!("line@{line}"),
        false,
        "Line",
        "line",
        false,
        attrs([("from", from.into()), ("to", to.into())]),
        None,
        None,
        line,
    )
}

/// A pipe read as a relation with its ends, as a binding with a relation rule reads one.
fn pipe(source: &str, target: &str, line: usize) -> FblElement {
    fbl(
        &format!("pipe@{line}"),
        false,
        "Pipe",
        "pipe",
        true,
        Map::default(),
        Some(source),
        Some(target),
        line,
    )
}

/// A tank read with the pumps it references, by the ids they were read with, as a binding reads a reference.
fn tank(id: &str, line: usize, spare: Option<&str>, feeds: &[&str]) -> FblElement {
    let mut attributes = attrs([
        ("tag", id.into()),
        ("feeds", Value::list(feeds.iter().map(|f| Value::from(*f)).collect())),
    ]);
    if let Some(spare) = spare {
        attributes.insert("spare", spare.into());
    }
    fbl(id, true, "Tank", "tank", false, attributes, None, None, line)
}

fn read(specification: &DislSpecification, elements: Vec<FblElement>) -> DislDiagram {
    builder::from(
        &FblModel {
            elements,
            ..Default::default()
        },
        specification,
    )
    .diagram
}

fn ids(elements: &[DislElement]) -> Vec<String> {
    elements.iter().map(DislElement::id).collect()
}

#[test]
fn an_element_of_a_type_with_a_prefix_is_the_prefix_followed_by_its_natural_id() {
    let diagram = read(&prefixed(), vec![node("p1", "Pump", 1), node("p2", "Pump", 2)]);

    assert_eq!(vec!["pump:p1", "pump:p2"], ids(&diagram.nodes()));
}

#[test]
fn a_relation_read_without_an_id_is_the_prefix_followed_by_its_ends_joined_by_a_bar() {
    let diagram = read(
        &prefixed(),
        vec![
            node("p1", "Pump", 1),
            node("p2", "Pump", 2),
            line_of("p1", "p2", 3),
            pipe("p2", "p1", 4),
        ],
    );

    let relations = diagram.relations();
    assert_eq!(vec!["pipe:p1|p2", "pipe:p2|p1"], ids(&relations));
    assert_eq!(
        vec!["pump:p1", "pump:p2"],
        relations.iter().map(|r| r.source().unwrap().id()).collect::<Vec<_>>()
    );
    assert_eq!(
        vec!["pump:p2", "pump:p1"],
        relations.iter().map(|r| r.target().unwrap().id()).collect::<Vec<_>>()
    );
}

#[test]
fn a_type_without_a_prefix_keeps_its_id() {
    let diagram = read(
        &prefixed(),
        vec![node("p1", "Pump", 1), node("t1", "Tank", 2), line_of("p1", "t1", 3)],
    );

    assert_eq!(vec!["pump:p1", "t1"], ids(&diagram.nodes()));
    let relations = diagram.relations();
    assert_eq!(
        ("pipe:p1|t1".to_owned(), "t1".to_owned()),
        (relations[0].id(), relations[0].target().unwrap().id())
    );
}

#[test]
fn an_end_that_names_nothing_is_still_part_of_the_id_and_stays_as_read() {
    let diagram = read(&prefixed(), vec![node("p1", "Pump", 1), line_of("p1", "ghost", 2)]);

    let relations = diagram.relations();
    assert_eq!(1, relations.len());
    assert_eq!(
        (
            "pipe:p1|ghost".to_owned(),
            Some("p1".to_owned()),
            Some("ghost".to_owned())
        ),
        (relations[0].id(), relations[0].source_id(), relations[0].target_id())
    );
    assert!(relations[0].target().is_none());
}

#[test]
fn two_nodes_read_with_one_id_both_get_the_prefix_and_a_relation_finds_the_first() {
    // A duplicated name is kept, not refused (DISL 11.5.4); the first in reading order keeps it.
    let diagram = read(
        &prefixed(),
        vec![node("p1", "Pump", 1), node("p1", "Pump", 2), line_of("p1", "p1", 3)],
    );

    let nodes = diagram.nodes();
    assert_eq!(vec!["pump:p1", "pump:p1"], ids(&nodes));
    let relations = diagram.relations();
    assert_eq!(1, relations.len());
    assert_eq!("pipe:p1|p1", relations[0].id());
    assert_eq!(Some(nodes[0].clone()), relations[0].source());
    assert_eq!(Some(nodes[0].clone()), relations[0].target());
}

#[test]
fn one_prefix_for_every_type_applies_to_each_of_them() {
    let specification = specification(r#"{ "strategy": "natural", "prefix": "id_" }"#);

    let diagram = read(
        &specification,
        vec![node("p1", "Pump", 1), node("t1", "Tank", 2), line_of("p1", "t1", 3)],
    );

    assert_eq!(vec!["id_p1", "id_t1", "id_p1|t1"], ids(&diagram.elements()));
}

#[test]
fn a_prefix_under_another_strategy_leaves_the_ids_read() {
    // A uuid strategy's prefix is for the ids the runtime mints, not for those it reads.
    let specification = specification(r#"{ "strategy": "uuid-v4", "prefix": { "Pump": "pump:" } }"#);

    let diagram = read(&specification, vec![node("p1", "Pump", 1)]);

    assert_eq!(vec!["p1"], ids(&diagram.nodes()));
}

#[test]
fn a_reference_read_by_the_id_its_target_was_read_with_resolves_to_that_element() {
    // The pumps' ids gain their prefix; the tank still names them as they were read.
    let diagram = read(
        &prefixed(),
        vec![
            node("p1", "Pump", 1),
            node("p2", "Pump", 2),
            tank("t1", 3, Some("p2"), &["p1", "p2"]),
        ],
    );

    let tank = diagram.nodes_of_type("Tank").remove(0);
    let feeds = tank.value_of("feeds");
    assert_eq!(
        vec!["pump:p1", "pump:p2"],
        feeds
            .as_list()
            .unwrap()
            .iter()
            .map(|f| DislElement::from_value(f).unwrap().id())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        Some(diagram.nodes()[1].clone()),
        DislElement::from_value(&tank.value_of("spare"))
    );
}

#[test]
fn a_reference_that_names_nothing_stays_as_read_and_resolves_to_null() {
    let diagram = read(
        &prefixed(),
        vec![node("p1", "Pump", 1), tank("t1", 2, Some("ghost"), &["p1", "ghost"])],
    );

    let tank = diagram.nodes_of_type("Tank").remove(0);
    let feeds = tank.value_of("feeds");
    let feeds = feeds.as_list().unwrap();
    assert_eq!(Some(diagram.nodes()[0].clone()), DislElement::from_value(&feeds[0]));
    assert!(feeds[1].is_null());
    assert!(tank.value_of("spare").is_null());
    assert_eq!(Some(&Value::from("ghost")), tank.attributes().get("spare"));
}

#[test]
fn a_reference_to_one_of_two_nodes_read_with_one_id_resolves_to_the_first() {
    let diagram = read(
        &prefixed(),
        vec![
            node("p1", "Pump", 1),
            node("p1", "Pump", 2),
            tank("t1", 3, Some("p1"), &[]),
        ],
    );

    assert_eq!(
        Some(diagram.nodes()[0].clone()),
        DislElement::from_value(&diagram.nodes_of_type("Tank")[0].value_of("spare"))
    );
}
