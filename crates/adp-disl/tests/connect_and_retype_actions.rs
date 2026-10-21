//! The `connect` and `retype` actions of an operation (DISL section 9.4), and a connect gesture released on empty
//! canvas creating its missing end through its tool's `createTarget` or `createSource` (sections 6.10 and 7.2) (the
//! standalone host's `ConnectAndRetypeActionsTests`).

mod common;

use std::sync::LazyLock;

use adp_cel::{Map, Value};
use adp_disl::behavior::{ids, operations};
use adp_disl::{DislChange, DislDiagram, DislSpecification, DislTransaction};
use common::{attrs, loaded, with};

static JSON: LazyLock<String> = LazyLock::new(|| {
    with(
        r#""metamodel": {
          "types": {
            "Item": { "abstract": true, "attributes": { "name": { "type": "string" }, "row": { "type": "int" } } },
            "Span": { "extends": ["Item"], "attributes": { "end": { "type": "string" } } },
            "Point": { "extends": ["Item"] }
          },
          "relations": { "Next": { "source": "Item", "target": "Item", "attributes": { "label": { "type": "string" } } } }
        },
        "notation": { "edges": { "Next": { "connect": { "from": { "right": "source", "left": "target" }, "tool": "relate" } } } },
        "toolbox": {
          "tools": {
            "relate": { "id": "relate", "creates": "Next", "mode": "drag",
              "createTarget": { "type": "Span", "initial": { "name": "New", "row": { "cel": "int(position.y)" } } },
              "createSource": "Point" }
          }
        },
        "behavior": {
          "operations": {
            "grow": { "label": "Grow", "for": ["Item"], "actions": [
              { "create": { "type": "'Span'", "attributes": { "name": "'Next'", "row": "self.row + 1" } }, "as": "next" },
              { "connect": { "type": "'Next'", "source": "self", "target": "next", "attributes": { "label": "'after ' + self.name" } }, "as": "link" },
              { "set": { "label": "link.label + '!'" }, "target": "link" } ] },
            "widen": { "label": "Widen", "for": ["Point"], "actions": [ { "retype": { "to": "'Span'" } }, { "set": { "end": "'later'" } } ] },
            "narrow": { "label": "Narrow", "for": ["Span"], "actions": [ { "unset": ["end"] }, { "retype": { "to": "'Point'" } }, { "set": { "name": "self.type" } } ] },
            "nowhere": { "label": "Nowhere", "for": ["Span"], "actions": [ { "retype": { "to": "'Item'" } } ] }
          },
          "retype": { "Point": { "to": ["Span"] }, "Span": { "to": ["Point"] } }
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
    let refusal = transaction
        .refusal
        .as_ref()
        .map(|r| format!("refused: {r}"))
        .unwrap_or_default();
    let changes: Vec<String> = transaction
        .changes
        .iter()
        .map(|change| match change {
            DislChange::Create {
                type_name,
                id,
                attributes,
                ..
            } => format!("create {type_name} {id} {}", values(attributes)),
            DislChange::Connect {
                type_name,
                id,
                source_id,
                target_id,
                attributes,
            } => format!(
                "connect {type_name} {id} {source_id}->{target_id} {}",
                values(attributes)
            ),
            DislChange::Set {
                element_id,
                type_name,
                attributes,
            } => format!("set {element_id} {type_name} {}", values(attributes)),
            DislChange::Retype {
                element_id, type_name, ..
            } => format!("retype {element_id} {type_name}"),
            other => format!("{other:?}"),
        })
        .collect();
    refusal + &changes.join("\n")
}

#[test]
fn a_connect_creates_the_relation_between_its_ends_and_names_it_for_the_actions_after() {
    // Arrange.
    let specification = specification();
    let diagram = DislDiagram::new(&specification);
    let a = diagram
        .add_node("Point", "a", attrs([("name", "A".into()), ("row", Value::Int(2))]))
        .unwrap();

    // Act.
    let transaction = operations::run(
        &specification,
        "grow",
        &diagram,
        Some(&a),
        &ids::fixed(&["n1", "r1"]),
        None,
        None,
    );

    // Assert.
    assert_eq!(
        "create Span n1 name=Next row=3\nconnect Next r1 a->n1 label=after A\nset r1 Next label=after A!",
        text(&transaction)
    );
    let relations = diagram.relations();
    assert_eq!(1, relations.len());
    assert_eq!(Some(a), relations[0].source());
    assert_eq!(Some("n1".to_owned()), relations[0].target().map(|t| t.id()));
}

#[test]
fn a_retype_changes_the_type_for_the_actions_after_it() {
    // Arrange.
    let specification = specification();
    let diagram = DislDiagram::new(&specification);
    let point = diagram.add_node("Point", "p", attrs([("name", "P".into())])).unwrap();

    // Act.
    let transaction = operations::run(
        &specification,
        "widen",
        &diagram,
        Some(&point),
        &ids::fixed(&[]),
        None,
        None,
    );

    // Assert.
    assert_eq!("retype p Span\nset p Span end=later", text(&transaction));
    assert!(point.is_a("Span"));
    assert_eq!(Value::from("P"), point.value_of("name"));
}

#[test]
fn a_retype_forgets_what_the_new_type_does_not_declare() {
    // Arrange.
    let specification = specification();
    let diagram = DislDiagram::new(&specification);
    let span = diagram
        .add_node("Span", "s", attrs([("name", "S".into()), ("end", "x".into())]))
        .unwrap();

    // Act.
    let transaction = operations::run(
        &specification,
        "narrow",
        &diagram,
        Some(&span),
        &ids::fixed(&[]),
        None,
        None,
    );

    // Assert.
    assert_eq!(
        "set s Span end=null\nretype s Point\nset s Point name=Point",
        text(&transaction)
    );
    assert!(!span.attributes().contains_key("end"));
}

#[test]
fn a_retype_the_definition_does_not_allow_refuses_the_transaction() {
    // Arrange.
    let specification = specification();
    let diagram = DislDiagram::new(&specification);
    let span = diagram.add_node("Span", "s", Map::default()).unwrap();

    // Act.
    let transaction = operations::run(
        &specification,
        "nowhere",
        &diagram,
        Some(&span),
        &ids::fixed(&[]),
        None,
        None,
    );

    // Assert.
    assert_eq!(
        "refused: A Span cannot become a Item: that is no type of node.",
        text(&transaction)
    );
}

#[test]
fn a_gesture_released_on_empty_canvas_creates_the_target_from_its_tool_then_the_relation() {
    // Arrange.
    let specification = specification();
    let diagram = DislDiagram::new(&specification);
    let a = diagram.add_node("Point", "a", Map::default()).unwrap();

    // Act.
    let transaction = operations::connect_to_new(
        &specification,
        "Next",
        &diagram,
        &a,
        "target",
        &attrs([("x", Value::Double(1.0)), ("y", Value::Double(4.0))]),
        &ids::fixed(&["n1", "r1"]),
        None,
    );

    // Assert.
    assert_eq!(
        "create Span n1 name=New row=4\nconnect Next r1 a->n1 ",
        text(&transaction)
    );
}

#[test]
fn a_gesture_from_a_target_anchor_creates_the_source_pointing_into_the_existing_element() {
    // Arrange.
    let specification = specification();
    let diagram = DislDiagram::new(&specification);
    let a = diagram.add_node("Span", "a", Map::default()).unwrap();

    // Act.
    let transaction = operations::connect_to_new(
        &specification,
        "Next",
        &diagram,
        &a,
        "source",
        &attrs([("x", Value::Double(1.0)), ("y", Value::Double(4.0))]),
        &ids::fixed(&["n1", "r1"]),
        None,
    );

    // Assert.
    assert_eq!("create Point n1 \nconnect Next r1 n1->a ", text(&transaction));
}

#[test]
fn an_operation_applies_to_the_types_its_for_names() {
    for (operation, type_name, expected) in [
        ("widen", "Point", true),
        ("widen", "Span", false),
        ("grow", "Span", true),
        ("absent", "Span", false),
    ] {
        // Arrange.
        let specification = specification();
        let diagram = DislDiagram::new(&specification);
        let element = diagram.add_node(type_name, "e", Map::default()).unwrap();

        // Act.
        let applies = operations::applies_to(&specification, operation, Some(&element));

        // Assert.
        assert_eq!(expected, applies, "{operation} {type_name}");
    }
}
