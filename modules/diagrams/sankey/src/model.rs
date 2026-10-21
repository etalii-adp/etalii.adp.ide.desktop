//! Everything one `.skv` document declares (etalii.adp spec 030, task T077).
//!
//! Ported from the standalone host's `src/diagrams/sankey/backend/EtAlii.Adp.Diagram.Sankey/_Model/SankeyModel.cs`
//! at `8b6ccfc`: how values read, how flows are coloured, how thick the bands are drawn, the nodes, the flows, and
//! what the parser could not read, which travels with the model rather than being thrown, so a document with one bad
//! entry still draws every good one.

use crate::lines::LineRange;

/// The version this module writes, and the only one it reads.
pub const CURRENT_VERSION: i32 = 1;

/// One thing the parser could not read, with the zero-based line it was found on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseProblem {
    pub line: usize,
    pub message: String,
}

/// A flow takes its source's colour.
pub const FROM_SOURCE: &str = "source";

/// A flow takes its target's colour: the default, which is how an income statement reads.
pub const FROM_TARGET: &str = "target";

/// The document-wide keys, each with the value it has when the document leaves it out, and the zero-based line it
/// was found on.
#[derive(Debug, Clone, PartialEq)]
pub struct SankeySettings {
    /// How a value is written, `{value}` standing for the number: `€{value}M`.
    pub format: String,
    /// Whose colour a flow takes when it states none: `source` or `target`.
    pub flow_color: String,
    /// How thick every band is drawn, as a multiple of the default.
    pub thickness: f64,
    pub format_line: Option<usize>,
    pub flow_color_line: Option<usize>,
    pub thickness_line: Option<usize>,
}

impl Default for SankeySettings {
    fn default() -> Self {
        SankeySettings {
            format: crate::format::PLAIN.to_owned(),
            flow_color: FROM_TARGET.to_owned(),
            thickness: 1.0,
            format_line: None,
            flow_color_line: None,
            thickness_line: None,
        }
    }
}

/// One node entry: something a quantity flows into and out of.
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyNode {
    /// Empty when the entry states none, which the rules report.
    pub id: String,
    pub name: String,
    /// A palette word or `#rrggbb`, verbatim; empty for the default.
    pub color: String,
    /// A line shown under its value.
    pub note: String,
    /// How its value is written, overriding the document's; empty for the document's.
    pub format: String,
    /// The one-based column it is drawn in, or none to have the flows decide.
    pub column: Option<i32>,
    pub description: String,
    pub range: LineRange,
}

impl SankeyNode {
    /// What the label says: the name, or the id when the entry states no name.
    pub fn label(&self) -> &str {
        if self.name.is_empty() { &self.id } else { &self.name }
    }
}

/// One flow entry: a quantity moving from one node to another.
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyFlow {
    /// The id the entry states, or empty: most flows state none.
    pub explicit_id: String,
    pub from: String,
    pub to: String,
    /// How much flows, or none when the document states nothing.
    pub value: Option<f64>,
    /// How much one increment adds, or none for the default.
    pub step: Option<f64>,
    pub color: String,
    pub description: String,
    pub range: LineRange,
}

impl SankeyFlow {
    /// The flow's id: the one it states, or `from->to`.
    pub fn id(&self) -> String {
        if self.explicit_id.is_empty() {
            Self::id_of(&self.from, &self.to)
        } else {
            self.explicit_id.clone()
        }
    }

    /// The id a flow from `from` to `to` has when it states none.
    pub fn id_of(from: &str, to: &str) -> String {
        format!("{from}->{to}")
    }

    /// Its value as drawn: none and below zero are nothing.
    pub fn amount(&self) -> f64 {
        self.value.unwrap_or(0.0).max(0.0)
    }
}

/// One `.skv` document as read.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SankeyModel {
    pub settings: SankeySettings,
    pub nodes: Vec<SankeyNode>,
    pub flows: Vec<SankeyFlow>,
    pub problems: Vec<ParseProblem>,
    pub version: Option<i32>,
}

impl SankeyModel {
    /// The node with `id`: the first entry with it, as the canvas draws it.
    pub fn node_of(&self, id: &str) -> Option<&SankeyNode> {
        self.nodes.iter().find(|node| !node.id.is_empty() && node.id == id)
    }

    /// The flow with `id`: the id it states, or its `from->to`.
    pub fn flow_of(&self, id: &str) -> Option<&SankeyFlow> {
        self.flows.iter().find(|flow| flow.id() == id)
    }

    /// Whether any entry already uses `id`.
    pub fn is_taken(&self, id: &str) -> bool {
        self.nodes.iter().any(|node| node.id == id) || self.flows.iter().any(|flow| flow.id() == id)
    }
}

/// The sizes the document never states (`SankeyGeometry`).
pub mod geometry {
    /// Every node bar's width, in canvas units.
    pub const NODE_WIDTH: f64 = 24.0;
    /// The distance between the left edges of two neighbouring columns.
    pub const COLUMN_PITCH: f64 = 320.0;
    /// The least clear space between two nodes in one column: room for a label's three lines.
    pub const NODE_GAP: f64 = 36.0;
    /// How tall the fullest column's bars are together, at a thickness of 1.
    pub const BASE_HEIGHT: f64 = 560.0;
    /// How tall a node is drawn when nothing flows through it yet, so it can still be seen and picked.
    pub const MINIMUM_NODE_HEIGHT: f64 = 6.0;
    /// How thin a band may be drawn, so a trickle still shows.
    pub const MINIMUM_THICKNESS: f64 = 1.0;
    /// The factor one "thicker" or "thinner" applies.
    pub const THICKNESS_FACTOR: f64 = 1.25;
    /// The thinnest and thickest a diagram may be drawn.
    pub const MINIMUM_SCALE: f64 = 0.1;
    pub const MAXIMUM_SCALE: f64 = 10.0;
}

/// The keys an edit may set, as the document spells them (`SankeyKeys`).
pub mod keys {
    pub const NAME: &str = "name";
    pub const COLOR: &str = "color";
    pub const NOTE: &str = "note";
    pub const FORMAT: &str = "format";
    pub const COLUMN: &str = "column";
    pub const DESCRIPTION: &str = "description";
    pub const VALUE: &str = "value";
    pub const STEP: &str = "step";
}
