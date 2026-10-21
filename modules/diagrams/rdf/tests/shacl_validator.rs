//! The shapes validator - facts about the shapes file, composed on top of the family's text facts, never a word about
//! whether any data conforms - and the vendored shapes graphs, which must parse, project and validate clean and carry
//! their paperwork. Ported from the standalone host's `ShaclValidator.Tests.cs` and `ShaclExamples.Tests.cs` at
//! `8b6ccfc`.

mod common;

use std::fs;

use adp_diagram::{DiagramProblem, DiagramProblemSeverity};
use adp_diagram_rdf::shacl::projection::{self, DEFAULT_BUDGET, ShaclTargetKind};
use adp_diagram_rdf::shacl::validator::{
    self, DATATYPE_AND_CLASS_RULE_ID, IMPOSSIBLE_COUNTS_RULE_ID, PATH_CARDINALITY_RULE_ID,
    REFERENCE_LEAVES_FILE_RULE_ID, UNKNOWN_TERM_RULE_ID,
};
use adp_diagram_rdf::validator::{DUPLICATE_PREFIX_RULE_ID, UNPARSEABLE_RULE_ID};

const PRELUDE: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\r\n@prefix ex: <http://example.org/> .\r\n@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\r\n";

fn validate(body: &str) -> Vec<DiagramProblem> {
    validator::validate(&format!("{PRELUDE}{body}"))
}

fn single<'a>(problems: &'a [DiagramProblem], rule: &str) -> &'a DiagramProblem {
    let found: Vec<&DiagramProblem> = problems.iter().filter(|p| p.rule_id == rule).collect();
    assert_eq!(found.len(), 1, "{problems:?}");
    found[0]
}

#[test]
fn a_well_formed_shapes_file_reports_nothing() {
    let problems = validate(
        "ex:PersonShape a sh:NodeShape ;\r\n  sh:targetClass ex:Person ;\r\n  sh:property [ sh:path ex:name ; sh:datatype xsd:string ; sh:minCount 1 ; sh:maxCount 1 ] .\n",
    );
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn a_property_shape_without_a_path_is_an_error() {
    let problems = validate("ex:S a sh:NodeShape ; sh:property [ sh:minCount 1 ] .\n");
    let problem = single(&problems, PATH_CARDINALITY_RULE_ID);
    assert_eq!(problem.severity, DiagramProblemSeverity::Error);
    assert!(problem.message.contains("no sh:path"));
}

#[test]
fn a_property_shape_with_two_paths_is_an_error() {
    let problems = validate("ex:S a sh:NodeShape ; sh:property [ sh:path ex:a ; sh:path ex:b ] .\n");
    let problem = single(&problems, PATH_CARDINALITY_RULE_ID);
    assert_eq!(problem.severity, DiagramProblemSeverity::Error);
    assert!(problem.message.contains("2 sh:path"));
}

#[test]
fn a_node_shape_without_a_path_is_not_a_finding() {
    let problems = validate("ex:S a sh:NodeShape ; sh:targetClass ex:Person ; sh:closed true .\n");
    assert!(!problems.iter().any(|p| p.rule_id == PATH_CARDINALITY_RULE_ID));
}

#[test]
fn min_count_above_max_count_is_a_warning() {
    let problems = validate("ex:S a sh:NodeShape ; sh:property [ sh:path ex:p ; sh:minCount 3 ; sh:maxCount 1 ] .\n");
    let problem = single(&problems, IMPOSSIBLE_COUNTS_RULE_ID);
    assert_eq!(problem.severity, DiagramProblemSeverity::Warning);
    assert!(problem.message.contains("nothing can ever conform"));
}

#[test]
fn datatype_and_class_together_is_a_warning() {
    let problems = validate(
        "ex:S a sh:NodeShape ; sh:property [ sh:path ex:p ; sh:datatype xsd:string ; sh:class ex:Person ] .\n",
    );
    assert_eq!(
        single(&problems, DATATYPE_AND_CLASS_RULE_ID).severity,
        DiagramProblemSeverity::Warning
    );
}

#[test]
fn a_misspelled_shacl_term_is_a_warning_naming_it() {
    let problems = validate("ex:S a sh:NodeShape ; sh:property [ sh:path ex:p ; sh:minCoumt 1 ] .\n");
    let problem = single(&problems, UNKNOWN_TERM_RULE_ID);
    assert_eq!(problem.severity, DiagramProblemSeverity::Warning);
    assert!(problem.message.contains("sh:minCoumt"));
}

#[test]
fn every_real_shacl_term_is_accepted() {
    let problems = validate(
        "ex:S a sh:NodeShape ;\r\n  sh:severity sh:Warning ;\r\n  sh:message \"careful\" ;\r\n  sh:deactivated false ;\r\n  sh:closed true ;\r\n  sh:ignoredProperties ( ex:x ) ;\r\n  sh:property [ sh:path [ sh:inversePath ex:parent ] ; sh:nodeKind sh:IRI ; sh:pattern \"^a\" ; sh:flags \"i\" ] ;\r\n  sh:or ( [ sh:datatype xsd:string ] [ sh:datatype xsd:integer ] ) .\n",
    );
    assert!(
        !problems.iter().any(|p| p.rule_id == UNKNOWN_TERM_RULE_ID),
        "{problems:?}"
    );
}

#[test]
fn a_target_naming_an_absent_term_is_not_a_finding() {
    let problems = validate(
        "ex:S a sh:NodeShape ;\r\n  sh:targetClass ex:NowhereDefined ;\r\n  sh:targetNode ex:AlsoAbsent ;\r\n  sh:targetSubjectsOf ex:unseen .\n",
    );
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn no_finding_ever_speaks_of_data_conformance() {
    let problems = validate(
        "ex:S a sh:NodeShape ;\r\n  sh:targetClass ex:Person ;\r\n  sh:property [ sh:path ex:name ; sh:minCount 3 ; sh:maxCount 1 ] .\r\nex:alice a ex:Person .\n",
    );
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0].rule_id, IMPOSSIBLE_COUNTS_RULE_ID);
    for problem in &problems {
        let message = problem.message.to_lowercase();
        assert!(!message.contains("alice") && !message.contains("conforms") && !message.contains("violat"));
    }
}

