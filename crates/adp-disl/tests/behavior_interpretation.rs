//! Operations, hooks, deletion and the writing of their changes (DISL sections 9.2 to 9.5) (the standalone host's
//! `BehaviorInterpretationTests`).

mod common;

use std::sync::LazyLock;

use adp_cel::{Map, Value};
use adp_disl::behavior::{deletion, hooks, ids, operations, write};
use adp_disl::{
    DislChange, DislConfirmation, DislDiagram, DislInvocation, DislSpecification, DislTransaction, HostAction,
};
use adp_fbl::ModelChange;
use common::{attrs, behavior_diagram, behavior_model, element, hype_cycle, hype_cycle_diagram, loaded, with};

static JSON: LazyLock<String> = LazyLock::new(|| {
    with(
        r#""metamodel": {
          "enums": { "Size": { "values": { "small": { "value": "S" }, "large": { "value": "L" } } } },
          "types": {
            "Box": { "attributes": { "name": { "type": "string" }, "size": { "type": "Size" }, "made": { "type": "yearMonth" }, "count": { "type": "int" }, "copy": { "type": "int" } } }
          },
          "relations": { "Link": { "source": "Box", "target": "Box", "directed": true, "attributes": { "weight": { "type": "number" } } } }
        },
        "persistence": { "typeMap": { "Crate": { "as": "Box", "attributes": { "title": "name" } } } },
        "toolbox": { "groups": [ { "id": "all", "tools": [ { "id": "box", "creates": "Box", "initial": { "name": { "cel": "'Box ' + string(int(position.x))" }, "count": 3 }, "after": "editLabel" } ] } ] },
        "behavior": {
          "operations": {
            "addBox": {
              "label": "Add", "for": "diagram",
              "actions": [
                { "let": { "n": "size(diagram.nodes) + 1" } },
                { "create": { "type": "'Box'", "at": "position", "attributes": { "name": "'Box ' + string(n)", "made": "yearMonth(2001, 2)" } }, "as": "added" },
                { "set": { "count": "n * 10" }, "target": "added" },
                { "set": { "size": "'large'" }, "target": "added", "when": "n > 5" },
                { "select": "added" },
                { "editLabel": { "target": "added" } }
              ]
            },
            "clear": {
              "label": "Clear", "for": ["Box"],
              "unavailable": [ { "when": "!has(self.count)", "message": "Nothing to clear." } ],
              "actions": [ { "unset": ["count", "size"] }, { "if": "self.name == ''", "then": [ { "set": { "name": "'unnamed'" } } ], "else": [ { "layout": { "algorithm": "tidy" } } ] } ]
            },
            "refuse": { "label": "Refuse", "for": ["Box"], "actions": [ { "set": { "count": "1" } }, { "abort": { "message": "'No, ' + self.name + '.'" } } ] },
            "unknown": { "label": "Unknown", "for": ["Box"], "actions": [ { "resize": { "width": "10" } } ] }
          },
          "hooks": [
            { "id": "copyCount", "on": "change", "for": "Box", "attribute": "count", "actions": [ { "set": { "copy": "self.count * 100 + (old == null ? 0 : old.count)" } } ] },
            { "id": "rename", "on": "change", "for": "Box", "attribute": ["copy"], "when": "self.copy > 0", "actions": [ { "set": { "name": "self.name + '!'" } } ] },
            { "id": "never", "on": "change", "for": "Box", "attribute": "name", "phase": "before", "actions": [ { "set": { "count": "0" } } ] }
          ],
          "deletion": {
            "Box": { "relations": "delete", "confirm": { "count": "self.incoming.size() + self.outgoing.size()", "threshold": 2, "title": "Remove box",
              "message": { "cel": "'Remove ' + self.name + ' and ' + string(count) + ' links?'" }, "confirmLabel": "Remove", "danger": false } }
          }
        }"#,
    )
});

fn specification() -> DislSpecification {
    loaded(&JSON)
}

fn values(values: &Map) -> String {
    values
        .iter()
        .map(|(k, v)| format!("{k}={}", if v.is_null() { "null".to_owned() } else { v.to_string() }))
        .collect::<Vec<_>>()
        .join(" ")
}

