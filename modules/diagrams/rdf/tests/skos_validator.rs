//! What makes a vocabulary lie (etalii.adp spec 030, task T083), ported from the standalone host's
//! `SkosValidator.Tests.cs` at `8b6ccfc`.

use std::fs;
use std::path::Path;

use adp_diagram::{DiagramProblem, DiagramProblemSeverity};
use adp_diagram_rdf::skos::validator::{
    CYCLE_RULE_ID, DUPLICATE_PREF_LABEL_RULE_ID, MISPLACED_HEADER_RULE_ID, MISSING_PREF_LABEL_RULE_ID,
    NONCONCEPT_RULE_ID, RELATED_OVERLAP_RULE_ID, UNFILED_CONCEPT_RULE_ID, XL_LABELS_RULE_ID, validate,
};
use adp_diagram_rdf::validator::UNPARSEABLE_RULE_ID;

const PRELUDE: &str = "@prefix skos: <http://www.w3.org/2004/02/skos/core#> .\n@prefix ex: <http://example.org/> .\n\n";

fn problems(turtle: &str, registration: Option<&Path>) -> Vec<DiagramProblem> {
    validate(&format!("{PRELUDE}{turtle}"), registration)
}

fn with<'a>(problems: &'a [DiagramProblem], rule: &str) -> Vec<&'a DiagramProblem> {
    problems.iter().filter(|p| p.rule_id == rule).collect()
}

#[test]
fn a_clean_vocabulary_reports_nothing() {
    let found = problems(
        "ex:scheme a skos:ConceptScheme ; skos:prefLabel \"Scheme\"@en ; skos:hasTopConcept ex:top .\n\
         ex:top a skos:Concept ; skos:prefLabel \"Top\"@en ; skos:topConceptOf ex:scheme .\n\
         ex:leaf a skos:Concept ; skos:prefLabel \"Leaf\"@en ; skos:inScheme ex:scheme ; skos:broader ex:top .\n",
        None,
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn an_unparseable_file_is_that_one_finding_and_nothing_else() {
    let found = problems("ex:a skos:broader", None);
    assert_eq!(1, found.len());
    assert_eq!(UNPARSEABLE_RULE_ID, found[0].rule_id);
}

#[test]
fn a_cycle_is_one_error_naming_its_concepts_from_the_layouts_own_detection() {
    let found = problems(
        "ex:a a skos:Concept ; skos:prefLabel \"A\"@en ; skos:broader ex:b .\n\
         ex:b a skos:Concept ; skos:prefLabel \"B\"@en ; skos:broader ex:a .\n",
        None,
    );
    let cycles = with(&found, CYCLE_RULE_ID);
    assert_eq!(1, cycles.len());
    assert_eq!(DiagramProblemSeverity::Error, cycles[0].severity);
    assert!(cycles[0].message.contains('A'));
    assert!(cycles[0].message.contains('B'));
}

#[test]
fn duplicate_preferred_labels_in_one_language_are_an_error() {
    let found = problems(
        "ex:a a skos:Concept ; skos:prefLabel \"Tea\"@en ; skos:prefLabel \"Cuppa\"@en .\n",
        None,
    );
    let duplicates = with(&found, DUPLICATE_PREF_LABEL_RULE_ID);
    assert_eq!(1, duplicates.len());
    assert!(duplicates[0].message.contains("'en'"));
}

#[test]
fn related_across_a_direct_hierarchy_link_warns_and_says_what_is_not_checked() {
    let found = problems(
        "ex:a a skos:Concept ; skos:prefLabel \"A\"@en .\n\
         ex:b a skos:Concept ; skos:prefLabel \"B\"@en ; skos:broader ex:a ; skos:related ex:a .\n",
        None,
    );
    let overlaps = with(&found, RELATED_OVERLAP_RULE_ID);
    assert_eq!(1, overlaps.len());
    assert!(overlaps[0].message.contains("direct case"));
}

#[test]
fn a_missing_preferred_label_warns_and_an_unfiled_concept_informs() {
    let found = problems("ex:orphan a skos:Concept .\n", None);
    assert_eq!(1, with(&found, MISSING_PREF_LABEL_RULE_ID).len());
    assert_eq!(1, with(&found, UNFILED_CONCEPT_RULE_ID).len());
}

#[test]
fn a_concept_reachable_from_a_top_is_not_reported_unfiled() {
    let found = problems(
        "ex:scheme a skos:ConceptScheme ; skos:prefLabel \"S\"@en ; skos:hasTopConcept ex:top .\n\
         ex:top a skos:Concept ; skos:prefLabel \"Top\"@en ; skos:topConceptOf ex:scheme .\n\
         ex:child a skos:Concept ; skos:prefLabel \"Child\"@en ; skos:broader ex:top .\n",
        None,
    );
    assert!(with(&found, UNFILED_CONCEPT_RULE_ID).is_empty());
}

#[test]
fn skos_xl_labels_are_one_info_per_file() {
    let found = problems(
        "@prefix skosxl: <http://www.w3.org/2008/05/skos-xl#> .\n\
         ex:a a skos:Concept ; skos:prefLabel \"A\"@en ; skosxl:prefLabel ex:aLabel ; skos:inScheme ex:s .\n\
         ex:b a skos:Concept ; skos:prefLabel \"B\"@en ; skosxl:altLabel ex:bLabel ; skos:inScheme ex:s .\n\
         ex:s a skos:ConceptScheme ; skos:prefLabel \"S\"@en .\n",
        None,
    );
    assert_eq!(1, with(&found, XL_LABELS_RULE_ID).len());
}

#[test]
fn a_hierarchy_end_that_is_not_a_concept_warns() {
    let found = problems(
        "ex:a a skos:Concept ; skos:prefLabel \"A\"@en ; skos:inScheme ex:s ; skos:broader ex:untyped .\n\
         ex:s a skos:ConceptScheme ; skos:prefLabel \"S\"@en .\n",
        None,
    );
    let warnings = with(&found, NONCONCEPT_RULE_ID);
    assert!(!warnings.is_empty());
    assert!(warnings.iter().any(|p| p.message.contains("untyped")));
}

#[test]
fn a_concept_past_the_drawing_budget_is_still_typed() {
    let mut lines = vec![
        "ex:s a skos:ConceptScheme ; skos:prefLabel \"S\"@en ; skos:hasTopConcept ex:top .".to_owned(),
        "ex:top a skos:Concept ; skos:prefLabel \"Top\"@en ; skos:topConceptOf ex:s .".to_owned(),
    ];
    for i in 0..1005 {
        lines.push(format!(
            "ex:c{i} a skos:Concept ; skos:prefLabel \"C{i}\"@en ; skos:inScheme ex:s ; skos:broader ex:top ."
        ));
    }
    let found = problems(&lines.join("\n"), None);
    assert!(with(&found, NONCONCEPT_RULE_ID).is_empty());
}

#[test]
fn a_misplaced_language_header_is_named_with_its_line() {
    let folder = tempfile::tempdir().unwrap();
    let registration = folder.path().join("scheme.adp");
    fs::write(&registration, "w3c/skos\nlanguage: nl\nbody: scheme.ttl\n").unwrap();
    let found = problems(
        "ex:s a skos:ConceptScheme ; skos:prefLabel \"S\"@en .\n",
        Some(&registration),
    );
    let misplaced = with(&found, MISPLACED_HEADER_RULE_ID);
    assert_eq!(1, misplaced.len());
    assert!(misplaced[0].message.contains("line 2"));
}
