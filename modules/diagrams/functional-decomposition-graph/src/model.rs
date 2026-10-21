//! What one `.fdg` declares: its elements, its connections, and what the parser could not read (etalii.adp spec
//! 030, task T072).
//!
//! Ported from the standalone host's `src/diagrams/functional-decomposition-graph/backend/` at `8b6ccfc`:
//! `_Model/FdgElement.cs`, `FdgConnection.cs`, `FdgGeometry.cs` and `FdgModel.cs`. Types are text rather than
//! enumerations, because the parser keeps a type it does not know: the entry survives, draws nothing and is reported.
//! The problems travel with the model rather than being thrown, and validity is the rule set's answer, never the
//! model's.

use crate::lines::LineRange;

/// The five element types, as the document names them.
pub mod element_types {
    pub const UI_ELEMENT: &str = "ui-element";
    pub const DATA_ELEMENT: &str = "data-element";
    pub const ACTION: &str = "action";
    pub const FUNCTION: &str = "function";
    pub const COMMENT: &str = "comment";

    /// The five, in the order the design lists them.
    pub const ALL: [&str; 5] = [UI_ELEMENT, DATA_ELEMENT, ACTION, FUNCTION, COMMENT];

    pub fn is_known(type_name: &str) -> bool {
        ALL.contains(&type_name)
    }
}

/// The five relations, as the document names them: the type names the relation.
pub mod connection_types {
    pub const UI_CHILD: &str = "ui-child";
    pub const OWNS_ACTION: &str = "owns-action";
    pub const OWNS_DATA: &str = "owns-data";
    pub const OWNS_FUNCTION: &str = "owns-function";
    pub const SHOWS: &str = "shows";

    /// The five, in the order the design lists them.
    pub const ALL: [&str; 5] = [UI_CHILD, OWNS_ACTION, OWNS_DATA, OWNS_FUNCTION, SHOWS];

    pub fn is_known(type_name: &str) -> bool {
        ALL.contains(&type_name)
    }
}

/// The sizes the document does not state per element (`FdgGeometry`).
pub mod geometry {
    /// The height of every element but a Comment, which the document never carries.
    pub const SHARED_HEIGHT: f64 = 48.0;
    /// The narrowest an element may be; a smaller width in the file opens and is reported.
    pub const MINIMUM_WIDTH: f64 = 80.0;
    /// The shortest a Comment may be.
    pub const MINIMUM_COMMENT_HEIGHT: f64 = 48.0;
}

/// One element entry, and the lines that declare it.
#[derive(Debug, Clone, PartialEq)]
pub struct FdgElement {
    /// Empty when the entry states none, which the rules report.
    pub id: String,
    /// Verbatim; an unknown type survives and is reported.
    pub element_type: String,
    /// A Comment has none.
    pub name: String,
    /// Prose about the element, never drawn.
    pub description: String,
    /// A Comment's body, from a block scalar.
    pub text: String,
    /// The left edge, not the centre.
    pub x: f64,
    /// The top edge, not the centre.
    pub y: f64,
    pub width: f64,
    /// A Comment's height; none for every other type, which share one.
    pub height: Option<f64>,
    pub range: LineRange,
}

impl FdgElement {
    /// The height to draw: a Comment's own, or the one the other four share.
    pub fn drawn_height(&self) -> f64 {
        self.height.unwrap_or(geometry::SHARED_HEIGHT)
    }

    pub fn is_comment(&self) -> bool {
        self.element_type == element_types::COMMENT
    }
}

/// One connection entry, and the lines that declare it.
#[derive(Debug, Clone, PartialEq)]
pub struct FdgConnection {
    pub id: String,
    /// The relation, verbatim.
    pub connection_type: String,
    /// The source element's id.
    pub from: String,
    /// The target element's id.
    pub to: String,
    pub name: String,
    /// Prose about the connection, never drawn.
    pub description: String,
    pub range: LineRange,
}

/// What the parser passed over, with the zero-based line that caused it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseProblem {
    pub line: usize,
    pub message: String,
}

/// Everything one document declares.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FdgModel {
    pub elements: Vec<FdgElement>,
    pub connections: Vec<FdgConnection>,
    pub problems: Vec<ParseProblem>,
    /// The header's version, or none when the header is missing or unreadable.
    pub version: Option<i64>,
}

impl FdgModel {
    /// The version this module writes, and the only one it understands.
    pub const CURRENT_VERSION: i64 = 1;

    /// The element with `id`: the first entry with it, as the canvas draws it.
    pub fn element_of(&self, id: &str) -> Option<&FdgElement> {
        self.elements.iter().find(|e| e.id == id)
    }

    /// The connection with `id`: the first entry with it.
    pub fn connection_of(&self, id: &str) -> Option<&FdgConnection> {
        self.connections.iter().find(|c| c.id == id)
    }

    /// Whether any element or connection already uses `id`.
    pub fn is_taken(&self, id: &str) -> bool {
        self.elements.iter().any(|e| e.id == id) || self.connections.iter().any(|c| c.id == id)
    }
}
