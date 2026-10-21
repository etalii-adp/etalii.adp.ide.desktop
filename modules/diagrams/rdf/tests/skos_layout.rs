//! The scheme reading's layered layout (etalii.adp spec 030, task T083), ported from the standalone host's
//! `SkosLayout.Tests.cs` at `8b6ccfc`.

mod common;

use adp_diagram_rdf::projection::DEFAULT_BUDGET;
use adp_diagram_rdf::skos::layout::{SkosLayoutResult, layout};
use adp_diagram_rdf::skos::projection::{SkosEdgeKind, project};

const PRELUDE: &str = "@prefix skos: <http://www.w3.org/2004/02/skos/core#> .\n@prefix ex: <http://example.org/> .\n\n";

fn laid_out(turtle: &str) -> SkosLayoutResult {
    layout(&project(&common::model(&format!("{PRELUDE}{turtle}")), DEFAULT_BUDGET))
}

fn at(result: &SkosLayoutResult, local: &str) -> (f64, f64) {
    result.positions[&format!("res:http://example.org/{local}")]
}

#[test]
fn the_hierarchy_layers_top_down_broader_above_narrower() {
    let result = laid_out(
        "ex:scheme a skos:ConceptScheme ; skos:hasTopConcept ex:top .\n\
         ex:top a skos:Concept ; skos:topConceptOf ex:scheme .\n\
         ex:mid a skos:Concept ; skos:inScheme ex:scheme ; skos:broader ex:top .\n\
         ex:leaf a skos:Concept ; skos:inScheme ex:scheme ; skos:broader ex:mid .\n",
    );
    assert!(at(&result, "scheme").1 < at(&result, "top").1);
    assert!(at(&result, "top").1 < at(&result, "mid").1);
    assert!(at(&result, "mid").1 < at(&result, "leaf").1);
    assert!(result.cycles.is_empty());
}

#[test]
fn a_polyhierarchical_concept_is_placed_once_below_its_deepest_broader() {
    let result = laid_out(
        "ex:scheme a skos:ConceptScheme ; skos:hasTopConcept ex:top .\n\
         ex:top a skos:Concept ; skos:topConceptOf ex:scheme .\n\
         ex:mid a skos:Concept ; skos:inScheme ex:scheme ; skos:broader ex:top .\n\
         ex:shared a skos:Concept ; skos:inScheme ex:scheme ; skos:broader ex:top ; skos:broader ex:mid .\n",
    );
    assert!(at(&result, "shared").1 > at(&result, "mid").1);
}

#[test]
fn a_cycle_is_broken_at_the_documented_edge_drawn_whole_and_reported_once() {
    let projection = project(
        &common::model(&format!(
            "{PRELUDE}ex:scheme a skos:ConceptScheme .\n\
             ex:a a skos:Concept ; skos:inScheme ex:scheme ; skos:broader ex:b .\n\
             ex:b a skos:Concept ; skos:inScheme ex:scheme ; skos:broader ex:c .\n\
             ex:c a skos:Concept ; skos:inScheme ex:scheme ; skos:broader ex:a .\n"
        )),
        DEFAULT_BUDGET,
    );
    let result = layout(&projection);
    assert_eq!(1, result.cycles.len());
    let cycle = &result.cycles[0];
    assert_eq!(
        vec![
            "res:http://example.org/a",
            "res:http://example.org/b",
            "res:http://example.org/c"
        ],
        cycle.concept_ids
    );
    assert_eq!(
        "edge:res:http://example.org/c|http://www.w3.org/2004/02/skos/core#broader|res:http://example.org/a",
        cycle.excluded_edge_id
    );
    assert_eq!(3, cycle.triples.len());
    assert_eq!(
        3,
        projection
            .edges
            .iter()
            .filter(|e| e.kind == SkosEdgeKind::Hierarchy)
            .count()
    );
    for local in ["a", "b", "c"] {
        assert!(
            result
                .positions
                .contains_key(&format!("res:http://example.org/{local}"))
        );
    }
}

#[test]
fn the_same_file_always_lays_out_the_same_way() {
    let turtle = "ex:scheme a skos:ConceptScheme ; skos:hasTopConcept ex:top .\n\
                  ex:top a skos:Concept ; skos:topConceptOf ex:scheme .\n\
                  ex:b a skos:Concept ; skos:inScheme ex:scheme ; skos:broader ex:top .\n\
                  ex:a a skos:Concept ; skos:inScheme ex:scheme ; skos:broader ex:top .\n\
                  ex:unfiled a skos:Concept ; skos:related ex:a .\n\
                  ex:group a skos:Collection ; skos:member ex:a .\n";
    let first = laid_out(turtle);
    let second = laid_out(turtle);
    assert_eq!(first.positions, second.positions);
    assert!(at(&first, "unfiled").1 > at(&first, "a").1);
    assert!(at(&first, "group").1 > at(&first, "unfiled").1);
}

#[test]
fn a_wide_layer_wraps_onto_centred_rows_with_siblings_together() {
    let mut turtle = String::from(
        "ex:scheme a skos:ConceptScheme ; skos:hasTopConcept ex:a , ex:b .\n\
         ex:a a skos:Concept ; skos:topConceptOf ex:scheme .\n\
         ex:b a skos:Concept ; skos:topConceptOf ex:scheme .\n\n",
    );
    for index in 1..=40u32 {
        let parent = if index.is_multiple_of(2) { "a" } else { "b" };
        turtle.push_str(&format!(
            "ex:c{index:02} a skos:Concept ; skos:inScheme ex:scheme ; skos:broader ex:{parent} .\n"
        ));
    }
    let result = laid_out(&turtle);

    let children: Vec<String> = (1..=40).map(|i| format!("res:http://example.org/c{i:02}")).collect();
    let mut ys: Vec<f64> = children.iter().map(|id| result.positions[id].1).collect();
    ys.sort_by(f64::total_cmp);
    ys.dedup();
    assert!(ys.len() > 1, "The forty narrower concepts sit on {} row(s).", ys.len());
    let xs: Vec<f64> = children.iter().map(|id| result.positions[id].0).collect();
    let left = xs.iter().copied().fold(f64::MAX, f64::min);
    let right = xs.iter().copied().fold(f64::MIN, f64::max);
    assert!(
        right - left < 40.0 * 240.0 / 2.0,
        "The narrower layer is {} wide.",
        right - left
    );

    let mut reading: Vec<&String> = children.iter().collect();
    reading.sort_by(|a, b| {
        let (pa, pb) = (result.positions[*a], result.positions[*b]);
        pa.1.total_cmp(&pb.1).then(pa.0.total_cmp(&pb.0))
    });
    let parents: Vec<&str> = reading
        .iter()
        .map(|id| {
            let n: u32 = id[id.len() - 2..].parse().unwrap();
            if n.is_multiple_of(2) { "a" } else { "b" }
        })
        .collect();
    let expected: Vec<&str> = std::iter::repeat_n("a", 20)
        .chain(std::iter::repeat_n("b", 20))
        .collect();
    assert_eq!(expected, parents);

    let tops = [at(&result, "a").0, at(&result, "b").0];
    let tops_centre = (tops[0].min(tops[1]) + tops[0].max(tops[1])) / 2.0;
    assert_eq!((left + right) / 2.0, tops_centre);
}
