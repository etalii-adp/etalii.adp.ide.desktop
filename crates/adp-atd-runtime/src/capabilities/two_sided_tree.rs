//! `layout.twoSidedTree` (ATD §13, `specifications/atd/capabilities/layout.twoSidedTree/`): a tree laid out to both
//! sides of its root. Moved, not rewritten, from the mind map module's layout (`modules/diagrams/mindmap/src/layout.rs`
//! at `fe5feec`, itself ported from the standalone host's `MindmapLayout.cs` at `8b6ccfc`), so that the code that
//! produced the reference bytes is the code the capability runs (research R14); the mind map calls it from here.
//!
//! The root is centred on the origin; its children are split to its right and its left, each list of siblings a
//! column centred on its parent, and depth is distance from the root. A child whose side is fixed keeps it; the others
//! alternate right, left, right. A collapsed node's descendants are not laid out, so a hidden node takes no room. The
//! same tree, sizes and collapsed nodes give the same boxes.

use adp_layout::Rect;
use indexmap::IndexMap;

/// The gaps the layout keeps (the capability's arguments).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gaps {
    /// `spacing`: between sibling subtrees, at least.
    pub spacing: f64,
    /// `levelGap`: between a node's edge and its children's edge, at least.
    pub level_gap: f64,
    /// `minimumGapRatio`: the share of a box's width a gap grows to.
    pub minimum_gap_ratio: f64,
}

impl Default for Gaps {
    fn default() -> Self {
        Gaps {
            spacing: 10.0,
            level_gap: 48.0,
            minimum_gap_ratio: 0.1,
        }
    }
}

impl Gaps {
    /// The gap between a node of `width` and whatever sits beside it.
    pub fn beside(&self, width: f64) -> f64 {
        self.level_gap.max(width * self.minimum_gap_ratio)
    }

    /// The gap between two stacked subtrees whose members are `first` and `second` wide.
    pub fn between(&self, first: f64, second: f64) -> f64 {
        self.spacing.max(first.max(second) * self.minimum_gap_ratio)
    }
}

/// The side of the root a first-level node is fixed to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Right,
    Left,
}

/// One node: its id, its measured box, the side it is fixed to (read on the root's children only), and its children
/// in order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TreeNode {
    pub id: String,
    pub width: f64,
    pub height: f64,
    pub side: Option<Side>,
    pub children: Vec<TreeNode>,
}

/// .NET's `Math.Round(value, 2)`: to two decimals, a midpoint to the even neighbour.
pub fn round2(value: f64) -> f64 {
    (value * 100.0).round_ties_even() / 100.0
}

/// Boxes for every visible node, by id, in the order they were placed; `is_collapsed` says which nodes hide their
/// children.
pub fn layout(root: &TreeNode, gaps: &Gaps, is_collapsed: &dyn Fn(&str) -> bool) -> IndexMap<String, Rect> {
    let mut boxes = IndexMap::new();
    let root_box = Rect::new(-root.width / 2.0, -root.height / 2.0, root.width, root.height);
    boxes.insert(root.id.clone(), root_box);
    if is_collapsed(&root.id) {
        return boxes;
    }
    let mut right = Vec::new();
    let mut left = Vec::new();
    let mut alternate = 0;
    for child in &root.children {
        let goes_left = match child.side {
            Some(Side::Left) => true,
            Some(Side::Right) => false,
            None => {
                let left = alternate % 2 == 1;
                alternate += 1;
                left
            }
        };
        if goes_left { left.push(child) } else { right.push(child) }
    }
    let layout = Layout { gaps, is_collapsed };
    layout.column(
        &right,
        root_box.right() + gaps.beside(root_box.width),
        root_box.center_y(),
        true,
        &mut boxes,
    );
    layout.column(
        &left,
        root_box.x - gaps.beside(root_box.width),
        root_box.center_y(),
        false,
        &mut boxes,
    );
    boxes
}

struct Layout<'a> {
    gaps: &'a Gaps,
    is_collapsed: &'a dyn Fn(&str) -> bool,
}

impl Layout<'_> {
    /// Lays sibling subtrees out as a column centred on `center_y`.
    fn column(
        &self,
        siblings: &[&TreeNode],
        edge_x: f64,
        center_y: f64,
        towards_right: bool,
        boxes: &mut IndexMap<String, Rect>,
    ) {
        if siblings.is_empty() {
            return;
        }
        let heights: Vec<f64> = siblings.iter().map(|s| self.subtree_height(s)).collect();
        let widths: Vec<f64> = siblings.iter().map(|s| s.width).collect();
        let mut total: f64 = heights.iter().sum();
        for i in 0..siblings.len() - 1 {
            total += self.gaps.between(widths[i], widths[i + 1]);
        }
        let mut y = center_y - total / 2.0;
        for i in 0..siblings.len() {
            let slot_center_y = y + heights[i] / 2.0;
            self.subtree(siblings[i], edge_x, slot_center_y, towards_right, boxes);
            y += heights[i];
            if i < siblings.len() - 1 {
                y += self.gaps.between(widths[i], widths[i + 1]);
            }
        }
    }

    /// Places one node at `edge_x`, centred on `center_y`, then its children beyond it.
    fn subtree(
        &self,
        node: &TreeNode,
        edge_x: f64,
        center_y: f64,
        towards_right: bool,
        boxes: &mut IndexMap<String, Rect>,
    ) {
        let (width, height) = (node.width, node.height);
        let x = if towards_right { edge_x } else { edge_x - width };
        let place = Rect::new(round2(x), round2(center_y - height / 2.0), width, height);
        boxes.insert(node.id.clone(), place);
        if (self.is_collapsed)(&node.id) {
            return;
        }
        let child_edge = if towards_right {
            place.right() + self.gaps.beside(place.width)
        } else {
            place.x - self.gaps.beside(place.width)
        };
        let children: Vec<&TreeNode> = node.children.iter().collect();
        self.column(&children, child_edge, center_y, towards_right, boxes);
    }

    /// The room a subtree needs across: its visible children stacked, or its own height when it shows none.
    fn subtree_height(&self, node: &TreeNode) -> f64 {
        let own = node.height;
        if (self.is_collapsed)(&node.id) || node.children.is_empty() {
            return own;
        }
        let mut stacked: f64 = node.children.iter().map(|c| self.subtree_height(c)).sum();
        for pair in node.children.windows(2) {
            stacked += self.gaps.between(pair[0].width, pair[1].width);
        }
        own.max(stacked)
    }
}
