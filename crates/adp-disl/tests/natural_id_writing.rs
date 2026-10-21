//! Writing what `persistence.ids.prefix` under the `natural` strategy read (DISL section 11.5.1): every id is written
//! as the binding read it, the prefix taken off again - the element changed, a relation's ends and an attribute that
//! holds elements - and a relation drawn as an add of its binding type with its ends in the attributes the type map
//! gives them (the standalone host's `NaturalIdWritingTests`).

mod common;

use adp_cel::{Map, Value};
use adp_disl::behavior::write;
use adp_disl::{DislChange, DislDiagram, DislSpecification};
use adp_fbl::ModelChange;
use common::{attrs, loaded, with};

fn specification(ids: &str) -> DislSpecification {
    loaded(&with(&format!(
        r#""metamodel": {{
          "types": {{ "Pump": {{ "attributes": {{ "tag": {{ "type": "string", "key": true }} }} }},
                     "Tank": {{ "attributes": {{ "tag": {{ "type": "string", "key": true }},
                                               "feeds": {{ "type": "Pump", "many": true }},
                                               "spare": {{ "type": "Pump" }} }} }} }},
          "relations": {{ "Pipe": {{ "source": ["Pump", "Tank"], "target": ["Pump", "Tank"],
                                   "attributes": {{ "bore": {{ "type": "int" }} }} }},
                         "Hose": {{ "source": ["Pump"], "target": ["Pump"] }} }} }},
        "persistence": {{
          "ids": {ids},
          "typeMap": {{ "Line": {{ "as": "Pipe", "attributes": {{ "from": "source", "to": "target" }} }} }} }}"#
    )))
}

fn prefixed() -> DislSpecification {
    specification(r#"{ "strategy": "natural", "prefix": { "Pump": "pump:", "Pipe": "pipe:", "Hose": "hose:" } }"#)
}

#[test]
fn a_set_and_a_remove_are_written_under_the_id_the_element_was_read_with() {
    let specification = prefixed();

    let set = write::to_fbl(
        &specification,
        &DislChange::Set {
            element_id: "pump:p1".into(),
            type_name: "Pump".into(),
            attributes: attrs([("tag", "p9".into())]),
        },
    );
    let removed = write::to_fbl(
        &specification,
        &DislChange::Remove {
            element_id: "pipe:p1|p2".into(),
        },
    );

    assert!(matches!(set, ModelChange::Set { id, .. } if id == "p1"));
    assert!(matches!(removed, ModelChange::Remove { id } if id == "p1|p2"));
}

#[test]
fn an_attribute_that_holds_elements_is_written_by_the_ids_they_were_read_with() {
    // Arrange: an action's create stores an element as its id; a value set directly may be the element.
    let specification = prefixed();
    let diagram = DislDiagram::new(&specification);
    let pump = diagram
        .add_node("Pump", "pump:p2", attrs([("tag", "p2".into())]))
        .unwrap();

    // Act.
    let created = write::to_fbl(
        &specification,
        &DislChange::Create {
            type_name: "Tank".into(),
            id: "t1".into(),
            attributes: attrs([
                ("tag", "t1".into()),
                ("feeds", Value::list(vec!["pump:p1".into(), pump.to_value()])),
            ]),
            parent_id: None,
            at: Value::Null,
        },
    );
    let set = write::to_fbl(
        &specification,
        &DislChange::Set {
            element_id: "t1".into(),
            type_name: "Tank".into(),
            attributes: attrs([("spare", pump.to_value())]),
        },
    );

    // Assert.
    let ModelChange::Add { attributes, .. } = created else {
        panic!("not an add");
    };
    assert_eq!(
        Some(&Value::list(vec!["p1".into(), "p2".into()])),
        attributes.get("feeds")
    );
    let ModelChange::Set { attributes, .. } = set else {
        panic!("not a set");
    };
    assert_eq!(Some(&Value::from("p2")), attributes.get("spare"));
}

#[test]
fn a_relation_drawn_is_an_add_of_its_binding_type_with_its_ends_in_the_attributes_the_type_map_gives_them() {
    let specification = prefixed();
    let connect = |type_name: &str, id: &str, attributes: Map| {
        write::to_fbl(
            &specification,
            &DislChange::Connect {
                type_name: type_name.into(),
                id: id.into(),
                source_id: "pump:p1".into(),
                target_id: "pump:p2".into(),
                attributes,
            },
        )
    };

    let ModelChange::Add {
        type_name,
        id,
        attributes,
        ..
    } = connect("Pipe", "pipe:p1|p2", attrs([("bore", Value::Int(4))]))
    else {
        panic!("not an add");
    };
    let ModelChange::Add {
        type_name: plain_type,
        attributes: plain,
        ..
    } = connect("Hose", "hose:p1|p2", Map::default())
    else {
        panic!("not an add");
    };

    assert_eq!(("Line", Some("p1|p2")), (type_name.as_str(), id.as_deref()));
    let mut written: Vec<(String, Value)> = attributes.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    written.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        vec![
            ("bore".to_owned(), Value::Int(4)),
            ("from".to_owned(), "p1".into()),
            ("to".to_owned(), "p2".into())
        ],
        written
    );
    assert_eq!(
        ("Hose", Some(&Value::from("p1"))),
        (plain_type.as_str(), plain.get("source"))
    );
    assert_eq!(Some(&Value::from("p2")), plain.get("target"));
}

#[test]
fn a_type_without_a_prefix_and_another_strategy_keep_their_ids() {
    let minted = specification(r#"{ "strategy": "uuid-v4", "prefix": { "Pump": "pump:" } }"#);

    let tank = write::to_fbl(
        &prefixed(),
        &DislChange::Remove {
            element_id: "t1".into(),
        },
    );
    let pump = write::to_fbl(
        &minted,
        &DislChange::Remove {
            element_id: "pump:p1".into(),
        },
    );

    assert!(matches!(tank, ModelChange::Remove { id } if id == "t1"));
    assert!(matches!(pump, ModelChange::Remove { id } if id == "pump:p1"));
}
