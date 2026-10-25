//! Changing the page (etalii.adp spec 036, T038): what the standalone host's `editing/AddControl.tsx`,
//! `EntryTextField.tsx`, `entryEditing.ts`, `useDragEntry.ts` and `ConfirmDelete.tsx` at a7fdf03 decide, apart from the
//! drawing, which is [`super::view`]'s.
//!
//! - A node is added by the controls at the end of every level's content and of the page, or by a key on a selected
//!   node, with no dialog; the node the add brought is found by [`added_node`], and its title opens for typing.
//! - A title or a description is typed in place, in an [`OpenField`]; what it holds is the one thing on the page that
//!   can be unsaved, written only when it is committed.
//! - A node is dragged by its grip, with everything in it: [`drop_of`] says where it would go, and [`slot_index`] where
//!   the page shows it while it is dragged.
//! - A delete asks first when anything concerns the node, [`delete_count`] of them.

use adp_desl::hierarchy::model::{SurfaceModel, SurfaceNode, roles};

use super::layout::HierarchyIndex;

/// Which text of a node a field is open on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attribute {
    Title,
    Description,
}

impl Attribute {
    /// The node's text it is open on.
    pub fn of(self, node: &SurfaceNode) -> &str {
        match self {
            Attribute::Title => &node.title,
            Attribute::Description => &node.description,
        }
    }

    /// The param the gesture that commits it carries.
    pub fn param(self) -> &'static str {
        match self {
            Attribute::Title => "title",
            Attribute::Description => "description",
        }
    }
}

/// The field open on the page: the node, which of its texts, and what has been typed so far.
#[derive(Debug, Clone, PartialEq)]
pub struct OpenField {
    pub node_id: String,
    pub attribute: Attribute,
    pub text: String,
    /// What the field is, for a screen reader: "Title of REQ-0004". Kept with the field, which can outlive its box.
    pub label: String,
    /// The lookup a `#` typed in a description opened, while it is open.
    pub lookup: Option<super::references::Lookup>,
}

impl OpenField {
    /// The field on `attribute` of `node`, holding its text.
    pub fn on(node: &SurfaceNode, attribute: Attribute) -> OpenField {
        let name = match attribute {
            Attribute::Title => "Title",
            Attribute::Description => "Description",
        };
        OpenField {
            node_id: node.id.clone(),
            attribute,
            text: attribute.of(node).to_owned(),
            label: format!("{name} of {}", node.number),
            lookup: None,
        }
    }
}

/// The node an add brought, between the page before it and the page after: the one node of `after` that is not in
/// `before`. A node without a number of its own is named by its line, so an edit that numbers a placeholder or moves
/// the lines below it gives that node a new id too; such a node is matched to the one it was by its role and title,
/// and what is left is the node the add made. None when the add made none.
pub fn added_node<'a>(before: &[SurfaceNode], after: &'a [SurfaceNode]) -> Option<&'a SurfaceNode> {
    let present: std::collections::HashSet<&str> = after.iter().map(|n| n.id.as_str()).collect();
    let known: std::collections::HashSet<&str> = before.iter().map(|n| n.id.as_str()).collect();
    let mut candidates: Vec<&SurfaceNode> = after.iter().filter(|n| !known.contains(n.id.as_str())).collect();
    for was in before.iter().filter(|n| !present.contains(n.id.as_str())) {
        if let Some(at) = candidates
            .iter()
            .position(|n| n.role == was.role && n.title == was.title)
        {
            candidates.remove(at);
        }
    }
    candidates.first().copied()
}

/// How much concerns a node a delete would take (the count of a delete's `confirm`): what is nested directly in it, the
/// notes about it or naming it, and the other nodes whose texts name it. A note or a node counts once, however often it
/// names the node; the names are the model's references, so the page needs no syntax of a designer's own.
pub fn delete_count(model: &SurfaceModel, index: &HierarchyIndex, node_id: &str) -> usize {
    let names =
        |references: &[adp_desl::hierarchy::model::Reference]| references.iter().any(|r| r.target_id == node_id);
    let children = index.children_of(node_id).len();
    let notes = model
        .notes
        .iter()
        .filter(|note| note.about_id == node_id || names(&note.references))
        .count();
    let texts = model
        .nodes
        .iter()
        .filter(|node| node.id != node_id && names(&node.references))
        .count();
    children + notes + texts
}

