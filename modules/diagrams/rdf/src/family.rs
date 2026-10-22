//! What the family's context providers ask of each reading's actions and property rows (etalii.adp spec 030, task
//! T083), ported from the shapes of `RdfContextActionProvider.cs` and `RdfContextPropertyProvider.cs` at `8b6ccfc`.
//!
//! The standalone host's providers were family-wide: one action provider and one property provider consulted, in
//! order, the shapes reading (`ShaclActions`, `ShaclProperties`), the scheme reading (`SkosActions`,
//! `SkosProperties`) and then the family's own answers. Each reading's part answers with the types here: what its
//! `ExecuteAsync` returned, or none where the action is not its own.

use std::path::Path;

use adp_context::Prompt;

use crate::commands::RdfEdit;

/// What a context target is in this family (`ContextTarget`): the selected element, the body it is in, and the tool
/// type the diagram was opened as (the target's `Origin`).
#[derive(Debug, Clone, Copy)]
pub struct Target<'a> {
    pub element_id: &'a str,
    pub body_path: &'a Path,
    pub origin: &'a str,
}

/// What executing an action gives (`ContextExecutionResult`).
#[derive(Debug, Clone, PartialEq)]
pub enum Run {
    /// One edit, dispatched through the project's history at once (`DispatchAsync`).
    Edit(RdfEdit),
    /// A question first: an input (`ContextExecutionRequiresInput`) or a confirmation
    /// (`ContextExecutionRequiresConfirmation`), whose answer is committed through the reading's `command_for`.
    Ask(Prompt),
    /// Refused with its sentence (`ContextExecutionFailed`).
    Refused(String),
    /// Nothing to do (`ContextExecutionCompleted`).
    Done,
}

/// An input prompt (`ContextInputRequest(title, icon, label, initial, confirm)`).
pub fn input(title: &str, icon: &str, label: &str, initial_value: &str, confirm_label: &str) -> Prompt {
    Prompt::Input {
        title: title.to_owned(),
        icon: icon.to_owned(),
        label: label.to_owned(),
        initial_value: initial_value.to_owned(),
        confirm_label: confirm_label.to_owned(),
    }
}

/// A confirmation (`ContextConfirmationRequest(title, icon, message, confirm, danger)`).
pub fn confirm(title: &str, icon: &str, message: &str, confirm_label: &str, danger: bool) -> Prompt {
    Prompt::Confirm {
        title: title.to_owned(),
        icon: icon.to_owned(),
        message: message.to_owned(),
        confirm_label: confirm_label.to_owned(),
        danger,
    }
}
