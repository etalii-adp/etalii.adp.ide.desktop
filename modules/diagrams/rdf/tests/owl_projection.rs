//! The ontology reading's projection (etalii.adp spec 030, task T083), after the standalone host's
//! `OwlProjection.Tests.cs` at `8b6ccfc`: classes, properties as edges, expression nodes on structural ids, punning,
//! anchors, and the neighbourhood-whole budget cut.

mod common;

use std::collections::HashSet;

use adp_diagram_rdf::owl::projection::{OwlEdgeKind, OwlGraph, OwlNode, OwlNodeKind, project};
use common::{fixture, model};

const NS: &str = "http://example.org/pizza#";

fn project_fixture(name: &str) -> OwlGraph {
    project(&model(&fixture(name)), 1000)
}

fn node<'a>(graph: &'a OwlGraph, id: &str) -> &'a OwlNode {
    let found: Vec<&OwlNode> = graph.nodes.iter().filter(|n| n.id == id).collect();
    assert_eq!(found.len(), 1, "{id}");
    found[0]
}

fn has_edge(graph: &OwlGraph, kind: OwlEdgeKind, from: &str, to: &str) -> bool {
    graph
        .edges
        .iter()
        .any(|e| e.kind == kind && e.from_id == from && e.to_id == to)
}

#[test]
fn the_ontology_fixture_projects_the_adopted_vocabulary() {
    let graph = project_fixture("owl-ontology.ttl");

    let headers: Vec<&OwlNode> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == OwlNodeKind::OntologyHeader)
        .collect();
    assert_eq!(headers.len(), 1);
    let header = headers[0];
    assert_eq!(header.display, "Pizza ontology");
    assert!(
        header
            .rows
            .iter()
            .any(|r| r.predicate == "owl:imports" && r.value == "http://example.org/base")
    );
    assert!(
        header
            .rows
            .iter()
            .any(|r| r.predicate == "owl:versionInfo" && r.value == "1.0")
    );

    let pizza = node(&graph, &format!("res:{NS}Pizza"));
    assert_eq!(pizza.kind, OwlNodeKind::Class);
    assert_eq!(pizza.display, "Pizza");

    assert!(
        graph
            .nodes
            .iter()
            .any(|n| n.id == format!("ind:{NS}Pizza") && n.kind == OwlNodeKind::Individual)
    );

    assert!(has_edge(
        &graph,
        OwlEdgeKind::Subclass,
        &format!("res:{NS}Vegetarian"),
        &format!("res:{NS}Pizza")
    ));
    assert!(has_edge(
        &graph,
        OwlEdgeKind::Equivalent,
        &format!("res:{NS}Vegetarian"),
        &format!("res:{NS}VeggiePizza")
    ));
    assert!(has_edge(
        &graph,
        OwlEdgeKind::Disjoint,
        &format!("res:{NS}VegetarianTopping"),
        &format!("res:{NS}MeatTopping")
    ));
    assert!(has_edge(
        &graph,
        OwlEdgeKind::Disjoint,
        &format!("res:{NS}Pizza"),
        &format!("res:{NS}Topping")
    ));

    let has_topping: Vec<_> = graph
        .edges
        .iter()
        .filter(|e| e.kind == OwlEdgeKind::ObjectProperty && e.property_iri == format!("{NS}hasTopping"))
        .collect();
    assert_eq!(has_topping.len(), 1);
    assert_eq!(has_topping[0].from_id, format!("res:{NS}Pizza"));
    assert_eq!(has_topping[0].to_id, format!("res:{NS}Topping"));
    assert!(has_topping[0].label.contains("inverse functional"));
    assert!(has_topping[0].label.contains("inverse of"));

    assert!(
        graph
            .nodes
            .iter()
            .any(|n| n.id == format!("thing:{NS}toppingOf|domain") && n.kind == OwlNodeKind::Thing)
    );
    assert!(graph.edges.iter().any(|e| e.property_iri == format!("{NS}toppingOf")
        && e.from_id == format!("thing:{NS}toppingOf|domain")
        && e.to_id == format!("thing:{NS}toppingOf|range")));

    let xsd_integer = node(&graph, "res:http://www.w3.org/2001/XMLSchema#integer");
    assert_eq!(xsd_integer.kind, OwlNodeKind::Datatype);
    assert!(has_edge(
        &graph,
        OwlEdgeKind::DatatypeProperty,
        &format!("res:{NS}Pizza"),
        &xsd_integer.id
    ));
    assert!(
        graph
            .nodes
            .iter()
            .any(|n| n.id == format!("dt:{NS}hasNote") && n.kind == OwlNodeKind::Datatype && n.display == "Literal")
    );

    let margherita = node(&graph, &format!("res:{NS}Margherita"));
    assert_eq!(margherita.kind, OwlNodeKind::Individual);
    assert!(margherita.badges.iter().any(|b| b == "Pizza"));
    assert!(
        margherita
            .rows
            .iter()
            .any(|r| r.predicate == ":hasCalories" && r.value == "850")
    );
    assert!(has_edge(
        &graph,
        OwlEdgeKind::Assertion,
        &margherita.id,
        &format!("res:{NS}Mozzarella")
    ));

    assert!(node(&graph, &format!("res:{NS}MeatTopping")).deprecated);
    assert!(node(&graph, "res:http://example.org/external#Base").external);
    assert!(!pizza.external);

    assert!(!graph.truncated());
}