/// Where a dropped node goes: into a level (empty for the top), after a sibling of its role there (empty for none).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryDrop {
    pub into: String,
    pub after: String,
}

/// How the box under the pointer takes a drop: the node goes into it, after it or before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropKind {
    Into,
    After,
    Before,
}

/// What the page shows while a node is dragged: the box under the pointer it would go into, after or before, and the
/// node and the place it would be dropped at, where the page draws it as it would stand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DropHint {
    pub node_id: String,
    pub kind: DropKind,
    pub dragged: SurfaceNode,
    pub drop: EntryDrop,
}

impl DropHint {
    /// The hint for dropping `dragged` on `over` as `drop` says.
    pub fn of(dragged: &SurfaceNode, over: &SurfaceNode, drop: EntryDrop) -> DropHint {
        let kind = if drop.into == over.id {
            DropKind::Into
        } else if drop.after == over.id {
            DropKind::After
        } else {
            DropKind::Before
        };
        DropHint {
            node_id: over.id.clone(),
            kind,
            dragged: dragged.clone(),
            drop,
        }
    }
}

/// Where the place a drop shows stands among `shown`, the children of its level as the page draws them, entries before
/// levels; none where it stands nowhere among them. A move after a sibling goes right after it; without one, as the
/// move of the definition says, to the front of its kind within its own level, and after the last of its kind into
/// another.
pub fn slot_index(shown: &[&SurfaceNode], hint: &DropHint) -> Option<usize> {
    let (dragged, drop) = (&hint.dragged, &hint.drop);
    if !drop.after.is_empty() {
        return shown.iter().position(|n| n.id == drop.after).map(|at| at + 1);
    }
    let entries = shown.iter().filter(|n| n.role != roles::LEVEL).count();
    let level = dragged.role == roles::LEVEL;
    Some(if dragged.parent_id == drop.into {
        if level { entries } else { 0 }
    } else if level {
        shown.len()
    } else {
        entries
    })
}

/// Where `dragged` goes when dropped on `over`'s box, in its lower half or its upper one; none where it cannot go - on
/// itself, into what it holds, or an entry onto the top. A level dropped on a level goes after it from the lower half
/// and into it from the upper one; on an entry, into that entry's level. An entry dropped on an entry goes after it from
/// the lower half and before it from the upper one; on a level, into it, where a new entry would go.
pub fn drop_of(
    index: &HierarchyIndex,
    dragged: &SurfaceNode,
    over: &SurfaceNode,
    lower_half: bool,
) -> Option<EntryDrop> {
    if over.id == dragged.id || index.ancestors_of(&over.id).iter().any(|n| n.id == dragged.id) {
        return None;
    }
    let drop = |into: &str, after: &str| {
        Some(EntryDrop {
            into: into.to_owned(),
            after: after.to_owned(),
        })
    };
    if dragged.role == roles::LEVEL {
        if over.role != roles::LEVEL {
            return drop(&over.parent_id, "");
        }
        return if lower_half {
            drop(&over.parent_id, &over.id)
        } else {
            drop(&over.id, "")
        };
    }
    if over.role == roles::LEVEL {
        return drop(&over.id, "");
    }
    if over.parent_id.is_empty() {
        return None;
    }
    if lower_half {
        return drop(&over.parent_id, &over.id);
    }
    let siblings: Vec<&SurfaceNode> = index
        .children_of(&over.parent_id)
        .iter()
        .filter(|n| n.role == roles::ENTRY && n.id != dragged.id)
        .copied()
        .collect();
    let before = siblings
        .iter()
        .position(|n| n.id == over.id)
        .and_then(|at| at.checked_sub(1))
        .map(|at| siblings[at].id.as_str())
        .unwrap_or_default();
    drop(&over.parent_id, before)
}

/// A delete waiting on its question's answer: the node, and what is asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deleting {
    pub node_id: String,
    pub question: String,
}
