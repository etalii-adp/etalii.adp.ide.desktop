//! `layout.tidyTree` (ATD §13.3; its contract and fixtures are in `specifications/atd/capabilities/`): a tidy tree
//! growing downward, children left to right in their order, each subtree placed as close to its left sibling as their
//! outlines allow (the Reingold-Tilford rule), a parent centred over its first and last child, the leftmost node at
//! x = 0. Moved (not rewritten) from agent behavior modelling's `layout.rs`, which calls it here (etalii.adp spec 076,
//! T008), itself ported from the standalone host's `AbmLayout.cs` at `8b6ccfc`.

use std::collections::{BTreeMap, HashMap};

use serde_json::Value as Json;

use super::{LayoutCapability, number_argument};

/// The gaps of the tree.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TidyGaps {
    /// The least space across between two neighbouring subtrees (`nodeGap`).
    pub node: f64,
    /// The space between a parent's bottom and its children's top (`layerGap`).
    pub layer: f64,
    /// The least space across between two whole trees (`componentGap`).
    pub component: f64,
}

impl Default for TidyGaps {
    fn default() -> Self {
        TidyGaps {
            node: 28.0,
            layer: 56.0,
            component: 56.0,
        }
    }
}

/// A tree to place: its nodes' children in order, by id, and its roots in order. Every node is `width` by `height`.
#[derive(Debug, Clone, Default)]
pub struct TidyTree {
    pub roots: Vec<String>,
    pub children: HashMap<String, Vec<String>>,
    pub width: f64,
    pub height: f64,
}

/// An outline: the left and right edge of a subtree at each depth, relative to its root's left edge.
type Outline = Vec<(f64, f64)>;

/// The top-left of every node of `tree`, by id, as the tree alone places it.
pub fn compute(tree: &TidyTree, gaps: &TidyGaps) -> HashMap<String, (f64, f64)> {
    // Each node's left edge relative to its parent's, and each subtree's outline per depth, children before parents.
    let mut offsets: HashMap<String, f64> = HashMap::new();
    let mut outlines: HashMap<String, Outline> = HashMap::new();
    for root in &tree.roots {
        outline_of(tree, root, gaps, &mut offsets, &mut outlines);
    }

    // The roots side by side, packed the same way with a wider gap between whole trees.
    let root_outlines: Vec<Outline> = tree.roots.iter().map(|r| outlines[r].clone()).collect();
    let (root_lefts, _) = pack(&root_outlines, gaps.component);
    let mut positions = HashMap::new();
    for (index, root) in tree.roots.iter().enumerate() {
        place(tree, root, root_lefts[index], 0, gaps, &offsets, &mut positions);
    }

    // From the leftmost node at zero, so the drawing starts where it always has.
    let shift = positions
        .values()
        .map(|p: &(f64, f64)| p.0)
        .fold(f64::INFINITY, f64::min);
    let shift = if positions.is_empty() { 0.0 } else { shift };
    positions.into_iter().map(|(id, (x, y))| (id, (x - shift, y))).collect()
}

fn outline_of(
    tree: &TidyTree,
    id: &str,
    gaps: &TidyGaps,
    offsets: &mut HashMap<String, f64>,
    outlines: &mut HashMap<String, Outline>,
) {
    let child_ids = tree.children.get(id).cloned().unwrap_or_default();
    for child in &child_ids {
        outline_of(tree, child, gaps, offsets, outlines);
    }
    let children: Vec<Outline> = child_ids.iter().map(|c| outlines[c].clone()).collect();
    let (placed, outline) = pack(&children, gaps.node);
    if placed.is_empty() {
        outlines.insert(id.to_owned(), vec![(0.0, tree.width)]);
        return;
    }
    // Centred over the first and last child, whose roots sit at their packed offsets.
    let left = (placed[0] + placed[placed.len() - 1]) / 2.0;
    for (index, offset) in placed.iter().enumerate() {
        offsets.insert(child_ids[index].clone(), offset - left);
    }
    let mut own = vec![(0.0, tree.width)];
    own.extend(outline.iter().map(|(l, r)| (l - left, r - left)));
    outlines.insert(id.to_owned(), own);
}

