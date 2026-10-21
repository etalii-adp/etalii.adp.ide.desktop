//! A `reparent` action (DISL section 9.4), applied to the working state and written as an FBL move: the node moves
//! with everything beneath it, to the place its `after` or `before` names, or last, and the change records that place
//! as an index counted before the move (the standalone host's `ReparentingTests`).

mod common;

use std::sync::LazyLock;

use adp_cel::{Map, Value};
use adp_disl::behavior::{ids, operations, write};
use adp_disl::{DislChange, DislDiagram, DislSpecification};
use adp_fbl::ModelChange;
use common::{loaded, with};

static JSON: LazyLock<String> = LazyLock::new(|| {
    with(
        r#""metamodel": { "types": { "Box": { "attributes": { "name": { "type": "string" } }, "children": { "allowed": ["Box"] } } } },
        "behavior": {
          "operations": {
            "last": { "label": "Last", "for": ["Box"], "actions": [
              { "reparent": { "parent": "diagram.elementById('p')" } },
              { "set": { "name": "string(size(self.parent.children)) + ' under ' + self.parent.id" } } ] },
            "afterP1": { "label": "After", "for": ["Box"], "actions": [ { "reparent": { "parent": "diagram.elementById('p')", "after": "diagram.elementById('p1')" } } ] },
            "beforeP1": { "label": "Before", "for": ["Box"], "actions": [ { "reparent": { "parent": "diagram.elementById('p')", "before": "diagram.elementById('p1')" } } ] },
            "top": { "label": "Top", "for": ["Box"], "actions": [ { "reparent": { "parent": "null" } } ] },
            "itself": { "label": "Itself", "for": ["Box"], "actions": [ { "reparent": { "parent": "self" } } ] },
            "beside": { "label": "Beside", "for": ["Box"], "actions": [ { "reparent": { "parent": "diagram.elementById('p')", "after": "diagram.elementById('q')" } } ] }
          }
        }"#,
    )
});

fn specification() -> DislSpecification {
    loaded(&JSON)
}

/// p (p1, p2) and q (q1 (q1a)), in document order.
fn tree() -> DislDiagram {
    let diagram = DislDiagram::new(&specification());
    let p = diagram.add_node("Box", "p", Map::default()).unwrap();
    diagram
        .add_child("Box", "p1", Map::default(), &p, Some("children"))
        .unwrap();
    diagram
        .add_child("Box", "p2", Map::default(), &p, Some("children"))
        .unwrap();
    let q = diagram.add_node("Box", "q", Map::default()).unwrap();
    let q1 = diagram
        .add_child("Box", "q1", Map::default(), &q, Some("children"))
        .unwrap();
    diagram
        .add_child("Box", "q1a", Map::default(), &q1, Some("children"))
        .unwrap();
    diagram
}

fn order(diagram: &DislDiagram) -> String {
    diagram
        .nodes()
        .iter()
        .map(|n| match n.parent() {
            None => n.id(),
            Some(parent) => format!("{}/{}", parent.id(), n.id()),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn run(diagram: &DislDiagram, operation: &str, element: &str) -> adp_disl::DislTransaction {
    operations::run(
        &specification(),
        operation,
        diagram,
        diagram.element_by_id(element).as_ref(),
        &ids::fixed(&[]),
        None,
        None,
    )
}

#[test]
fn a_reparent_moves_the_subtree_to_its_place_and_records_the_index() {
    for (operation, index, expected) in [
        ("last", -1, "p p/p1 p/p2 p/q1 q1/q1a q"),
        ("afterP1", 1, "p p/p1 p/q1 q1/q1a p/p2 q"),
        ("beforeP1", 0, "p p/q1 q1/q1a p/p1 p/p2 q"),
    ] {
        // Arrange.
        let diagram = tree();

        // Act.
        let transaction = run(&diagram, operation, "q1");

        // Assert.
        assert!(transaction.was_applied(), "{:?}", transaction.refusal);
        assert!(
            matches!(
                &transaction.changes[0],
                DislChange::Reparent { element_id, parent_id: Some(parent), index: i } if element_id == "q1" && parent == "p" && *i == index
            ),
            "{operation}: {:?}",
            transaction.changes[0]
        );
        assert_eq!(expected, order(&diagram), "{operation}");
        assert!(diagram.element_by_id("q").unwrap().children().is_empty());
    }
}

#[test]
fn an_action_after_a_reparent_sees_the_move() {
    // Arrange.
    let diagram = tree();

    // Act.
    let transaction = run(&diagram, "last", "q1");

    // Assert.
    let DislChange::Set { attributes, .. } = &transaction.changes[1] else {
        panic!("not a set");
    };
    assert_eq!(Some(&Value::from("3 under p")), attributes.get("name"));
}

#[test]
fn a_reparent_to_the_top_level_puts_the_node_last() {
    // Arrange.
    let diagram = tree();

    // Act.
    let transaction = run(&diagram, "top", "p1");

    // Assert.
    assert!(transaction.was_applied(), "{:?}", transaction.refusal);
    assert_eq!("p p/p2 q q/q1 q1/q1a p1", order(&diagram));
    assert!(matches!(
        write::to_fbl(&specification(), &transaction.changes[0]),
        ModelChange::Move { id, new_parent_id: None, index: -1 } if id == "p1"
    ));
}

#[test]
fn a_reparent_that_cannot_be_refuses_the_transaction() {
    for (operation, element, reason) in [
        ("itself", "q1", "A node cannot move beneath itself."),
        ("last", "p", "A node cannot move beneath itself."),
        (
            "beside",
            "q1",
            "places the element beside one that is not beneath its new parent",
        ),
    ] {
        // Arrange.
        let diagram = tree();

        // Act.
        let transaction = run(&diagram, operation, element);

        // Assert.
        assert!(!transaction.was_applied());
        assert!(transaction.refusal.as_deref().unwrap().contains(reason), "{operation}");
    }
}

#[test]
fn a_reparent_is_written_as_a_move_to_its_parent_and_index() {
    // Act.
    let change = write::to_fbl(
        &specification(),
        &DislChange::Reparent {
            element_id: "q1".into(),
            parent_id: Some("p".into()),
            index: 1,
        },
    );

    // Assert.
    assert!(matches!(
        change,
        ModelChange::Move { id, new_parent_id: Some(parent), index: 1 } if id == "q1" && parent == "p"
    ));
}
