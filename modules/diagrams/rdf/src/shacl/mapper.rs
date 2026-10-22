//! The shapes projection as the elements the canvas is sent (etalii.adp spec 030, task T083), ported from the
//! standalone host's `Shacl/ShaclElementMapper.cs` at `8b6ccfc`. Two of the reading's structures are deliberately no
//! elements and travel inside their card's payload instead: target chips, because the data they name lives in another
//! file, and constraint rows, because they are written as blank nodes with no position to store. Payloads are
//! `api/shacl.proto`'s, carried as [`crate::payload`] says.

use adp_diagram::DiagramElement;

use crate::graph::{edge_element, node_element};
use crate::payload::Payload;
use crate::reading::Positions;
use crate::selection;
use crate::shacl::projection::{ShaclProjectionResult, ShaclTargetKind};
use crate::shacl::{EDGE_TYPE, SHAPE_TYPE, TRUNCATION_TYPE};

/// A chip's kind as `ShaclTargetKindProto`'s JSON name.
pub fn kind_name(kind: ShaclTargetKind) -> &'static str {
    match kind {
        ShaclTargetKind::Class => "SHACL_TARGET_KIND_PROTO_CLASS",
        ShaclTargetKind::Node => "SHACL_TARGET_KIND_PROTO_NODE",
        ShaclTargetKind::SubjectsOf => "SHACL_TARGET_KIND_PROTO_SUBJECTS_OF",
        ShaclTargetKind::ObjectsOf => "SHACL_TARGET_KIND_PROTO_OBJECTS_OF",
        ShaclTargetKind::ImplicitClass => "SHACL_TARGET_KIND_PROTO_IMPLICIT_CLASS",
    }
}

fn flag(value: bool) -> String {
    if value { "true" } else { "false" }.to_owned()
}

fn at(positions: &Positions, id: &str) -> (f64, f64) {
    positions.get(id).copied().unwrap_or_default()
}

/// The drawn shapes, plus the family's truncation banner when the budget cut them (`ShaclElementMapper.Elements`).
pub fn elements(projection: &ShaclProjectionResult, positions: &Positions) -> Vec<DiagramElement> {
    let mut elements = Vec::new();
    for card in &projection.cards {
        let targets: Vec<Vec<(&str, String)>> = card
            .targets
            .iter()
            .map(|chip| {
                vec![
                    ("kind", kind_name(chip.kind).to_owned()),
                    ("termDisplay", chip.term_display.clone()),
                    ("termIri", chip.term_iri.clone()),
                    ("describedInFile", flag(chip.described_in_file)),
                    ("shapeIri", chip.shape_iri.clone()),
                    ("predicateIri", chip.predicate_iri.clone()),
                ]
            })
            .collect();
        let rows: Vec<Vec<(&str, String)>> = card
            .rows
            .iter()
            .map(|row| {
                vec![
                    ("path", row.path.clone()),
                    ("name", row.name.clone()),
                    ("summary", row.summary.clone()),
                    ("cardinality", row.cardinality.clone()),
                    ("sparql", flag(row.sparql)),
                    ("blank", flag(row.blank)),
                    ("severity", row.severity.clone()),
                ]
            })
            .collect();
        let payload = Payload::new()
            .text("iri", card.iri.clone())
            .text("display", card.display.clone())
            .flag("blank", card.blank)
            .flag("deactivated", card.deactivated)
            .text("severity", card.severity.clone())
            .flag("closed", card.closed)
            .text("name", card.name.clone())
            .text("description", card.description.clone())
            .records("targets", &targets)
            .records("rows", &rows);
        elements.push(node_element(&card.id, at(positions, &card.id), SHAPE_TYPE, payload));
    }
    for edge in &projection.edges {
        let payload = Payload::new()
            .text("fromElementId", edge.from_id.clone())
            .text("toElementId", edge.to_id.clone())
            .text("kind", edge.kind.clone())
            .text("label", edge.label.clone());
        elements.push(edge_element(&edge.id, &edge.from_id, &edge.to_id, EDGE_TYPE, payload));
    }
    if projection.truncated() {
        let payload = Payload::new()
            .number("shown", projection.shown as i64)
            .number("total", projection.total as i64);
        elements.push(node_element(
            selection::TRUNCATION_ID,
            at(positions, selection::TRUNCATION_ID),
            TRUNCATION_TYPE,
            payload,
        ));
    }
    elements
}
