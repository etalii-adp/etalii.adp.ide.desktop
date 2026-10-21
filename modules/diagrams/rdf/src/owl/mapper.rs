//! Turns the ontology reading's graph into what the canvas is sent (etalii.adp spec 030, task T083), ported from the
//! standalone host's `Owl/OwlElementMapper.cs` at `8b6ccfc`, its payloads (`OwlNodePayload`, `OwlEdgePayload`,
//! `OwlExpressionPayload`, `RdfTruncationPayload`) carried as attributes (see [`crate::payload`]). Positions come in
//! already merged, so an expression node's position here is always its computed one.

use adp_diagram::DiagramElement;

use crate::graph::{edge_element, node_element};
use crate::owl::projection::{OwlEdgeKind, OwlGraph, OwlNodeKind};
use crate::payload::Payload;
use crate::reading::Positions;
use crate::selection;

/// The kinds the canvas switches on.
pub const NODE_TYPE: &str = "w3c/owl+node";
pub const EDGE_TYPE: &str = "w3c/owl+edge";
pub const EXPRESSION_TYPE: &str = "w3c/owl+expression";
pub const TRUNCATION_TYPE: &str = "w3c/owl+truncation";

/// The payload word of a node kind.
pub fn node_kind_word(kind: OwlNodeKind) -> &'static str {
    match kind {
        OwlNodeKind::Class => "class",
        OwlNodeKind::Datatype => "datatype",
        OwlNodeKind::Individual => "individual",
        OwlNodeKind::Thing => "thing",
        OwlNodeKind::OntologyHeader => "ontology",
        _ => "class",
    }
}

/// The payload word of an edge kind.
pub fn edge_kind_word(kind: OwlEdgeKind) -> &'static str {
    match kind {
        OwlEdgeKind::Subclass => "subclass",
        OwlEdgeKind::Equivalent => "equivalent",
        OwlEdgeKind::Disjoint => "disjoint",
        OwlEdgeKind::ObjectProperty => "object-property",
        OwlEdgeKind::DatatypeProperty => "datatype-property",
        OwlEdgeKind::Assertion => "assertion",
        OwlEdgeKind::Expression => "expression",
    }
}

fn at(positions: &Positions, id: &str) -> (f64, f64) {
    positions.get(id).copied().unwrap_or_default()
}

/// The drawn ontology, plus the banner when the budget cut it down (`OwlElementMapper.Elements`).
pub fn elements(graph: &OwlGraph, positions: &Positions) -> Vec<DiagramElement> {
    let mut elements = Vec::new();
    for node in &graph.nodes {
        if matches!(node.kind, OwlNodeKind::Restriction | OwlNodeKind::Operator) {
            let payload = Payload::new()
                .text("label", node.display.clone())
                .flag("elided", node.elided)
                .flag("malformed", node.malformed)
                .text("ownerElementId", node.owner_id.clone())
                .text(
                    "kind",
                    if node.kind == OwlNodeKind::Operator {
                        "operator"
                    } else {
                        "restriction"
                    },
                );
            elements.push(node_element(
                &node.id,
                at(positions, &node.id),
                EXPRESSION_TYPE,
                payload,
            ));
            continue;
        }
        let rows: Vec<Vec<(&str, String)>> = node
            .rows
            .iter()
            .map(|row| {
                vec![
                    ("predicate", row.predicate.clone()),
                    ("value", row.value.clone()),
                    ("annotation", row.annotation.clone()),
                ]
            })
            .collect();
        let payload = Payload::new()
            .text("kind", node_kind_word(node.kind))
            .text("iri", node.iri.clone())
            .text("display", node.display.clone())
            .list("badges", &node.badges)
            .records("rows", &rows)
            .flag("deprecated", node.deprecated)
            .flag("external", node.external);
        elements.push(node_element(&node.id, at(positions, &node.id), NODE_TYPE, payload));
    }
    for edge in &graph.edges {
        let payload = Payload::new()
            .text("kind", edge_kind_word(edge.kind))
            .text("fromElementId", edge.from_id.clone())
            .text("toElementId", edge.to_id.clone())
            .text("label", edge.label.clone())
            .text("propertyIri", edge.property_iri.clone());
        elements.push(edge_element(&edge.id, &edge.from_id, &edge.to_id, EDGE_TYPE, payload));
    }
    if graph.truncated() {
        let payload = Payload::new()
            .number("shown", graph.shown as i64)
            .number("total", graph.total as i64);
        elements.push(node_element(
            selection::TRUNCATION_ID,
            at(positions, selection::TRUNCATION_ID),
            TRUNCATION_TYPE,
            payload,
        ));
    }
    elements
}
