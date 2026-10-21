//! The scheme projection (etalii.adp spec 030, task T083), ported from the standalone host's
//! `SkosProjection.Tests.cs` at `8b6ccfc`.

mod common;

use adp_diagram_rdf::projection::DEFAULT_BUDGET;
use adp_diagram_rdf::skos::labels::SkosLabelSource;
use adp_diagram_rdf::skos::projection::{SkosEdgeKind, SkosProjectionResult, has_xl_labels, project};

const PRELUDE: &str = "@prefix skos: <http://www.w3.org/2004/02/skos/core#> .\n@prefix ex: <http://example.org/> .\n\n";

fn projected(turtle: &str, budget: usize) -> SkosProjectionResult {
    project(&common::model(&format!("{PRELUDE}{turtle}")), budget)
}

#[test]
fn either_asserted_direction_is_one_edge_and_nothing_is_completed() {
    let projection = projected(
        "ex:scheme a skos:ConceptScheme .\n\
         ex:a a skos:Concept ; skos:inScheme ex:scheme .\n\
         ex:b a skos:Concept ; skos:inScheme ex:scheme ; skos:broader ex:a .\n\
         ex:c a skos:Concept ; skos:inScheme ex:scheme .\n\
         ex:d a skos:Concept ; skos:inScheme ex:scheme ; skos:broader ex:c .\n\
         ex:c skos:narrower ex:d .\n\
         ex:e a skos:Concept ; skos:inScheme ex:scheme .\n\
         ex:a skos:narrower ex:e .\n",
        DEFAULT_BUDGET,
    );
    let hierarchy: Vec<_> = projection
        .edges
        .iter()
        .filter(|e| e.kind == SkosEdgeKind::Hierarchy)
        .collect();
    assert_eq!(3, hierarchy.len());

    let b_under_a = hierarchy
        .iter()
        .find(|e| e.to_id == "res:http://example.org/b")
        .unwrap();
    assert_eq!("res:http://example.org/a", b_under_a.from_id);
    assert_eq!(
        "edge:res:http://example.org/b|http://www.w3.org/2004/02/skos/core#broader|res:http://example.org/a",
        b_under_a.id
    );
    assert!(!b_under_a.asserted_both_ways);
    assert_eq!(1, b_under_a.triples.len());

    let d_under_c = hierarchy
        .iter()
        .find(|e| e.to_id == "res:http://example.org/d")
        .unwrap();
    assert!(d_under_c.asserted_both_ways);
    assert_eq!(2, d_under_c.triples.len());

    let e_under_a = hierarchy
        .iter()
        .find(|e| e.to_id == "res:http://example.org/e")
        .unwrap();
    assert_eq!("res:http://example.org/a", e_under_a.from_id);
    assert_eq!(1, e_under_a.triples.len());
}

#[test]
fn membership_is_assertion_only_and_the_unfiled_band_catches_the_rest() {
    let projection = projected(
        "ex:scheme a skos:ConceptScheme ; skos:hasTopConcept ex:a .\n\
         ex:a a skos:Concept ; skos:topConceptOf ex:scheme .\n\
         ex:b a skos:Concept ; skos:inScheme ex:scheme ; skos:broader ex:a .\n\
         ex:c a skos:Concept ; skos:broader ex:b .\n",
        DEFAULT_BUDGET,
    );
    let a = projection
        .concepts
        .iter()
        .find(|c| c.iri == "http://example.org/a")
        .unwrap();
    assert_eq!(vec!["http://example.org/scheme"], a.scheme_iris);
    let c = projection
        .concepts
        .iter()
        .find(|c| c.iri == "http://example.org/c")
        .unwrap();
    assert!(c.scheme_iris.is_empty());
    assert_eq!(1, projection.schemes.len());
    assert_eq!(vec!["res:http://example.org/a"], projection.schemes[0].top_concept_ids);
}

