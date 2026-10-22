//! `diagram.knots(relType)` (DISL section 12.4; the standalone host's `DiagramKnotsTests` at `c9af7c5`): the strongly
//! connected components over the relations of a type and its subtypes that have more than one member or a self-loop,
//! members in model order, components ordered by their first member.

mod common;

use adp_cel::{Map, Value};
use adp_disl::{DislDiagram, DislEnv, DislSpecification, contexts};
use common::{loaded, vars, with};

fn specification() -> DislSpecification {
    loaded(&with(
        r#""metamodel": {
          "types": { "Station": {} },
          "relations": {
            "Track": { "source": "Station", "target": "Station" },
            "Branch": { "extends": "Track" },
            "Road": { "source": "Station", "target": "Station" } } }"#,
    ))
}

fn diagram(stations: &str, relations: &[(&str, &str, &str)]) -> DislDiagram {
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

fn knots(diagram: &DislDiagram, type_name: &str) -> Vec<String> {
    let value = diagram
        .specification()
        .environment(contexts::CONSTRAINT, &[])
        .compile(&format!("diagram.knots('{type_name}')"))
        .unwrap()
        .evaluate(&vars([
            ("self", diagram.to_value()),
            ("diagram", diagram.to_value()),
            ("env", Value::map(DislEnv::default().to_cel())),
        ]))
        .unwrap();
    value
        .as_list()
        .unwrap()
        .iter()
        .map(|knot| {
            knot.as_list()
                .unwrap()
                .iter()
                .map(|member| adp_disl::DislElement::from_value(member).unwrap().id())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

#[test]
fn two_cycles_sharing_a_member_are_one_knot_in_model_order() {
    // e -> b -> e and b -> d -> b share b; a stands apart.
    let diagram = diagram(
        "a b c d e",
        &[
            ("e", "b", "Track"),
            ("b", "e", "Track"),
            ("b", "d", "Track"),
            ("d", "b", "Track"),
            ("a", "c", "Track"),
        ],
    );

    assert_eq!(vec!["b d e"], knots(&diagram, "Track"));
}

#[test]
fn a_self_loop_is_a_knot_of_one_and_a_lone_node_is_none() {
    let diagram = diagram("a b c", &[("c", "c", "Track"), ("a", "b", "Track")]);

    assert_eq!(vec!["c"], knots(&diagram, "Track"));
}

#[test]
fn knots_are_ordered_by_their_first_member_over_the_type_and_its_subtypes_only() {
    let diagram = diagram(
        "a b c d",
        &[
            ("d", "c", "Track"),
            ("c", "d", "Branch"),
            ("b", "a", "Track"),
            ("a", "b", "Track"),
            ("a", "d", "Road"),
            ("d", "a", "Road"),
        ],
    );

    assert_eq!(vec!["a b", "c d"], knots(&diagram, "Track"));
    assert_eq!(vec!["a d"], knots(&diagram, "Road"));
}

#[test]
fn a_knot_found_first_is_still_listed_after_one_whose_first_member_comes_earlier() {
    // The search from a finishes b and c before a itself.
    let diagram = diagram(
        "a b c",
        &[
            ("a", "a", "Track"),
            ("a", "b", "Track"),
            ("b", "c", "Track"),
            ("c", "b", "Track"),
        ],
    );

    assert_eq!(vec!["a", "b c"], knots(&diagram, "Track"));
}
