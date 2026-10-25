//! The page's layout rules as pure functions (etalii.adp spec 036, T027, research R8), ported from the standalone host's
//! `hierarchySurface/library/layout.ts` and `iconsLayout.ts` at a7fdf03, with the sizes of `hierarchySurface.css` the
//! spec names as constants: how many levels fit, where a node stands in the hierarchy, and whether a box's controls
//! stand in a row beside its first line or in a column at its right.

use std::collections::{HashMap, HashSet};

use adp_desl::hierarchy::model::{SurfaceNode, roles};

/// The width of a level's own box, in points: each level that fits takes this much of the page.
pub const LEVEL_WIDTH: f32 = 240.0;
/// The least width the entries of the deepest level shown keep.
pub const ENTRY_MIN_WIDTH: f32 = 280.0;
/// The most columns of levels the page counts; more than any document holds.
const MOST_COLUMNS: usize = 1000;

/// The size of a box's controls: every icon of its list, and its delete.
pub const ICON_SIZE: f32 = 20.0;
/// The space between a box's border and what it holds, on every side, and between its text and its controls.
pub const BOX_SPACING: f32 = 6.0;
/// The space between the page's edge and its boxes, and the trail's and the bars' side padding.
pub const PAGE_MARGIN: f32 = 12.0;
/// The space between the parts of a box's first line: the grip, the collapse control, the number and the title.
pub const HEADER_GAP: f32 = 4.0;
/// The width of the grip a box is dragged by.
pub const GRIP_WIDTH: f32 = 14.0;
/// The height of a line of a title or a description.
pub const LINE_HEIGHT: f32 = 20.0;

/// How many levels fit a page `width` wide, at least one, the page's columns from the left having the widths
/// `column_width` gives them, counted from 0. An unmeasured page, width 0, shows them all.
pub fn levels_that_fit(width: f32, column_width: impl Fn(usize) -> f32) -> usize {
    if width <= 0.0 {
        return usize::MAX;
    }
    let mut used = ENTRY_MIN_WIDTH;
    let mut fit = 0;
    while fit < MOST_COLUMNS && used + column_width(fit) <= width {
        used += column_width(fit);
        fit += 1;
    }
    fit.max(1)
}

/// How a box's controls stand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconsLayout {
    /// In a row beside the title, when the whole first line fits.
    Row,
    /// In a column at the box's right otherwise.
    Column,
}

/// The width of a box's first line on one line: the grip, the collapse control, the number and the title, with the gaps
/// between them, the number and the title as measured.
pub fn header_width(grip: bool, number: f32, title: f32) -> f32 {
    (if grip { GRIP_WIDTH + HEADER_GAP } else { 0.0 }) + ICON_SIZE + HEADER_GAP + number + HEADER_GAP + title
}

/// Whether the controls stand in a row beside the first line or in a column at the right of the box: the row only when
/// the first line and every control fit side by side in the box, with the space between them; a box not yet measured
/// takes the column, which never stands over its text.
pub fn icons_layout(box_width: Option<f32>, header: Option<f32>, icons: usize) -> IconsLayout {
    match (box_width, header) {
        (Some(width), Some(header)) if header + BOX_SPACING + icons as f32 * ICON_SIZE <= width - 2.0 * BOX_SPACING => {
            IconsLayout::Row
        }
        _ => IconsLayout::Column,
    }
}

/// The nodes of one model by their place in the hierarchy.
pub struct HierarchyIndex<'m> {
    pub by_id: HashMap<&'m str, &'m SurfaceNode>,
    /// The children of each node, in the file's order; the top under "".
    pub children: HashMap<&'m str, Vec<&'m SurfaceNode>>,
}

