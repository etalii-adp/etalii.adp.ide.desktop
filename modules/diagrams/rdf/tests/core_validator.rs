//! The family's findings: facts about the text with their line, no inference, no network, and nothing at all for a
//! clean file. Ported from the standalone host's `RdfValidator.Tests.cs` at `8b6ccfc`; the registration of one
//! validator per reading is each reading's `problems` here (`AddRdf_RegistersTheValidatorsPerReading` is checked by
//! the readings opening through the module).

use adp_diagram::{DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity};
use adp_diagram_rdf::validator::{
    self, DUPLICATE_PREFIX_RULE_ID, LANGUAGE_TAG_RULE_ID, RELATIVE_IRI_RULE_ID, UNKNOWN_DATATYPE_RULE_ID,
    UNPARSEABLE_RULE_ID,
};

fn single<'a>(problems: &'a [DiagramProblem], rule_id: &str) -> &'a DiagramProblem {
    let found: Vec<_> = problems.iter().filter(|p| p.rule_id == rule_id).collect();
    assert_eq!(found.len(), 1, "{problems:?}");
    found[0]
}

#[test]
fn a_clean_file_reports_nothing() {
    let problems = validator::validate(
        "@prefix ex: <http://example.org/> .\r\nex:alice ex:name \"Alice\"@en ;\r\n    ex:age \"42\"^^<http://www.w3.org/2001/XMLSchema#integer> .\r\n",
    );
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn an_unparseable_file_reports_once_with_its_line() {
    let problems = validator::validate("@prefix ex: <http://example.org/> .\r\nex:alice ex:knows\r\n");

    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].rule_id, UNPARSEABLE_RULE_ID);
    assert_eq!(problems[0].severity, DiagramProblemSeverity::Error);
    assert!(matches!(problems[0].location, Some(DiagramProblemLocation::Line(_))));
}

#[test]
fn a_duplicate_prefix_is_warned_at_the_redeclaration() {
    let problems = validator::validate(
        "@prefix ex: <http://example.org/> .\r\n@prefix ex: <http://elsewhere.example/> .\r\nex:a ex:b ex:c .\r\n",
    );

    let problem = single(&problems, DUPLICATE_PREFIX_RULE_ID);
    assert_eq!(problem.severity, DiagramProblemSeverity::Warning);
    assert_eq!(problem.location, Some(DiagramProblemLocation::Line(2)));
}

#[test]
fn relative_iris_without_a_base_are_warned_once() {
    let problems =
        validator::validate("@prefix ex: <http://example.org/> .\r\n<one> ex:p <two> .\r\n<three> ex:p <four> .\r\n");

    let problem = single(&problems, RELATIVE_IRI_RULE_ID);
    assert_eq!(problem.severity, DiagramProblemSeverity::Warning);
}

#[test]
fn with_a_base_relative_iris_are_fine() {
    let problems = validator::validate(
        "@base <http://example.org/> .\r\n@prefix ex: <http://example.org/ns#> .\r\n<one> ex:p <two> .\r\n",
    );
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn a_malformed_language_tag_is_warned_with_its_line() {
    let problems = validator::validate("@prefix ex: <http://example.org/> .\r\nex:a ex:label \"x\"@en-- .\r\n");

    let problem = single(&problems, LANGUAGE_TAG_RULE_ID);
    assert!(problem.message.contains("en--"));
    assert_eq!(problem.location, Some(DiagramProblemLocation::Line(2)));
}

#[test]
fn an_unknown_xsd_datatype_is_warned_but_foreign_datatypes_are_not() {
    let problems = validator::validate(
        "@prefix ex: <http://example.org/> .\r\nex:a ex:v \"x\"^^<http://www.w3.org/2001/XMLSchema#nope> .\r\nex:a ex:w \"y\"^^ex:ownType .\r\n",
    );

    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0].rule_id, UNKNOWN_DATATYPE_RULE_ID);
    assert!(problems[0].message.contains("nope"));
}
