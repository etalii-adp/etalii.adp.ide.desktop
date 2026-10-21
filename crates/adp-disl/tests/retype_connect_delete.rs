//! A node's type changed through `behavior.retype`, a line of a derived relation drawn through its `edits.connect`
//! operation, and a deletion that leaves derived relations and nested children to the removal that takes them (the
//! standalone host's `RetypeConnectDeleteTests`).

mod common;

use std::sync::LazyLock;

use adp_cel::{Map, Value};
use adp_disl::behavior::{deletion, ids, operations, retype, write};
use adp_disl::model::builder;
use adp_disl::{DislChange, DislDiagram, DislSpecification};
use adp_fbl::ModelChange;
use common::{attrs, loaded, with};

static JSON: LazyLock<String> = LazyLock::new(|| {
    with(
        r#""metamodel": {
          "types": {
            "Box": { "attributes": { "name": { "type": "string" } }, "children": { "allowed": ["Box"] } },
            "Bin": { "extends": ["Box"], "attributes": { "size": { "type": "int" } } },
            "Bag": { "extends": ["Box"] },
            "Lid": { "attributes": { "name": { "type": "string" } } }
          },
          "relations": {
            "Holds": { "source": "Box", "target": "Box", "derived": {
              "from": "diagram.nodes.filter(n, n.parent != null)", "id": "'holds:' + item.id", "source": "item.parent", "target": "item",
              "edits": { "connect": "putUnder" } } },
            "Sees": { "source": "Box", "target": "Box", "derived": {
              "from": "diagram.nodes.filter(n, n.parent != null)", "id": "'sees:' + item.id", "source": "item.parent", "target": "item",
              "reason": "Seeing follows from holding." } }
          }
        },
        "behavior": {
          "operations": {
            "putUnder": { "label": "Put under", "for": ["Holds"], "actions": [
              { "if": "self.source.id == self.target.id", "then": [ { "abort": { "message": "'Not under itself.'" } } ] },
              { "reparent": { "target": "self.target", "parent": "self.source" } } ] }
          },
          "deletion": { "Box": { "children": "delete", "relations": "delete" } },
          "retype": {
            "Box": { "to": ["Bin"], "attributeMapping": { "size": "has(old.size) ? old.size + 1 : 3", "missing": "1 / 0" } },
            "Bin": { "to": ["Box", "Bag"], "attributeMapping": { "size": "old.size * 2" } }
          }
        }"#,
    )
});

fn specification() -> DislSpecification {
    loaded(&JSON)
}

/// p (p1, p2 (p2a)) and q, as a reader completes them, the derived lines computed.
fn tree() -> DislDiagram {
    let diagram = DislDiagram::new(&specification());
    let p = diagram.add_node("Box", "p", Map::default()).unwrap();
    diagram
        .add_child("Box", "p1", Map::default(), &p, Some("children"))
        .unwrap();
    let p2 = diagram
        .add_child("Bin", "p2", attrs([("size", Value::Int(5))]), &p, Some("children"))
        .unwrap();
    diagram
        .add_child("Box", "p2a", Map::default(), &p2, Some("children"))
        .unwrap();
    diagram.add_node("Box", "q", Map::default()).unwrap();
    builder::complete(&diagram);
    diagram
}

fn retype_attributes(change: &DislChange) -> (String, String, Map) {
    match change {
        DislChange::Retype {
            element_id,
            type_name,
            attributes,
        } => (element_id.clone(), type_name.clone(), attributes.clone()),
        other => panic!("not a retype: {other:?}"),
    }
}

#[test]
fn a_retype_carries_the_mapped_values_of_the_new_types_attributes_only() {
    // Arrange.
    let diagram = tree();

    // Act.
    let transaction = retype::change(&specification(), &diagram.element_by_id("q").unwrap(), "Bin", None);

    // Assert.
    assert!(transaction.was_applied(), "{:?}", transaction.refusal);
    assert_eq!(1, transaction.changes.len());
    let (id, type_name, attributes) = retype_attributes(&transaction.changes[0]);
    assert_eq!(("q", "Bin"), (id.as_str(), type_name.as_str()));
    assert_eq!(1, attributes.len());
    assert!(matches!(attributes.get("size"), Some(Value::Int(3))));
}