/// Places outlines left to right, each as far left as keeps `gap` clear of everything before it at every depth they
/// share; each one's offset and the combined outline.
pub fn pack(outlines: &[Outline], gap: f64) -> (Vec<f64>, Outline) {
    let mut offsets = Vec::new();
    let mut combined: Outline = Vec::new();
    for outline in outlines {
        let mut offset = 0.0;
        if !combined.is_empty() {
            offset = f64::NEG_INFINITY;
            for depth in 0..combined.len().min(outline.len()) {
                offset = f64::max(offset, combined[depth].1 + gap - outline[depth].0);
            }
        }
        offsets.push(offset);
        for (depth, (left, right)) in outline.iter().enumerate() {
            let (left, right) = (left + offset, right + offset);
            if depth < combined.len() {
                combined[depth] = (combined[depth].0.min(left), combined[depth].1.max(right));
            } else {
                combined.push((left, right));
            }
        }
    }
    (offsets, combined)
}

fn place(
    tree: &TidyTree,
    id: &str,
    left: f64,
    depth: usize,
    gaps: &TidyGaps,
    offsets: &HashMap<String, f64>,
    positions: &mut HashMap<String, (f64, f64)>,
) {
    positions.insert(id.to_owned(), (left, depth as f64 * (tree.height + gaps.layer)));
    for child in tree.children.get(id).into_iter().flatten() {
        place(tree, child, left + offsets[child], depth + 1, gaps, offsets, positions);
    }
}

/// `layout.tidyTree` over `$defs/LayoutInput`: the tree is the nodes' `parent` and `order`, its roots `roots` in their
/// order, else the nodes without a parent in input order; every node takes the first node's size.
pub struct TidyTreeLayout;

impl LayoutCapability for TidyTreeLayout {
    fn place(&self, input: &Json, args: &Json) -> Result<Vec<(String, f64, f64)>, String> {
        let defaults = TidyGaps::default();
        let gaps = TidyGaps {
            node: number_argument(args, "nodeGap", defaults.node),
            layer: number_argument(args, "layerGap", defaults.layer),
            component: number_argument(args, "componentGap", defaults.component),
        };
        let nodes = input["nodes"].as_array().ok_or("The input has no `nodes`.")?;
        let mut children: BTreeMap<&str, Vec<(i64, usize)>> = BTreeMap::new();
        for (index, node) in nodes.iter().enumerate() {
            if let Some(parent) = node["parent"].as_str() {
                let order = node["order"].as_i64().unwrap_or(index as i64);
                children.entry(parent).or_default().push((order, index));
            }
        }
        let id = |index: usize| nodes[index]["id"].as_str().unwrap_or_default().to_owned();
        let roots: Vec<String> = match input["roots"].as_array() {
            Some(roots) => roots.iter().filter_map(|r| r.as_str().map(str::to_owned)).collect(),
            None => (0..nodes.len())
                .filter(|&i| nodes[i]["parent"].is_null())
                .map(id)
                .collect(),
        };
        let first = nodes.first();
        let tree = TidyTree {
            roots,
            children: children
                .into_iter()
                .map(|(parent, mut kids)| {
                    kids.sort_by_key(|(order, index)| (*order, *index));
                    (
                        parent.to_owned(),
                        kids.into_iter().map(|(_, index)| id(index)).collect(),
                    )
                })
                .collect(),
            width: first.and_then(|n| n["width"].as_f64()).unwrap_or(0.0),
            height: first.and_then(|n| n["height"].as_f64()).unwrap_or(0.0),
        };
        let placed = compute(&tree, &gaps);
        Ok(nodes
            .iter()
            .filter_map(|node| {
                let id = node["id"].as_str()?;
                let (x, y) = placed.get(id)?;
                Some((id.to_owned(), *x, *y))
            })
            .collect())
    }
}
