//! Where each node is drawn: a top-down tree, children left to right in the order the Markdown lists them, each parent
//! centred over its children; and what a drag of one node means (etalii.adp spec 030, task T074).
//!
//! Ported from the standalone host's `AbmLayout.cs` and the `AbmArrangement` of `Commands/ArrangeAbmNodeCommand.cs` at
//! `8b6ccfc`:
//!
//! - The layout is computed, because the order is the meaning: in a behavior tree the left-to-right order of a node's
//!   children is the order they are tried in. Each subtree is placed as close to its left sibling as their outlines
//!   allow (the Reingold-Tilford rule), a parent centred over its first and last child, the leftmost node at x = 0.
//! - An author's drag moves a row, never a single box. Across, a node's place is its order among its siblings, so the
//!   computed x always holds; down, every child of one parent sits at the same height, the first height the
//!   registration stores for any node of the row, or 56 below its parent when none is, never closer than 16.
//! - Positions are top-left corners, as the registration stores them.

use std::collections::HashMap;

use adp_atd_runtime::capabilities::tidy_tree::{self, TidyGaps, TidyTree};

use crate::model::{Model, Node};

/// Every node's width.
pub const NODE_WIDTH: f64 = 200.0;
/// Every node's height: the keyword line and the label line.
pub const NODE_HEIGHT: f64 = 60.0;
/// The space between two neighbouring subtrees.
pub const HORIZONTAL_GAP: f64 = 28.0;
/// The space between a parent's bottom and its children's top.
pub const VERTICAL_GAP: f64 = 56.0;
/// The least space a dragged row keeps between its parent's bottom and its own top.
pub const MINIMUM_GAP: f64 = 16.0;

/// A top-left corner, by node id.
pub type Positions = HashMap<String, (f64, f64)>;

/// Where each node is drawn, the author's drags included: the computed x, and the height of its row.
pub fn arrange(model: &Model, stored: &Positions) -> Positions {
    let computed = compute(model);
    let mut positions = Positions::new();
    place_row(
        model,
        &model.roots(),
        0.0,
        f64::NEG_INFINITY,
        &computed,
        stored,
        &mut positions,
    );
    positions
}

fn place_row(
    model: &Model,
    row: &[&Node],
    hanging_y: f64,
    floor: f64,
    computed: &Positions,
    stored: &Positions,
    positions: &mut Positions,
) {
    let dragged = row.iter().find_map(|node| stored.get(&node.id).map(|p| p.1));
    let y = dragged.unwrap_or(hanging_y).max(floor);
    for node in row {
        positions.insert(node.id.clone(), (computed[&node.id].0, y));
        place_row(
            model,
            &model.children_of(node),
            y + NODE_HEIGHT + VERTICAL_GAP,
            y + NODE_HEIGHT + MINIMUM_GAP,
            computed,
            stored,
            positions,
        );
    }
}

/// The top-left of every node, by id, as the tree alone places it: the capability `layout.tidyTree`, which this code
/// moved to (etalii.adp spec 076, T008), over the model's tree in document order.
pub fn compute(model: &Model) -> Positions {
    let tree = TidyTree {
        roots: model.roots().iter().map(|root| root.id.clone()).collect(),
        children: model
            .nodes
            .iter()
            .map(|node| (node.id.clone(), node.child_ids.clone()))
            .collect(),
        width: NODE_WIDTH,
        height: NODE_HEIGHT,
    };
    let gaps = TidyGaps {
        node: HORIZONTAL_GAP,
        layer: VERTICAL_GAP,
        component: HORIZONTAL_GAP * 2.0,
    };
    tidy_tree::compute(&tree, &gaps)
}

/// The centre of the node whose top-left is `top_left`.
pub fn centre(top_left: (f64, f64)) -> (f64, f64) {
    (top_left.0 + NODE_WIDTH / 2.0, top_left.1 + NODE_HEIGHT / 2.0)
}

/// What one drop means for a node's row: its place among its siblings before and after, and how far the row moves
/// down (negative is up).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arrangement {
    pub from: usize,
    pub to: usize,
    pub dy: f64,
}