#[test]
fn a_shape_reference_leaving_the_file_is_an_info_not_a_warning() {
    let problems = validate("ex:S a sh:NodeShape ; sh:node ex:DefinedNextDoor .\n");
    let problem = single(&problems, REFERENCE_LEAVES_FILE_RULE_ID);
    assert_eq!(problem.severity, DiagramProblemSeverity::Info);
    assert!(problem.message.contains("elsewhere, or missing"));
    assert!(!problems.iter().any(|p| p.severity > DiagramProblemSeverity::Info));
}

#[test]
fn a_shape_reference_this_file_describes_is_not_a_finding() {
    let problems = validate("ex:S a sh:NodeShape ; sh:node ex:Other .\r\nex:Other a sh:NodeShape .\n");
    assert!(!problems.iter().any(|p| p.rule_id == REFERENCE_LEAVES_FILE_RULE_ID));
}

#[test]
fn a_target_leaving_the_file_stays_silent_while_a_reference_does_not() {
    let problems = validate("ex:S a sh:NodeShape ; sh:targetClass ex:UnseenClass ; sh:node ex:UnseenShape .\n");
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0].rule_id, REFERENCE_LEAVES_FILE_RULE_ID);
    assert!(problems[0].message.contains("UnseenShape"));
    assert!(!problems[0].message.contains("UnseenClass"));
}

#[test]
fn an_unparseable_file_is_that_one_finding_and_nothing_else() {
    let problems = validate("ex:S a sh:NodeShape ; sh:property [ sh:path ex:p \n");
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].rule_id, UNPARSEABLE_RULE_ID);
}

