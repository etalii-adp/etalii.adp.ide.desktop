//! What the Hierarchical Requirement Designer declares for the hierarchy surface (spec 029 R9), the standalone host's
//! `client/hierarchicalRequirementDefinition.ts` at a7fdf03: its levels are items and its entries requirements, the
//! labels its `.des` gives under `surface.labels`, the icon of each assessment kind from its `AssessmentKind`, the icon
//! R10 settles for several open assessments, the operations and words of the assessments' list, the operations its
//! gestures send with their labels, the tag operations of its tag field, the category and state operations of its
//! pickers, and the filter operations of its filter bar. `tests/surface_definition.rs` holds each to the vendored
//! definition.

use adp_desl::hierarchy::definition::{
    AddDefinition, Deletion, FilterOperations, FilterParams, Filters, LabelDefinition, LabelEdit, LabelEditOperations,
    LabelEditWords, LabelLook, LabelPick, LabelPickOperations, ListDeletion, ListParams, MoveParams, NoteOperations,
    NoteParams, NoteWords, NotesDefinition, Operations, PickParams, PickWords, SortKey, SurfaceDefinition, Widths,
};

use crate::model::{MAXIMUM_WIDTH, MINIMUM_WIDTH};

fn label(label: &str, prefix: &str, many: bool, look: Option<LabelLook>, on: &[&str]) -> LabelDefinition {
    LabelDefinition {
        label: label.into(),
        prefix: prefix.into(),
        many,
        look,
        on: on.iter().map(|t| (*t).to_owned()).collect(),
        edit: None,
        pick: None,
    }
}

fn list_params() -> ListParams {
    ListParams {
        name: "name".into(),
        colour: "colour".into(),
        after: "after".into(),
    }
}

/// The six tag operations, their params and labels, and the `confirm` of deleteTag with its CEL written out.
fn tag_edit() -> LabelEdit {
    LabelEdit {
        operations: LabelEditOperations {
            add: "addTag".into(),
            remove: "removeTag".into(),
            rename: "renameTag".into(),
            recolour: "recolourTag".into(),
            reorder: "moveTag".into(),
            delete: "deleteTag".into(),
        },
        params: list_params(),
        words: LabelEditWords {
            list: "Tags".into(),
            add: "Add tag".into(),
            remove: "Remove tag".into(),
        },
        deletion: ListDeletion {
            threshold: 1,
            message: "{count} entries carry #{name}. Delete it and remove it from them?".into(),
        },
    }
}

/// The seven operations of a label of one value, named `<verb><Name>`, their params and labels, and the `confirm` of
/// its delete with its CEL written out.
fn pick(label: &str, name: &str, list: &str) -> LabelPick {
    let operation = |verb: &str| format!("{verb}{name}");
    LabelPick {
        operations: LabelPickOperations {
            set: operation("set"),
            clear: operation("clear"),
            add: operation("add"),
            rename: operation("rename"),
            recolour: operation("recolour"),
            reorder: operation("move"),
            delete: operation("delete"),
        },
        params: PickParams {
            value: label.into(),
            list: list_params(),
        },
        words: PickWords {
            list: list.into(),
            value: name.into(),
            clear: format!("No {label}"),
        },
        deletion: ListDeletion {
            threshold: 1,
            message: "{count} requirements are {name}. Delete it and clear them?".into(),
        },
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
            LabelDefinition {
                edit: Some(tag_edit()),
                ..label("tags", "#", true, None, &["Item", "Requirement"])
            },
            LabelDefinition {
                pick: Some(pick("category", "Category", "Categories")),
                ..label("category", "category:", false, None, &["Requirement"])
            },
            LabelDefinition {
                pick: Some(pick("state", "State", "States")),
                ..label("state", "@", false, Some(LabelLook::Status), &["Requirement"])
            },
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
        notes: NotesDefinition {
            operations: NoteOperations {
                add: "addAssessment".into(),
                change: "changeAssessment".into(),
                resolve: "resolveAssessment".into(),
                remove: "removeAssessment".into(),
            },
            params: NoteParams {
                kind: "kind".into(),
                text: "text".into(),
                detail: "detail".into(),
            },
            words: NoteWords {
                list: "Assessments".into(),
                add: "New assessment".into(),
                change: "Change".into(),
                resolve: "Resolve".into(),
                reopen: "Reopen".into(),
                remove: "Remove".into(),
            },
        },
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
        // The `confirm` of deleteEntry, its CEL written out: the count is the page's, from the model's references.
        deletion: Deletion {
            threshold: 1,
            message:
                "Delete {number} with everything in it? {count} entries, references or assessments concern it; the \
                      references and assessments are kept, and reported."
                    .into(),
        },
        // The filter operations and their params; a sort by number or by title, besides the labels of one value (DESL
        // section 7.7.3).
        filters: Some(Filters {
            operations: FilterOperations {
                store: "storeFilter".into(),
                rename: "renameFilter".into(),
                remove: "removeFilter".into(),
            },
            params: FilterParams {
                name: "name".into(),
                query: "query".into(),
            },
            sort_keys: vec![
                SortKey {
                    key: "number".into(),
                    label: "Number".into(),
                },
                SortKey {
                    key: "title".into(),
                    label: "Title".into(),
                },
            ],
            entries: "requirements".into(),
        }),
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
