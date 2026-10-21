//! A form item at commit (DISL 0.3 section 7.5): an inspector row's typed value, refused or made into a transaction,
//! and a params form's dialog and the refusal its value meets; and a label's inline edit (DISL 0.3 section 6.12): the
//! text its editor opens on, and its `parse`'s `accepts`, `refusal` and `write` (the standalone host's
//! `FormCommitTests`), or without a `parse` the attribute its `editText` or its `text` binds.

mod common;

use std::sync::LazyLock;

use adp_cel::Value;
use adp_disl::derivation::form_commit;
use adp_disl::{DislChange, DislDiagram, DislElement, DislEnv, DislSpecification, DislTransaction};
use common::{attrs, loaded, with};

static JSON: LazyLock<String> = LazyLock::new(|| {
    with(
        r#""disl": "0.3",
        "metamodel": { "types": { "Gauge": { "attributes": {
          "level": { "type": "number" },
          "mode": { "type": "string" },
          "note": { "type": "string" },
          "code": { "type": "string" } } } } },
        "notation": { "nodes": { "Gauge": { "shape": "rect", "labels": [
          { "id": "caption", "text": { "cel": "self.note" }, "editable": "inline", "editText": { "cel": "self.note + '?'" },
            "parse": { "accepts": "value != '' && !value.contains('#')", "write": [ { "set": { "note": "value.trim()" } } ],
              "refusal": { "cel": "'\\'' + value + '\\' is no caption.'" } } },
          { "id": "badge", "text": { "cel": "self.code" }, "editable": "inline",
            "parse": { "write": [ { "set": { "code": "value" } } ] } },
          { "id": "mode", "text": { "attribute": "mode" } },
          { "id": "summary", "text": { "cel": "self.note + '!'" }, "editable": "inline", "editText": { "attribute": "note" } },
          { "id": "shown", "text": { "cel": "self.note" }, "editable": "inline" } ] } } },
        "forms": {
          "gauge": {
            "for": "Gauge",
            "items": [
              { "kind": "section", "label": "Reading", "items": [
                { "attribute": "level", "label": "Level", "widget": "text",
                  "parse": {
                    "accepts": "value.trim() == '' || value.trim().matches('^[0-9]+(\\\\.[0-9]+)?$')",
                    "write": [
                      { "unset": ["level"], "when": "value.trim() == ''" },
                      { "set": { "level": "double(value.trim())" }, "when": "value.trim() != ''" } ],
                    "refusal": { "cel": "'\\'' + value + '\\' is not a level.'" } } },
                { "attribute": "mode", "label": "Mode", "widget": "select",
                  "options": [ { "value": "f", "label": "Fast" }, { "value": "s", "label": "Slow" } ],
                  "validate": [ { "timing": "commit", "rule": "value in ['f', 's']", "message": { "cel": "'\\'' + value + '\\' is not a mode.'" } } ] }
              ] },
              { "attribute": "note", "label": "Note", "widget": "text" },
              { "attribute": "code", "label": "Code", "widget": "text", "readOnlyReasons": ["The code is fixed."] },
              { "kind": "computed", "label": "Summary", "widget": "text", "value": { "cel": "self.note" } }
            ]
          },
          "pick": {
            "for": "Gauge",
            "label": "Pick a gauge",
            "icon": "mdi-target",
            "submitLabel": "Go",
            "items": [
              { "attribute": "target", "label": "Target", "widget": "text", "initial": "'G' + string(diagram.nodesOfType('Gauge').size() + 1)",
                "validate": [ { "timing": "commit", "rule": "diagram.nodesOfType('Gauge').exists(g, g.note == value)",
                  "message": { "cel": "'\\'' + value + '\\' is no gauge.'" } } ] }
            ]
          }
        }"#,
    )
});

fn specification() -> DislSpecification {
    loaded(&JSON)
}

/// A gauge, with the diagram that holds it: an element is a handle into its diagram.
fn gauge(specification: &DislSpecification) -> (DislDiagram, DislElement) {
    let diagram = DislDiagram::new(specification);
    let gauge = diagram
        .add_node(
            "Gauge",
            "g1",
            attrs([
                ("level", Value::Double(1.0)),
                ("mode", "f".into()),
                ("note", "first".into()),
                ("code", "x".into()),
            ]),
        )
        .unwrap();
    (diagram, gauge)
}

fn row(name: &str, value: &str, env: &DislEnv) -> DislTransaction {
    let specification = specification();
    let (_diagram, gauge) = gauge(&specification);
    form_commit::row(&specification, &gauge, name, value, env, "inspector")
}