#[test]
fn mappings_draw_only_in_file_and_surface_as_rows_otherwise() {
    let projection = projected(
        "ex:a a skos:Concept ; skos:exactMatch ex:b ; skos:closeMatch <http://elsewhere.org/x> .\nex:b a skos:Concept .\n",
        DEFAULT_BUDGET,
    );
    let mappings: Vec<_> = projection
        .edges
        .iter()
        .filter(|e| e.kind == SkosEdgeKind::Mapping)
        .collect();
    assert_eq!(1, mappings.len());
    assert_eq!("res:http://example.org/a", mappings[0].from_id);
    assert_eq!("res:http://example.org/b", mappings[0].to_id);
    assert_eq!(1, projection.out_of_file_mappings.len());
    assert_eq!(
        "http://www.w3.org/2004/02/skos/core#closeMatch",
        projection.out_of_file_mappings[0].predicate.iri
    );
    assert!(!projection.concepts.iter().any(|c| c.iri == "http://elsewhere.org/x"));
}

#[test]
fn collections_keep_their_order_and_stay_off_the_hierarchy() {
    let projection = projected(
        "ex:a a skos:Concept ; skos:prefLabel \"Milk\"@en ; skos:notation \"M1\" .\n\
         ex:b a skos:Concept .\n\
         ex:group a skos:OrderedCollection ; skos:prefLabel \"Additives\"@en ; skos:memberList ( ex:b ex:a ) .\n",
        DEFAULT_BUDGET,
    );
    assert_eq!(1, projection.collections.len());
    let group = &projection.collections[0];
    assert!(group.ordered);
    assert_eq!(
        vec!["res:http://example.org/b", "res:http://example.org/a"],
        group.member_ids
    );
    assert!(!projection.edges.iter().any(|e| e.kind == SkosEdgeKind::Hierarchy));
    let a = projection
        .concepts
        .iter()
        .find(|c| c.iri == "http://example.org/a")
        .unwrap();
    assert_eq!(vec!["M1"], a.notations);
    assert!(
        a.labels
            .iter()
            .any(|l| l.source == SkosLabelSource::Preferred && l.text == "Milk" && l.language.as_deref() == Some("en"))
    );
}

#[test]
fn the_budget_cuts_the_top_of_the_vocabulary_not_a_triple_order_slice() {
    let projection = projected(
        "ex:zzz a skos:Concept .\n\
         ex:scheme a skos:ConceptScheme ; skos:hasTopConcept ex:top .\n\
         ex:top a skos:Concept ; skos:topConceptOf ex:scheme .\n\
         ex:child a skos:Concept ; skos:inScheme ex:scheme ; skos:broader ex:top .\n",
        3,
    );
    assert!(projection.truncated());
    assert_eq!(3, projection.shown);
    assert_eq!(4, projection.total);
    assert_eq!(1, projection.schemes.len());
    let iris: Vec<&str> = projection.concepts.iter().map(|c| c.iri.as_str()).collect();
    assert_eq!(vec!["http://example.org/top", "http://example.org/child"], iris);
}

#[test]
fn polyhierarchy_draws_once_and_skos_xl_is_detected_but_never_read() {
    let model = common::model(&format!(
        "{PRELUDE}@prefix skosxl: <http://www.w3.org/2008/05/skos-xl#> .\n\
         ex:a a skos:Concept .\n\
         ex:b a skos:Concept .\n\
         ex:c a skos:Concept ; skos:broader ex:a ; skos:broader ex:b ; skosxl:prefLabel ex:cLabel .\n"
    ));
    let projection = project(&model, DEFAULT_BUDGET);
    let cs: Vec<_> = projection
        .concepts
        .iter()
        .filter(|c| c.iri == "http://example.org/c")
        .collect();
    assert_eq!(1, cs.len());
    assert_eq!(
        2,
        projection
            .edges
            .iter()
            .filter(|e| e.kind == SkosEdgeKind::Hierarchy && e.to_id == "res:http://example.org/c")
            .count()
    );
    assert!(has_xl_labels(&model));
    assert!(cs[0].labels.is_empty());
}
