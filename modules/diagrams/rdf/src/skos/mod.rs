//! The scheme reading, `w3c/skos` (etalii.adp spec 030, task T083), ported from the standalone host's
//! `src/diagrams/rdf/backend/EtAlii.Adp.Diagram.Rdf/Skos` at `8b6ccfc`: a thesaurus over the same bytes, concepts under
//! their schemes, broader and narrower drawn as a hierarchy, related links across it. It never claims a bare body; a
//! file becomes a scheme diagram only through a registration that names it.
//!
//! - [`projection`], [`labels`] and [`layout`]: what the file states as a vocabulary, the one name each node wears, and
//!   where it sits.
//! - [`mapper`]: what the canvas is sent.
//! - [`selection`], [`actions`] and [`properties`]: the reading's part of the family's menus and rows.
//! - [`validator`] and [`registration_language`]: the findings, and the registration's `language:` header.

pub mod actions;
pub mod labels;
pub mod layout;
pub mod mapper;
pub mod projection;
pub mod properties;
pub mod registration_language;
pub mod selection;
pub mod validator;

use std::collections::HashSet;
use std::path::Path;

use adp_diagram::{DiagramElement, DiagramProblem, DiagramViewport, ToolboxItem};

use crate::model::RdfModel;
use crate::projection::DEFAULT_BUDGET;
use crate::reading::{self, Positions, Reading};
use crate::store::RdfDocument;
use crate::{selection as family_selection, toolbox};

use self::projection::SkosProjectionResult;

/// The scheme reading's origin.
pub const ORIGIN: &str = "w3c/skos";

/// What a drag onto another parent answers (`SkosReading.ParentRefusal`).
pub const PARENT_REFUSAL: &str =
    "A concept is filed under a parent with the hierarchy gesture; dragging changes where it sits on the canvas.";

/// The cell an element occupies for deciding whether it is in view: the layout's own column and row pitch.
pub const CELL_WIDTH: f64 = 240.0;
pub const CELL_HEIGHT: f64 = 110.0;

/// The scheme reading (`SkosReading`), drawing labels in the registration's language, lowercased.
#[derive(Debug, Clone)]
pub struct SkosReading {
    pub language: String,
}

impl Default for SkosReading {
    fn default() -> Self {
        SkosReading {
            language: labels::DEFAULT_LANGUAGE.to_owned(),
        }
    }
}

impl SkosReading {
    /// The reading in `language`, lowercased.
    pub fn new(language: &str) -> Self {
        SkosReading {
            language: language.to_lowercase(),
        }
    }

    /// The reading of a diagram opened with `registration`: its `language:` header, else the default language; a
    /// misplaced header falls back to the default rather than being guessed at (`AddSkos`).
    pub fn for_registration(registration: Option<&Path>) -> Self {
        let (language, _) = registration_language::read(registration);
        SkosReading::new(language.as_deref().unwrap_or(labels::DEFAULT_LANGUAGE))
    }
}

/// The scheme palette: a Concept and nothing else (`SkosToolboxProvider`).
pub fn toolbox_items() -> Vec<ToolboxItem> {
    vec![toolbox::item(
        "skos.toolbox.concept",
        "Concept",
        "mdi-tag-plus-outline",
        "A concept, filed into the file's first scheme. Drop it where it should sit; you will be asked for its preferred label.",
        actions::ADD_CONCEPT,
    )]
}

impl Reading for SkosReading {
    type Projection = SkosProjectionResult;

    fn origin(&self) -> &'static str {
        ORIGIN
    }

    fn parent_refusal(&self) -> &'static str {
        PARENT_REFUSAL
    }

    fn project(&self, model: &RdfModel) -> SkosProjectionResult {
        projection::project(model, usize::MAX)
    }

    fn positions(&self, projection: &SkosProjectionResult, stored: &Positions) -> Positions {
        reading::apply(layout::layout(projection).positions, stored)
    }

    fn in_view(
        &self,
        projection: &SkosProjectionResult,
        positions: &Positions,
        viewport: DiagramViewport,
    ) -> SkosProjectionResult {
        let visible = |id: &str| reading::admits(viewport, positions, id, CELL_WIDTH, CELL_HEIGHT);
        let schemes: Vec<_> = projection.schemes.iter().filter(|s| visible(&s.id)).cloned().collect();
        let concepts: Vec<_> = projection.concepts.iter().filter(|c| visible(&c.id)).cloned().collect();
        let collections: Vec<_> = projection
            .collections
            .iter()
            .filter(|c| visible(&c.id))
            .cloned()
            .collect();
        let truncated = projection.total > DEFAULT_BUDGET;
        let drawn = schemes.len() + concepts.len() + collections.len();
        let kept: HashSet<&str> = schemes
            .iter()
            .map(|s| s.id.as_str())
            .chain(concepts.iter().map(|c| c.id.as_str()))
            .chain(collections.iter().map(|c| c.id.as_str()))
            .collect();
        let edges = projection
            .edges
            .iter()
            .filter(|e| kept.contains(e.from_id.as_str()) && kept.contains(e.to_id.as_str()))
            .cloned()
            .collect();
        SkosProjectionResult {
            schemes,
            concepts,
            collections,
            edges,
            out_of_file_mappings: projection.out_of_file_mappings.clone(),
            shown: if truncated { DEFAULT_BUDGET } else { drawn },
            total: if truncated { projection.total } else { drawn },
        }
    }

    fn elements(&self, projection: &SkosProjectionResult, positions: &Positions) -> Vec<DiagramElement> {
        mapper::elements(projection, positions, &self.language)
    }

    fn refusal(&self, element_id: &str) -> Option<String> {
        if element_id.starts_with("edge:") || element_id == family_selection::TRUNCATION_ID {
            return Some(family_selection::NOT_MOVABLE.to_owned());
        }
        family_selection::is_blank(element_id).then(|| family_selection::BLANK_NOT_MOVABLE.to_owned())
    }

    fn toolbox(&self) -> Vec<ToolboxItem> {
        toolbox_items()
    }

    fn problems(&self, document: &RdfDocument, registration: Option<&Path>) -> Vec<DiagramProblem> {
        validator::validate(&document.text, registration)
    }

    fn connect_action(&self) -> Option<&'static str> {
        Some(actions::FILE_UNDER)
    }
}
