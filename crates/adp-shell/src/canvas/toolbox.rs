//! The toolbox pane (etalii.adp spec 030, FR-018, task T057): the palette of the active diagram, each entry dragged
//! onto the canvas to make what it makes where it is dropped.
//!
//! Ported from the standalone host's `src/client/src/shell/panels/ToolboxPanel.tsx` at `8b6ccfc`: the entries the
//! diagram's session gives (`DescribeToolbox`), in its order, by label with the description as the tooltip, in the
//! side bar's colours and row height, and a sentence where a diagram has none.

use adp_diagram::workspace::DiagramWorkspace;
use adp_theme::Theme;
use eframe::egui::{self, RichText};

use super::{DiagramTab, ToolboxDrag};
use crate::theme::{self, Palette};

/// Draws the toolbox of `tab`.
pub fn show(ui: &mut egui::Ui, tab: &DiagramTab, workspace: &DiagramWorkspace, theme: &Theme, palette: &Palette) {
    let items = tab.session.toolbox(workspace.documents());
    if items.is_empty() {
        ui.label(RichText::new("This diagram has no toolbox.").color(theme::color(
            theme,
            &["descriptionForeground"],
            palette.foreground,
        )));
        return;
    }
    ui.label(
        RichText::new("Drag an entry onto the canvas.")
            .size(theme.sizes.workbench_font - 1.0)
            .color(theme::color(theme, &["descriptionForeground"], palette.foreground)),
    );
    ui.add_space(4.0);
    for item in items {
        let id = egui::Id::unique(("toolbox", tab.opened.body_path.clone(), item.id.clone()));
        let payload = ToolboxDrag {
            tool_id: item.id.clone(),
        };
        let response = ui
            .dnd_drag_source(id, payload, |ui| {
                ui.set_min_height(theme.sizes.row_height);
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(RichText::new(&item.label).color(palette.side_bar_foreground));
                });
            })
            .response;
        if !item.description.is_empty() {
            response.on_hover_text(&item.description);
        }
    }
}
