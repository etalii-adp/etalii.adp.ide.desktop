//! An attribute with a `derived` expression (DISL section 4.3) is computed, never stored: evaluated in the `element`
//! context with `self` the element, whoever reads it - CEL, a form, a menu (the standalone host's
//! `DerivedAttributesTests`).

mod common;

use adp_cel::{Map, Value};
use adp_disl::derivation::forms;
use adp_disl::{DislDiagram, DislElement, DislEnv, DislSpecification, WireIdMap, contexts};
use common::{attrs, loaded, vars, with};

struct Fixture {
    specification: DislSpecification,
    diagram: DislDiagram,
    first: DislElement,
    second: DislElement,
    belt: DislElement,
}

fn fixture() -> Fixture {
    let specification = loaded(&with(
        r#""functions": {
          "twice": { "params": [{ "name": "n", "type": "int" }], "returns": "int", "cel": "n * 2" }
        },
        "metamodel": {
          "types": {
            "Crate": { "attributes": {
              "name": { "type": "string" },
              "shout": { "type": "string", "derived": "self.name.upperAscii()" },
              "echo": { "type": "string", "derived": "self.shout + '!'" },
              "arrivals": { "type": "int", "derived": "twice(self.incoming.size())" },
              "broken": { "type": "int", "derived": "1 / 0" },
              "itself": { "type": "string", "derived": "self.itself" } } },
            "Pallet": { "extends": "Crate" } },
          "relations": {
            "Belt": { "source": "Crate", "target": "Crate", "attributes": {
              "route": { "type": "string", "derived": "self.source.name + '>' + self.target.name" } } } } },
        "forms": {
          "crate": { "for": "Crate", "items": [ { "attribute": "shout" }, { "kind": "computed", "label": "Echo", "value": { "cel": "self.echo" } } ] }
        }"#,
    ));
    let diagram = DislDiagram::new(&specification);
    let first = diagram
        .add_node("Crate", "first", attrs([("name", "apples".into())]))
        .unwrap();
    let second = diagram
        .add_node("Pallet", "second", attrs([("name", "pears".into())]))
        .unwrap();
    let belt = diagram
        .add_relation("Belt", "belt", Some(&first), Some(&second), Map::default())
        .unwrap();
    Fixture {
        specification,
        diagram,
        first,
        second,
        belt,
    }
}

impl Fixture {
    fn evaluate(&self, expression: &str, element: &DislElement) -> Result<Value, String> {
        self.specification
            .environment(contexts::ELEMENT, &[])
            .compile(expression)
            .unwrap()
            .evaluate(&vars([
                ("self", element.to_value()),
                ("diagram", self.diagram.to_value()),
                ("env", Value::map(DislEnv::default().to_cel())),
            ]))
    }
}

#[test]
fn a_derived_attribute_reads_as_its_expression() {
    assert_eq!(Value::from("APPLES"), fixture().first.value_of("shout"));
}

#[test]
fn a_derived_attribute_is_computed_for_the_element_reading_it_inherited_ones_included() {
    assert_eq!(Value::from("PEARS"), fixture().second.value_of("shout"));
}

#[test]
fn a_derived_attribute_may_read_another_and_user_functions_and_the_model() {
    let f = fixture();
    assert_eq!(Value::from("APPLES!"), f.first.value_of("echo"));
    assert!(matches!(f.first.value_of("arrivals"), Value::Int(0)));
    assert!(matches!(f.second.value_of("arrivals"), Value::Int(2)));
}

#[test]
fn a_relations_derived_attribute_is_computed_too() {
    assert_eq!(Value::from("apples>pears"), fixture().belt.value_of("route"));
}

#[test]
fn cel_reads_a_derived_attribute_and_has_holds_for_it() {
    let f = fixture();
    assert_eq!(Ok(Value::from("APPLES!")), f.evaluate("self.echo", &f.first));
    assert_eq!(Ok(Value::Bool(true)), f.evaluate("has(self.shout)", &f.first));
}

#[test]
fn a_derived_attribute_is_not_stored_even_once_read() {
    let f = fixture();
    let _ = f.first.value_of("shout");

    assert!(!f.first.attributes().contains_key("shout"));
}

#[test]
fn a_derived_attribute_that_fails_reads_as_its_types_zero_value() {
    let f = fixture();
    assert!(matches!(f.first.value_of("broken"), Value::Int(0)));
    assert_eq!(Value::from(""), f.first.value_of("itself"));
}

#[test]
fn a_form_row_shows_the_derived_value() {
    let f = fixture();

    let rows = forms::derive(
        &f.specification,
        &f.first,
        &DislEnv::default(),
        &WireIdMap::none(),
        "inspector",
    );

    assert_eq!(
        vec!["APPLES", "APPLES!"],
        rows.iter().map(|r| r.value.as_str()).collect::<Vec<_>>()
    );
}

#[test]
fn an_expression_reading_a_failing_derived_attribute_still_evaluates() {
    let f = fixture();
    assert!(f.evaluate("self.broken + 1", &f.first).is_ok());
}