impl<'m> HierarchyIndex<'m> {
    pub fn of(nodes: &'m [SurfaceNode]) -> HierarchyIndex<'m> {
        let mut by_id = HashMap::new();
        for node in nodes {
            by_id.entry(node.id.as_str()).or_insert(node);
        }
        let mut children: HashMap<&str, Vec<&SurfaceNode>> = HashMap::new();
        for node in nodes {
            // A parent that is not in the model leaves its node at the top, where it is still seen.
            let parent = if !node.parent_id.is_empty() && by_id.contains_key(node.parent_id.as_str()) {
                node.parent_id.as_str()
            } else {
                ""
            };
            children.entry(parent).or_default().push(node);
        }
        HierarchyIndex { by_id, children }
    }

    /// The children of `id`, in the file's order.
    pub fn children_of(&self, id: &str) -> &[&'m SurfaceNode] {
        self.children.get(id).map(Vec::as_slice).unwrap_or_default()
    }

    /// The levels above `id`, from the top down; empty for a node at the top.
    pub fn ancestors_of(&self, id: &str) -> Vec<&'m SurfaceNode> {
        let mut chain = Vec::new();
        let mut seen: HashSet<&str> = HashSet::from([id]);
        let mut node = self.by_id.get(id).copied();
        while let Some(current) = node.filter(|n| !n.parent_id.is_empty()) {
            let Some(parent) = self.by_id.get(current.parent_id.as_str()).copied() else {
                break;
            };
            if !seen.insert(parent.id.as_str()) {
                break;
            }
            chain.insert(0, parent);
            node = Some(parent);
        }
        chain
    }

    /// How many levels stand below `id`, counting the levels nested in it to their deepest.
    pub fn levels_below(&self, id: &str) -> usize {
        self.levels_below_seen(id, &mut HashSet::new())
    }

    fn levels_below_seen<'a>(&'a self, id: &'a str, seen: &mut HashSet<&'a str>) -> usize {
        if !seen.insert(id) {
            return 0;
        }
        self.children_of(id)
            .iter()
            .filter(|child| child.role == roles::LEVEL)
            .map(|child| 1 + self.levels_below_seen(&child.id, seen))
            .max()
            .unwrap_or(0)
    }
}

/// What the page changes to show a node (the standalone host's `references/reveal.ts`): the focus it takes, and the
/// collapsed levels above it it expands, from the top down.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevealPlan {
    /// The page's new left edge: unchanged when the node is already in sight of it; empty for the whole page.
    pub focus: String,
    pub expand: Vec<String>,
}

impl HierarchyIndex<'_> {
    /// How to show `target` from the page focused at `focused` with `fit` columns of levels (FR-018); none when the
    /// document has no such node. The focus is kept when the node is inside it and its level fits; otherwise it moves
    /// to the deepest level the two have in common, the whole page when they share none, and from there down the
    /// node's levels only as far as its own level needs to fit. Nothing here writes the document.
    pub fn reveal_of(
        &self,
        target: &str,
        focused: &str,
        fit: usize,
        is_collapsed: impl Fn(&SurfaceNode) -> bool,
    ) -> Option<RevealPlan> {
        let node = self.by_id.get(target).copied()?;
        let above = self.ancestors_of(target);
        // The levels from the top down to the one the node stands in: itself for a level, its parent for an entry.
        let mut levels = above.clone();
        if node.role == roles::LEVEL {
            levels.push(node);
        }
        // The level the page starts from, as its place in `levels`; -1 for the whole page.
        let mut edge = levels.iter().position(|l| l.id == focused).map_or(-1, |at| at as isize);
        if !focused.is_empty() && edge < 0 {
            let shared: HashSet<&str> = self.ancestors_of(focused).iter().map(|n| n.id.as_str()).collect();
            edge = levels
                .iter()
                .rposition(|l| shared.contains(l.id.as_str()))
                .map_or(-1, |at| at as isize);
        }
        // The page's first column holds the edge's level, or the top levels for the whole page; the node's level
        // stands as many columns to the right as it is deeper.
        while levels.len() as isize - 1 - edge.max(0) >= fit as isize {
            edge += 1;
        }
        Some(RevealPlan {
            focus: if edge < 0 {
                String::new()
            } else {
                levels[edge as usize].id.clone()
            },
            expand: above.iter().filter(|l| is_collapsed(l)).map(|l| l.id.clone()).collect(),
        })
    }
}
