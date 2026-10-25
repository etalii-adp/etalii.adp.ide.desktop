//! The gesture table (the standalone host's `HierarchicalRequirementGestures` at a7fdf03): which operation of
//! `hierarchical-requirement.des` a gesture of the hierarchy surface names, and what it changes. The surface passes
//! the operation's name through and knows none; an unknown name is refused, and an operation of the specification
//! not built yet is refused with the reason (etalii.adp spec 036 builds them pull request by pull request).

use adp_desl::hierarchy::model::{Gesture, SurfaceModel};
use adp_fbl::Setting;

use crate::vocabulary;

/// What a gesture planned against a file that changed since its model was sent is refused with.
pub const DRIFTED: &str =
    "The file changed while you were editing, so this change was not written. The page now shows the file as it is.";

/// The block of the registration that keeps what is collapsed, outside the history (hierarchical-requirement.fbl).
const COLLAPSED_BLOCK: &str = "collapsed";

const TOGGLE_COLLAPSED: &str = "toggleCollapsed";

/// The change of the registration `gesture` asks for, against `model`, the model it was made in as the session
/// holds it now; or the sentence saying why it is refused.
pub fn plan(model: &SurfaceModel, gesture: &Gesture) -> Result<Setting, String> {
    if !model.read_only_reason.is_empty() {
        return Err(model.read_only_reason.clone());
    }
    match gesture.operation.as_str() {
        TOGGLE_COLLAPSED => toggle_collapsed(model, &gesture.target_id),
        operation if vocabulary::OPERATIONS.contains(&operation) => {
            Err(format!("'{operation}' cannot be done here yet."))
        }
        operation => Err(format!(
            "'{operation}' is not an operation of the {}.",
            vocabulary::NAME
        )),
    }
}

/// Collapses an expanded entry or expands a collapsed one, in the registration's `collapsed` block. An entry without
/// a usable number is collapsed for the page only, which is the view's (DESL section 5.3.3).
fn toggle_collapsed(model: &SurfaceModel, target_id: &str) -> Result<Setting, String> {
    let Some(node) = model.nodes.iter().find(|node| node.id == target_id) else {
        return Err(DRIFTED.into());
    };
    if node.id != node.number {
        return Err(format!(
            "{} has no number of its own yet, so it is collapsed on this page only.",
            node.number
        ));
    }
    Ok(if node.collapsed {
        Setting {
            block: COLLAPSED_BLOCK.into(),
            key: node.id.clone(),
            remove_entry: true,
            ..Setting::default()
        }
    } else {
        Setting {
            block: COLLAPSED_BLOCK.into(),
            key: node.id.clone(),
            value: Some("yes".into()),
            ..Setting::default()
        }
    })
}
