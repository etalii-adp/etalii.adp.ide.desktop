//! The ontology reading, `w3c/owl` (etalii.adp spec 030, task T083), ported from the standalone host's
//! `src/diagrams/rdf/backend/EtAlii.Adp.Diagram.Rdf/Owl` at `8b6ccfc`: an OWL 2 ontology's classes, hierarchy,
//! properties and restrictions, drawn in the VOWL vocabulary - what the file asserts, not what a reasoner would infer.
//!
//! - [`index`], [`projection`] and [`expression`]: the asserted ontology and what it draws.
//! - [`hierarchy`] and [`layout`]: the class tree as columns, satellites in lanes, individuals in a band.
//! - [`mapper`], [`selection`], [`toolbox`] and [`validator`]: what the canvas is sent, what an id names, the palette
//!   and the findings.

pub mod expression;
pub mod hierarchy;
pub mod index;
pub mod layout;
pub mod mapper;
pub mod projection;
pub mod selection;
pub mod toolbox;
pub mod validator;

use std::collections::HashSet;
use std::path::Path;

use adp_diagram::{DiagramElement, DiagramProblem, DiagramViewport, ToolboxItem};

use crate::model::RdfModel;
use crate::owl::projection::OwlGraph;
use crate::projection::DEFAULT_BUDGET;
use crate::reading::{self, Positions, Reading};
use crate::selection as family_selection;
use crate::store::RdfDocument;

/// The ontology reading's origin.
pub const ORIGIN: &str = "w3c/owl";

/// What a drag onto another parent answers (`OwlReading.ParentRefusal`).
pub const PARENT_REFUSAL: &str =
    "An ontology element has no parent to move it under; dragging changes where it sits on the canvas.";

/// What moving an expression node answers.
pub const EXPRESSION_NOT_MOVABLE: &str = "That is an anonymous class expression, whose identity does not survive an edit to the file, so a stored position could not be trusted. It takes its place beside the class that uses it; edit the expression as text.";

/// What moving a materialized anchor answers.
pub const ANCHOR_NOT_MOVABLE: &str =
    "That element is drawn where its property needs it and cannot be arranged on its own.";

/// The ontology reading (`OwlReading`).
#[derive(Debug, Clone, Copy, Default)]
pub struct OwlReading;

impl Reading for OwlReading {
    type Projection = OwlGraph;

    fn origin(&self) -> &'static str {
        ORIGIN
    }

    fn parent_refusal(&self) -> &'static str {
        PARENT_REFUSAL
    }

    fn project(&self, model: &RdfModel) -> OwlGraph {
        projection::project(model, usize::MAX)
    }

    fn positions(&self, projection: &OwlGraph, stored: &Positions) -> Positions {
        layout::apply(layout::positions(projection), stored)
    }

    /// Each node is judged on the box it occupies, not on a nominal cell; the counts handed on are the document's.
    fn in_view(&self, projection: &OwlGraph, positions: &Positions, viewport: DiagramViewport) -> OwlGraph {
        let visible: Vec<_> = projection
            .nodes
            .iter()
            .filter(|node| {
                let (width, height) = layout::size_of(node);
                reading::admits(viewport, positions, &node.id, width, height)
            })
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
        OwlGraph {
            nodes: kept,
            edges,
            shown: if truncated { DEFAULT_BUDGET } else { count },
            total: if truncated { projection.total } else { count },
        }
    }

    fn elements(&self, projection: &OwlGraph, positions: &Positions) -> Vec<DiagramElement> {
        mapper::elements(projection, positions)
    }

    fn refusal(&self, element_id: &str) -> Option<String> {
        if element_id.starts_with("edge:") || element_id == family_selection::TRUNCATION_ID {
            return Some(family_selection::NOT_MOVABLE.to_owned());
        }
        if element_id.starts_with("expr:") {
            return Some(EXPRESSION_NOT_MOVABLE.to_owned());
        }
        (!layout::is_positionable(element_id)).then(|| ANCHOR_NOT_MOVABLE.to_owned())
    }

    fn toolbox(&self) -> Vec<ToolboxItem> {
        toolbox::items()
    }

    fn problems(&self, document: &RdfDocument, _registration: Option<&Path>) -> Vec<DiagramProblem> {
        validator::validate(&document.text)
    }

    fn connect_action(&self) -> Option<&'static str> {
        Some("owl.subclass")
    }
}
