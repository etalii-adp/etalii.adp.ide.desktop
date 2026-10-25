//! What the Hierarchical Requirement Designer declares for the hierarchy surface (spec 029 R9), the standalone host's
//! `client/hierarchicalRequirementDefinition.ts` at a7fdf03: its levels are items and its entries requirements, the
//! labels its `.des` gives under `surface.labels`, the icon of each assessment kind from its `AssessmentKind`, the icon
//! R10 settles for several open assessments, and the operations its gestures send with their labels.
//! `tests/surface_definition.rs` holds each to the vendored definition.

use adp_desl::hierarchy::definition::{
    AddDefinition, LabelDefinition, LabelLook, MoveParams, Operations, SurfaceDefinition, Widths,
};

use crate::model::{MAXIMUM_WIDTH, MINIMUM_WIDTH};

fn label(label: &str, prefix: &str, many: bool, look: Option<LabelLook>, on: &[&str]) -> LabelDefinition {
    LabelDefinition {
        label: label.into(),
        prefix: prefix.into(),
        many,
        look,
        on: on.iter().map(|t| (*t).to_owned()).collect(),
    }
}

fn add(operation: &str, label: &str, icon: &str) -> AddDefinition {
    AddDefinition {
        operation: operation.into(),
        label: label.into(),
        icon: icon.into(),
    }
}

/// The designer's surface definition.
pub fn definition() -> SurfaceDefinition {
    SurfaceDefinition {
        level_type: "Item".into(),
        entry_type: "Requirement".into(),
        labels: vec![
            label("tags", "#", true, None, &["Item", "Requirement"]),
            label("category", "category:", false, None, &["Requirement"]),
            label("state", "@", false, Some(LabelLook::Status), &["Requirement"]),
        ],
        note_icons: [
            ("Question", "mdi-help-circle-outline"),
            ("Remark", "mdi-comment-text-outline"),
            ("Warning", "mdi-alert-outline"),
            ("Idea", "mdi-lightbulb-outline"),
        ]
        .iter()
        .map(|(k, i)| ((*k).to_owned(), (*i).to_owned()))
        .collect(),
        several_notes_icon: "mdi-comment-multiple-outline".into(),
        operations: Operations {
            toggle_collapsed: "toggleCollapsed".into(),
            set_title: "setTitle".into(),
            set_description: "setDescription".into(),
            add_top_level: add("addTopItem", "New item", "mdi-folder-plus-outline"),
            add_entry: add("addRequirement", "New requirement", "mdi-playlist-plus"),
            add_level: add("addItem", "New item", "mdi-folder-plus-outline"),
            move_level: "moveItem".into(),
            move_entry: "moveRequirement".into(),
            delete: "deleteEntry".into(),
        },
        move_params: MoveParams {
            into: "item".into(),
            after: "after".into(),
        },
        // The widths of the columns of items, kept in the registration's widths block; Column.width's range.
        widths: Some(Widths {
            add: "addColumnWidth".into(),
            set: "setColumnWidth".into(),
            reset: "resetColumnWidth".into(),
            level_param: "level".into(),
            width_param: "width".into(),
            min: MINIMUM_WIDTH,
            max: MAXIMUM_WIDTH,
        }),
        // Ctrl+Enter and Ctrl+Shift+Enter, the context actions of an entry.
        shortcut_keys: vec!["Enter".into()],
    }
}
