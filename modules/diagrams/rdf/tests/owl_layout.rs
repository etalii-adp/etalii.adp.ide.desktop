//! The ontology layout (etalii.adp spec 030, task T083), after the standalone host's `OwlLayout.Tests.cs` at
//! `8b6ccfc`: subclass-depth columns with cycle collapse, satellites beside their owners, the individuals band,
//! determinism, the overlay boundary, and no overlaps on the real OWL-Time ontology.

mod common;

use adp_diagram_rdf::owl::layout::{apply, is_positionable, positions, size_of};
use adp_diagram_rdf::owl::projection::{OwlEdgeKind, OwlGraph, project};
use adp_diagram_rdf::reading::Positions;
use common::{example, model};

const PRELUDE: &str = "@prefix : <http://example.org/t#> .\n@prefix owl: <http://www.w3.org/2002/07/owl#> .\n@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\n";

fn graph(body: &str) -> OwlGraph {
    project(&model(&format!("{PRELUDE}{body}")), 1000)
}

fn at(positions: &Positions, id: &str) -> (f64, f64) {
    positions[id]
}

#[test]
fn classes_fall_into_columns_by_asserted_depth_roots_left() {
    let graph = graph(
        ":Leaf a owl:Class ; rdfs:subClassOf :Middle .\n:Middle a owl:Class ; rdfs:subClassOf :Root .\n:Root a owl:Class .\n",
    );
    let p = positions(&graph);
    let root = at(&p, "res:http://example.org/t#Root");
    let middle = at(&p, "res:http://example.org/t#Middle");
    let leaf = at(&p, "res:http://example.org/t#Leaf");
    assert!(root.0 < middle.0);
    assert!(middle.0 < leaf.0);
}

#[test]
fn a_subclass_cycle_collapses_to_one_column_and_terminates() {
    let graph = graph(
        ":A a owl:Class ; rdfs:subClassOf :B .\n:B a owl:Class ; rdfs:subClassOf :A .\n:Below a owl:Class ; rdfs:subClassOf :A .\n",
    );
    let p = positions(&graph);
    assert_eq!(
        at(&p, "res:http://example.org/t#A").0,
        at(&p, "res:http://example.org/t#B").0
    );
    assert!(at(&p, "res:http://example.org/t#Below").0 > at(&p, "res:http://example.org/t#A").0);
}

#[test]
fn satellites_stack_beside_their_owner_and_bands_hold_the_rest() {
    let graph = graph(
        "<http://example.org/t> a owl:Ontology .\n:A a owl:Class ;\n    rdfs:subClassOf [ a owl:Restriction ; owl:onProperty :p ; owl:someValuesFrom :B ] .\n:B a owl:Class .\n:hasName a owl:DatatypeProperty ; rdfs:domain :A ; rdfs:range <http://www.w3.org/2001/XMLSchema#string> .\n:a1 a :A .\n",
    );
    let p = positions(&graph);
    assert_eq!(at(&p, "res:http://example.org/t"), (0.0, 0.0));
    let owner = at(&p, "res:http://example.org/t#A");
    let expressions: Vec<(f64, f64)> = p
        .iter()
        .filter(|(id, _)| id.starts_with("expr:"))
        .map(|(_, p)| *p)
        .collect();
    assert_eq!(expressions.len(), 1);
    let expression = expressions[0];
    let datatype = at(&p, "res:http://www.w3.org/2001/XMLSchema#string");
    assert!(expression.0 > owner.0);
    assert!(expression.1 > owner.1);
    assert!(datatype.0 > owner.0);
    let individual = at(&p, "res:http://example.org/t#a1");
    let lowest_class = owner.1.max(at(&p, "res:http://example.org/t#B").1);
    assert!(individual.1 > lowest_class);
    for node in &graph.nodes {
        assert!(p.contains_key(&node.id));
    }
}

#[test]
fn the_layout_is_deterministic() {
    let body = ":A a owl:Class ; rdfs:subClassOf :B .\n:B a owl:Class .\n:C a owl:Class ; rdfs:subClassOf [ a owl:Restriction ; owl:onProperty :p ; owl:someValuesFrom :A ] .\n";
    assert_eq!(positions(&graph(body)), positions(&graph(body)));
}

#[test]
fn the_overlay_wins_for_iri_derived_ids_and_never_applies_to_expressions() {
    let graph = graph(
        ":A a owl:Class ;\n    rdfs:subClassOf [ a owl:Restriction ; owl:onProperty :p ; owl:someValuesFrom :B ] .\n:B a owl:Class .\n",
    );
    let computed = positions(&graph);
    let expression_id = computed.keys().find(|id| id.starts_with("expr:")).unwrap().clone();
    let mut stored = Positions::new();
    stored.insert("res:http://example.org/t#A".to_owned(), (999.0, 888.0));
    stored.insert(expression_id.clone(), (111.0, 222.0));
    let merged = apply(computed.clone(), &stored);
    assert_eq!(merged["res:http://example.org/t#A"], (999.0, 888.0));
    assert_eq!(merged[&expression_id], computed[&expression_id]);
    assert!(!is_positionable(&expression_id));
    assert!(is_positionable("res:http://example.org/t#A"));
    assert!(is_positionable("ind:http://example.org/t#A"));
}

fn owl_time() -> OwlGraph {
    project(&model(&example("owl-time/owl-time.ttl")), 1000)
}

#[test]
fn no_two_elements_overlap_on_a_real_ontology() {
    let graph = owl_time();
    let p = positions(&graph);
    let boxes: Vec<(&str, f64, f64, f64, f64)> = graph
        .nodes
        .iter()
        .map(|n| {
            let (w, h) = size_of(n);
            let (x, y) = p
                .get(&n.id)
                .copied()
                .unwrap_or_else(|| panic!("{} was not placed", n.id));
            (n.id.as_str(), x, y, x + w, y + h)
        })
        .collect();
    let mut collisions = Vec::new();
    for i in 0..boxes.len() {
        for j in i + 1..boxes.len() {
            let a = boxes[i];
            let b = boxes[j];
            if a.1 < b.3 && b.1 < a.3 && a.2 < b.4 && b.2 < a.4 {
                collisions.push(format!("{} overlaps {}", a.0, b.0));
            }
        }
    }
    assert!(
        collisions.is_empty(),
        "{}",
        collisions.iter().take(8).cloned().collect::<Vec<_>>().join("; ")
    );
}

#[test]
fn a_subclass_edge_runs_between_neighbouring_columns_rather_than_across_the_canvas() {
    let graph = owl_time();
    let p = positions(&graph);
    let drops: Vec<f64> = graph
        .edges
        .iter()
        .filter(|e| e.kind == OwlEdgeKind::Subclass && p.contains_key(&e.from_id) && p.contains_key(&e.to_id))
        .map(|e| (p[&e.from_id].1 - p[&e.to_id].1).abs())
        .collect();
    assert!(!drops.is_empty());
    let average = drops.iter().sum::<f64>() / drops.len() as f64;
    assert!(average < 900.0, "subclass edges average a {average:.0} unit drop");
}
