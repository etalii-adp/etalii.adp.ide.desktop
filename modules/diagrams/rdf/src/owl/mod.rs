//! The ontology reading, `w3c/owl` (etalii.adp spec 030, task T083), ported from the standalone host's
//! `src/diagrams/rdf/backend/EtAlii.Adp.Diagram.Rdf/Owl` at `8b6ccfc`. STUB: filled in by the OWL port.

pub mod selection;

use std::path::Path;

use adp_diagram::{DiagramElement, DiagramProblem, DiagramViewport, ToolboxItem};

use crate::model::RdfModel;
use crate::reading::{Positions, Reading};
use crate::store::RdfDocument;

/// The ontology reading's origin.
pub const ORIGIN: &str = "w3c/owl";

/// The ontology reading (`OwlReading`).
#[derive(Debug, Clone, Copy, Default)]
pub struct OwlReading;

impl Reading for OwlReading {
    type Projection = ();

    fn origin(&self) -> &'static str {
        ORIGIN
    }

    fn parent_refusal(&self) -> &'static str {
        ""
    }

    fn project(&self, _model: &RdfModel) {}

    fn positions(&self, _projection: &(), _stored: &Positions) -> Positions {
        Positions::new()
    }

    fn in_view(&self, _projection: &(), _positions: &Positions, _viewport: DiagramViewport) {}

    fn elements(&self, _projection: &(), _positions: &Positions) -> Vec<DiagramElement> {
        Vec::new()
    }

    fn refusal(&self, _element_id: &str) -> Option<String> {
        None
    }

    fn toolbox(&self) -> Vec<ToolboxItem> {
        Vec::new()
    }

    fn problems(&self, document: &RdfDocument, _registration: Option<&Path>) -> Vec<DiagramProblem> {
        crate::validator::validate(&document.text)
    }

    fn connect_action(&self) -> Option<&'static str> {
        Some("owl.subclass")
    }
}
