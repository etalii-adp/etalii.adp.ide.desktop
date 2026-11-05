//! Which groupings a runtime keeps (DISL 0.9, sections 12.4 and 12.5; spec 075, T016 and T017): one made of an
//! unchanged list of a type is given again, one is made again once a value it read, or the list, changes, and one whose
//! key read anything but its own item's stored values is made again every time, with the same result.

mod common;

use std::collections::HashMap;

use adp_cel::{Value, groupings_made};
use adp_disl::{DislDiagram, DislElement, DislSpecification, contexts};
use common::{attrs, loaded, with};

fn specification() -> DislSpecification {
    loaded(&with(
        r#""functions": {
          "same": { "params": [ { "name": "s", "type": "string" } ], "returns": "string", "cel": "s" } },
        "metamodel": { "types": {
          "Thing": { "attributes": {
            "name": { "type": "string" }, "family": { "type": "string" },
            "shout": { "type": "string", "derived": "self.family.upperAscii()" },
            "buddy": { "type": "Thing" } } },
          "Other": { "attributes": { "name": { "type": "string" } } } } }"#,
    ))
}

struct Fixture {
    specification: DislSpecification,
    diagram: DislDiagram,
    things: Vec<DislElement>,
}

fn fixture() -> Fixture {
    let specification = specification();
    let diagram = DislDiagram::new(&specification);
    let things = ["a", "b", "a"]
        .iter()
        .enumerate()
        .map(|(i, kind)| {
            diagram
                .add_node(
                    "Thing",
                    &format!("t{i}"),
                    attrs([("name", format!("t{i}").into()), ("family", (*kind).into())]),
                )
                .unwrap()
        })
        .collect();
    Fixture {
        specification,
        diagram,
        things,
    }
}

impl Fixture {
    /// The value of `expression`, and how many groupings evaluating it made.
    fn evaluate(&self, expression: &str) -> (Value, u64) {
        let program = self
            .specification
            .environment(contexts::ELEMENT, &["k"])
            .compile(expression)
            .unwrap();
        let variables: HashMap<String, Value> = [
            ("diagram".to_owned(), self.diagram.to_value()),
            ("k".to_owned(), Value::from("a")),
        ]
        .into_iter()
        .collect();
        let before = groupings_made();
        let value = program.evaluate(&variables).unwrap();
        (value, groupings_made() - before)
    }
}

const SIZES: &str = "cel.bind(g, diagram.nodesOfType('Thing').groupBy(t, t.family), g.keys().map(k, g.get(k).size()))";

fn sizes(values: &[i64]) -> Value {
    Value::list(values.iter().map(|v| Value::Int(*v)).collect())
}

#[test]
fn a_grouping_of_an_unchanged_list_is_made_once() {
    let f = fixture();
    assert_eq!(f.evaluate(SIZES), (sizes(&[2, 1]), 1));
    assert_eq!(f.evaluate(SIZES), (sizes(&[2, 1]), 0));
    assert_eq!(f.evaluate(SIZES), (sizes(&[2, 1]), 0));
}

#[test]
fn a_grouping_is_made_again_after_a_value_it_read_changes() {
    let f = fixture();
    f.evaluate(SIZES);
    f.things[1].store("family", "a".into()).unwrap();
    assert_eq!(f.evaluate(SIZES), (sizes(&[3]), 1));
    assert_eq!(f.evaluate(SIZES), (sizes(&[3]), 0));
}

#[test]
fn a_grouping_is_made_again_after_a_node_of_its_list_is_added() {
    let f = fixture();
    f.evaluate(SIZES);
    f.diagram
        .add_node("Thing", "t3", attrs([("family", "c".into())]))
        .unwrap();
    assert_eq!(f.evaluate(SIZES), (sizes(&[2, 1, 1]), 1));
}

#[test]
fn a_grouping_is_kept_when_a_node_of_another_type_is_added() {
    let f = fixture();
    f.evaluate(SIZES);
    f.diagram
        .add_node("Other", "o1", attrs([("name", "o".into())]))
        .unwrap();
    assert_eq!(f.evaluate(SIZES), (sizes(&[2, 1]), 0));
}

#[test]
fn a_grouping_whose_key_reads_more_than_its_item_is_made_every_time_with_the_same_result() {
    let f = fixture();
    f.things[0].store("buddy", "t1".into()).unwrap();
    for key in [
        // another variable than its own
        "t.family == k",
        // diagram
        "size(diagram.nodes) > 0 && t.family == 'a'",
        // an element's id
        "t.id",
        // a derived attribute
        "t.shout",
        // another element
        "has(t.buddy) && t.buddy != null ? t.buddy.family : ''",
        // a description's function
        "same(t.family)",
        // a host object's method
        "t.isA('Thing') ? t.family : ''",
    ] {
        let expression = format!("diagram.nodesOfType('Thing').groupBy(t, {key}).keys()");
        let (first, made) = f.evaluate(&expression);
        assert_eq!(made, 1, "{key}");
        let (again, made) = f.evaluate(&expression);
        assert_eq!(made, 1, "{key} is made again");
        assert_eq!(again, first, "{key}");
    }
}

#[test]
fn a_grouping_of_a_list_made_afresh_is_made_once_per_evaluation() {
    let f = fixture();
    let expression = "diagram.nodesOfType('Thing').map(t, t.family).groupBy(v, v).keys()";
    assert_eq!(f.evaluate(expression), (Value::list(vec!["a".into(), "b".into()]), 1));
    assert_eq!(f.evaluate(expression).1, 1);
}