fn single_set(transaction: &DislTransaction) -> (String, adp_cel::Map) {
    assert_eq!(1, transaction.changes.len(), "{transaction:?}");
    match &transaction.changes[0] {
        DislChange::Set {
            element_id, attributes, ..
        } => (element_id.clone(), attributes.clone()),
        other => panic!("not a set: {other:?}"),
    }
}

#[test]
fn a_parsed_value_is_written_by_the_parses_write() {
    let transaction = row("level", " 2.5 ", &DislEnv::default());

    assert!(transaction.refusal.is_none());
    let (id, attributes) = single_set(&transaction);
    assert_eq!(
        ("g1".to_owned(), Some(&Value::Double(2.5))),
        (id, attributes.get("level"))
    );
}

#[test]
fn an_empty_parsed_value_is_unset_by_the_parses_write() {
    let transaction = row("level", "", &DislEnv::default());

    let (_, attributes) = single_set(&transaction);
    assert_eq!(Some(&Value::Null), attributes.get("level"));
}

#[test]
fn a_value_the_parse_does_not_accept_is_refused_with_its_refusal() {
    assert_eq!(
        Some("'high' is not a level.".to_owned()),
        row("level", "high", &DislEnv::default()).refusal
    );
}

#[test]
fn a_selects_option_chosen_by_its_label_is_written_as_its_value() {
    let (_, attributes) = single_set(&row("mode", "Slow", &DislEnv::default()));

    assert_eq!(Some(&Value::from("s")), attributes.get("mode"));
}

#[test]
fn a_value_the_validation_refuses_is_refused_with_its_message() {
    assert_eq!(
        Some("'Turbo' is not a mode.".to_owned()),
        row("mode", "Turbo", &DislEnv::default()).refusal
    );
}

#[test]
fn a_plain_field_sets_its_attribute() {
    let (_, attributes) = single_set(&row("note", "second", &DislEnv::default()));

    assert_eq!(Some(&Value::from("second")), attributes.get("note"));
}

#[test]
fn a_row_with_a_read_only_reason_is_refused_with_it() {
    assert_eq!(
        Some("The code is fixed.".to_owned()),
        row("code", "y", &DislEnv::default()).refusal
    );
}

#[test]
fn a_computed_item_without_a_parse_is_refused() {
    let transaction = row("Summary", "anything", &DislEnv::default());

    assert!(transaction.refusal.is_some());
    assert!(transaction.changes.is_empty());
}

#[test]
fn a_row_of_a_read_only_diagram_is_refused_with_the_standard_sentence() {
    let env = DislEnv {
        read_only: true,
        ..Default::default()
    };

    assert_eq!(
        Some("This diagram is read-only.".to_owned()),
        row("note", "second", &env).refusal
    );
}

#[test]
fn a_params_form_gives_the_dialog_a_host_shows() {
    let specification = specification();
    let (_diagram, gauge) = gauge(&specification);

    let request = form_commit::request(&specification, "pick", Some(&gauge), &DislEnv::default(), None).unwrap();

    assert_eq!(
        ("Pick a gauge", Some("mdi-target"), Some("Go")),
        (
            request.label.as_str(),
            request.icon.as_deref(),
            request.submit_label.as_deref()
        )
    );
    assert_eq!(1, request.fields.len());
    let field = &request.fields[0];
    assert_eq!(
        ("target", "Target", Some("text"), Value::from("G2")),
        (
            field.attribute.as_str(),
            field.label.as_str(),
            field.widget.as_deref(),
            field.initial.clone()
        )
    );
}

#[test]
fn a_params_forms_value_is_refused_by_its_validation_and_accepted_when_it_holds() {
    let specification = specification();
    let (_diagram, gauge) = gauge(&specification);
    let env = DislEnv::default();

    let refused = form_commit::refusal(
        &specification,
        "pick",
        "target",
        "nope".into(),
        Some(&gauge),
        &env,
        None,
    );
    let accepted = form_commit::refusal(
        &specification,
        "pick",
        "target",
        "first".into(),
        Some(&gauge),
        &env,
        None,
    );

    assert_eq!(Some("'nope' is no gauge.".to_owned()), refused);
    assert_eq!(None, accepted);
}

