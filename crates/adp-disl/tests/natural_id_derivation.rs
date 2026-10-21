//! The `natural` strategy as a binding's reader asks for it (DISL section 11.5.1): an entry whose rule stores no id
//! is read with the string forms of its type's `key` attributes, joined by `|`, under the binding's names for them;
//! the prefix is the model builder's (the standalone host's `NaturalIdDerivationTests`).

mod common;

use std::rc::Rc;

use adp_cel::{Map, Value};
use adp_disl::DislSpecification;
use adp_disl::model::{builder, natural_ids};
use adp_fbl::documents::ProblemSeverity;
use adp_fbl::{FblOptions, IdRequest, OpenBody};
use common::{loaded, with};

fn specification(strategy: &str) -> DislSpecification {
    loaded(&with(&format!(
        r#""metamodel": {{
          "types": {{ "Pump": {{ "attributes": {{ "tag": {{ "type": "string", "key": true }}, "label": {{ "type": "string" }} }} }},
                     "Valve": {{ "attributes": {{ "site": {{ "type": "string", "key": true }}, "number": {{ "type": "int", "key": true }} }} }},
                     "Note": {{ "attributes": {{ "text": {{ "type": "string" }} }} }} }},
          "relations": {{ "Pipe": {{ "source": ["Pump"], "target": ["Pump"],
                                   "attributes": {{ "fromTag": {{ "type": "string", "key": true }}, "toTag": {{ "type": "string", "key": true }} }} }} }} }},
        "persistence": {{
          "ids": {{ "strategy": "{strategy}", "prefix": {{ "Pump": "pump:", "Pipe": "pipe:" }} }},
          "typeMap": {{ "Line": {{ "as": "Pipe", "attributes": {{ "from": "source", "to": "target", "a": "fromTag", "b": "toTag" }} }} }} }}"#
    )))
}

fn natural(type_name: &str, attributes: &[(&str, Value)], strategy: &str) -> Option<String> {
    let attributes: Map = attributes.iter().cloned().collect();
    let rule = type_name.to_lowercase();
    let request = IdRequest {
        rule: &rule,
        type_name,
        attributes: &attributes,
        source: None,
        target: None,
        line: 1,
        parent_id: None,
        parent_slot: None,
        position_in_slot: 0,
    };
    natural_ids::of(&specification(strategy))(&request)
}

#[test]
fn a_node_is_read_with_its_key_attribute_without_the_prefix() {
    assert_eq!(
        Some("p1".to_owned()),
        natural("Pump", &[("tag", "p1".into()), ("label", "Pump one".into())], "natural")
    );
}

#[test]
fn several_key_attributes_are_joined_by_a_bar_in_declaration_order() {
    // A number is written in its invariant form.
    assert_eq!(
        Some("north|7".to_owned()),
        natural(
            "Valve",
            &[("number", Value::Int(7)), ("site", "north".into())],
            "natural"
        )
    );
}

#[test]
fn a_binding_type_is_mapped_by_the_type_map_and_its_keys_found_under_the_bindings_names() {
    assert_eq!(
        Some("p1|p2".to_owned()),
        natural(
            "Line",
            &[
                ("from", "p1".into()),
                ("to", "p2".into()),
                ("a", "p1".into()),
                ("b", "p2".into())
            ],
            "natural"
        )
    );
}

#[test]
fn no_id_is_derived_without_a_key_value_without_a_key_attribute_for_an_unknown_type_or_under_another_strategy() {
    assert_eq!(None, natural("Pump", &[("label", "No tag".into())], "natural"));
    assert_eq!(None, natural("Pump", &[("tag", "".into())], "natural"));
    assert_eq!(None, natural("Note", &[("text", "Hello".into())], "natural"));
    assert_eq!(None, natural("Gauge", &[("tag", "g1".into())], "natural"));
    assert_eq!(None, natural("Pump", &[("tag", "p1".into())], "uuid-v4"));
}

#[test]
fn a_body_read_with_the_derivation_gives_each_element_its_prefix_followed_by_its_key_and_references_resolve() {
    // Arrange: a lines binding whose rules store no id, as a binding that leaves ids to DISL is.
    let specification = specification("natural");
    let (problems, document) = adp_fbl::documents::load(
        br#"{
          "fbl": "0.2",
          "bindings": {
            "plant": {
              "title": "Plant",
              "claims": { "extensions": [".plant"] },
              "body": { "kind": "file", "family": "lines" },
              "reader": "declared",
              "elements": [
                { "name": "pump", "type": "Pump", "line": "^pump (?<tag>\\S+)$",
                  "attributes": { "tag": { "group": "tag" } } },
                { "name": "line", "type": "Line", "line": "^line (?<from>\\S+) (?<to>\\S+)$",
                  "attributes": { "from": { "group": "from", "word": "^.+$", "reference": { "to": ["pump"], "by": "tag" } },
                                  "to": { "group": "to", "word": "^.+$", "reference": { "to": ["pump"], "by": "tag" } },
                                  "a": { "value": "groups.from" }, "b": { "value": "groups.to" } } }
              ]
            }
          }
        }"#,
    );
    let document = document.expect("the binding loads");
    assert!(
        !problems.iter().any(|p| p.severity == ProblemSeverity::Error),
        "{problems:?}"
    );
    let binding = Rc::new(document.binding("plant").unwrap().clone());
    let options = FblOptions {
        derive_id: Some(natural_ids::of(&specification)),
        ..Default::default()
    };

    // Act.
    let reading = OpenBody::open(b"pump p1\npump p2\nline p1 p2\n".to_vec(), binding, Rc::new(options)).model();
    let diagram = builder::from(&reading, &specification).diagram;

    // Assert.
    assert_eq!(
        vec!["pump:p1", "pump:p2", "pipe:p1|p2"],
        diagram.elements().iter().map(|e| e.id()).collect::<Vec<_>>()
    );
    let relations = diagram.relations();
    assert_eq!(1, relations.len());
    assert_eq!(
        (Some("pump:p1".to_owned()), Some("pump:p2".to_owned())),
        (
            relations[0].source().map(|s| s.id()),
            relations[0].target().map(|t| t.id())
        )
    );
}
