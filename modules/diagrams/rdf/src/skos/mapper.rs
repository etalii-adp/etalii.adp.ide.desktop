//! Turns the scheme projection into what the canvas is sent (etalii.adp spec 030, task T083), ported from the
//! standalone host's `Skos/SkosElementMapper.cs` at `8b6ccfc`, its protobuf payloads (`api/skos.proto`) carried as
//! attributes ([`crate::payload`]). Labels arrive already chosen: [`labels::choose`] runs here once per element.

use adp_diagram::DiagramElement;

use crate::graph::{edge_element, node_element};
use crate::payload::Payload;
use crate::projection::local_part;
use crate::reading::Positions;
use crate::selection::TRUNCATION_ID;
use crate::skos::labels::{self, SkosChosenLabel, SkosLabel};
use crate::skos::projection::{SkosEdgeKind, SkosProjectionResult};

/// The kinds the canvas switches on.
pub const CONCEPT_TYPE: &str = "w3c/skos+concept";
pub const SCHEME_TYPE: &str = "w3c/skos+scheme";
pub const COLLECTION_TYPE: &str = "w3c/skos+collection";
pub const EDGE_TYPE: &str = "w3c/skos+edge";
pub const TRUNCATION_TYPE: &str = "w3c/skos+truncation";

/// A short display for an IRI with no label: its fragment or last path segment (`SkosElementMapper.Shorten`).
pub fn shorten(iri: &str) -> String {
    local_part(iri)
}

fn chosen(labels: &[SkosLabel], display_language: &str, iri: &str) -> SkosChosenLabel {
    labels::choose(labels, display_language, &shorten(iri))
}

fn at(positions: &Positions, id: &str) -> (f64, f64) {
    positions.get(id).copied().unwrap_or_default()
}

/// The drawn vocabulary, plus the truncation banner when the budget cut it (`SkosElementMapper.Elements`).
pub fn elements(
    projection: &SkosProjectionResult,
    positions: &Positions,
    display_language: &str,
) -> Vec<DiagramElement> {
    let mut elements = Vec::new();
    for scheme in &projection.schemes {
        let label = chosen(&scheme.labels, display_language, &scheme.iri);
        let members = projection
            .concepts
            .iter()
            .filter(|c| c.scheme_iris.contains(&scheme.iri))
            .count();
        let payload = Payload::new()
            .text("iri", scheme.iri.clone())
            .text("label", label.text)
            .text("languageTag", label.language_tag)
            .number("labelKind", label.kind as i64)
            .number("memberCount", members as i64);
        elements.push(node_element(
            &scheme.id,
            at(positions, &scheme.id),
            SCHEME_TYPE,
            payload,
        ));
    }
    for concept in &projection.concepts {
        let label = chosen(&concept.labels, display_language, &concept.iri);
        let chip = !label.language_tag.is_empty() && label.language_tag != display_language;
        let payload = Payload::new()
            .text("iri", concept.iri.clone())
            .text("label", label.text)
            .text("languageTag", label.language_tag)
            .number("labelKind", label.kind as i64)
            .text("notation", concept.notations.first().cloned().unwrap_or_default())
            .list("schemeIris", &concept.scheme_iris)
            .flag("blank", concept.blank)
            .flag("languageChip", chip);
        elements.push(node_element(
            &concept.id,
            at(positions, &concept.id),
            CONCEPT_TYPE,
            payload,
        ));
    }
    for collection in &projection.collections {
        let label = chosen(&collection.labels, display_language, &collection.iri);
        let payload = Payload::new()
            .text("iri", collection.iri.clone())
            .text("label", label.text)
            .text("languageTag", label.language_tag)
            .number("labelKind", label.kind as i64)
            .flag("ordered", collection.ordered)
            .list("memberIds", &collection.member_ids);
        elements.push(node_element(
            &collection.id,
            at(positions, &collection.id),
            COLLECTION_TYPE,
            payload,
        ));
    }
    for edge in &projection.edges {
        let predicate = if edge.kind == SkosEdgeKind::Mapping {
            shorten(&edge.predicate_iri)
        } else {
            String::new()
        };
        let payload = Payload::new()
            .text("fromElementId", edge.from_id.clone())
            .text("toElementId", edge.to_id.clone())
            .number("kind", edge.kind as i64)
            .text("predicate", predicate)
            .flag("assertedBothWays", edge.asserted_both_ways);
        elements.push(edge_element(&edge.id, &edge.from_id, &edge.to_id, EDGE_TYPE, payload));
    }
    if projection.truncated() {
        let payload = Payload::new()
            .number("shown", projection.shown as i64)
            .number("total", projection.total as i64);
        elements.push(node_element(
            TRUNCATION_ID,
            at(positions, TRUNCATION_ID),
            TRUNCATION_TYPE,
            payload,
        ));
    }
    elements
}
