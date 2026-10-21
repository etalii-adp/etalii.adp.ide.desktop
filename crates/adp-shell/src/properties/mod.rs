//! The property grid (etalii.adp spec 030, FR-017, Story 2 scenario 4, task T057): the rows of what is selected on
//! the canvas, one element or several, each edited with the editor its row names, and every change one command on
//! the project's history, a change to several elements included.
//!
//! Ported from the standalone host's `src/client/src/shell/panels/PropertyGridPanel.tsx` and `PropertyRow.tsx` at
//! `8b6ccfc`: the selection chain above the rows, the rows under their group headings in the order the session
//! gives them, a label column and a value column, a line or text field committed when it loses the focus (or on
//! Enter for a line), a toggle, a choice and an ordered slider committed at once, and a read-only row showing its
//! value with the reason it cannot be changed. For several elements the session gives the rows they share, a value
//! left empty where they differ, and applies a change to all of them as one undo.

use std::collections::HashMap;

use adp_context::{PropertyEditor, PropertyRow};
use adp_diagram::workspace::DiagramWorkspace;
use adp_theme::Theme;
use eframe::egui::{self, RichText};

use crate::canvas::DiagramTab;
use crate::table::TableTab;
use crate::theme::{self, Palette};

/// What the grid keeps between frames: the text of a field being edited, by row id, for the selection it was
/// opened on.
#[derive(Debug, Default)]
pub struct Grid {
    editing: HashMap<String, String>,
    selection: Vec<String>,
}

/// Sets property `property_id` of everything selected on `tab` to `value`, as one command.
pub fn set(tab: &mut DiagramTab, workspace: &mut DiagramWorkspace, property_id: &str, value: &str) {
    let ids: Vec<&str> = tab.selection.iter().map(String::as_str).collect();
    let result = tab.session.set_property(workspace, &ids, property_id, value);
    tab.report(result);
}

/// The rows of what is selected on `tab`.
pub fn rows(tab: &DiagramTab, workspace: &DiagramWorkspace) -> Vec<PropertyRow> {
    let ids: Vec<&str> = tab.selection.iter().map(String::as_str).collect();
    if ids.is_empty() {
        return Vec::new();
    }
    tab.session.properties(workspace.documents(), &ids)
}

/// Draws the property grid of `tab`'s selection.
pub fn show(
    ui: &mut egui::Ui,
    tab: &mut DiagramTab,
    workspace: &mut DiagramWorkspace,
    theme: &Theme,
    palette: &Palette,
) {
    if tab.grid.selection != tab.selection {
        tab.grid.editing.clear();
        tab.grid.selection = tab.selection.clone();
    }
    let muted = theme::color(theme, &["descriptionForeground"], palette.foreground);
    let small = theme.sizes.workbench_font - 2.0;
    if tab.selection.is_empty() {
        ui.label(RichText::new("Select an element to see its properties.").color(muted));
        return;
    }
    // The selection chain: the diagram, then what is selected in it.
    let chain = match tab.selection.as_slice() {
        [one] => tab
            .session
            .select(workspace.documents(), one)
            .map(|s| s.path.join(" › "))
            .unwrap_or_else(|| one.clone()),
        many => format!("{} elements", many.len()),
    };
    ui.label(
        RichText::new(format!("{} › {chain}", tab.name()))
            .size(small)
            .color(muted),
    );
    ui.add_space(4.0);
    let rows = rows(tab, workspace);
    if rows.is_empty() {
        ui.label(RichText::new("Nothing here can be edited.").color(muted));
        return;
    }
    let mut groups: Vec<(String, Vec<&PropertyRow>)> = Vec::new();
    for row in &rows {
        match groups.iter_mut().find(|(g, _)| *g == row.group) {
            Some((_, members)) => members.push(row),
            None => groups.push((row.group.clone(), vec![row])),
        }
    }
    let mut commit: Option<(String, String)> = None;
    let label_width = (ui.available_width() * 0.4).max(80.0);
    for (group, members) in groups {
        if !group.is_empty() {
            ui.add_space(6.0);
            ui.label(
                RichText::new(group.to_uppercase())
                    .size(small)
                    .strong()
                    .color(palette.side_bar_title),
            );
        }
        egui::Grid::new(("property-grid", tab.opened.body_path.clone(), group.clone()))
            .num_columns(2)
            .min_col_width(label_width)
            .spacing(egui::vec2(8.0, 4.0))
            .show(ui, |ui| {
                for row in members {
                    ui.label(RichText::new(&row.label).color(palette.side_bar_foreground))
                        .on_hover_text(&row.id);
                    if let Some(value) = editor(ui, &mut tab.grid.editing, row, muted) {
                        commit = Some((row.id.clone(), value));
                    }
                    ui.end_row();
                }
            });
    }
    if let Some((id, value)) = commit {
        set(tab, workspace, &id, &value);
    }
}

