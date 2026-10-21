//! What a Wardley map offers in the toolbox, and the body a new map starts as (etalii.adp spec 030, task T070).
//!
//! Ported from the standalone host's `WardleyToolboxProvider.cs` and `ServiceCollection.AddWardleyMap.cs`
//! (`EmptyDocument`) at `8b6ccfc`. Market and Ecosystem are components carrying a decorator, so their entries drop
//! exactly that. There is no Link entry: a link needs two ends and a drop has one, so a link is made from a
//! component's menu. Pipeline drops onto the component it belongs to, through the menu's own "Start a pipeline…".

use adp_diagram::ToolboxItem;

use crate::actions;

fn item(id: &str, label: &str, icon: &str, description: &str, action: &str) -> ToolboxItem {
    ToolboxItem {
        id: id.to_owned(),
        label: label.to_owned(),
        icon: icon.to_owned(),
        description: description.to_owned(),
        drop_action_id: action.to_owned(),
    }
}

/// The entries, in the order the palette shows them: the two that make a value chain, the two decorated components,
/// the structures, then the two kinds of writing.
pub fn items() -> Vec<ToolboxItem> {
    vec![
        item(
            "wardley.toolbox.component",
            "Component",
            "mdi-shape-outline",
            "Something the value chain is made of. Where you put it is what you are claiming about it.",
            actions::ADD_COMPONENT,
        ),
        item(
            "wardley.toolbox.anchor",
            "Anchor",
            "mdi-anchor",
            "The user need the whole chain hangs from. A map usually has one.",
            actions::ADD_ANCHOR,
        ),
        item(
            "wardley.toolbox.market",
            "Market",
            "mdi-store-outline",
            "A component supplied by a market rather than by one supplier.",
            actions::ADD_MARKET,
        ),
        item(
            "wardley.toolbox.ecosystem",
            "Ecosystem",
            "mdi-graph-outline",
            "A component with an ecosystem of others built around it.",
            actions::ADD_ECOSYSTEM,
        ),
        item(
            "wardley.toolbox.submap",
            "Submap",
            "mdi-map-outline",
            "A reference to another map, which activating opens.",
            actions::ADD_SUBMAP,
        ),
        item(
            "wardley.toolbox.pipeline",
            "Pipeline",
            "mdi-pipe",
            "A set of choices behind one component. Drop it on the component it belongs to.",
            actions::ADD_TO_PIPELINE,
        ),
        item(
            "wardley.toolbox.note",
            "Note",
            "mdi-note-outline",
            "Free text pinned to a place on the map.",
            actions::ADD_NOTE,
        ),
        item(
            "wardley.toolbox.annotation",
            "Annotation",
            "mdi-comment-text-outline",
            "A numbered remark, shown on the map and listed in its own block.",
            actions::ADD_ANNOTATION,
        ),
    ]
}

/// The body a new map starts as: a `title` line carrying the file's base name, LF-terminated, and nothing else. A map
/// with no components opens showing its two axes and the four evolution bands.
pub fn empty_document(base_name: &str) -> String {
    format!("title {base_name}\n")
}
