//! The scheme reading as a view draws it (etalii.adp spec 030, task T083), ported from the reading parts of the
//! standalone host's `SkosSession.Tests.cs` at `8b6ccfc`: the language seeded from the registration, the refusals the
//! reading names, viewport filtering over the whole document's layout, and the vendored thesaurus. A move without a
//! registration and the shared store are the view's and the store's own tests; the definition's routing stance
//! (`SharedExtension`, `SuggestsBody`) is the module's.

mod common;

use std::collections::HashSet;
use std::fs;

use adp_diagram::{DiagramElement, DiagramViewport};
use adp_diagram_rdf::payload;
use adp_diagram_rdf::reading::{Positions, Reading};
use adp_diagram_rdf::skos::labels::{DEFAULT_LANGUAGE, SkosLabelKind};
use adp_diagram_rdf::skos::layout::layout;
use adp_diagram_rdf::skos::mapper::{CONCEPT_TYPE, SCHEME_TYPE};
use adp_diagram_rdf::skos::projection::project;
use adp_diagram_rdf::skos::{PARENT_REFUSAL, SkosReading};

const VOCABULARY: &str = "@prefix skos: <http://www.w3.org/2004/02/skos/core#> .\n\
@prefix ex: <http://example.org/> .\n\
\n\
ex:scheme a skos:ConceptScheme ; skos:prefLabel \"Drinks\"@en ; skos:hasTopConcept ex:tea .\n\
ex:tea a skos:Concept ; skos:topConceptOf ex:scheme ;\n\
    skos:prefLabel \"Tea\"@en ; skos:prefLabel \"Thee\"@nl ; skos:notation \"T1\" .\n";

const UNBOUNDED: DiagramViewport = DiagramViewport {
    min_x: f64::MIN,
    min_y: f64::MIN,
    max_x: f64::MAX,
    max_y: f64::MAX,
};

fn render(reading: &SkosReading, text: &str, viewport: DiagramViewport) -> Vec<DiagramElement> {
    let model = common::model(text);
    let projection = reading.project(&model);
    let positions = reading.positions(&projection, &Positions::new());
    reading.elements(&reading.in_view(&projection, &positions, viewport), &positions)
}

#[test]
fn the_reading_renders_the_vocabulary_in_the_registrations_language() {
    let folder = tempfile::tempdir().unwrap();
    let registration = folder.path().join("drinks.adp");
    fs::write(&registration, "w3c/skos\r\nbody: drinks.ttl\r\nlanguage: nl\r\n").unwrap();
    let reading = SkosReading::for_registration(Some(&registration));
    assert_eq!("nl", reading.language);

    let elements = render(&reading, VOCABULARY, UNBOUNDED);
    let tea = elements.iter().find(|e| e.id == "res:http://example.org/tea").unwrap();
    assert_eq!(CONCEPT_TYPE, tea.element_type);
    assert_eq!("Thee", payload::text(tea, "label"));
    assert_eq!("nl", payload::text(tea, "languageTag"));
    assert_eq!(SkosLabelKind::Preferred as i64, payload::number(tea, "labelKind"));
    assert_eq!("T1", payload::text(tea, "notation"));

    let schemes: Vec<_> = elements.iter().filter(|e| e.element_type == SCHEME_TYPE).collect();
    assert_eq!(1, schemes.len());
    assert_eq!("res:http://example.org/scheme", schemes[0].id);
}

#[test]
fn a_misplaced_header_falls_back_to_the_default_rather_than_guessing() {
    let folder = tempfile::tempdir().unwrap();
    let registration = folder.path().join("drinks.adp");
    fs::write(&registration, "w3c/skos\r\nlanguage: nl\r\nbody: drinks.ttl\r\n").unwrap();
    assert_eq!(
        DEFAULT_LANGUAGE,
        SkosReading::for_registration(Some(&registration)).language
    );
}

#[test]
fn the_refusals_name_their_reasons() {
    let reading = SkosReading::default();
    assert!(reading.refusal("blank:0").unwrap().contains("blank node"));
    assert!(reading.refusal("edge:x|y|z").unwrap().contains("not something"));
    assert!(reading.refusal("truncation").unwrap().contains("not something"));
    assert_eq!(None, reading.refusal("res:http://example.org/tea"));
    assert!(reading.parent_refusal() == PARENT_REFUSAL);
}

#[test]
fn a_viewport_change_drops_what_left_and_keeps_the_anchor() {
    let reading = SkosReading::default();
    let all = render(&reading, VOCABULARY, UNBOUNDED);
    let anchor = all
        .iter()
        .min_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)))
        .unwrap();
    let narrowed = render(
        &reading,
        VOCABULARY,
        DiagramViewport {
            min_x: anchor.x,
            min_y: anchor.y,
            max_x: anchor.x + 1.0,
            max_y: anchor.y + 1.0,
        },
    );
    let kept: HashSet<&str> = narrowed.iter().map(|e| e.id.as_str()).collect();
    assert!(kept.contains(anchor.id.as_str()));
    assert!(kept.len() < all.len());
}

#[test]
fn panning_does_not_move_the_elements_it_brings_into_view() {
    let reading = SkosReading::default();
    let at_open = render(&reading, VOCABULARY, UNBOUNDED);
    let _ = render(
        &reading,
        VOCABULARY,
        DiagramViewport {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 1.0,
            max_y: 1.0,
        },
    );
    assert_eq!(at_open, render(&reading, VOCABULARY, UNBOUNDED));
}

#[test]
fn the_vendored_thesaurus_never_strands_a_related_link() {
    let text = common::example("stw/business-economics.ttl");
    let reading = SkosReading::default();
    let all = render(&reading, &text, UNBOUNDED);
    assert!(all.iter().any(|e| e.id.starts_with("edge:")));
    let anchor = all
        .iter()
        .filter(|e| e.element_type == CONCEPT_TYPE)
        .min_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)))
        .unwrap()
        .clone();
    for viewport in [
        DiagramViewport {
            min_x: anchor.x,
            min_y: anchor.y,
            max_x: anchor.x + 1.0,
            max_y: anchor.y + 1.0,
        },
        DiagramViewport {
            min_x: -50_000.0,
            min_y: -50_000.0,
            max_x: -49_000.0,
            max_y: -49_000.0,
        },
        UNBOUNDED,
    ] {
        let held = render(&reading, &text, viewport);
        let ids: HashSet<&str> = held.iter().map(|e| e.id.as_str()).collect();
        for edge in held.iter().filter(|e| e.id.starts_with("edge:")) {
            let parts: Vec<&str> = edge.id["edge:".len()..].split('|').collect();
            assert!(ids.contains(parts[0]));
            assert!(ids.contains(parts[2]));
        }
    }
}

#[test]
fn the_thesaurus_layout_places_every_concept_it_draws() {
    let model = common::model(&common::example("stw/business-economics.ttl"));
    let projection = project(&model, usize::MAX);
    let positions = layout(&projection).positions;
    let unplaced: Vec<&String> = projection
        .concepts
        .iter()
        .map(|c| &c.id)
        .chain(projection.schemes.iter().map(|s| &s.id))
        .chain(projection.collections.iter().map(|c| &c.id))
        .filter(|id| !positions.contains_key(*id))
        .collect();
    assert!(unplaced.is_empty(), "{unplaced:?}");
}