#[test]
fn a_retype_reads_the_old_element() {
    // Arrange.
    let diagram = tree();
    let p2 = diagram.element_by_id("p2").unwrap();

    // Act.
    let transaction = retype::change(&specification(), &p2, "Box", None);
    let bag = retype::change(&specification(), &p2, "Bag", None);

    // Assert: Box declares no size, so nothing is mapped; a Bag inherits Box's attributes, and has none of its own.
    assert_eq!(1, transaction.changes.len());
    assert!(retype_attributes(&transaction.changes[0]).2.is_empty());
    assert_eq!(1, bag.changes.len());
    assert!(retype_attributes(&bag.changes[0]).2.is_empty());
}

#[test]
fn a_retype_not_declared_is_refused() {
    for (id, type_name, reason) in [
        ("p1", "Bag", "A Box cannot become a Bag."),
        ("p1", "Lid", "A Box cannot become a Lid."),
        ("p1", "Crate", "A Box cannot become a Crate: that is no type of node."),
        ("p1", "Holds", "A Box cannot become a Holds: that is no type of node."),
    ] {
        // Arrange.
        let diagram = tree();

        // Act.
        let transaction = retype::change(&specification(), &diagram.element_by_id(id).unwrap(), type_name, None);

        // Assert.
        assert_eq!(Some(reason.to_owned()), transaction.refusal);
    }
}

#[test]
fn a_retype_is_written_as_an_fbl_retype() {
    // Act.
    let change = write::to_fbl(
        &specification(),
        &DislChange::Retype {
            element_id: "q".into(),
            type_name: "Bin".into(),
            attributes: attrs([("size", Value::Int(3))]),
        },
    );

    // Assert.
    let ModelChange::Retype {
        id,
        type_name,
        attributes,
    } = change
    else {
        panic!("not a retype");
    };
    assert_eq!(("q", "Bin"), (id.as_str(), type_name.as_str()));
    assert!(matches!(attributes.get("size"), Some(Value::Int(3))));
}

#[test]
fn a_connect_runs_the_derived_relations_operation_with_its_ends_as_self() {
    // Arrange.
    let diagram = tree();

    // Act.
    let transaction = operations::connect(
        &specification(),
        "Holds",
        &diagram,
        &diagram.element_by_id("q").unwrap(),
        &diagram.element_by_id("p1").unwrap(),
        &ids::fixed(&[]),
        None,
    );

    // Assert.
    assert!(transaction.was_applied(), "{:?}", transaction.refusal);
    assert_eq!(1, transaction.changes.len());
    assert!(matches!(
        &transaction.changes[0],
        DislChange::Reparent { element_id, parent_id: Some(parent), index: -1 } if element_id == "p1" && parent == "q"
    ));
}

#[test]
fn a_connect_is_refused_by_the_operations_abort() {
    // Arrange.
    let diagram = tree();
    let q = diagram.element_by_id("q").unwrap();

    // Act.
    let transaction = operations::connect(&specification(), "Holds", &diagram, &q, &q, &ids::fixed(&[]), None);

    // Assert.
    assert_eq!(Some("Not under itself.".to_owned()), transaction.refusal);
}

#[test]
fn a_connect_without_a_connect_edit_is_refused() {
    for (relation, reason) in [
        ("Sees", "Seeing follows from holding."),
        ("Lid", "There is no relation type 'Lid'."),
    ] {
        // Arrange.
        let diagram = tree();

        // Act.
        let transaction = operations::connect(
            &specification(),
            relation,
            &diagram,
            &diagram.element_by_id("q").unwrap(),
            &diagram.element_by_id("p1").unwrap(),
            &ids::fixed(&[]),
            None,
        );

        // Assert.
        assert_eq!(Some(reason.to_owned()), transaction.refusal);
    }
}

#[test]
fn a_deletion_leaves_the_derived_lines_and_nested_children_to_the_removal_that_takes_them() {
    for (nested, expected) in [(false, "remove p2a\nremove p2"), (true, "remove p2")] {
        // Arrange.
        let diagram = tree();

        // Act.
        let transaction = deletion::changes(&specification(), &diagram.element_by_id("p2").unwrap(), nested);

        // Assert.
        assert!(transaction.was_applied(), "{:?}", transaction.refusal);
        assert_eq!(
            expected,
            transaction
                .changes
                .iter()
                .map(|c| match c {
                    DislChange::Remove { element_id } => format!("remove {element_id}"),
                    other => format!("{other:?}"),
                })
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}