/// Draws the property grid of the row selected in `tab`'s table (the standalone host's `KnowledgeRowPropertyProvider`,
/// T064): one row per property, each edited as its type is, and each change the same edit it is in the table.
pub fn show_row(ui: &mut egui::Ui, tab: &mut TableTab, theme: &Theme, palette: &Palette) {
    if tab.grid_row != tab.selected {
        tab.grid_editing.clear();
        tab.grid_row = tab.selected.clone();
    }
    let muted = theme::color(theme, &["descriptionForeground"], palette.foreground);
    let small = theme.sizes.workbench_font - 2.0;
    let Some(title) = tab.selected_title() else {
        ui.label(RichText::new("Select a row to see its properties.").color(muted));
        return;
    };
    ui.label(
        RichText::new(format!("{} › {title}", tab.name()))
            .size(small)
            .color(muted),
    );
    ui.add_space(4.0);
    let rows = tab.property_rows();
    let mut commit: Option<(String, String)> = None;
    let label_width = (ui.available_width() * 0.4).max(80.0);
    egui::Grid::new(("row-grid", tab.body_path().to_path_buf()))
        .num_columns(2)
        .min_col_width(label_width)
        .spacing(egui::vec2(8.0, 4.0))
        .show(ui, |ui| {
            for row in &rows {
                ui.label(RichText::new(&row.label).color(palette.side_bar_foreground))
                    .on_hover_text(&row.id);
                if let Some(value) = editor(ui, &mut tab.grid_editing, row, muted) {
                    commit = Some((row.id.clone(), value));
                }
                ui.end_row();
            }
        });
    if let Some((id, value)) = commit {
        tab.set_property(&id, &value);
    }
}

/// One row's editor; the new value when it was committed this frame.
fn editor(
    ui: &mut egui::Ui,
    editing: &mut HashMap<String, String>,
    row: &PropertyRow,
    muted: egui::Color32,
) -> Option<String> {
    if row.is_read_only() {
        let shown = if row.value.is_empty() {
            "—"
        } else {
            row.value.as_str()
        };
        ui.vertical(|ui| {
            ui.label(RichText::new(shown).color(muted));
            ui.label(RichText::new(&row.read_only_reason).small().color(muted));
        });
        return None;
    }
    let id = egui::Id::unique(("property", &row.id));
    match row.editor {
        PropertyEditor::Line | PropertyEditor::Tags | PropertyEditor::Text => {
            let mut text = editing.get(&row.id).cloned().unwrap_or_else(|| row.value.clone());
            let field = if row.editor == PropertyEditor::Text {
                egui::TextEdit::multiline(&mut text).desired_rows(3)
            } else {
                egui::TextEdit::singleline(&mut text)
            };
            let response = ui.add(field.id(id).desired_width(f32::INFINITY));
            if response.has_focus() {
                editing.insert(row.id.clone(), text.clone());
            }
            if response.lost_focus() {
                editing.remove(&row.id);
                if text != row.value {
                    return Some(text);
                }
            }
            None
        }
        PropertyEditor::Toggle => {
            let mut on = row.value == "true";
            let response = ui.checkbox(&mut on, "");
            response.changed().then(|| on.to_string())
        }
        PropertyEditor::Choice => {
            let mut chosen = None;
            let shown = if row.value.is_empty() {
                "—"
            } else {
                row.value.as_str()
            };
            egui::ComboBox::from_id_salt(id)
                .selected_text(shown)
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for candidate in &row.candidates {
                        if ui.selectable_label(*candidate == row.value, candidate).clicked() && *candidate != row.value
                        {
                            chosen = Some(candidate.clone());
                        }
                    }
                });
            chosen
        }
        PropertyEditor::Slider => {
            if row.candidates.is_empty() {
                ui.label(&row.value);
                return None;
            }
            let mut index = row.candidates.iter().position(|c| *c == row.value).unwrap_or(0);
            let last = row.candidates.len() - 1;
            let candidates = row.candidates.clone();
            let response = ui.add(
                egui::Slider::new(&mut index, 0..=last)
                    .custom_formatter(move |v, _| candidates.get(v as usize).cloned().unwrap_or_default())
                    .show_value(true),
            );
            let value = row.candidates[index].clone();
            (response.changed() && value != row.value).then_some(value)
        }
    }
}
