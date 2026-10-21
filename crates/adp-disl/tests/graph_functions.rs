//! The node-specific graph functions of DISL section 12.4: `n.successors(t)`, `n.predecessors(t)`, `n.reachable(t)`
//! (the transitive successors, excluding the node itself unless it is on a cycle) and `n.inCycle(t)`, each over the
//! relations of a type and its subtypes (section 2.7). The standalone host's evaluator at `8b6ccfc` lacked them, so
//! its loader refused the functional decomposition graph's, the Azure DevOps pipeline's, the Databricks job's and the
//! OWL ontology's specifications; these hold what the prose says of them.

mod common;

use std::sync::LazyLock;

use adp_cel::{Map, Value};
use adp_disl::{DislDiagram, DislLoader, DislSpecification, contexts};
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

/// A diagram with one Station per name, in that order, and a relation of its type per triple.
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

fn evaluate(diagram: &DislDiagram, node: &str, expression: &str) -> Value {
    let node = diagram.element_by_id(node).unwrap();
    diagram
        .specification()
        .environment(contexts::CONSTRAINT, &[])
        .compile(expression)
        .unwrap()
        .evaluate(&vars([
            ("self", node.to_value()),
            ("diagram", diagram.to_value()),
            ("env", Value::map(Map::default())),
        ]))
        .unwrap()
}

/// The ids of the list `expression` gives on `node`, joined by a space.
fn ids(diagram: &DislDiagram, node: &str, expression: &str) -> String {
    match evaluate(diagram, node, &format!("({expression}).map(e, e.id).join(' ')")) {
        Value::String(text) => text,
        other => panic!("not a list of elements: {other:?}"),
    }
}

fn holds(diagram: &DislDiagram, node: &str, expression: &str) -> bool {
    evaluate(diagram, node, expression) == Value::Bool(true)
}

#[test]
fn successors_and_predecessors_are_the_nodes_one_relation_of_the_type_away_each_once() {
    let diagram = diagram_of(
        "a b c d",
        &[
            ("a", "b", "Track"),
            ("a", "c", "Branch"),
            ("a", "b", "Track"),
            ("a", "d", "Road"),
            ("c", "a", "Track"),
        ],
    );

    // A subtype counts, a relation of another type does not, and a node two relations lead to is listed once.
    assert_eq!(ids(&diagram, "a", "self.successors('Track')"), "b c");
    assert_eq!(ids(&diagram, "a", "self.successors('Branch')"), "c");
    assert_eq!(ids(&diagram, "a", "self.successors('Road')"), "d");
    assert_eq!(ids(&diagram, "a", "self.predecessors('Track')"), "c");
    assert_eq!(ids(&diagram, "b", "self.predecessors('Track')"), "a");
    assert_eq!(ids(&diagram, "d", "self.successors('Track')"), "");
}

#[test]
fn reachable_is_every_transitive_successor_without_the_node_itself_off_a_cycle() {
    // a -> b -> c -> d, and a road from d back to a that tracks do not follow.
    let diagram = diagram_of(
        "a b c d",
        &[
            ("a", "b", "Track"),
            ("b", "c", "Branch"),
            ("c", "d", "Track"),
            ("d", "a", "Road"),
        ],
    );

    assert_eq!(ids(&diagram, "a", "self.reachable('Track')"), "b c d");
    assert_eq!(ids(&diagram, "c", "self.reachable('Track')"), "d");
    assert_eq!(ids(&diagram, "d", "self.reachable('Track')"), "");
    // Along roads only, a reaches nothing and d reaches a.
    assert_eq!(ids(&diagram, "d", "self.reachable('Road')"), "a");
    assert!(!holds(&diagram, "a", "self.inCycle('Track')"));
    assert!(holds(
        &diagram,
        "b",
        "self in diagram.elementById('a').reachable('Track')"
    ));
}

#[test]
fn a_node_on_a_cycle_reaches_itself_and_is_in_it() {
    // a -> b -> c -> a, and d hangs off c.
    let diagram = diagram_of(
        "a b c d",
        &[
            ("a", "b", "Track"),
            ("b", "c", "Track"),
            ("c", "a", "Branch"),
            ("c", "d", "Track"),
        ],
    );

    assert_eq!(ids(&diagram, "a", "self.reachable('Track')"), "b c a d");
    for member in ["a", "b", "c"] {
        assert!(holds(&diagram, member, "self.inCycle('Track')"), "{member}");
        assert!(holds(&diagram, member, "self in self.reachable('Track')"), "{member}");
    }
    assert!(!holds(&diagram, "d", "self.inCycle('Track')"));
    // The cycle closes through a Branch: over Branches alone there is none.
    assert!(!holds(&diagram, "a", "self.inCycle('Branch')"));
}

#[test]
fn a_self_loop_is_a_cycle_of_one() {
    let diagram = diagram_of("a b", &[("a", "a", "Track"), ("a", "b", "Track")]);

    assert_eq!(ids(&diagram, "a", "self.reachable('Track')"), "a b");
    assert!(holds(&diagram, "a", "self.inCycle('Track')"));
    assert!(!holds(&diagram, "b", "self.inCycle('Track')"));
}

/// The specifications the standalone host refused for these functions alone now load.
#[test]
fn the_specifications_that_call_them_load() {
    let root = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vendor/etalii.adp/definitions/diagrams"
    );
    for name in [
        "functional-decomposition-graph.dis",
        "azure-devops-pipeline.dis",
        "databricks-job.dis",
        "w3c-owl.dis",
    ] {
        let bytes = std::fs::read(format!("{root}/{name}")).unwrap();
        let result = DislLoader::load_bytes(&bytes, &adp_disl::DislPluginFunctions::none());
        assert!(
            result.specification.is_some(),
            "{name}: {:?}",
            result.errors().collect::<Vec<_>>()
        );
    }
}
