//! The data graph's palette (etalii.adp spec 030, task T083), ported from the standalone host's
//! `RdfToolboxProvider.cs` at `8b6ccfc`: each entry carries only data and names the add action its drop commits, so a
//! drop and the menu run the same action, through a placement id.

use adp_diagram::ToolboxItem;

use crate::actions;

/// A palette entry.
pub fn item(id: &str, label: &str, icon: &str, description: &str, drop_action_id: &str) -> ToolboxItem {
    ToolboxItem {
        id: id.to_owned(),
        label: label.to_owned(),
        icon: icon.to_owned(),
        description: description.to_owned(),
        drop_action_id: drop_action_id.to_owned(),
    }
}

/// The data graph's palette.
pub fn items() -> Vec<ToolboxItem> {
    vec![
        item(
            "rdf.toolbox.resource",
            "Resource",
            "mdi-card-plus-outline",
            "An IRI-named resource. Drop it where it should sit; you will be asked for its name.",
            actions::ADD_RESOURCE,
        ),
        item(
            "rdf.toolbox.prefix",
            "Prefix",
            "mdi-at",
            "A namespace declaration, so terms can be written short.",
            actions::ADD_PREFIX,
        ),
    ]
}