fn text(transaction: &DislTransaction) -> String {
    let mut lines: Vec<String> = Vec::new();
    for change in &transaction.changes {
        lines.push(match change {
            DislChange::Create {
                type_name,
                id,
                attributes,
                ..
            } => format!("create {type_name} {id} {}", values(attributes)),
            DislChange::Set {
                element_id, attributes, ..
            } => format!("set {element_id} {}", values(attributes)),
            DislChange::Remove { element_id } => format!("remove {element_id}"),
            other => format!("{other:?}"),
        });
    }
    for action in &transaction.host_actions {
        lines.push(match action {
            HostAction::Select { element_ids } => format!("select {}", element_ids.join(",")),
            HostAction::EditLabel { element_id, .. } => format!("editLabel {element_id}"),
            HostAction::Layout { algorithm } => format!("layout {algorithm}"),
            other => format!("{other:?}"),
        });
    }
    let refusal = match &transaction.refusal {
        Some(refusal) => format!("refused: {refusal}\n"),
        None => String::new(),
    };
    refusal + &lines.join("\n")
}

#[test]
fn an_operation_runs_its_actions_in_order_each_seeing_the_ones_before() {
    let specification = specification();
    let diagram = DislDiagram::new(&specification);
    diagram.add_node("Box", "a", attrs([("name", "A".into())])).unwrap();

    let transaction = operations::run(
        &specification,
        "addBox",
        &diagram,
        None,
        &ids::fixed(&["b"]),
        Some(&DislInvocation {
            position: Some(attrs([("x", Value::Double(1.0)), ("y", Value::Double(2.0))])),
            ..Default::default()
        }),
        None,
    );

    assert_eq!(
        "create Box b name=Box 2 made=24013\nset b count=20\nselect b\neditLabel b",
        text(&transaction)
    );
    assert!(matches!(
        diagram.element_by_id("b").unwrap().value_of("count"),
        Value::Int(20)
    ));
}

#[test]
fn an_unavailable_operation_is_refused_with_its_reason_and_an_abort_or_an_unknown_action_rolls_back() {
    let specification = specification();
    let diagram = DislDiagram::new(&specification);
    let empty = diagram.add_node("Box", "a", attrs([("name", "".into())])).unwrap();
    let full = diagram
        .add_node(
            "Box",
            "b",
            attrs([("name", "B".into()), ("count", Value::Int(4)), ("size", "small".into())]),
        )
        .unwrap();
    let run = |operation: &str, element| {
        text(&operations::run(
            &specification,
            operation,
            &diagram,
            Some(element),
            &ids::fixed(&[]),
            None,
            None,
        ))
    };

    assert_eq!("refused: Nothing to clear.\n", run("clear", &empty));
    assert_eq!("set b count=null size=null\nlayout tidy", run("clear", &full));
    assert_eq!("refused: No, B.\n", run("refuse", &full));
    assert!(run("unknown", &full).starts_with("refused: This runtime cannot run a `resize` action"));
}

#[test]
fn a_drop_creates_its_type_with_its_initial_values_and_hands_back_its_after() {
    let specification = specification();
    let diagram = DislDiagram::new(&specification);

    let transaction = operations::drop(
        &specification,
        "box",
        &diagram,
        &attrs([("x", Value::Double(7.5)), ("y", Value::Double(0.0))]),
        &ids::fixed(&["n"]),
        None,
    );

    assert_eq!("create Box n name=Box 7 count=3\neditLabel n", text(&transaction));
}

#[test]
fn a_changes_hooks_run_after_it_with_old_each_once_per_element_and_their_changes_run_further_hooks() {
    let specification = specification();
    let diagram = DislDiagram::new(&specification);
    let a_box = diagram
        .add_node("Box", "a", attrs([("name", "A".into()), ("count", Value::Int(2))]))
        .unwrap();
    let old = a_box.snapshot();
    a_box.store("count", Value::Int(5)).unwrap();

    let transaction = hooks::after_change(&specification, &a_box, &old, &["count"], &ids::fixed(&[]), None, "user");

    assert_eq!("set a copy=502\nset a name=A!", text(&transaction));
    assert_eq!(
        "",
        text(&hooks::after_change(
            &specification,
            &a_box,
            &old,
            &["made"],
            &ids::fixed(&[]),
            None,
            "user"
        ))
    );
}

