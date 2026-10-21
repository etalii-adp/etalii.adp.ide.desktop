//! `diagram.cycles(relType, max)` and `diagram.cyclesTruncated(relType, max)` (DISL section 12.4): the elementary
//! cycles over the relations of a type and its subtypes, each starting at its member that comes first in model
//! order, ordered by that member and then lexicographically by member order, a self-loop a cycle of one, and never
//! more than `max` of them (the standalone host's `DiagramCyclesTests`).

mod common;

use std::sync::LazyLock;

use adp_cel::{Map, Value};
use adp_disl::{DislDiagram, DislSpecification, contexts};
use common::{loaded, vars, with};

static JSON: LazyLock<String> = LazyLock::new(|| {
    with(
        r#""metamodel": {
          "types": { "Station": {} },
          "relations": {
            "Track": { "source": "Station", "target": "Station" },
            "Branch": { "extends": "Track" },
            "Road": { "source": "Station", "target": "Station" } } }"#,
    )
});

fn specification() -> DislSpecification {
    loaded(&JSON)
}

/// A diagram with one Station per name, in that order, and a relation of its type per pair.
fn diagram_of(stations: &str, relations: &[(&str, &str, &str)]) -> DislDiagram {
    let diagram = DislDiagram::new(&specification());
    for name in stations.split(' ') {
        diagram.add_node("Station", name, Map::default()).unwrap();
    }
    for (index, (from, to, type_name)) in relations.iter().enumerate() {
        let (from, to) = (diagram.element_by_id(from), diagram.element_by_id(to));
        diagram
            .add_relation(
                type_name,
                &format!("r{index}"),
                from.as_ref(),
                to.as_ref(),
                Map::default(),
            )
            .unwrap();
    }
    diagram
}

fn tracks(stations: &str, tracks: &[(&str, &str)]) -> DislDiagram {
    let relations: Vec<(&str, &str, &str)> = tracks.iter().map(|(f, t)| (*f, *t, "Track")).collect();
    diagram_of(stations, &relations)
}

fn evaluate(diagram: &DislDiagram, expression: &str) -> Result<Value, String> {
    diagram
        .specification()
        .environment(contexts::CONSTRAINT, &[])
        .compile(expression)
        .unwrap()
        .evaluate(&vars([
            ("self", diagram.to_value()),
            ("diagram", diagram.to_value()),
            ("env", Value::map(Map::default())),
        ]))
}

/// The cycles as text: each cycle its member ids joined by a space, the cycles by a comma.
fn cycles(diagram: &DislDiagram, max: i64) -> String {
    match evaluate(
        diagram,
        &format!("diagram.cycles('Track', {max}).map(c, c.map(e, e.id).join(' ')).join(', ')"),
    ) {
        Ok(Value::String(text)) => text,
        other => panic!("The cycles were not computed: {other:?}"),
    }
}

fn truncated(diagram: &DislDiagram, max: i64) -> bool {
    match evaluate(diagram, &format!("diagram.cyclesTruncated('Track', {max})")) {
        Ok(Value::Bool(b)) => b,
        other => panic!("cyclesTruncated did not give a bool: {other:?}"),
    }
}

#[test]
fn a_self_loop_is_a_cycle_of_one() {
    let diagram = tracks("a b", &[("a", "a"), ("a", "b")]);

    assert_eq!("a", cycles(&diagram, 1000));
}

#[test]
fn two_cycles_sharing_a_member_are_each_found_once() {
    let diagram = tracks("a b c", &[("a", "b"), ("b", "a"), ("b", "c"), ("c", "b")]);

    assert_eq!("a b, b c", cycles(&diagram, 1000));
}

#[test]
fn a_cycle_starts_at_its_member_that_comes_first_in_model_order() {
    // The relations are declared from b, so the cycle is entered at b.
    let diagram = tracks("a b c", &[("b", "c"), ("c", "a"), ("a", "b")]);

    assert_eq!("a b c", cycles(&diagram, 1000));
}

