//! The Manchester-style expression renderer (etalii.adp spec 030, task T083), after the standalone host's
//! `ExpressionRenderer.Tests.cs` at `8b6ccfc`: every construct's documented form, the depth-2 elision against the
//! uncapped form, the cycle guard, and the malformed short-circuits.

mod common;

use adp_diagram_rdf::model::RdfTerm;
use adp_diagram_rdf::owl::expression::{self, CANVAS_DEPTH, ExpressionText};
use adp_diagram_rdf::owl::projection::{self};
use adp_diagram_rdf::vocabulary::owl;
use common::model;

const PRELUDE: &str = "@prefix : <http://example.org/t#> .\n@prefix owl: <http://www.w3.org/2002/07/owl#> .\n@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\n";

/// Renders the blank object of :A's subClassOf axiom in `body`.
fn render(body: &str, max_depth: Option<usize>) -> ExpressionText {
    let model = model(&format!("{PRELUDE}{body}"));
    let roots: Vec<&RdfTerm> = model
        .triples
        .iter()
        .filter(|t| t.subject.is_iri("http://example.org/t#A") && t.predicate.iri == owl::SUB_CLASS_OF)
        .map(|t| &t.object)
        .collect();
    assert_eq!(roots.len(), 1);
    expression::render(roots[0], &model, max_depth)
}

#[test]
fn every_construct_renders_its_documented_form() {
    let cases = [
        (
            "[ a owl:Restriction ; owl:onProperty :p ; owl:someValuesFrom :B ]",
            "∃ :p.:B",
        ),
        (
            "[ a owl:Restriction ; owl:onProperty :p ; owl:allValuesFrom :B ]",
            "∀ :p.:B",
        ),
        ("[ a owl:Restriction ; owl:onProperty :p ; owl:hasValue :b ]", "∋ :p.:b"),
        ("[ a owl:Restriction ; owl:onProperty :p ; owl:hasValue 42 ]", "∋ :p.42"),
        (
            "[ a owl:Restriction ; owl:onProperty :p ; owl:cardinality 1 ]",
            "= 1 :p",
        ),
        (
            "[ a owl:Restriction ; owl:onProperty :p ; owl:minCardinality 2 ]",
            "≥ 2 :p",
        ),
        (
            "[ a owl:Restriction ; owl:onProperty :p ; owl:maxCardinality 3 ]",
            "≤ 3 :p",
        ),
        (
            "[ a owl:Restriction ; owl:onProperty :p ; owl:qualifiedCardinality 1 ; owl:onClass :B ]",
            "= 1 :p.:B",
        ),
        (
            "[ a owl:Restriction ; owl:onProperty :p ; owl:minQualifiedCardinality 2 ; owl:onDataRange xsd:integer ]",
            "≥ 2 :p.xsd:integer",
        ),
        ("[ a owl:Class ; owl:unionOf ( :B :C ) ]", "(:B ∪ :C)"),
        ("[ a owl:Class ; owl:intersectionOf ( :B :C ) ]", "(:B ∩ :C)"),
        ("[ a owl:Class ; owl:complementOf :B ]", "¬:B"),
        ("[ a owl:Class ; owl:oneOf ( :b :c ) ]", "{:b, :c}"),
    ];
    for (expression, expected) in cases {
        let rendered = render(&format!(":A rdfs:subClassOf {expression} ."), None);
        assert_eq!(rendered.text, expected, "{expression}");
        assert!(!rendered.elided);
        assert!(!rendered.cyclic);
    }
}

#[test]
fn a_named_filler_prefers_its_label() {
    let rendered = render(
        ":A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty :p ; owl:someValuesFrom :B ] .\n:B rdfs:label \"Topping\" .\n",
        None,
    );
    assert_eq!(rendered.text, "∃ :p.Topping");
}

const DEEP: &str = ":A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty :p ; owl:someValuesFrom\n    [ a owl:Class ; owl:unionOf ( :B\n        [ a owl:Restriction ; owl:onProperty :q ; owl:allValuesFrom\n            [ a owl:Class ; owl:intersectionOf ( :C :D ) ] ] ) ] ] .\n";

#[test]
fn deep_nesting_elides_at_the_canvas_cap_and_the_uncapped_form_does_not() {
    let capped = render(DEEP, Some(CANVAS_DEPTH));
    let uncapped = render(DEEP, None);
    assert_eq!(capped.text, "∃ :p.(:B ∪ …)");
    assert!(capped.elided);
    assert_eq!(uncapped.text, "∃ :p.(:B ∪ ∀ :q.(:C ∩ :D))");
    assert!(!uncapped.elided);
}

#[test]
fn a_cyclic_structure_renders_the_guard_instead_of_recursing_forever() {
    let rendered = render(
        ":A rdfs:subClassOf _:r . _:r a owl:Restriction ; owl:onProperty :p ; owl:someValuesFrom _:r .",
        None,
    );
    assert_eq!(rendered.text, "∃ :p.…");
    assert!(rendered.cyclic);
}

#[test]
fn malformed_structures_short_circuit_to_the_problem_label() {
    for (expression, expected) in [
        ("[ a owl:Restriction ; owl:someValuesFrom :B ]", "∃ ?.:B"),
        ("[ a owl:Restriction ; owl:onProperty :p ]", "? :p"),
        ("[ a owl:Class ]", "? ?"),
    ] {
        assert_eq!(
            render(&format!(":A rdfs:subClassOf {expression} ."), None).text,
            expected
        );
    }
}

#[test]
fn the_projection_carries_the_rendered_label_and_the_elision_flag() {
    let model = model(&format!(
        "{PRELUDE}:A a owl:Class ;\n    rdfs:subClassOf [ a owl:Restriction ; owl:onProperty :p ; owl:someValuesFrom\n        [ a owl:Class ; owl:unionOf ( :B\n            [ a owl:Restriction ; owl:onProperty :q ; owl:allValuesFrom\n                [ a owl:Class ; owl:intersectionOf ( :C :D ) ] ] ) ] ] .\n"
    ));
    let graph = projection::project(&model, 1000);
    let id = format!("expr:http://example.org/t#A|{}|0", owl::SUB_CLASS_OF);
    let root = graph.nodes.iter().find(|n| n.id == id).unwrap();
    assert_eq!(root.display, "∃ :p.(:B ∪ …)");
    assert!(root.elided);
    assert!(!root.malformed);
}
