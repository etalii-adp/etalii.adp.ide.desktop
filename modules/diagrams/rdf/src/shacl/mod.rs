//! The shapes reading, `w3c/shacl` (etalii.adp spec 030, task T083), ported from the standalone host's
//! `src/diagrams/rdf/backend/EtAlii.Adp.Diagram.Rdf/Shacl` at `8b6ccfc`: the constraints a shapes graph states, drawn
//! and never executed.

pub mod actions;
pub mod discovery;
pub mod mapper;
pub mod projection;
pub mod properties;
pub mod refusals;
pub mod validator;
pub mod writer;

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use adp_diagram::{DiagramElement, DiagramProblem, DiagramViewport, ToolboxItem};

use crate::model::RdfModel;
use crate::reading::{self, Positions, Reading};
use crate::selection;
use crate::store::RdfDocument;

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
    let nodes: Vec<(String, f64, f64)> = result
        .cards
        .iter()
        .map(|card| (card.id.clone(), COLUMN_WIDTH, card_height(card)))
        .collect();
    adp_atd_runtime::capabilities::grid::place_grid(&nodes, COLUMNS, 0.0, ROW_GAP)
        .into_iter()
        .map(|(id, x, y)| (id, (x, y)))
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

/// The shapes reading (`ShaclReading`): the projection and the grid layout over the family's store, with the
/// registration's `layout:` positions on top. It consumes no registration header, deliberately: a shapes graph
/// constrains any number of data graphs, so binding a registration to one would misstate the medium.
#[derive(Debug, Clone, Copy, Default)]
pub struct ShaclReading;

impl Reading for ShaclReading {
    type Projection = projection::ShaclProjectionResult;

    fn origin(&self) -> &'static str {
        ORIGIN
    }

    fn parent_refusal(&self) -> &'static str {
        PARENT_REFUSAL
    }

    fn project(&self, model: &RdfModel) -> Self::Projection {
        projection::project(model, usize::MAX)
    }

    fn positions(&self, projection: &Self::Projection, stored: &Positions) -> Positions {
        reading::apply(positions(projection), stored)
    }

    /// The cards the viewport admits, the edges both of whose ends survived, the budget as a floor; the counts stay
    /// the document's (`ShaclReading.InView`).
    fn in_view(
        &self,
        projection: &Self::Projection,
        positions: &Positions,
        viewport: DiagramViewport,
    ) -> Self::Projection {
        let visible: Vec<projection::ShaclCard> = projection
            .cards
            .iter()
            .filter(|card| reading::admits(viewport, positions, &card.id, CARD_WIDTH, CARD_HEIGHT))
            .cloned()
            .collect();
        let truncated = projection.total > projection::DEFAULT_BUDGET;
        let kept: Vec<projection::ShaclCard> = visible.into_iter().take(projection::DEFAULT_BUDGET).collect();
        let kept_ids: HashSet<&str> = kept.iter().map(|c| c.id.as_str()).collect();
        let edges = projection
            .edges
            .iter()
            .filter(|e| kept_ids.contains(e.from_id.as_str()) && kept_ids.contains(e.to_id.as_str()))
            .cloned()
            .collect();
        let count = kept.len();
        projection::ShaclProjectionResult {
            cards: kept,
            edges,
            shown: if truncated { projection::DEFAULT_BUDGET } else { count },
            total: if truncated { projection.total } else { count },
        }
    }

    fn elements(&self, projection: &Self::Projection, positions: &Positions) -> Vec<DiagramElement> {
        mapper::elements(projection, positions)
    }

    fn refusal(&self, element_id: &str) -> Option<String> {
        refusal(element_id)
    }

    fn toolbox(&self) -> Vec<ToolboxItem> {
        toolbox()
    }

    fn problems(&self, document: &RdfDocument, _registration: Option<&Path>) -> Vec<DiagramProblem> {
        validator::validate(&document.text)
    }
}

/// Whether Add should offer this reading for `text`: a textual marker test, judging whether the reading is worth
/// offering rather than whether the file is good SHACL (`Diagram.Shacl.SuggestsBody`).
pub fn suggests_body(text: &str) -> bool {
    text.contains("sh:NodeShape") || text.contains("sh:property") || text.contains(crate::vocabulary::shacl::NODE_SHAPE)
}

/// The shapes palette, each entry naming the action its drop commits (`ShaclToolboxProvider`).
pub fn toolbox() -> Vec<ToolboxItem> {
    vec![
        crate::toolbox::item(
            "shacl.toolbox.node-shape",
            "Node shape",
            "mdi-check-decagram-outline",
            "A shape that constrains focus nodes. Drop it where it should sit; you will be asked for its name.",
            actions::ADD_NODE_SHAPE,
        ),
        crate::toolbox::item(
            "shacl.toolbox.property-row",
            "Property row",
            "mdi-table-row-plus-after",
            "A constraint on the values reached over one path. Drop it on a shape; you will be asked for the path.",
            actions::ADD_PROPERTY_ROW,
        ),
    ]
}
