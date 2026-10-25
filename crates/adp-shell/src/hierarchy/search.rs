//! A list searched by typing (the standalone host's `components/SearchList.tsx` at a7fdf03), which a label's field and
//! picker open ([`super::labels`], etalii.adp spec 036, T047): it picks with Enter or a press and, when its owner
//! allows, creates an item from text that matches none. It holds no choice of its own: what is chosen comes in, and a
//! pick goes out, so the same list serves one value and several. To a screen reader it is a combo box over its rows,
//! each an option, selected when it is chosen.

use eframe::egui::{
    self, Color32, FontId, RichText, Sense, Vec2,
    accesskit::Role,
    text::{LayoutJob, TextFormat},
};

use crate::a11y;
use crate::icons;
use crate::table::view::Colours;

/// The most rows the list shows before it scrolls.
const ROWS_HEIGHT: f32 = 220.0;
/// The size of a row's text and of its mark.
const ROW_FONT: f32 = 13.0;

/// What the list holds between frames: the typed text, the row the arrows are on, and whether its field is yet to
/// take the keyboard, as it does once the list opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchList {
    pub text: String,
    pub active: usize,
    pub focusing: bool,
}

impl Default for SearchList {
    fn default() -> SearchList {
        SearchList {
            text: String::new(),
            active: 0,
            focusing: true,
        }
    }
}

/// What became of the list in a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Searched {
    /// An item was picked, or picked again when it was already chosen; its id.
    Picked(String),
    /// The typed text matches no item and was asked to exist.
    Created(String),
}

/// A row of the list: an item to pick, by its index, or the offer to create what was typed.
enum Row {
    Item(usize),
    Create(String),
}

/// The items whose label holds the typed text, case ignored; all of them for no text (`matching`).
pub fn matching(items: &[(String, String)], text: &str) -> Vec<usize> {
    let wanted = text.trim().to_lowercase();
    (0..items.len())
        .filter(|&i| wanted.is_empty() || items[i].1.to_lowercase().contains(&wanted))
        .collect()
}

/// Draws the list in `ui`: a field named `label`, then a row for each item, `(id, label)`, the `chosen` ones checked,
/// and, when `creates`, the offer to create text that names no item exactly.
#[allow(clippy::too_many_arguments)]
pub fn search_list(
    ui: &mut egui::Ui,
    list: &mut SearchList,
    id: egui::Id,
    label: &str,
    items: &[(String, String)],
    chosen: &[String],
    placeholder: &str,
    creates: bool,
    colours: &Colours,
) -> Option<Searched> {
    let mut rows: Vec<Row> = matching(items, &list.text).into_iter().map(Row::Item).collect();
    let typed = list.text.trim().to_owned();
    // Offered only for text that names no item exactly: creating a second "Capital" beside the first is never meant.
    if creates && !typed.is_empty() && !items.iter().any(|(_, l)| l.to_lowercase() == typed.to_lowercase()) {
        rows.push(Row::Create(typed));
    }
    let current = list.active.min(rows.len().saturating_sub(1));
    let field = id.with("field");
    let mut chose = None;
    // Read before the field asks for the keyboard: the key that opened the list is not one typed in it.
    let typing = ui.memory(|m| m.has_focus(field));
    if list.focusing {
        ui.memory_mut(|m| m.request_focus(field));
    }
    if typing {
        ui.input_mut(|i| {
            if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
                list.active = (current + 1).min(rows.len().saturating_sub(1));
            }
            if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
                list.active = current.saturating_sub(1);
            }
            if i.consume_key(egui::Modifiers::NONE, egui::Key::Enter) {
                chose = rows.get(current).map(|_| current);
            }
        });
    }
    let response = ui.add(
        egui::TextEdit::singleline(&mut list.text)
            .id(field)
            .hint_text(placeholder)
            .desired_width(f32::INFINITY),
    );
    response.ctx.accesskit_node_builder(response.id, |node| {
        node.set_role(Role::ComboBox);
        node.set_label(label);
        node.set_expanded(true);
    });
    if response.changed() {
        list.active = 0;
    }
    if response.has_focus() {
        list.focusing = false;
    }
    egui::ScrollArea::vertical()
        .id_salt(id.with("rows"))
        .max_height(ROWS_HEIGHT)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(0.0, 0.0);
            if rows.is_empty() {
                ui.label(RichText::new("Nothing matches.").color(colours.muted));
            }
            for (i, row) in rows.iter().enumerate() {
                let (mark, text, picked) = match row {
                    Row::Item(item) => {
                        let picked = chosen.contains(&items[*item].0);
                        (
                            if picked { "mdi-check" } else { "" },
                            items[*item].1.clone(),
                            Some(picked),
                        )
                    }
                    Row::Create(text) => ("mdi-plus", format!("Create “{text}”"), None),
                };
                let fill = if i == current {
                    colours.raised
                } else {
                    Color32::TRANSPARENT
                };
                // The mark of a chosen item, or the room of one, before the text.
                let mut job = LayoutJob::default();
                let icon = TextFormat::simple(icons::font(ROW_FONT), colours.text);
                job.append(&icons::text(mark), 0.0, icon);
                let lead = if mark.is_empty() { ROW_FONT } else { 0.0 } + 6.0;
                job.append(
                    &text,
                    lead,
                    TextFormat::simple(FontId::proportional(ROW_FONT), colours.text),
                );
                let button = egui::Button::new(job)
                    .fill(fill)
                    .stroke(egui::Stroke::NONE)
                    .min_size(Vec2::new(ui.available_width(), 24.0))
                    .sense(Sense::click());
                let row_response = ui.add(button);
                a11y::describe(&row_response, Role::ListBoxOption, &text);
                row_response.ctx.accesskit_node_builder(row_response.id, |node| {
                    node.clear_toggled();
                    node.set_selected(picked.unwrap_or(false));
                });
                if row_response.hovered() {
                    list.active = i;
                }
                if row_response.clicked() {
                    chose = Some(i);
                    // The field keeps the keyboard: a press on a row took it.
                    list.focusing = true;
                }
            }
        });
    let picked = chose.and_then(|i| rows.get(i))?;
    Some(match picked {
        Row::Item(item) => Searched::Picked(items[*item].0.clone()),
        Row::Create(text) => {
            list.text.clear();
            list.active = 0;
            Searched::Created(text.clone())
        }
    })
}
