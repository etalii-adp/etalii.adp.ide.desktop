//! The scheme reading, `w3c/skos` (etalii.adp spec 030, task T083), ported from the standalone host's
//! `src/diagrams/rdf/backend/EtAlii.Adp.Diagram.Rdf/Skos` at `8b6ccfc`. STUB: filled in by the SKOS port.

pub mod actions;
pub mod properties;

use std::path::Path;

use adp_diagram::{DiagramElement, DiagramProblem, DiagramViewport, ToolboxItem};

use crate::model::RdfModel;
use crate::reading::{Positions, Reading};
use crate::store::RdfDocument;

/// The scheme reading's origin.
pub const ORIGIN: &str = "w3c/skos";

/// The scheme reading (`SkosReading`), drawing labels in the registration's language.
#[derive(Debug, Clone, Default)]
pub struct SkosReading {
    pub language: String,
}

impl SkosReading {
    /// The reading of a diagram opened with `registration`: its `language:` header, else the default language.
    pub fn for_registration(registration: Option<&Path>) -> Self {
        let _ = registration;
        SkosReading::default()
    }
}

impl Reading for SkosReading {
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
        Some("skos.file-under")
    }
}
