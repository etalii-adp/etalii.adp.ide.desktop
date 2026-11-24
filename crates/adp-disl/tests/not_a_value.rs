//! A row of an attribute of an enumeration type (DISL 0.11, sections 7.5 and 9.1): a value that is no value of the
//! enumeration is refused, with `std.notAValue` where the specification words it, the text given bound as `value`,
//! whether the row's form is declared or generated; a value of it, by its key or its stored form, is written.

mod common;

use adp_cel::Value;
use adp_disl::derivation::form_commit;
use adp_disl::{DislChange, DislDiagram, DislEnv, DislSpecification};
use common::{attrs, loaded, with};

fn specification(worded: bool) -> DislSpecification {
    let messages = if worded {
        r#", "behavior": { "messages": { "std.notAValue": { "cel": "'\\'' + value + '\\' is no status.'" } } }"#
    } else {
        ""
    };
    loaded(&with(&format!(
        r#""disl": "0.11",
        "metamodel": {{
          "enums": {{ "Status": {{ "values": {{ "open": {{ "value": "Open" }}, "done": {{ "value": "Done" }} }} }} }},
          "types": {{
            "Card": {{ "attributes": {{ "status": {{ "type": "Status" }} }} }},
            "Chip": {{ "attributes": {{ "status": {{ "type": "Status" }} }} }} }} }},
        "notation": {{ "nodes": {{ "Card": {{ "shape": "rect" }}, "Chip": {{ "shape": "rect" }} }} }},
        "forms": {{ "card": {{ "for": "Card", "items": [ {{ "attribute": "status", "label": "Status" }} ] }} }}{messages}"#
    )))
}

/// What committing `value` to the status row of a node of `type_name` does: its refusal, or the status it writes.
fn commit(specification: &DislSpecification, type_name: &str, value: &str) -> Result<Value, String> {
    let diagram = DislDiagram::new(specification);
    let node = diagram
        .add_node(type_name, "n1", attrs([("status", "open".into())]))
        .unwrap();
    let transaction = form_commit::row(specification, &node, "status", value, &DislEnv::default(), "inspector");
    if let Some(refusal) = transaction.refusal {
        return Err(refusal);
    }
    match transaction.changes.as_slice() {
        [DislChange::Set { attributes, .. }] => Ok(attributes.get("status").cloned().unwrap_or(Value::Null)),
        other => panic!("not one set: {other:?}"),
    }
}

#[test]
fn a_value_that_is_none_of_the_enumerations_is_refused_in_the_specifications_words() {
    let specification = specification(true);
    for type_name in ["Card", "Chip"] {
        assert_eq!(
            Err("'stuck' is no status.".to_owned()),
            commit(&specification, type_name, "stuck"),
            "{type_name}"
        );
    }
}

#[test]
fn without_the_message_the_runtime_words_the_refusal() {
    let specification = specification(false);
    for type_name in ["Card", "Chip"] {
        assert_eq!(
            Err("'stuck' is not a value of status.".to_owned()),
            commit(&specification, type_name, "stuck"),
            "{type_name}"
        );
    }
}

#[test]
fn a_value_of_the_enumeration_is_written() {
    let specification = specification(true);
    for type_name in ["Card", "Chip"] {
        assert!(commit(&specification, type_name, "done").is_ok(), "{type_name}");
        assert!(commit(&specification, type_name, "Done").is_ok(), "{type_name}");
        assert!(commit(&specification, type_name, "").is_ok(), "{type_name}");
    }
}
