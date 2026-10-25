//! The properties pane of a hierarchy tab (etalii.adp spec 036, T057): what is selected on the page, as the standalone
//! host's property grid shows an entry its `HierarchicalRequirementEntrySourceResolver` found at a7fdf03, under the
//! file it is in, and the actions it offers, each with its key and each the same as that key on the page.
//!
//! The standalone host also offers those actions from a box's context menu; the explorer's menu here names files, and
//! an entry's actions are the page's own, so this pane, the ribbon and the keys are where they are reached.

use adp_desl::hierarchy::model::{SurfaceAction, SurfaceNode};
use adp_theme::Theme;
use eframe::egui::{self, RichText};

use super::HierarchyTab;
use crate::a11y;
use crate::theme::{self, Palette};

/// "ITEM-0003 Payment", or the number alone when the title is empty: a node as the selection chain names it.
pub fn describe(node: &SurfaceNode) -> String {
    if node.title.is_empty() {
        node.number.clone()
    } else {
        format!("{} {}", node.number, node.title)
    }
}

/// "Ctrl+Shift+Enter": an action's key as the ribbon's tooltips write one.
pub fn shortcut(action: &SurfaceAction) -> String {
    if action.key.is_empty() {
        return String::new();
    }
    let shift = if action.shift { "Shift+" } else { "" };
    format!("Ctrl+{shift}{}", action.key)
}

/// Draws the pane of `tab`: the selection chain, the selected node's kind and where it stands, and its actions.
pub fn show(ui: &mut egui::Ui, tab: &mut HierarchyTab, theme: &Theme, palette: &Palette) {
    let muted = theme::color(theme, &["descriptionForeground"], palette.foreground);
    let small = theme.sizes.workbench_font - 2.0;
    let Some(node) = tab.model.nodes.iter().find(|n| n.id == tab.selected).cloned() else {
        ui.label(RichText::new("Select a box to see its properties.").color(muted));
        return;
    };
    ui.label(
        RichText::new(format!("{} › {}", tab.name(), describe(&node)))
            .size(small)
            .color(muted),
    );
    ui.add_space(4.0);
    let parent = tab.model.nodes.iter().find(|n| n.id == node.parent_id);
    let within = parent.map_or_else(|| tab.model.title.clone(), describe);
    egui::Grid::new(("hierarchy-properties", tab.id()))
        .num_columns(2)
        .min_col_width((ui.available_width() * 0.4).max(80.0))
        .spacing(egui::vec2(8.0, 4.0))
        .show(ui, |ui| {
            for (label, value) in [("Kind", node.type_name.as_str()), ("In", within.as_str())] {
                ui.label(RichText::new(label).color(palette.side_bar_foreground));
                ui.label(RichText::new(value).color(muted));
                ui.end_row();
            }
        });
    let actions = tab.actions();
    if actions.is_empty() {
        return;
    }
    ui.add_space(8.0);
    ui.label(RichText::new("ACTIONS").size(small).color(palette.side_bar_title));
    let mut run = None;
    for action in &actions {
        let available = action.unavailable.is_empty();
        let key = shortcut(action);
        let text = if key.is_empty() {
            action.label.clone()
        } else {
            format!("{}  {key}", action.label)
        };
        let response = ui.add_enabled(available, egui::Button::new(text));
        a11y::button(&response, &action.label, &key);
        if !available {
            a11y::describe_state(&response, &action.unavailable);
            response.on_disabled_hover_text(&action.unavailable);
        } else if response.clicked() {
            run = Some(action.id.clone());
        }
    }
    if let Some(id) = run {
        tab.run_action(&id);
    }
}
