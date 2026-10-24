//! A `create` placed beside a sibling (DISL section 9.4, `after` and `before`): the new node goes directly after or
//! before it in its parent's child order, and the change says where (etalii.adp spec 035, research R16).

mod common;

use adp_disl::behavior::{ids, operations};
use adp_disl::{DislChange, DislDiagram};
use common::{attrs, loaded, with};

fn specification() -> adp_disl::DislSpecification {
    loaded(&with(
        r#""metamodel": {
          "types": { "Topic": { "attributes": { "name": { "type": "string" } }, "children": { "allowed": ["Topic"], "ordered": true } } }
        },
        "behavior": {
          "operations": {
            "after": { "label": "After", "for": ["Topic"], "actions": [ { "create": { "type": "'Topic'", "after": "self", "attributes": { "name": "'new'" } } } ] },
            "before": { "label": "Before", "for": ["Topic"], "actions": [ { "create": { "type": "'Topic'", "parent": "self.parent", "before": "self", "attributes": { "name": "'new'" } } } ] },
            "stray": { "label": "Stray", "for": ["Topic"], "actions": [ { "create": { "type": "'Topic'", "parent": "self", "after": "self" } } ] }
          }
        }"#,
    ))
}

fn names(diagram: &DislDiagram, parent: &str) -> Vec<String> {
    diagram
        .element_by_id(parent)
        .unwrap()
        .children()
        .iter()
        .map(|c| c.value_of("name").to_string())
        .collect()
}

#[test]
fn a_create_after_or_before_a_sibling_goes_beside_it() {
    let specification = specification();
    for (operation, sibling, expected, index) in [
        ("after", "a", ["a", "new", "b", "c"], 1),
        ("after", "c", ["a", "b", "c", "new"], 3),
        ("before", "a", ["new", "a", "b", "c"], 0),
        ("before", "c", ["a", "b", "new", "c"], 2),
    ] {
        let diagram = DislDiagram::new(&specification);
        let root = diagram
            .add_node("Topic", "r", attrs([("name", "root".into())]))
            .unwrap();
        for id in ["a", "b", "c"] {
            diagram
                .add_child("Topic", id, attrs([("name", id.into())]), &root, None)
                .unwrap();
        }
        let target = diagram.element_by_id(sibling).unwrap();
        let transaction = operations::run(
            &specification,
            operation,
            &diagram,
            Some(&target),
            &ids::fixed(&["n"]),
            None,
            None,
        );
        assert!(transaction.refusal.is_none(), "{:?}", transaction.refusal);
        assert_eq!(names(&diagram, "r"), expected, "{operation} {sibling}");
        match &transaction.changes[0] {
            DislChange::Create {
                parent_id,
                index: placed,
                ..
            } => {
                assert_eq!(parent_id.as_deref(), Some("r"));
                assert_eq!(*placed, Some(index), "{operation} {sibling}");
            }
            other => panic!("{other:?}"),
        }
    }
}

#[test]
fn a_create_after_a_node_that_is_not_a_sibling_is_refused() {
    let specification = specification();
    let diagram = DislDiagram::new(&specification);
    let root = diagram
        .add_node("Topic", "r", attrs([("name", "root".into())]))
        .unwrap();
    let transaction = operations::run(
        &specification,
        "stray",
        &diagram,
        Some(&root),
        &ids::fixed(&["n"]),
        None,
        None,
    );
    let refusal = transaction.refusal.unwrap_or_default();
    assert!(
        refusal.contains("placed after a node that is not its sibling"),
        "{refusal}"
    );
}
