//! The data-graph reading, `w3c/rdf` (etalii.adp spec 030, task T083): every resource a card, every triple between
//! them an edge, and what the canvas is sent for them.
//!
//! Ported from the standalone host's `RdfReading.cs` and `RdfElementMapper.cs` at `8b6ccfc`. The viewport is where
//! this reading pays for itself: the drawn-node budget alone discards by document order, so a reader of a large file
//! could never reach past its first thousand resources however far they panned. The budget survives as a floor
//! against a pathological view; the counts handed on are the document's, so the banner never contradicts what the
//! document may do.

use std::collections::HashSet;
use std::path::Path;

use adp_diagram::{DiagramElement, DiagramProblem, DiagramViewport, ElementKind, ToolboxItem};

use crate::model::RdfModel;
use crate::payload::Payload;
use crate::projection::{self, CELL_HEIGHT, CELL_WIDTH, DEFAULT_BUDGET, RdfProjection};
use crate::reading::{self, Positions, Reading};
use crate::selection;
use crate::store::RdfDocument;
use crate::{toolbox, validator};

/// The data graph's origin, the family's anchor.
pub const ORIGIN: &str = "w3c/rdf";

/// The kinds the canvas switches on (`RdfElementMapper`).
pub const RESOURCE_TYPE: &str = "w3c/rdf+resource";
pub const EDGE_TYPE: &str = "w3c/rdf+edge";
pub const TRUNCATION_TYPE: &str = "w3c/rdf+truncation";

/// What a drag onto another parent answers (`RdfReading.ParentRefusal`).
pub const PARENT_REFUSAL: &str =
    "A resource has no parent to move it under; dragging changes where it sits on the canvas.";

/// The data-graph reading (`RdfReading`).
#[derive(Debug, Clone, Copy, Default)]
pub struct RdfReading;

impl Reading for RdfReading {
    type Projection = RdfProjection;

    fn origin(&self) -> &'static str {
        ORIGIN
    }

    fn parent_refusal(&self) -> &'static str {
        PARENT_REFUSAL
    }

    fn project(&self, model: &RdfModel) -> RdfProjection {
        projection::project(model, usize::MAX)
    }

    fn positions(&self, projection: &RdfProjection, stored: &Positions) -> Positions {
        reading::apply(projection::positions(projection), stored)
    }

    fn in_view(&self, projection: &RdfProjection, positions: &Positions, viewport: DiagramViewport) -> RdfProjection {
        let visible: Vec<_> = projection
            .nodes
            .iter()
            .filter(|node| reading::admits(viewport, positions, &node.id, CELL_WIDTH, CELL_HEIGHT))
            .cloned()
            .collect();
        let truncated = projection.total > DEFAULT_BUDGET;
        let kept: Vec<_> = visible.into_iter().take(DEFAULT_BUDGET).collect();
        let kept_ids: HashSet<&str> = kept.iter().map(|n| n.id.as_str()).collect();
        let edges = projection
            .edges
            .iter()
            .filter(|e| kept_ids.contains(e.from_id.as_str()) && kept_ids.contains(e.to_id.as_str()))
            .cloned()
            .collect();
        let count = kept.len();
        RdfProjection {
            nodes: kept,
            edges,
            shown: if truncated { DEFAULT_BUDGET } else { count },
            total: if truncated { projection.total } else { count },
        }
    }

    fn elements(&self, projection: &RdfProjection, positions: &Positions) -> Vec<DiagramElement> {
        elements(projection, positions)
    }

    fn refusal(&self, element_id: &str) -> Option<String> {
        if element_id.starts_with("edge:") || element_id == selection::TRUNCATION_ID {
            return Some(selection::NOT_MOVABLE.to_owned());
        }
        selection::is_blank(element_id).then(|| selection::BLANK_NOT_MOVABLE.to_owned())
    }

    fn toolbox(&self) -> Vec<ToolboxItem> {
        toolbox::items()
    }

    fn problems(&self, document: &RdfDocument, _registration: Option<&Path>) -> Vec<DiagramProblem> {
        validator::validate(&document.text)
    }

    fn connect_action(&self) -> Option<&'static str> {
        Some("rdf.connect")
    }
}

fn at(positions: &Positions, id: &str) -> (f64, f64) {
    positions.get(id).copied().unwrap_or_default()
}

/// The drawn graph, plus the banner when the budget cut it down (`RdfElementMapper.Elements`).
pub fn elements(projection: &RdfProjection, positions: &Positions) -> Vec<DiagramElement> {
    let mut elements = Vec::new();
    for node in &projection.nodes {
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
            .text("iri", node.iri.clone())
            .text("display", node.display.clone())
            .list("typeBadges", &node.types)
            .records("rows", &rows)
            .flag("blank", node.blank);
        elements.push(node_element(&node.id, at(positions, &node.id), RESOURCE_TYPE, payload));
    }
    for edge in &projection.edges {
        let payload = Payload::new()
            .text("fromElementId", edge.from_id.clone())
            .text("toElementId", edge.to_id.clone())
            .text("predicate", edge.predicate.clone())
            .text("predicateIri", edge.predicate_iri.clone());
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

/// A node of `element_type` at `(x, y)` carrying `payload`.
pub fn node_element(id: &str, (x, y): (f64, f64), element_type: &str, payload: Payload) -> DiagramElement {
    DiagramElement {
        id: id.to_owned(),
        x,
        y,
        placed: true,
        element_type: element_type.to_owned(),
        kind: ElementKind::Node,
        parent: None,
        source: None,
        target: None,
        attributes: payload.into_attributes(),
    }
}

/// An edge of `element_type` from `from` to `to` carrying `payload`, placed nowhere of its own (it follows its ends).
pub fn edge_element(id: &str, from: &str, to: &str, element_type: &str, payload: Payload) -> DiagramElement {
    DiagramElement {
        id: id.to_owned(),
        x: 0.0,
        y: 0.0,
        placed: false,
        element_type: element_type.to_owned(),
        kind: ElementKind::Relation,
        parent: None,
        source: Some(from.to_owned()),
        target: Some(to.to_owned()),
        attributes: payload.into_attributes(),
    }
}
