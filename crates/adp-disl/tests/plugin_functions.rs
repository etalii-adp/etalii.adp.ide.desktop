//! Plugin functions in CEL (DISL section 13.1.1): the host's implementation when it has one, the declared fallback
//! when it does not, and an evaluation error when there is neither (the standalone host's `PluginFunctionsTests`).

mod common;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use adp_cel::{Map, Value};
use adp_disl::derivation::constraints;
use adp_disl::{
    DislConstraintOptions, DislDiagram, DislElement, DislLoader, DislPluginFunctions, DislSpecification, NewElement,
    contexts,
};
use common::{diagnostics, with};

const PLUGIN: &str = r#""plugins": { "org.example.words": { "version": "^1.0.0", "provides": ["celFunctions"], "required": false,
      "celFunctions": [
        { "name": "shout", "params": [ { "name": "text", "type": "string" } ], "returns": "string", "cost": 3,
          "fallback": "text + '!'" },
        { "name": "secret", "params": [ "string" ], "returns": "string" } ] } },
    "functions": { "twiceShouted": { "params": [ { "name": "text", "type": "string" } ], "returns": "string", "cel": "shout(shout(text))" } }"#;

fn loaded(plugins: &DislPluginFunctions) -> DislSpecification {
    DislLoader::load_bytes(with(PLUGIN).as_bytes(), plugins)
        .specification
        .expect("the specification loads")
}

fn evaluate(specification: &DislSpecification, expression: &str) -> Result<Value, String> {
    specification
        .environment(contexts::ELEMENT, &[])
        .compile(expression)
        .unwrap()
        .evaluate(&HashMap::new())
}

#[test]
fn the_hosts_implementation_is_called_by_user_functions_too() {
    let specification = loaded(&DislPluginFunctions::none().add("shout", |a| {
        Ok(Value::String(a[0].as_str().unwrap_or("").to_uppercase()))
    }));

    assert_eq!(Ok(Value::from("HEY")), evaluate(&specification, "shout('hey')"));
    assert_eq!(Ok(Value::from("HEY")), evaluate(&specification, "twiceShouted('hey')"));
}

#[test]
fn without_an_implementation_the_fallback_is_evaluated() {
    let specification = loaded(&DislPluginFunctions::none());

    assert_eq!(Ok(Value::from("hey!")), evaluate(&specification, "shout('hey')"));
    assert_eq!(
        Ok(Value::from("hey!!")),
        evaluate(&specification, "twiceShouted('hey')")
    );
}

#[test]
fn without_an_implementation_or_a_fallback_a_call_is_an_evaluation_error() {
    let specification = loaded(&DislPluginFunctions::none());

    let error = evaluate(&specification, "secret('x')").unwrap_err();
    assert!(error.contains("org.example.words"));
}

#[test]
fn an_element_argument_reaches_the_host_as_the_models_element_also_under_its_written_id() {
    let received: Rc<RefCell<Option<DislElement>>> = Rc::new(RefCell::new(None));
    let json = with(
        r#""plugins": { "org.example.ids": { "provides": ["celFunctions"],
          "celFunctions": [ { "name": "seen", "params": [ { "name": "element", "type": "dyn" } ], "returns": "bool" } ] } },
        "constraints": { "rules": [ { "id": "always", "scope": "Thing", "rule": "seen(self)" } ] }"#,
    );
    let sink = Rc::clone(&received);
    let specification = DislLoader::load_bytes(
        json.as_bytes(),
        &DislPluginFunctions::none().add("seen", move |a| {
            *sink.borrow_mut() = DislElement::from_value(&a[0]);
            Ok(Value::Bool(false))
        }),
    )
    .specification
    .unwrap();
    let diagram = DislDiagram::new(&specification);
    let mut new = NewElement::new("Thing", "ephemeral#1");
    new.attributes = Map::default();
    let thing = diagram.add(new).unwrap();

    let findings = constraints::evaluate(
        &specification,
        &diagram,
        &DislConstraintOptions {
            written_id: Some(Rc::new(|_| "written".to_owned())),
            ..Default::default()
        },
    );

    assert_eq!(1, findings.len());
    assert_eq!(Some(thing), *received.borrow());
    assert_eq!(vec!["written"], findings[0].element_ids);
}

#[test]
fn a_name_that_clashes_or_a_fallback_over_unnamed_parameters_is_refused() {
    let clash = with(
        r#""plugins": { "org.example.a": { "celFunctions": [ { "name": "size", "params": [ "string" ], "returns": "int" } ] } }"#,
    );
    let unnamed = with(
        r#""plugins": { "org.example.b": { "celFunctions": [ { "name": "echo", "params": [ "string" ], "returns": "string", "fallback": "'x'" } ] } }"#,
    );

    assert_eq!(
        vec!["error at /plugins/org.example.a/celFunctions/0/name: 'size' is already a function of the DISL library or of another plugin (DISL §13.1.1).".to_owned()],
        diagnostics(&clash)
    );
    assert_eq!(
        vec!["error at /plugins/org.example.b/celFunctions/0/params: The fallback of 'echo' reads its parameters by name, so each is declared as {name, type} (DISL §13.1.1).".to_owned()],
        diagnostics(&unnamed)
    );
}
