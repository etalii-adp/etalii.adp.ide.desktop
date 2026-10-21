//! The two-sided mind map layout, `net.etalii.adp.freeplane.mindmapLayout` (etalii.adp spec 030, FR-018; `mindmap.dis`
//! `layout.algorithms.mindmap`), ported from the standalone host's `MindmapLayout.cs` and `_Model/MindmapMetrics.cs` at
//! `8b6ccfc`.
//!
//! The conventional arrangement, computed from the tree alone: the central topic centred on the origin, its first-level
//! branches split to its right and its left, each list of siblings a column centred on its parent, and depth as
//! distance from the centre (mindmap Requirement 5.1). It is a function: the same tree and the same folds give the same
//! boxes (Requirement 5.2), and a folded branch's descendants are not laid out at all, so a hidden node takes no room.
//! A first-level branch goes where Freeplane's `POSITION` puts it (`left` or `right` from 1.11, `top_or_left` or
//! `bottom_or_right` from 1.12) and otherwise alternates right, left, right, as Freeplane lays out an unpositioned map.

use indexmap::IndexMap;

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
pub fn round2(value: f64) -> f64 {
    (value * 100.0).round_ties_even() / 100.0
}

impl MindmapMetrics {
    /// The box a node with `text` occupies: its width and height.
    pub fn measure(&self, text: &str) -> (f64, f64) {
        let width = self
            .minimum_width
            .max(text_metric::width_of(text, self.font_size) + 2.0 * self.horizontal_padding);
        let height = self.font_size * self.line_height + 2.0 * self.vertical_padding;
        (round2(width), round2(height))
    }

    /// The gap between a node of `width` and whatever sits beside it.
    pub fn gap_beside(&self, width: f64) -> f64 {
        self.horizontal_gap.max(width * self.minimum_gap_ratio)
    }

    /// The gap between two stacked subtrees whose members are `first` and `second` wide.
    pub fn gap_between(&self, first: f64, second: f64) -> f64 {
        self.vertical_gap.max(first.max(second) * self.minimum_gap_ratio)
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
    let mut boxes = IndexMap::new();
    let (width, height) = metrics.measure(&root.text);
    let root_box = Rect::new(-width / 2.0, -height / 2.0, width, height);
    boxes.insert(root.id.clone(), root_box);
    if is_folded(&root.id) {
        return boxes;
    }
    let mut right = Vec::new();
    let mut left = Vec::new();
    let mut alternate = 0;
    for child in &root.children {
        let side = child.position.as_deref().map(str::to_lowercase);
        let goes_left = match side.as_deref() {
            Some("left" | "top_or_left") => true,
            Some("right" | "bottom_or_right") => false,
            _ => {
                let left = alternate % 2 == 1;
                alternate += 1;
                left
            }
        };
        if goes_left { left.push(child) } else { right.push(child) }
    }
    let layout = Layout { metrics, is_folded };
    layout.column(
        &right,
        root_box.right() + metrics.gap_beside(root_box.width),
        root_box.center_y(),
        true,
        &mut boxes,
    );
    layout.column(
        &left,
        root_box.x - metrics.gap_beside(root_box.width),
        root_box.center_y(),
        false,
        &mut boxes,
    );
    boxes
}

struct Layout<'a> {
    metrics: &'a MindmapMetrics,
    is_folded: &'a dyn Fn(&str) -> bool,
}

impl Layout<'_> {
    /// Lays sibling subtrees out as a column centred on `center_y`.
    fn column(
        &self,
        siblings: &[&LayoutNode],
        edge_x: f64,
        center_y: f64,
        towards_right: bool,
        boxes: &mut IndexMap<String, Rect>,
    ) {
        if siblings.is_empty() {
            return;
        }
        let heights: Vec<f64> = siblings.iter().map(|s| self.subtree_height(s)).collect();
        let widths: Vec<f64> = siblings.iter().map(|s| self.metrics.measure(&s.text).0).collect();
        let mut total: f64 = heights.iter().sum();
        for i in 0..siblings.len() - 1 {
            total += self.metrics.gap_between(widths[i], widths[i + 1]);
        }
        let mut y = center_y - total / 2.0;
        for i in 0..siblings.len() {
            let slot_center_y = y + heights[i] / 2.0;
            self.subtree(siblings[i], edge_x, slot_center_y, towards_right, boxes);
            y += heights[i];
            if i < siblings.len() - 1 {
                y += self.metrics.gap_between(widths[i], widths[i + 1]);
            }
        }
    }

    /// Places one node at `edge_x`, centred on `center_y`, then its children beyond it.
    fn subtree(
        &self,
        node: &LayoutNode,
        edge_x: f64,
        center_y: f64,
        towards_right: bool,
        boxes: &mut IndexMap<String, Rect>,
    ) {
        let (width, height) = self.metrics.measure(&node.text);
        let x = if towards_right { edge_x } else { edge_x - width };
        let place = Rect::new(round2(x), round2(center_y - height / 2.0), width, height);
        boxes.insert(node.id.clone(), place);
        if (self.is_folded)(&node.id) {
            return;
        }
        let child_edge = if towards_right {
            place.right() + self.metrics.gap_beside(place.width)
        } else {
            place.x - self.metrics.gap_beside(place.width)
        };
        let children: Vec<&LayoutNode> = node.children.iter().collect();
        self.column(&children, child_edge, center_y, towards_right, boxes);
    }

    /// The room a subtree needs across: its visible children stacked, or its own height when it shows none.
    fn subtree_height(&self, node: &LayoutNode) -> f64 {
        let own = self.metrics.measure(&node.text).1;
        if (self.is_folded)(&node.id) || node.children.is_empty() {
            return own;
        }
        let mut stacked: f64 = node.children.iter().map(|c| self.subtree_height(c)).sum();
        for pair in node.children.windows(2) {
            stacked += self.metrics.gap_between(
                self.metrics.measure(&pair[0].text).0,
                self.metrics.measure(&pair[1].text).0,
            );
        }
        own.max(stacked)
    }
}
