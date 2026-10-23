//! The two-sided mind map layout, `net.etalii.adp.freeplane.mindmapLayout` (etalii.adp spec 030, FR-018; `mindmap.dis`
//! `layout.algorithms.mindmap`), ported from the standalone host's `MindmapLayout.cs` and `_Model/MindmapMetrics.cs` at
//! `8b6ccfc`. Its placement is the capability `layout.twoSidedTree`, moved to `adp-atd-runtime` (etalii.adp spec 035,
//! T125); what stays here is what is the mind map's: the metric that sizes a node from its text, and Freeplane's
//! `POSITION`.
//!
//! The conventional arrangement, computed from the tree alone: the central topic centred on the origin, its first-level
//! branches split to its right and its left, each list of siblings a column centred on its parent, and depth as
//! distance from the centre (mindmap Requirement 5.1). It is a function: the same tree and the same folds give the same
//! boxes (Requirement 5.2), and a folded branch's descendants are not laid out at all, so a hidden node takes no room.
//! A first-level branch goes where Freeplane's `POSITION` puts it (`left` or `right` from 1.11, `top_or_left` or
//! `bottom_or_right` from 1.12) and otherwise alternates right, left, right, as Freeplane lays out an unpositioned map.

use indexmap::IndexMap;

use adp_atd_runtime::capabilities::two_sided_tree::{self, Gaps, Side, TreeNode};
use adp_layout::Rect;
use adp_layout::text_metric;

use crate::document::MindmapDocument;
use crate::xml::NodeId;

/// The declared font metric the layout sizes nodes with (mindmap Requirement 5.6), and the room it keeps around them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindmapMetrics {
    /// In canvas units.
    pub font_size: f64,
    /// As a fraction of the font size.
    pub line_height: f64,
    /// Inside the node box, each side.
    pub horizontal_padding: f64,
    /// Inside the node box, top and bottom.
    pub vertical_padding: f64,
    /// Between a parent's edge and its children's edge, at least.
    pub horizontal_gap: f64,
    /// Between sibling subtrees, at least.
    pub vertical_gap: f64,
    /// So an empty node is still something to click on.
    pub minimum_width: f64,
    /// Elements keep at least this share of a node's width between them.
    pub minimum_gap_ratio: f64,
}

impl Default for MindmapMetrics {
    fn default() -> Self {
        MindmapMetrics {
            font_size: 14.0,
            line_height: 1.4,
            horizontal_padding: 10.0,
            vertical_padding: 6.0,
            horizontal_gap: 48.0,
            vertical_gap: 10.0,
            minimum_width: 32.0,
            minimum_gap_ratio: 0.1,
        }
    }
}

/// .NET's `Math.Round(value, 2)`: to two decimals, a midpoint to the even neighbour.
pub use adp_atd_runtime::capabilities::two_sided_tree::round2;

impl MindmapMetrics {
    /// The box a node with `text` occupies: its width and height.
    pub fn measure(&self, text: &str) -> (f64, f64) {
        let width = self
            .minimum_width
            .max(text_metric::width_of(text, self.font_size) + 2.0 * self.horizontal_padding);
        let height = self.font_size * self.line_height + 2.0 * self.vertical_padding;
        (round2(width), round2(height))
    }

    /// The gaps of the capability `layout.twoSidedTree`.
    pub fn gaps(&self) -> Gaps {
        Gaps {
            spacing: self.vertical_gap,
            level_gap: self.horizontal_gap,
            minimum_gap_ratio: self.minimum_gap_ratio,
        }
    }
}

/// One node as the layout sees it: its id, its text, its side when the file states one, and its children in order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LayoutNode {
    pub id: String,
    pub text: String,
    pub position: Option<String>,
    pub children: Vec<LayoutNode>,
}

impl LayoutNode {
    /// The tree of `document`, each node by its `ID`.
    pub fn of(document: &MindmapDocument) -> LayoutNode {
        Self::from_node(document, document.root(), &|node| document.id(node))
    }

    /// The tree of `document`, each node by the id `id_of` gives it.
    pub fn with_ids(document: &MindmapDocument, id_of: &dyn Fn(NodeId) -> String) -> LayoutNode {
        Self::from_node(document, document.root(), id_of)
    }

    fn from_node(document: &MindmapDocument, node: NodeId, id_of: &dyn Fn(NodeId) -> String) -> LayoutNode {
        LayoutNode {
            id: id_of(node),
            text: document.text(node),
            position: document.position(node).map(str::to_owned),
            children: document
                .children(node)
                .into_iter()
                .map(|child| Self::from_node(document, child, id_of))
                .collect(),
        }
    }
}

/// Boxes for every visible node, by id, in the order they were placed; `is_folded` says which nodes hide their
/// children for the viewer being laid out for.
pub fn compute(
    root: &LayoutNode,
    metrics: &MindmapMetrics,
    is_folded: &dyn Fn(&str) -> bool,
) -> IndexMap<String, Rect> {
    two_sided_tree::layout(&tree(root, metrics), &metrics.gaps(), is_folded)
}

/// The tree as the capability takes it: each node sized by the metric, a first-level node's `POSITION` as its side.
fn tree(node: &LayoutNode, metrics: &MindmapMetrics) -> TreeNode {
    let (width, height) = metrics.measure(&node.text);
    let side = match node.position.as_deref().map(str::to_lowercase).as_deref() {
        Some("left" | "top_or_left") => Some(Side::Left),
        Some("right" | "bottom_or_right") => Some(Side::Right),
        _ => None,
    };
    TreeNode {
        id: node.id.clone(),
        width,
        height,
        side,
        children: node.children.iter().map(|child| tree(child, metrics)).collect(),
    }
}
