//! The shapes reading, `w3c/shacl` (etalii.adp spec 030, task T083), ported from the standalone host's
//! `src/diagrams/rdf/backend/EtAlii.Adp.Diagram.Rdf/Shacl` at `8b6ccfc`: the constraints a shapes graph states, drawn
//! and never executed.

pub mod actions;
pub mod discovery;
pub mod projection;
pub mod properties;
pub mod refusals;
pub mod validator;
pub mod writer;

use std::collections::BTreeMap;

use crate::model::RdfModel;
use crate::selection;

/// The shapes reading's origin.
pub const ORIGIN: &str = "w3c/shacl";

/// The kinds the canvas switches on (`ShaclElementMapper`).
pub const SHAPE_TYPE: &str = "w3c/shacl+shape";
pub const EDGE_TYPE: &str = "w3c/shacl+edge";
pub const TRUNCATION_TYPE: &str = "w3c/shacl+truncation";

const COLUMN_WIDTH: f64 = 320.0;
const HEADER_HEIGHT: f64 = 64.0;
const LINE_HEIGHT: f64 = 22.0;
const ROW_GAP: f64 = 48.0;
const COLUMNS: usize = 3;

/// The cell a card occupies for deciding whether it is in view (`ShaclReading`).
pub const CARD_WIDTH: f64 = 320.0;
pub const CARD_HEIGHT: f64 = 400.0;

/// What a drag onto another parent answers (`ShaclReading.ParentRefusal`).
pub const PARENT_REFUSAL: &str = "A shape is not held inside another shape; a reference between them is stated as a triple. Dragging changes where a card sits on the canvas.";

/// The height a card is laid out at: its header, and a line per target and row.
pub fn card_height(card: &projection::ShaclCard) -> f64 {
    HEADER_HEIGHT + (card.targets.len() + card.rows.len()) as f64 * LINE_HEIGHT
}

/// The computed layout: cards grid-placed in discovery order, each grid row as tall as its tallest card
/// (`ShaclLayout.Positions`).
pub fn positions(result: &projection::ShaclProjectionResult) -> BTreeMap<String, (f64, f64)> {
    let sizes: Vec<(f64, f64)> = result.cards.iter().map(|c| (COLUMN_WIDTH, card_height(c))).collect();
    let grid = adp_layout::grid_packer::pack(&sizes, COLUMNS, 0.0, ROW_GAP);
    result
        .cards
        .iter()
        .zip(grid.cells.iter())
        .map(|(card, cell)| (card.id.clone(), (cell.x, cell.y)))
        .collect()
}

/// Why `element_id` cannot be given a stored position, or none when it can (`ShaclReading.Refusal`).
pub fn refusal(element_id: &str) -> Option<String> {
    if element_id.starts_with("shacl-edge:") || element_id == selection::TRUNCATION_ID {
        return Some(selection::NOT_MOVABLE.to_owned());
    }
    element_id
        .starts_with("blank:")
        .then(|| refusals::BLANK_ROOTED.to_owned())
}

/// What the gate decided about a selection (`ShaclEditGate.Decision`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    /// Whether this reading has anything to say about the selection at all.
    pub applies: bool,
    pub available: bool,
    pub reason: String,
    /// The IRI-named shape the gestures act on; empty when there is none.
    pub shape_iri: String,
}

impl Decision {
    fn not_applicable() -> Self {
        Decision {
            applies: false,
            available: false,
            reason: String::new(),
            shape_iri: String::new(),
        }
    }
}

/// Whether a selected element may be edited through the shapes reading, decided from the selection and the model
/// alone (`ShaclEditGate.For`).
pub fn gate(model: &RdfModel, element_id: &str, truncated: bool) -> Decision {
    if element_id.is_empty() {
        return Decision::not_applicable();
    }
    let shapes = discovery::discover(model);
    if element_id.starts_with("blank:") {
        return if shapes.iter().any(|s| projection::id_of(&s.key) == element_id) {
            Decision {
                applies: true,
                available: false,
                reason: refusals::BLANK_ROOTED.to_owned(),
                shape_iri: String::new(),
            }
        } else {
            Decision::not_applicable()
        };
    }
    let Some(iri) = element_id.strip_prefix("res:") else {
        return Decision::not_applicable();
    };
    if !shapes.iter().any(|s| s.term.is_iri(iri)) {
        return Decision::not_applicable();
    }
    if truncated {
        Decision {
            applies: true,
            available: false,
            reason: selection::TRUNCATED_REFUSAL.to_owned(),
            shape_iri: iri.to_owned(),
        }
    } else {
        Decision {
            applies: true,
            available: true,
            reason: String::new(),
            shape_iri: iri.to_owned(),
        }
    }
}

/// The drawn shapes edge an id names, as "from → to" (`ShaclSelection.Describe`).
pub fn describe(model: &RdfModel, element_id: &str) -> Option<String> {
    if !element_id.starts_with("shacl-edge:") {
        return None;
    }
    let result = projection::project(model, usize::MAX);
    let edge = result.edges.iter().find(|e| e.id == element_id)?;
    let display_of = |id: &str| {
        result
            .cards
            .iter()
            .find(|c| c.id == id)
            .map_or_else(|| id.to_owned(), |c| c.display.clone())
    };
    Some(format!("{} → {}", display_of(&edge.from_id), display_of(&edge.to_id)))
}