#[test]
fn a_params_form_without_an_element_reads_the_diagram_given() {
    let specification = specification();
    let (diagram, _gauge) = gauge(&specification);
    let env = DislEnv::default();

    let request = form_commit::request(&specification, "pick", None, &env, Some(&diagram)).unwrap();
    let refused = form_commit::refusal(
        &specification,
        "pick",
        "target",
        "nope".into(),
        None,
        &env,
        Some(&diagram),
    );

    assert_eq!(1, request.fields.len());
    assert_eq!(Value::from("G2"), request.fields[0].initial);
    assert_eq!(Some("'nope' is no gauge.".to_owned()), refused);
}

#[test]
fn a_labels_edit_that_its_parse_does_not_accept_is_refused_with_its_refusal() {
    let specification = specification();
    let (_diagram, gauge) = gauge(&specification);

    let refused = form_commit::label(&specification, &gauge, "caption", "a # b", &DislEnv::default());

    assert_eq!(Some("'a # b' is no caption.".to_owned()), refused.refusal);
    assert!(refused.changes.is_empty());
}

#[test]
fn a_labels_edit_that_its_parse_accepts_is_written_by_its_write() {
    let specification = specification();
    let (_diagram, gauge) = gauge(&specification);

    let transaction = form_commit::label(&specification, &gauge, "caption", " second ", &DislEnv::default());

    assert!(transaction.refusal.is_none());
    let (id, attributes) = single_set(&transaction);
    assert_eq!(
        ("g1".to_owned(), Some(&Value::from("second"))),
        (id, attributes.get("note"))
    );
}

#[test]
fn a_labels_parse_without_accepts_takes_every_text() {
    let specification = specification();
    let (_diagram, gauge) = gauge(&specification);

    let (_, attributes) = single_set(&form_commit::label(
        &specification,
        &gauge,
        "badge",
        "",
        &DislEnv::default(),
    ));

    assert_eq!(Some(&Value::from("")), attributes.get("code"));
}

#[test]
fn a_labels_editor_opens_on_its_edit_text() {
    let specification = specification();
    let (_diagram, gauge) = gauge(&specification);

    assert_eq!(
        Some("first?".to_owned()),
        form_commit::edit_text(&specification, &gauge, "caption", &DislEnv::default())
    );
}

#[test]
fn a_labels_edit_without_a_parse_is_written_to_the_attribute_its_text_binds() {
    let specification = specification();
    let (_diagram, gauge) = gauge(&specification);

    let transaction = form_commit::label(&specification, &gauge, "mode", "s", &DislEnv::default());

    assert!(transaction.refusal.is_none(), "{transaction:?}");
    let (id, attributes) = single_set(&transaction);
    assert_eq!(("g1".to_owned(), Some(&Value::from("s"))), (id, attributes.get("mode")));
}

#[test]
fn a_computed_labels_edit_is_written_to_the_attribute_its_edit_text_binds() {
    let specification = specification();
    let (_diagram, gauge) = gauge(&specification);

    let (_, attributes) = single_set(&form_commit::label(
        &specification,
        &gauge,
        "summary",
        "second",
        &DislEnv::default(),
    ));

    assert_eq!(Some(&Value::from("second")), attributes.get("note"));
    assert_eq!(
        Some("first".to_owned()),
        form_commit::edit_text(&specification, &gauge, "summary", &DislEnv::default())
    );
}

#[test]
fn a_computed_label_that_binds_no_attribute_cannot_be_edited() {
    let specification = specification();
    let (_diagram, gauge) = gauge(&specification);

    let refused = form_commit::label(&specification, &gauge, "shown", "second", &DislEnv::default());

    assert_eq!(Some("'shown' cannot be edited here.".to_owned()), refused.refusal);
}

#[test]
fn a_bound_labels_edit_on_a_read_only_diagram_is_refused_with_the_standard_sentence() {
    let specification = specification();
    let (_diagram, gauge) = gauge(&specification);
    let env = DislEnv {
        read_only: true,
        ..DislEnv::default()
    };

    let refused = form_commit::label(&specification, &gauge, "mode", "s", &env);

    assert_eq!(Some("This diagram is read-only.".to_owned()), refused.refusal);
}

#[test]
fn an_attribute_is_written_with_the_text_typed_as_it_is() {
    let specification = specification();
    let (_diagram, gauge) = gauge(&specification);

    let (_, attributes) = single_set(&form_commit::attribute(
        &specification,
        &gauge,
        "level",
        "3",
        &DislEnv::default(),
    ));
    let refused = form_commit::attribute(&specification, &gauge, "nothing", "3", &DislEnv::default());

    assert_eq!(Some(&Value::Double(3.0)), attributes.get("level"));
    assert_eq!(Some("'nothing' cannot be edited here.".to_owned()), refused.refusal);
}