#[test]
fn the_familys_own_findings_come_through_composed_not_restated() {
    let text = "@prefix sh: <http://www.w3.org/ns/shacl#> .\r\n@prefix ex: <http://example.org/> .\r\n@prefix ex: <http://elsewhere.example/> .\r\n\r\nex:S a sh:NodeShape .\n";
    let problems = validator::validate(text);
    assert_eq!(
        problems
            .iter()
            .filter(|p| p.rule_id == DUPLICATE_PREFIX_RULE_ID)
            .count(),
        1
    );
}

// ShaclExamples.Tests.cs

const FOLDERS: [&str; 2] = ["w3c-shacl", "fair-data-point"];

#[test]
fn every_vendored_folder_carries_its_licence_file_and_provenance() {
    for folder in FOLDERS {
        let path = common::example_path(&format!("shacl/{folder}"));
        let licence = path.join("LICENSE.md");
        assert!(licence.is_file(), "{folder} has no vendored LICENSE.md file");
        assert!(
            fs::metadata(&licence).unwrap().len() > 500,
            "{folder}'s LICENSE looks truncated"
        );
        let readme = fs::read_to_string(path.join("readme.md")).unwrap();
        for word in ["Source", "Retrieved", "License"] {
            assert!(readme.contains(word), "{folder}: {word}");
        }
    }
}

#[test]
fn every_vendored_shapes_graph_parses_and_validates_clean() {
    for folder in FOLDERS {
        let path = common::example_path(&format!("shacl/{folder}"));
        let mut bodies: Vec<_> = fs::read_dir(&path)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "ttl"))
            .collect();
        bodies.sort();
        assert!(!bodies.is_empty());
        for body in bodies {
            let text = fs::read_to_string(&body).unwrap();
            let model = common::model(&text);
            assert!(!model.triples.is_empty());
            let registration = body.with_extension("adp");
            assert!(registration.is_file(), "{} has no .adp registration", body.display());
            assert!(fs::read_to_string(&registration).unwrap().starts_with("w3c/shacl"));
            assert!(!projection::project(&model, DEFAULT_BUDGET).cards.is_empty());
            let problems = validator::validate(&text);
            assert!(
                !problems.iter().any(|p| p.severity > DiagramProblemSeverity::Info),
                "{}: {problems:?}",
                body.display()
            );
        }
    }
}

#[test]
fn the_spec_corpus_exercises_requirement_9_4s_construct_list() {
    let model = common::model(&common::example("shacl/w3c-shacl/spec-examples.ttl"));
    let projection = projection::project(&model, DEFAULT_BUDGET);
    assert!(projection.cards.iter().any(|c| c.rows.iter().any(|r| r.blank)));
    let kinds: Vec<ShaclTargetKind> = projection
        .cards
        .iter()
        .flat_map(|c| &c.targets)
        .map(|t| t.kind)
        .collect();
    assert!(kinds.contains(&ShaclTargetKind::Class));
    assert!(kinds.contains(&ShaclTargetKind::Node));
    assert!(
        projection
            .cards
            .iter()
            .any(|c| c.rows.iter().any(|r| r.summary.contains("or(")))
    );
    assert!(
        projection
            .cards
            .iter()
            .any(|c| c.rows.iter().any(|r| r.path.contains('/')))
    );
}

#[test]
fn the_deployed_shapes_draw_their_outward_targets_as_chips_and_their_cycles_as_edges() {
    let model = common::model(&common::example("shacl/fair-data-point/navigation-shapes.ttl"));
    let projection = projection::project(&model, DEFAULT_BUDGET);
    let chips: Vec<_> = projection.cards.iter().flat_map(|c| &c.targets).collect();
    assert!(!chips.is_empty());
    assert!(chips.iter().all(|c| !c.described_in_file));
    let card_ids: Vec<&str> = projection.cards.iter().map(|c| c.id.as_str()).collect();
    assert!(
        projection
            .edges
            .iter()
            .all(|e| card_ids.contains(&e.from_id.as_str()) && card_ids.contains(&e.to_id.as_str()))
    );
    assert!(projection.edges.iter().any(|e| e.kind == "node"));
    assert!(projection.edges.len() >= 4);
}
