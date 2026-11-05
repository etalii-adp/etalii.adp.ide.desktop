//! What a grouping charges (DISL 0.9, section 12.4; spec 075, T015): one evaluation of its key per item and nothing
//! more per item, `get` one plus the size of its group, `keys` one plus the number of keys, and a grouping given
//! again what making it cost, so that whether it was kept never changes a budget.

mod common;

use std::collections::HashMap;

use adp_cel::{Budget, Value, groupings_made};
use adp_disl::{DislDiagram, DislSpecification, contexts};
use common::{attrs, loaded, with};

fn specification() -> DislSpecification {
    loaded(&with(
        r#""metamodel": { "types": { "Thing": { "attributes": {
          "name": { "type": "string" }, "family": { "type": "string" } } } } }"#,
    ))
}

/// A diagram of things of the kinds `kinds`, in order.
fn diagram(specification: &DislSpecification, kinds: &[&str]) -> DislDiagram {
    let diagram = DislDiagram::new(specification);
    for (i, kind) in kinds.iter().enumerate() {
        diagram
            .add_node(
                "Thing",
                &format!("t{i}"),
                attrs([("name", format!("t{i}").into()), ("family", (*kind).into())]),
            )
            .unwrap();
    }
    diagram
}

/// What evaluating `expression` charged, with its value.
fn charged(specification: &DislSpecification, diagram: &DislDiagram, expression: &str) -> (i64, Value) {
    let program = specification
        .environment(contexts::ELEMENT, &[])
        .compile(expression)
        .unwrap();
    let variables: HashMap<String, Value> = [("diagram".to_owned(), diagram.to_value())].into_iter().collect();
    let budget = Budget::new(i64::MAX);
    let value = program.evaluate_within(&variables, &budget).unwrap();
    (budget.used(), value)
}

#[test]
fn a_grouping_charges_one_evaluation_of_its_key_per_item_as_map_does() {
    let specification = specification();
    for kinds in [&["a"][..], &["a", "b", "a"], &["a", "b", "c", "a", "b", "a", "d"]] {
        let diagram = diagram(&specification, kinds);
        let (mapped, _) = charged(
            &specification,
            &diagram,
            "diagram.nodesOfType('Thing').map(t, t.family + '|' + t.name)",
        );
        let (grouped, _) = charged(
            &specification,
            &diagram,
            "diagram.nodesOfType('Thing').groupBy(t, t.family + '|' + t.name)",
        );
        assert_eq!(grouped, mapped, "{kinds:?}");
    }
}

#[test]
fn get_charges_one_plus_the_size_of_its_group_and_keys_one_plus_the_number_of_keys() {
    let specification = specification();
    let diagram = diagram(&specification, &["a", "b", "a", "a", "c"]);
    let grouping = "diagram.nodesOfType('Thing').groupBy(t, t.family)";
    let (alone, _) = charged(&specification, &diagram, grouping);
    // The call is one step of the expression, its literal argument none.
    for (key, size) in [("a", 3), ("b", 1), ("z", 0)] {
        let (read, value) = charged(&specification, &diagram, &format!("{grouping}.get('{key}')"));
        assert_eq!(value.as_list().unwrap().len(), size);
        assert_eq!(read - alone, 1 + 1 + size as i64, "get('{key}')");
    }
    let (keys, value) = charged(&specification, &diagram, &format!("{grouping}.keys()"));
    assert_eq!(value.as_list().unwrap().len(), 3);
    assert_eq!(keys - alone, 1 + 1 + 3);
}

#[test]
fn a_grouping_given_again_charges_what_making_it_cost() {
    let specification = specification();
    let diagram = diagram(&specification, &["a", "b", "a", "c"]);
    let expression = "diagram.nodesOfType('Thing').groupBy(t, t.family).get('a').size()";
    let before = groupings_made();
    let (first, _) = charged(&specification, &diagram, expression);
    let (second, value) = charged(&specification, &diagram, expression);
    assert_eq!(
        groupings_made() - before,
        1,
        "the second evaluation is given the kept grouping"
    );
    assert_eq!(value, Value::Int(2));
    assert_eq!(second, first);
}

#[test]
fn a_grouping_written_per_item_charges_one_pass_per_item() {
    let specification = specification();
    let diagram = diagram(&specification, &["a", "b", "a", "c"]);
    let (once, _) = charged(
        &specification,
        &diagram,
        "diagram.nodesOfType('Thing').groupBy(t, t.family)",
    );
    let (per_item, _) = charged(
        &specification,
        &diagram,
        "diagram.nodesOfType('Thing').map(u, diagram.nodesOfType('Thing').groupBy(t, t.family).get(u.family).size())",
    );
    assert!(per_item > 4 * once, "{per_item} against {once}");
}
