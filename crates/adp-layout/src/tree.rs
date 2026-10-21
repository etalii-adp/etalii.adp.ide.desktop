//! The `tree` layout mode (etalii.adp spec 030, FR-018), ported from `treeLayout` in the standalone host's
//! `src/client/src/canvas/library/layout/layoutAlgorithm.ts` at `8b6ccfc`.
//!
//! A directional hierarchy: children fan out from their parent along the declared direction, each subtree given the
//! cross-axis room its leaves need, the shape a mind map draws. The hierarchy hangs on the parent the model states
//! where it states one, and on the connections (source as parent) otherwise, so both structural and drawn trees lay
//! out.

use std::collections::HashMap;

use crate::algorithm::{
    LayoutAlgorithm, LayoutDefinition, LayoutElement, LayoutInput, LayoutMode, LayoutPlacement, Placements,
    TreeDirection,
};

/// The distance between a parent's edge and its children's, along the direction the tree grows.
const TREE_MAIN_GAP: f64 = 60.0;
/// The distance between two sibling subtrees, across.
const TREE_CROSS_GAP: f64 = 20.0;

/// The `tree` mode.
#[derive(Debug, Clone, Copy, Default)]
pub struct TreeLayout;

impl LayoutAlgorithm for TreeLayout {
    fn mode(&self) -> LayoutMode {
        LayoutMode::Tree
    }

    fn place(
        &self,
        input: &LayoutInput,
        definition: &LayoutDefinition,
        _previous: Option<&Placements>,
    ) -> Option<Placements> {
        Some(place(input, definition.tree_direction.unwrap_or_default()))
    }
}

struct Tree<'a> {
    children: HashMap<&'a str, Vec<&'a LayoutElement>>,
    horizontal: bool,
    main_sign: f64,
}

impl<'a> Tree<'a> {
    fn kids(&self, element: &LayoutElement) -> &[&'a LayoutElement] {
        self.children.get(element.id.as_str()).map(Vec::as_slice).unwrap_or(&[])
    }

    fn cross_size(&self, element: &LayoutElement) -> f64 {
        if self.horizontal { element.height } else { element.width }
    }

    fn main_size(&self, element: &LayoutElement) -> f64 {
        if self.horizontal { element.width } else { element.height }
    }

    /// The cross-axis room a subtree needs: its own size, or its children's sum, whichever is larger.
    fn span_of(&self, element: &LayoutElement) -> f64 {
        let kids = self.kids(element);
        if kids.is_empty() {
            return self.cross_size(element);
        }
        self.cross_size(element).max(self.children_span(element))
    }

    fn children_span(&self, element: &LayoutElement) -> f64 {
        let kids = self.kids(element);
        if kids.is_empty() {
            return 0.0;
        }
        kids.iter().fold(0.0, |sum, kid| sum + self.span_of(kid)) + TREE_CROSS_GAP * (kids.len() - 1) as f64
    }

    fn place_subtree(&self, element: &LayoutElement, main: f64, cross_start: f64, positions: &mut Placements) {
        let span = self.span_of(element);
        let centre_cross = cross_start + span / 2.0;
        let placement = if self.horizontal {
            LayoutPlacement::at(main, centre_cross)
        } else {
            LayoutPlacement::at(centre_cross, main)
        };
        positions.insert(element.id.clone(), placement);

        let mut cursor = cross_start + (span - self.children_span(element)) / 2.0;
        for kid in self.kids(element) {
            let offset = self.main_size(element) / 2.0 + TREE_MAIN_GAP + self.main_size(kid) / 2.0;
            self.place_subtree(kid, main + self.main_sign * offset, cursor, positions);
            cursor += self.span_of(kid) + TREE_CROSS_GAP;
        }
    }
}

fn place(input: &LayoutInput, direction: TreeDirection) -> Placements {
    let mut parents: HashMap<&str, &str> = HashMap::new();
    for element in &input.elements {
        if let Some(parent) = &element.parent_id {
            parents.insert(element.id.as_str(), parent.as_str());
        }
    }
    if parents.is_empty() {
        for connection in &input.connections {
            parents
                .entry(connection.target_id.as_str())
                .or_insert(connection.source_id.as_str());
        }
    }

    let known: std::collections::HashSet<&str> = input.elements.iter().map(|element| element.id.as_str()).collect();
    let mut children: HashMap<&str, Vec<&LayoutElement>> = HashMap::new();
    let mut roots = Vec::new();
    for element in &input.elements {
        match parents.get(element.id.as_str()) {
            Some(parent) if known.contains(parent) => children.entry(parent).or_default().push(element),
            _ => roots.push(element),
        }
    }

    let horizontal = matches!(direction, TreeDirection::LeftToRight | TreeDirection::RightToLeft);
    let main_sign = if matches!(direction, TreeDirection::LeftToRight | TreeDirection::TopDown) {
        1.0
    } else {
        -1.0
    };
    let tree = Tree {
        children,
        horizontal,
        main_sign,
    };

    let mut positions = Placements::new();
    let mut root_cursor = 0.0;
    for root in roots {
        tree.place_subtree(root, 0.0, root_cursor, &mut positions);
        root_cursor += tree.span_of(root) + TREE_CROSS_GAP * 2.0;
    }

    positions
}
