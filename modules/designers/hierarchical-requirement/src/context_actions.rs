//! The keys that add an entry directly below the selected one, with no dialog (the standalone host's
//! `HierarchicalRequirementContextActionProvider` at a7fdf03; spec 029 FR-012, US4 scenario 3; etalii.adp spec 036,
//! T057): Ctrl+Enter adds an item below an item and a requirement below a requirement, Ctrl+Shift+Enter a requirement
//! to an item. They are offered the way every other action is, so the ribbon and the properties pane show them too,
//! each with its key; the labels and the unavailable message are the specification's.
//!
//! - **The edit is the page's own.** An action names the operation a gesture on the page sends for the entry, so it is
//!   one step of the document's history, and the page opens the new entry's title for typing.
//! - **Only a requirement document answers.** A file without this designer's envelope, or one that cannot be changed,
//!   offers nothing ([`discover`]).

use std::path::Path;

use adp_desl::hierarchy::model::SurfaceAction;

use crate::operations::structure;
use crate::selection;

/// The ids of the two actions, the standalone host's.
pub const ADD_BELOW: &str = "hierarchical-requirement.add-below";
pub const ADD_REQUIREMENT: &str = "hierarchical-requirement.add-requirement";

/// The key both actions are pressed with, Ctrl or Cmd held.
const KEY: &str = "Enter";

/// What an entry offers: an item an item below it and a requirement in it, a requirement a requirement below it,
/// which a requirement in no item cannot have.
pub fn offered(is_item: bool, in_an_item: bool) -> Vec<SurfaceAction> {
    let action = |id: &str, label: &str, icon: &str, shift: bool, operation: &str| SurfaceAction {
        id: id.into(),
        label: label.into(),
        icon: icon.into(),
        key: KEY.into(),
        shift,
        unavailable: String::new(),
        operation: operation.into(),
    };
    if is_item {
        return vec![
            action(
                ADD_BELOW,
                "New item below",
                "mdi-plus-box-outline",
                false,
                structure::ADD_ITEM_BELOW,
            ),
            action(
                ADD_REQUIREMENT,
                "New requirement",
                "mdi-plus",
                true,
                structure::ADD_REQUIREMENT,
            ),
        ];
    }
    let mut below = action(
        ADD_BELOW,
        "New requirement below",
        "mdi-plus",
        false,
        structure::ADD_REQUIREMENT_BELOW,
    );
    if !in_an_item {
        below.unavailable = structure::NO_ITEM.into();
    }
    vec![below]
}

/// What the entry `element_id` of the file at `body_path` offers, as the file now is: nothing for another tool's file,
/// for an entry it does not hold, and for a document that cannot be changed.
pub fn discover(body_path: &Path, element_id: &str) -> Vec<SurfaceAction> {
    match selection::find(body_path, element_id) {
        Some(found) if found.document.read_only_reason.is_empty() => {
            offered(found.entry.is_item(), found.entry.parent_id.is_some())
        }
        _ => Vec::new(),
    }
}

/// The action pressed with `key`, Shift held or not, among `actions`: none when no action takes that key, and none
/// for one that cannot run now.
pub fn by_shortcut<'a>(actions: &'a [SurfaceAction], key: &str, shift: bool) -> Option<&'a SurfaceAction> {
    actions
        .iter()
        .find(|a| a.key == key && a.shift == shift && a.unavailable.is_empty())
}