#[test]
fn a_deletion_removes_the_relations_first_and_asks_only_from_its_threshold() {
    let specification = specification();
    let diagram = DislDiagram::new(&specification);
    let a = diagram.add_node("Box", "a", attrs([("name", "A".into())])).unwrap();
    let b = diagram.add_node("Box", "b", attrs([("name", "B".into())])).unwrap();
    diagram
        .add_relation("Link", "ab", Some(&a), Some(&b), Map::default())
        .unwrap();

    assert_eq!(
        "remove ab\nremove a",
        text(&deletion::changes(&specification, &a, false))
    );
    assert_eq!(None, deletion::confirmation(&specification, &a, None, None));

    diagram
        .add_relation("Link", "ba", Some(&b), Some(&a), Map::default())
        .unwrap();
    assert_eq!(
        Some(DislConfirmation {
            title: "Remove box".into(),
            message: "Remove A and 2 links?".into(),
            confirm_label: "Remove".into(),
            danger: false,
            count: Some(2)
        }),
        deletion::confirmation(&specification, &a, None, None)
    );
}

#[test]
fn a_change_is_written_in_the_bindings_terms_and_stored_forms() {
    let specification = specification();
    let created = write::to_fbl(
        &specification,
        &DislChange::Create {
            type_name: "Box".into(),
            id: "n".into(),
            attributes: attrs([
                ("name", "N".into()),
                ("made", Value::Int(2001 * 12 + 1)),
                ("size", "large".into()),
            ]),
            parent_id: None,
            at: Value::Null,
        },
    );
    let ModelChange::Add {
        type_name,
        id,
        attributes,
        ..
    } = created
    else {
        panic!("not an add");
    };

    assert_eq!(
        "Crate n title=N made=2001-02 size=L",
        format!("{type_name} {} {}", id.unwrap(), values(&attributes))
    );
    assert_eq!(
        Value::from("-0001-12"),
        write::stored_form(&specification, "Box", "made", &Value::Int(-1))
    );
    assert!(matches!(
        write::to_fbl(
            &specification,
            &DislChange::Reparent {
                element_id: "n".into(),
                parent_id: None,
                index: -1
            }
        ),
        ModelChange::Move { id, new_parent_id: None, index: -1 } if id == "n"
    ));
}

#[test]
fn a_base36_id_is_25_lowercase_characters() {
    let specification = loaded(&with(
        r#""persistence": { "ids": { "strategy": "uuid-v4", "encoding": "base36" } }"#,
    ));

    let id = adp_disl::IdSource::next(&ids::of(&specification).unwrap(), "Thing").unwrap();

    assert_eq!(25, id.len());
    assert!(id.chars().all(|c| c.is_ascii_digit() || c.is_ascii_lowercase()));
}

/// A bundled definition's `create` with an `at` hands the host the point it was asked at, placement being the host's.
#[test]
fn the_hype_cycle_graphs_add_trend_here_hands_back_the_clicked_point() {
    // Arrange.
    let specification = hype_cycle();
    let diagram = DislDiagram::new(&specification);
    let position = attrs([("x", Value::Double(1900.0 * 12.0)), ("y", Value::Double(3.0))]);

    // Act.
    let transaction = operations::run(
        &specification,
        "addTrendHere",
        &diagram,
        None,
        &ids::fixed(&["t"]),
        Some(&DislInvocation {
            position: Some(position),
            ..Default::default()
        }),
        None,
    );

    // Assert.
    assert_eq!(1, transaction.changes.len(), "{transaction:?}");
    let DislChange::Create { at, .. } = &transaction.changes[0] else {
        panic!("not a create");
    };
    let at = at.as_map().unwrap();
    assert!(matches!(at.get("x"), Some(Value::Double(x)) if *x == 1900.0 * 12.0));
    assert!(matches!(at.get("y"), Some(Value::Double(y)) if *y == 3.0));
}