#[test]
fn class_expressions_draw_on_structural_ids_attached_to_their_owner() {
    let graph = project_fixture("owl-ontology.ttl");

    let restriction_id = format!("expr:{NS}Vegetarian|http://www.w3.org/2000/01/rdf-schema#subClassOf|1");
    let restriction = node(&graph, &restriction_id);
    assert_eq!(restriction.kind, OwlNodeKind::Restriction);
    assert_eq!(restriction.owner_id, format!("res:{NS}Vegetarian"));
    assert!(!restriction.malformed);
    assert!(restriction.expression_root.is_some());

    assert!(has_edge(
        &graph,
        OwlEdgeKind::Subclass,
        &format!("res:{NS}Vegetarian"),
        &restriction_id
    ));
    assert!(has_edge(
        &graph,
        OwlEdgeKind::Expression,
        &restriction_id,
        &format!("res:{NS}VegetarianTopping")
    ));

    let union_id = format!("expr:{NS}Special|http://www.w3.org/2002/07/owl#equivalentClass|0");
    assert_eq!(node(&graph, &union_id).kind, OwlNodeKind::Operator);
    assert!(
        graph
            .nodes
            .iter()
            .any(|n| n.id == format!("{union_id}/0") && n.kind == OwlNodeKind::Restriction)
    );
    assert!(
        graph
            .nodes
            .iter()
            .any(|n| n.id == format!("{union_id}/0/0") && n.kind == OwlNodeKind::Operator)
    );
    assert!(has_edge(
        &graph,
        OwlEdgeKind::Expression,
        &union_id,
        &format!("res:{NS}Vegetarian")
    ));
    assert!(has_edge(
        &graph,
        OwlEdgeKind::Expression,
        &format!("{union_id}/0/0"),
        "res:http://example.org/external#Smoked"
    ));
}

#[test]
fn a_malformed_restriction_draws_as_a_marked_problem_node() {
    let graph = project_fixture("owl-malformed.ttl");
    let restrictions: Vec<_> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == OwlNodeKind::Restriction)
        .collect();
    assert_eq!(restrictions.len(), 1);
    assert!(restrictions[0].malformed);
}

#[test]
fn structural_ids_are_deterministic_within_a_parse() {
    let first = project_fixture("owl-ontology.ttl");
    let second = project_fixture("owl-ontology.ttl");
    let ids = |g: &OwlGraph| g.nodes.iter().map(|n| n.id.clone()).collect::<Vec<_>>();
    let edge_ids = |g: &OwlGraph| g.edges.iter().map(|e| e.id.clone()).collect::<Vec<_>>();
    assert_eq!(ids(&first), ids(&second));
    assert_eq!(edge_ids(&first), edge_ids(&second));
}

#[test]
fn structural_ids_are_deliberately_unstable_across_an_inserted_axiom() {
    let before = "@prefix : <http://example.org/pizza#> .\n@prefix owl: <http://www.w3.org/2002/07/owl#> .\n@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\n:Vegetarian a owl:Class ;\n    rdfs:subClassOf :Pizza ;\n    rdfs:subClassOf [ a owl:Restriction ; owl:onProperty :hasTopping ; owl:allValuesFrom :VegetarianTopping ] .\n";
    let after = "@prefix : <http://example.org/pizza#> .\n@prefix owl: <http://www.w3.org/2002/07/owl#> .\n@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\n:Vegetarian a owl:Class ;\n    rdfs:subClassOf :Pizza ;\n    rdfs:subClassOf :Food ;\n    rdfs:subClassOf [ a owl:Restriction ; owl:onProperty :hasTopping ; owl:allValuesFrom :VegetarianTopping ] .\n";
    let restriction = |text: &str| {
        let graph = project(&model(text), 1000);
        let found: Vec<String> = graph
            .nodes
            .iter()
            .filter(|n| n.kind == OwlNodeKind::Restriction)
            .map(|n| n.id.clone())
            .collect();
        assert_eq!(found.len(), 1);
        found[0].clone()
    };
    let before_id = restriction(before);
    let after_id = restriction(after);
    assert!(before_id.ends_with("|1"));
    assert!(after_id.ends_with("|2"));
    assert_ne!(before_id, after_id);
}

#[test]
fn the_budget_cut_keeps_class_neighborhoods_whole() {
    let mut text = String::from(
        "@prefix : <http://example.org/big#> .\n@prefix owl: <http://www.w3.org/2002/07/owl#> .\n@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\n",
    );
    for i in 0..5 {
        text.push_str(&format!(":Class{i} a owl:Class ;\n"));
        text.push_str(&format!(
            "    rdfs:subClassOf [ a owl:Restriction ; owl:onProperty :p ; owl:someValuesFrom :Class{i} ] .\n"
        ));
    }
    let graph = project(&model(&text), 5);
    assert!(graph.truncated());
    assert_eq!(graph.shown, 4);
    assert_eq!(graph.total, 10);
    let drawn: HashSet<&str> = graph.nodes.iter().map(|n| n.id.as_str()).collect();
    for node in graph.nodes.iter().filter(|n| !n.owner_id.is_empty()) {
        assert!(drawn.contains(node.owner_id.as_str()));
    }
    for edge in &graph.edges {
        assert!(drawn.contains(edge.from_id.as_str()));
        assert!(drawn.contains(edge.to_id.as_str()));
    }
}
