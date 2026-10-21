//! The shacl reading's part of the family's context menu. STUB: filled in by the shacl port.

use adp_context::ActionGroup;

use crate::commands::RdfEdit;
use crate::family::{Run, Target};
use crate::store::RdfDocument;

/// The groups this reading leads the menu of `target` with (`Discover`).
pub fn discover(document: &RdfDocument, target: &Target) -> Vec<ActionGroup> {
    let _ = (document, target);
    Vec::new()
}

/// Runs `action_id` when it is this reading's, none otherwise (`ExecuteAsync`).
pub fn execute(document: &RdfDocument, target: &Target, action_id: &str) -> Option<Run> {
    let _ = (document, target, action_id);
    None
}

/// Judges an answer to this reading's prompt, none for another reading's action (`Validate`).
pub fn validate(document: &RdfDocument, action_id: &str, value: &str) -> Option<Result<(), String>> {
    let _ = (document, action_id, value);
    None
}

/// The edit an answer to this reading's prompt makes, none for another reading's action (`CommandFor`).
pub fn command_for(document: &RdfDocument, target: &Target, action_id: &str, value: &str) -> Option<RdfEdit> {
    let _ = (document, target, action_id, value);
    None
}
