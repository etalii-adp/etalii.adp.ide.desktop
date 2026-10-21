//! Everything one `.supply` document declares (etalii.adp spec 030, task T076): its groups, its nodes, its flows and
//! what the parser could not read, with the stages a node may be and the sizes the document never states.
//!
//! Ported from the standalone host's `src/diagrams/supply-chain/backend/EtAlii.Adp.Diagram.SupplyChain/_Model/
//! SupplyChainModel.cs` at `8b6ccfc`. The problems travel with the model rather than being thrown: a document with one
//! bad entry still draws every good one, and the validator reports the rest without reading the text again.

use crate::lines::LineRange;

/// What the parser could not read, on the 0-based line it was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseProblem {
    pub line: usize,
    pub message: String,
}

/// One group entry: a region, a company or a tier that nodes are drawn inside.
#[derive(Debug, Clone, PartialEq)]
pub struct SupplyChainGroup {
    /// Empty when the entry states none, which the rules report.
    pub id: String,
    pub name: String,
    pub description: String,
    pub range: LineRange,
    /// The authored left and top of the frame while the group has no members.
    pub x: Option<f64>,
    pub y: Option<f64>,
}

impl SupplyChainGroup {
    /// Whether the document places this group's frame for when it has no members.
    pub fn is_placed(&self) -> bool {
        self.x.is_some() && self.y.is_some()
    }
}

/// One node entry: a stage of the chain.
#[derive(Debug, Clone, PartialEq)]
pub struct SupplyChainNode {
    pub id: String,
    /// Its stage, a legacy word read as the stage it became; an unknown value survives verbatim and is reported.
    pub node_type: String,
    pub name: String,
    pub description: String,
    /// The id of the group it is drawn inside, or empty.
    pub group: String,
    pub quantity: Option<f64>,
    pub unit: String,
    pub step: Option<f64>,
    /// The authored left and top, or none when the layout places it.
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub range: LineRange,
}

impl SupplyChainNode {
    /// Whether the document places this node itself, rather than leaving it to the layout.
    pub fn is_placed(&self) -> bool {
        self.x.is_some() && self.y.is_some()
    }
}

/// One flow entry: goods moving from a supplying node to a consuming one.
#[derive(Debug, Clone, PartialEq)]
pub struct SupplyChainFlow {
    pub id: String,
    pub from: String,
    pub to: String,
    pub product: String,
    pub description: String,
    pub volume: Option<f64>,
    pub unit: String,
    pub step: Option<f64>,
    pub range: LineRange,
}

/// One document.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SupplyChainModel {
    pub groups: Vec<SupplyChainGroup>,
    pub nodes: Vec<SupplyChainNode>,
    pub flows: Vec<SupplyChainFlow>,
    pub problems: Vec<ParseProblem>,
    pub version: Option<i64>,
}

/// The version this module writes, and the only one it reads.
pub const CURRENT_VERSION: i64 = 1;

impl SupplyChainModel {
    /// The node with `id`: the first entry with it, as the canvas draws it.
    pub fn node_of(&self, id: &str) -> Option<&SupplyChainNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn group_of(&self, id: &str) -> Option<&SupplyChainGroup> {
        self.groups.iter().find(|g| g.id == id)
    }

    pub fn flow_of(&self, id: &str) -> Option<&SupplyChainFlow> {
        self.flows.iter().find(|f| f.id == id)
    }

    /// Whether any entry already uses `id`.
    pub fn is_taken(&self, id: &str) -> bool {
        self.groups.iter().any(|g| g.id == id)
            || self.nodes.iter().any(|n| n.id == id)
            || self.flows.iter().any(|f| f.id == id)
    }
}

/// The stages a node may declare (`SupplyChainNodeTypes`): strings rather than an enum, because the parser keeps a
/// stage it does not recognise; and roles rather than industries, so one notation reads for goods, energy, data or
/// services alike. The first words this module wrote still read, as the stage each became.
pub mod stages {
    pub const SOURCE: &str = "source";
    pub const PROCESSOR: &str = "processor";
    pub const PRODUCER: &str = "producer";
    pub const INTEGRATOR: &str = "integrator";
    pub const HUB: &str = "hub";
    pub const OUTLET: &str = "outlet";
    pub const CONSUMER: &str = "consumer";

    /// The seven, in the order goods pass through them.
    pub const ALL: [&str; 7] = [SOURCE, PROCESSOR, PRODUCER, INTEGRATOR, HUB, OUTLET, CONSUMER];

    const LEGACY: [(&str, &str); 6] = [
        ("raw-material", SOURCE),
        ("supplier", PROCESSOR),
        ("manufacturer", PRODUCER),
        ("assembler", INTEGRATOR),
        ("distributor", HUB),
        ("retailer", OUTLET),
    ];

    /// The stage a document's word means: a legacy word reads as its successor, anything else as itself.
    pub fn normalize(stage: &str) -> &str {
        LEGACY
            .iter()
            .find(|(old, _)| *old == stage)
            .map_or(stage, |(_, current)| current)
    }

    /// Where a stage comes in the chain: 0 for a source, 6 for a consumer, 0 for a stage this module does not know.
    pub fn rank_of(stage: &str) -> i32 {
        let normal = normalize(stage);
        ALL.iter().position(|s| *s == normal).map_or(0, |p| p as i32)
    }

    /// Whether the text names one of the seven, or a legacy word for one.
    pub fn is_known(stage: &str) -> bool {
        ALL.contains(&normalize(stage))
    }

    /// The stage as a reader names it: `source` reads "Source".
    pub fn display(stage: &str) -> String {
        let normal = normalize(stage);
        if !ALL.contains(&normal) {
            return stage.to_owned();
        }
        let mut characters = normal.chars();
        characters
            .next()
            .map(|first| first.to_uppercase().chain(characters).collect())
            .unwrap_or_default()
    }
}

/// The sizes the document never states (`SupplyChainGeometry`).
pub mod geometry {
    pub const NODE_WIDTH: f64 = 192.0;
    pub const NODE_HEIGHT: f64 = 96.0;
    /// The space between a group's frame and its members, left, right and below.
    pub const GROUP_PADDING: f64 = 20.0;
    /// The space above a group's members, where its title sits.
    pub const GROUP_HEADER: f64 = 40.0;
    /// An empty group's frame: exactly the frame around one node.
    pub const EMPTY_GROUP_WIDTH: f64 = NODE_WIDTH + 2.0 * GROUP_PADDING;
    pub const EMPTY_GROUP_HEIGHT: f64 = GROUP_HEADER + NODE_HEIGHT + GROUP_PADDING;
    /// What one increment adds when a node or a flow states no step.
    pub const DEFAULT_STEP: f64 = 1.0;
}