impl Arrangement {
    /// What dropping `node` with its top-left at (`x`, `y`) means. Across, the order: the node goes before the first
    /// other sibling whose middle is right of the drop's middle. Down, the row: every child of one parent stays at one
    /// height, so the whole row moves by what the node moved, never closer to the parent than [`MINIMUM_GAP`].
    pub fn of(model: &Model, stored: &Positions, node: &Node, x: f64, y: f64) -> Arrangement {
        let positions = arrange(model, stored);
        let siblings = model.siblings_of(node);
        let from = siblings.iter().position(|s| s.id == node.id).unwrap_or_default();
        let middle = x + NODE_WIDTH / 2.0;
        let to = siblings
            .iter()
            .filter(|s| s.id != node.id && positions[&s.id].0 + NODE_WIDTH / 2.0 < middle)
            .count();
        let floor = match &node.parent_id {
            Some(parent) => positions[parent].1 + NODE_HEIGHT + MINIMUM_GAP,
            None => f64::NEG_INFINITY,
        };
        let dy = y.max(floor) - positions[&node.id].1;
        Arrangement { from, to, dy }
    }

    /// Whether the drop changes nothing: the same place, and the same height.
    pub fn is_nothing(&self) -> bool {
        self.from == self.to && self.dy.abs() < 0.5
    }

    /// The stored positions with their ids following the nodes after the move from `from` to `to`: an id is a place
    /// in the tree, so a reorder renames every id in the siblings' subtrees.
    pub fn renamed(&self, parent_id: Option<&str>, sibling_count: usize, stored: &Positions) -> Positions {
        let mut order: Vec<usize> = (0..sibling_count).filter(|i| *i != self.from).collect();
        order.insert(self.to, self.from);
        let mut renamed = Positions::new();
        for (id, position) in stored {
            let mut key = id.clone();
            for (new_index, old_index) in order.iter().enumerate() {
                let old = place_of(parent_id, *old_index);
                if Model::is_within(id, &old) {
                    key = format!("{}{}", place_of(parent_id, new_index), &id[old.len()..]);
                    break;
                }
            }
            renamed.insert(key, *position);
        }
        renamed
    }

    /// The index a move takes for this arrangement: before the child now there, the node itself still counted.
    pub fn move_index(&self) -> i64 {
        if self.to <= self.from {
            self.to as i64
        } else {
            self.to as i64 + 1
        }
    }
}

fn place_of(parent_id: Option<&str>, index: usize) -> String {
    match parent_id {
        None => (index + 1).to_string(),
        Some(parent) => format!("{parent}.{}", index + 1),
    }
}

/// Where a node dropped at (`x`, `y`) belongs: under the nearest node above it that takes another child (by the
/// distance across plus twice the distance down, the first in document order on a tie), before the first of its
/// children drawn right of it; the root of an empty tree; or the refusal.
pub fn place_drop(model: &Model, stored: &Positions, x: f64, y: f64) -> Result<(Option<String>, usize), String> {
    if model.nodes.is_empty() {
        return Ok((None, 0));
    }
    let positions = arrange(model, stored);
    let rank = |node: &Node| {
        let (cx, cy) = centre(positions[&node.id]);
        (x - cx).abs() + 2.0 * (y - cy)
    };
    let mut best: Option<(&Node, f64)> = None;
    for node in &model.nodes {
        if !node.takes_another_child() || centre(positions[&node.id]).1 >= y {
            continue;
        }
        let r = rank(node);
        if best.is_none_or(|(_, b)| r < b) {
            best = Some((node, r));
        }
    }
    let Some((parent, _)) = best else {
        return Err("Drop the node below the node it belongs under: a Do in order, a Try in order, a Do together, or a Retry, Repeat until, Only while or Ask approval before that has no child yet.".to_owned());
    };
    let index = model
        .children_of(parent)
        .iter()
        .filter(|child| centre(positions[&child.id]).0 < x)
        .count();
    Ok((Some(parent.id.clone()), index))
}
