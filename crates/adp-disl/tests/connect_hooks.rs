//! The `after` hooks of a relation drawn (DISL section 9.2, `on: connect`): every hook whose `for` takes the new
//! relation runs, with `self` the relation and `old` null, its actions seeing what earlier iterations made (section
//! 9.4), and a hook for another event or another type does not (the standalone host's `ConnectHooksTests`).

mod common;

use std::sync::LazyLock;

use adp_cel::{Map, Value};
use adp_disl::behavior::{hooks, ids};
use adp_disl::{DislChange, DislDiagram, DislSpecification, DislTransaction};
use common::{attrs, loaded, with};

static JSON: LazyLock<String> = LazyLock::new(|| {
    with(
        r#""disl": "0.3",
        "metamodel": {
          "types": { "Box": { "attributes": { "name": { "type": "string" } } },
                     "Tag": { "attributes": { "text": { "type": "string" }, "about": { "type": "Box", "many": true } } } },
          "relations": { "Wire": { "source": ["Box"], "target": ["Box"] }, "Cord": { "source": ["Box"], "target": ["Box"] } } },
        "behavior": {
          "hooks": [
            { "id": "tagEnds", "on": "connect", "for": "Wire",
              "actions": [ { "forEach": "[self.source, self.target]", "as": "end",
                             "do": [ { "create": { "type": "'Tag'",
                                                   "attributes": { "text": "end.name + ' ' + string(diagram.nodesOfType('Tag').size() + 1) + (old == null ? '' : '!')",
                                                                   "about": "[end]" } } } ] } ] },
            { "id": "notOnCreate", "on": "create", "for": "Wire",
              "actions": [ { "create": { "type": "'Tag'", "attributes": { "text": "'create'" } } } ] },
            { "id": "notWhenFalse", "on": "connect", "for": "Wire", "when": "self.source.name == 'nobody'",
              "actions": [ { "create": { "type": "'Tag'", "attributes": { "text": "'when'" } } } ] },
            { "id": "notForCord", "on": ["connect"], "for": "Cord",
              "actions": [ { "create": { "type": "'Tag'", "attributes": { "text": "'cord'" } } } ] }
          ]
        }"#,
    )
});

fn specification() -> DislSpecification {
    loaded(&JSON)
}

fn created(transaction: &DislTransaction, attribute: &str) -> Vec<Value> {
    transaction
        .changes
        .iter()
        .map(|change| match change {
            DislChange::Create { attributes, .. } => attributes.get(attribute).cloned().unwrap_or(Value::Null),
            other => panic!("not a create: {other:?}"),
        })
        .collect()
}

#[test]
fn a_relation_drawn_runs_the_connect_hooks_for_its_type_with_self_the_relation_each_iteration_seeing_the_ones_before() {
    // Arrange: the host draws the relation on the working diagram first.
    let specification = specification();
    let diagram = DislDiagram::new(&specification);
    let a = diagram.add_node("Box", "a", attrs([("name", "A".into())])).unwrap();
    let b = diagram.add_node("Box", "b", attrs([("name", "B".into())])).unwrap();
    let wire = diagram
        .add_relation("Wire", "w", Some(&a), Some(&b), Map::default())
        .unwrap();

    // Act.
    let transaction = hooks::after_connect(&specification, &wire, &ids::fixed(&["t1", "t2"]), None, "user").unwrap();

    // Assert.
    assert!(transaction.was_applied(), "{:?}", transaction.refusal);
    assert_eq!(
        vec![Value::from("A 1"), Value::from("B 2")],
        created(&transaction, "text")
    );
    assert_eq!(
        vec![Value::list(vec!["a".into()]), Value::list(vec!["b".into()])],
        created(&transaction, "about")
    );
}

#[test]
fn a_relation_of_another_type_runs_none_of_them() {
    // Arrange.
    let specification = specification();
    let diagram = DislDiagram::new(&specification);
    let a = diagram.add_node("Box", "a", attrs([("name", "A".into())])).unwrap();
    let cord = diagram
        .add_relation("Cord", "c", Some(&a), Some(&a), Map::default())
        .unwrap();
    let hose = diagram
        .add_relation("Wire", "w", Some(&a), Some(&a), Map::default())
        .unwrap();

    // Act.
    let corded = hooks::after_connect(&specification, &cord, &ids::fixed(&["t1"]), None, "user").unwrap();
    let changed = hooks::after_change(
        &specification,
        &hose,
        &hose.snapshot(),
        &[],
        &ids::fixed(&[]),
        None,
        "user",
    );

    // Assert: the cord's own hook runs and nothing of the wire's; a change of the wire runs no connect hook.
    assert_eq!(vec![Value::from("cord")], created(&corded, "text"));
    assert!(changed.changes.is_empty());
}
