//! What a runtime that lowers another language onto DISL adds (etalii.adp spec 035, task T124, research R16):
//! functions in the environment every expression compiles in, and `extension` actions whose body the runtime runs,
//! in an operation or a hook, acting through DISL's own actions in the same transaction.

mod common;

use std::rc::Rc;

use adp_cel::{Function, Value};
use adp_disl::behavior::{hooks, ids, operations};
use adp_disl::{
    DislActionExtension, DislActions, DislChange, DislDiagram, DislExtensions, DislLoader, DislPluginFunctions,
    DislSpecification, DislTransaction, HostAction,
};
use common::{attrs, with};
use serde_json::json;

fn json() -> String {
    with(
        r#""metamodel": {
          "types": { "Box": { "attributes": { "name": { "type": "string" }, "count": { "type": "int" } } } }
        },
        "behavior": {
          "operations": {
            "add": { "label": "Add", "for": "diagram", "actions": [ { "extension": "add" } ] },
            "refuse": { "label": "Refuse", "for": "diagram", "actions": [ { "extension": "refuse" } ] }
          },
          "hooks": [
            { "id": "mark", "on": "change", "for": "Box", "attribute": "count", "actions": [ { "extension": "mark" } ] }
          ]
        }"#,
    )
}

/// A runtime's script, written out by hand: `add` creates a box named with a function of the runtime and sets its
/// count from a value it binds; `refuse` refuses; `mark` appends to the name of the box whose count changed.
struct Script;

impl DislActionExtension for Script {
    fn run(&self, name: &str, actions: &mut DislActions<'_, '_>) -> Result<(), String> {
        match name {
            "add" => {
                actions.perform(&json!({
                    "create": { "type": "'Box'", "attributes": { "name": "'Box ' + string(twice(size(diagram.nodes) + 1))" } },
                    "as": "made"
                }))?;
                let made = actions.variable("made").ok_or("`made` is bound")?;
                assert!(
                    actions.diagram().elements().len() == 2,
                    "the working diagram has the new box"
                );
                actions.bind("count", Value::Int(7));
                actions.perform(&json!({ "set": { "count": "count" }, "target": "made" }))?;
                actions.perform(&json!({ "select": "made" }))?;
                assert!(made.as_map().is_none(), "an element is the model's");
                Ok(())
            }
            "refuse" => {
                actions.perform(&json!({ "create": { "type": "'Box'", "attributes": { "name": "'gone'" } } }))?;
                Err("No room.".into())
            }
            "mark" => actions.perform(&json!({ "set": { "name": "self.name + '!'" } })),
            other => Err(format!("No extension `{other}`.")),
        }
    }
}

fn extended() -> DislSpecification {
    let extensions = DislExtensions {
        environment: Some(Box::new(|mut environment| {
            environment.add_function(Function::global("twice", 1, |a| {
                Ok(Value::Int(a[0].as_int().ok_or("an int")? * 2))
            }));
            environment
        })),
        actions: Some(Rc::new(Script)),
    };
    let result = DislLoader::load_extended(json().as_bytes(), &DislPluginFunctions::none(), extensions);
    result
        .specification
        .unwrap_or_else(|| panic!("{:?}", result.diagnostics))
}

fn text(transaction: &DislTransaction) -> String {
    let mut lines = Vec::new();
    if let Some(refusal) = &transaction.refusal {
        lines.push(format!("refused: {refusal}"));
    }
    for change in &transaction.changes {
        lines.push(match change {
            DislChange::Create { id, attributes, .. } => format!("create {id} {}", attributes.get("name").unwrap()),
            DislChange::Set {
                element_id, attributes, ..
            } => {
                let (name, value) = attributes.iter().next().unwrap();
                format!("set {element_id} {name}={value}")
            }
            other => format!("{other:?}"),
        });
    }
    for action in &transaction.host_actions {
        if let HostAction::Select { element_ids } = action {
            lines.push(format!("select {}", element_ids.join(",")));
        }
    }
    lines.join("\n")
}

#[test]
fn an_extension_action_acts_through_disls_actions_in_the_operations_transaction() {
    let specification = extended();
    let diagram = DislDiagram::new(&specification);
    diagram.add_node("Box", "a", attrs([("name", "A".into())])).unwrap();
    let transaction = operations::run(&specification, "add", &diagram, None, &ids::fixed(&["b"]), None, None);
    // The function the runtime added is called, and a name an action binds is the later actions'.
    assert_eq!(text(&transaction), "create b Box 4\nset b count=7\nselect b");
    assert!(matches!(
        diagram.element_by_id("b").unwrap().value_of("count"),
        Value::Int(7)
    ));
}

#[test]
fn an_extension_that_refuses_refuses_the_transaction() {
    let specification = extended();
    let diagram = DislDiagram::new(&specification);
    let transaction = operations::run(
        &specification,
        "refuse",
        &diagram,
        None,
        &ids::fixed(&["b"]),
        None,
        None,
    );
    assert_eq!(transaction.refusal.as_deref(), Some("No room."));
    assert!(transaction.changes.is_empty());
}

#[test]
fn a_hooks_extension_action_runs_after_the_change() {
    let specification = extended();
    let diagram = DislDiagram::new(&specification);
    let element = diagram.add_node("Box", "a", attrs([("name", "A".into())])).unwrap();
    let old = element.snapshot();
    element.store("count", Value::Int(2)).unwrap();
    let transaction = hooks::after_change(
        &specification,
        &element,
        &old,
        &["count"],
        &ids::fixed(&[]),
        None,
        "test",
    );
    assert_eq!(text(&transaction), "set a name=A!");
}

#[test]
fn without_a_runtime_an_extension_action_is_refused() {
    let specification = common::loaded(&json());
    let diagram = DislDiagram::new(&specification);
    let transaction = operations::run(&specification, "refuse", &diagram, None, &ids::fixed(&[]), None, None);
    let refusal = transaction.refusal.unwrap_or_default();
    assert!(
        refusal.starts_with("This runtime cannot run the extension `refuse`"),
        "{refusal}"
    );
}
