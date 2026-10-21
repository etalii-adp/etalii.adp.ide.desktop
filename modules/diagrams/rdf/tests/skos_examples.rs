//! The vendored STW extracts, judged by the reading that opens them (etalii.adp spec 030, task T083), ported from the
//! standalone host's `SkosExamples.Tests.cs` at `8b6ccfc`.

mod common;

use adp_diagram_rdf::skos::layout::layout;
use adp_diagram_rdf::skos::projection::project;
use adp_diagram_rdf::skos::validator::validate;

const DOCUMENTS: [&str; 2] = ["business-economics", "geographic-names"];

#[test]
fn every_stw_extract_validates_clean_under_the_skos_rules() {
    for name in DOCUMENTS {
        let text = common::example(&format!("stw/{name}.ttl"));
        let registration = common::example_path(&format!("stw/{name}.adp"));
        let problems = validate(&text, Some(&registration));
        assert!(problems.is_empty(), "{name}: {problems:?}");
    }
}

#[test]
fn every_stw_extract_lays_out_as_a_hierarchy_not_as_a_line() {
    for name in DOCUMENTS {
        let text = common::example(&format!("stw/{name}.ttl"));
        let projection = project(&common::model(&text), usize::MAX);
        let positions: Vec<(f64, f64)> = layout(&projection).positions.values().copied().collect();
        let min_x = positions.iter().map(|p| p.0).fold(f64::MAX, f64::min);
        let max_x = positions.iter().map(|p| p.0).fold(f64::MIN, f64::max);
        let min_y = positions.iter().map(|p| p.1).fold(f64::MAX, f64::min);
        let max_y = positions.iter().map(|p| p.1).fold(f64::MIN, f64::max);
        let ratio = (max_x - min_x + 240.0) / (max_y - min_y + 110.0);
        assert!((0.5..=4.0).contains(&ratio), "{name}: {ratio}");
    }
}