#[test]
fn the_cycles_are_ordered_by_first_member_then_lexicographically_by_member_order_not_by_relation_order() {
    // The relations through c are declared before those through b, and d before both.
    let diagram = tracks(
        "a b c d",
        &[
            ("d", "b"),
            ("b", "d"),
            ("a", "c"),
            ("c", "a"),
            ("c", "b"),
            ("b", "c"),
            ("a", "b"),
            ("b", "a"),
        ],
    );

    // a b before a b c (a prefix first), a b c before a c, a c before a c b; then b's.
    assert_eq!("a b, a b c, a c, a c b, b c, b d", cycles(&diagram, 1000));
}

#[test]
fn the_bound_keeps_the_first_cycles_in_order_and_says_that_it_cut() {
    let diagram = tracks("a b c", &[("c", "a"), ("a", "c"), ("b", "a"), ("a", "b"), ("c", "c")]);

    assert_eq!("a b, a c, c", cycles(&diagram, 3));
    assert!(!truncated(&diagram, 3));
    assert_eq!("a b, a c", cycles(&diagram, 2));
    assert!(truncated(&diagram, 2));
    assert_eq!("", cycles(&diagram, 0));
    assert!(truncated(&diagram, 0));
}

#[test]
fn a_diagram_without_cycles_is_not_truncated() {
    let diagram = tracks("a b c", &[("a", "b"), ("b", "c")]);

    assert_eq!("", cycles(&diagram, 0));
    assert!(!truncated(&diagram, 0));
}

#[test]
fn the_cycles_follow_the_type_and_its_subtypes_and_no_other_relation() {
    // A Branch is a Track, a Road is not, and a relation whose end names nothing is not walked.
    let diagram = diagram_of(
        "a b c",
        &[
            ("a", "b", "Track"),
            ("b", "a", "Branch"),
            ("b", "c", "Road"),
            ("c", "b", "Track"),
        ],
    );
    let c = diagram.element_by_id("c");
    diagram
        .add_relation("Track", "dangling", c.as_ref(), None, Map::default())
        .unwrap();

    assert_eq!("a b", cycles(&diagram, 1000));
}

#[test]
fn a_cycle_is_found_again_after_the_diagram_changes() {
    let diagram = tracks("a b", &[("a", "b")]);
    let before = cycles(&diagram, 1000);

    let (b, a) = (diagram.element_by_id("b"), diagram.element_by_id("a"));
    diagram
        .add_relation("Track", "back", b.as_ref(), a.as_ref(), Map::default())
        .unwrap();
    let after = cycles(&diagram, 1000);

    assert_eq!(("".to_owned(), "a b".to_owned()), (before, after));
}

#[test]
fn two_nodes_with_one_id_are_two_members_and_nothing_throws() {
    // A reading can hold a duplicated id (DISL 11.5.4); the search goes by element, not by id.
    let diagram = DislDiagram::new(&specification());
    let first = diagram.add_node("Station", "a", Map::default()).unwrap();
    let second = diagram.add_node("Station", "a", Map::default()).unwrap();
    diagram
        .add_relation("Track", "there", Some(&first), Some(&second), Map::default())
        .unwrap();
    diagram
        .add_relation("Track", "back", Some(&second), Some(&first), Map::default())
        .unwrap();

    assert_eq!("a a", cycles(&diagram, 1000));
}

#[test]
fn a_negative_bound_is_an_error() {
    let diagram = tracks("a", &[("a", "a")]);

    assert!(evaluate(&diagram, "diagram.cycles('Track', -1)").is_err());
}

#[test]
fn the_cost_is_the_graph_times_the_bound() {
    // A budget below (nodes + relations) × (max + 1) refuses the call however few cycles there are.
    let specification = loaded(&with(
        r#""language": { "id": "org.example.test", "version": "1.0.0", "limits": { "celCost": 2000 } },
        "metamodel": { "types": { "Station": {} }, "relations": { "Track": { "source": "Station", "target": "Station" } } }"#,
    ));
    let diagram = DislDiagram::new(&specification);
    let a = diagram.add_node("Station", "a", Map::default()).unwrap();
    diagram
        .add_relation("Track", "loop", Some(&a), Some(&a), Map::default())
        .unwrap();
    let program = specification
        .environment(contexts::CONSTRAINT, &[])
        .compile("diagram.cycles('Track', 1000).size()")
        .unwrap();

    let value = program.evaluate(&vars([
        ("self", diagram.to_value()),
        ("diagram", diagram.to_value()),
        ("env", Value::map(Map::default())),
    ]));

    assert!(value.is_err());
}
