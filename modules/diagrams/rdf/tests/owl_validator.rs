//! The ontology validator (etalii.adp spec 030, task T083), after the standalone host's `OwlValidator.Tests.cs` at
//! `8b6ccfc`: each finding firing on its fixture, all quiet on a clean file, and the family's rules delegated.

mod common;

use adp_diagram::{DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity};
use adp_diagram_rdf::owl::validator::{self, *};
use common::model;

const PRELUDE: &str = "@prefix : <http://example.org/t#> .\n@prefix owl: <http://www.w3.org/2002/07/owl#> .\n@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\n";

fn judge(body: &str) -> Vec<DiagramProblem> {
    validator::judge(&model(&format!("{PRELUDE}{body}")))
}

fn single(problems: Vec<DiagramProblem>) -> DiagramProblem {
    assert_eq!(problems.len(), 1, "{problems:?}");
    problems.into_iter().next().unwrap()
}

#[test]
fn a_clean_ontology_reports_nothing() {
    let problems = judge(
        ":Pizza a owl:Class .\n:Topping a owl:Class .\n:Vegetarian a owl:Class ;\n    rdfs:subClassOf :Pizza ;\n    rdfs:subClassOf [ a owl:Restriction ; owl:onProperty :hasTopping ; owl:allValuesFrom :Topping ] .\n:hasTopping a owl:ObjectProperty ; rdfs:domain :Pizza ; rdfs:range :Topping .\n:m a :Pizza .\n",
    );
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn a_subclass_cycle_warns_naming_its_members() {
    let problem = single(judge(
        ":A a owl:Class ; rdfs:subClassOf :B .\n:B a owl:Class ; rdfs:subClassOf :A .\n",
    ));
    assert_eq!(problem.rule_id, SUBCLASS_CYCLE_RULE_ID);
    assert_eq!(problem.severity, DiagramProblemSeverity::Warning);
    assert!(problem.message.contains(":A"));
    assert!(problem.message.contains(":B"));
}

#[test]
fn a_malformed_expression_is_an_error_with_its_line() {
    let problem = single(judge(
        ":A a owl:Class ;\n    rdfs:subClassOf [ a owl:Restriction ; owl:someValuesFrom :B ] .\n:B a owl:Class .\n",
    ));
    assert_eq!(problem.rule_id, MALFORMED_EXPRESSION_RULE_ID);
    assert_eq!(problem.severity, DiagramProblemSeverity::Error);
    assert!(matches!(problem.location, Some(DiagramProblemLocation::Line(n)) if n > 0));
}

#[test]
fn an_undeclared_property_is_named_once() {
    let problem = single(judge(":A a owl:Class .\n:p rdfs:domain :A .\n:p rdfs:range :A .\n"));
    assert_eq!(problem.rule_id, UNDECLARED_PROPERTY_RULE_ID);
    assert_eq!(problem.severity, DiagramProblemSeverity::Info);
    assert!(problem.message.contains(":p"));
    assert!(problem.message.contains("not fetched"));
}

#[test]
fn a_deprecated_entity_still_referenced_is_reported() {
    let problem = single(judge(
        ":Old a owl:Class ; owl:deprecated true .\n:New a owl:Class ; rdfs:subClassOf :Old .\n",
    ));
    assert_eq!(problem.rule_id, DEPRECATED_REFERENCE_RULE_ID);
    assert_eq!(problem.severity, DiagramProblemSeverity::Info);
    assert!(problem.message.contains(":Old"));
}

#[test]
fn imports_are_named_as_not_fetched() {
    let problem = single(judge(
        "<http://example.org/t> a owl:Ontology ;\n    owl:imports <http://example.org/base> .\n",
    ));
    assert_eq!(problem.rule_id, IMPORTS_NOT_FETCHED_RULE_ID);
    assert_eq!(problem.severity, DiagramProblemSeverity::Info);
    assert!(problem.message.contains("http://example.org/base"));
    assert!(problem.message.contains("not fetched"));
}

#[test]
fn an_unparseable_file_carries_the_family_finding_exactly_once() {
    let problem = single(validator::validate("not turtle at all {{{"));
    assert_eq!(problem.rule_id, adp_diagram_rdf::validator::UNPARSEABLE_RULE_ID);
}