/// A bundled definition's `editLabel` that names a label hands that name to the host.
#[test]
fn the_behavior_models_add_child_opens_the_editor_on_the_label_it_names() {
    // Arrange.
    let diagram = behavior_diagram();
    let invocation = DislInvocation {
        parameters: Some(attrs([("kind", "Check".into())])),
        ..Default::default()
    };

    // Act.
    let transaction = operations::run(
        &behavior_model(),
        "addChild",
        &diagram,
        Some(&element(&diagram, "1")),
        &ids::fixed(&["n"]),
        Some(&invocation),
        None,
    );

    // Assert.
    let Some(HostAction::EditLabel { element_id, label }) = transaction.host_actions.last() else {
        panic!("no editLabel: {transaction:?}");
    };
    assert_eq!("n", element_id);
    assert_eq!(Some("label"), label.as_deref());
}

/// A bundled definition's operation that a plugin carries out is handed back whole, naming the plugin.
#[test]
fn the_behavior_models_arrange_hands_back_the_plugin_it_names() {
    // Act.
    let transaction = operations::run(
        &behavior_model(),
        "arrange",
        &behavior_diagram(),
        None,
        &ids::fixed(&[]),
        None,
        None,
    );

    // Assert.
    assert_eq!(1, transaction.host_actions.len());
    let HostAction::Plugin { name, .. } = &transaction.host_actions[0] else {
        panic!("not a plugin");
    };
    assert_eq!("net.etalii.adp.etalii.abmArrange", name);
}

/// A bundled definition's deletion confirmation carries the count it was asked from.
#[test]
fn the_hype_cycle_graphs_trend_removal_is_confirmed_with_its_influence_count() {
    // Arrange: the steam engine has two influences in and one out.
    let diagram = hype_cycle_diagram();

    // Act.
    let confirmation = deletion::confirmation(&hype_cycle(), &element(&diagram, "steam"), None, None);

    // Assert.
    assert_eq!(Some(3), confirmation.unwrap().count);
}

#[test]
fn an_operations_parameters_are_its_params_in_order_each_with_its_default_on_the_target() {
    let specification = loaded(&with(
        r#""metamodel": { "types": { "Box": { "attributes": { "name": { "type": "string" }, "count": { "type": "int" } } } } },
        "behavior": { "operations": {
          "recount": { "label": "Recount", "for": ["Box"],
            "params": {
              "count": { "type": "int", "label": "Count", "default": { "cel": "self.count + 1" } },
              "note": { "type": "string" },
              "sure": { "type": "bool", "label": "Sure", "default": true } },
            "actions": [ { "set": { "count": "p.count", "name": "p.note" } } ] },
          "plain": { "label": "Plain", "for": ["Box"], "actions": [] } } }"#,
    ));
    let diagram = DislDiagram::new(&specification);
    let b = diagram.add_node("Box", "b", attrs([("count", Value::Int(4))])).unwrap();

    let parameters = operations::parameters(&specification, "recount", &diagram, Some(&b), None);

    let shown: Vec<String> = parameters
        .iter()
        .map(|p| format!("{} {} {} {}", p.name, p.label, p.type_name, p.default))
        .collect();
    assert_eq!(
        vec!["count Count int 5", "note note string null", "sure Sure bool true"],
        shown
    );
    assert!(operations::parameters(&specification, "plain", &diagram, Some(&b), None).is_empty());
    assert_eq!(Some(Value::Int(7)), parameters[0].value_of(" 7 "));
    assert_eq!(None, parameters[0].value_of("seven"));
    assert_eq!(Some(Value::from(" as typed ")), parameters[1].value_of(" as typed "));
    assert_eq!(Some(Value::Bool(false)), parameters[2].value_of("False"));

    let answered = DislInvocation {
        parameters: Some(attrs([("count", Value::Int(7)), ("note", "seven".into())])),
        ..Default::default()
    };
    let transaction = operations::run(
        &specification,
        "recount",
        &diagram,
        Some(&b),
        &ids::fixed(&[]),
        Some(&answered),
        None,
    );
    assert_eq!("set b count=7 name=seven", text(&transaction));
}
