//! A column's options, edited (the standalone host's `header/OptionsEditor.tsx` at a7fdf03): each renamed, recoloured,
//! put in its place and deleted, and a new one added. The table's header opens it under a column, and the hierarchy
//! surface opens it for a label's list (etalii.adp spec 036, T047), which is why it holds its own state and returns
//! its gestures rather than raising them on a tab.
//!
//! - Every change is one gesture, made as it is chosen; the editor shows the options the model has, so it shows what
//!   was accepted and nothing it only hoped for. A name is committed when its field is left or Enter is pressed, not on
//!   every key: a rename is an edit of the file.
//! - Its lines stand in columns: every option's tag as wide as the widest, its name, its colour and its controls lined
//!   up below each other.
//! - An option is put in its place by its up and down controls, or by dragging its handle, one gesture when the drag
//!   ends; while it is dragged, the list stands as it would once it is dropped, the dragged option raised.
//! - Deleting an option some rows have asks first, in its owner's words.

use std::collections::HashMap;

use adp_desl::table::model::{TableColumn, TableGesture, TableOption};
use eframe::egui::{self, Align2, FontId, Rect, Sense, Stroke, StrokeKind, Vec2, accesskit::Role};

use super::view::Colours;
use crate::a11y;
use crate::icons;

/// The ten colours an option may have (`OPTION_COLORS`).
pub const OPTION_COLOURS: [&str; 10] = [
    "default", "gray", "brown", "orange", "yellow", "green", "blue", "purple", "pink", "red",
];
/// The setting a column says under how many rows have one of its options (`USES_PREFIX`).
pub const USES_PREFIX: &str = "uses:";

/// A line's height and the space between two lines (`.table-options-line`, `.table-options-list`).
const LINE: f32 = 26.0;
const GAP: f32 = 4.0;
/// The widths of a line's handle, its name field and its colour's choice, and of a control.
const HANDLE: f32 = 16.0;
const NAME: f32 = 120.0;
const COLOUR: f32 = 96.0;
const CONTROL: f32 = 22.0;
/// A tag's side padding and its font size (`.table-option-tag`).
const TAG_PADDING: f32 = 7.0;
const TAG_FONT: f32 = 12.0;

/// What the editor holds between frames: the names being typed, by option id, the new option's name, the option a
/// delete asks about, and the drag of an option while it lasts.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OptionsEditing {
    names: HashMap<String, String>,
    adding: String,
    asking: Option<String>,
    moving: Option<Moving>,
}

/// An option dragged by its handle: where it stood, and how far it has come.
#[derive(Debug, Clone, PartialEq)]
struct Moving {
    from: usize,
    dy: f32,
}

impl Moving {
    /// Where the option would be dropped, among `count` lines.
    fn to(&self, count: usize) -> usize {
        let lines = (self.dy / (LINE + GAP)).round() as isize;
        (self.from as isize + lines).clamp(0, count.saturating_sub(1) as isize) as usize
    }
}

/// What deleting an option of a table's column asks (`tableDeleteQuestion`).
pub fn table_question(_option: &TableOption, uses: usize) -> String {
    if uses == 1 {
        "1 row has this option. Deleting it clears that value.".to_owned()
    } else {
        format!("{uses} rows have this option. Deleting it clears those values.")
    }
}

/// How many rows have `option`, as the column says it; nothing said is none.
fn uses_of(column: &TableColumn, option: &TableOption) -> usize {
    column
        .settings
        .get(&format!("{USES_PREFIX}{}", option.id))
        .and_then(|u| u.parse().ok())
        .unwrap_or(0)
}

/// A colour's name as its choice shows it.
fn capitalised(word: &str) -> String {
    let mut chars = word.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// Draws the options of `column` in `ui`, and a field that adds one when `addable`; the gestures they made this frame.
/// `question` is what deleting an option some rows have asks, for the option and how many have it.
pub fn options_editor(
    ui: &mut egui::Ui,
    editing: &mut OptionsEditing,
    id: egui::Id,
    column: &TableColumn,
    colours: &Colours,
    addable: bool,
    question: &dyn Fn(&TableOption, usize) -> String,
) -> Vec<TableGesture> {
    let mut gestures = Vec::new();
    let count = column.options.len();
    let font = FontId::proportional(TAG_FONT);
    // The tags' column, as wide as the widest of them.
    let tag_width = column
        .options
        .iter()
        .map(|o| {
            ui.painter()
                .layout_no_wrap(o.name.clone(), font.clone(), colours.text)
                .size()
                .x
        })
        .fold(0.0, f32::max)
        + 2.0 * TAG_PADDING;
    // While an option is dragged, the lines stand as they would once it is dropped.
    let mut order: Vec<usize> = (0..count).collect();
    if let Some(moving) = editing.moving.as_ref().filter(|m| m.from < count) {
        let dragged = order.remove(moving.from);
        order.insert(moving.to(count), dragged);
    }
    ui.spacing_mut().item_spacing = Vec2::splat(GAP);
    for index in order {
        let option = &column.options[index];
        let gesture = |kind: &str| TableGesture::new(kind).column(&column.id).target(&option.id);
        let dragged = editing.moving.as_ref().is_some_and(|m| m.from == index);
        let line = ui.horizontal(|ui| {
            ui.set_min_height(LINE);
            // Named by the option, not by its place, which a drag changes.
            let (handle, _) = ui.allocate_exact_size(Vec2::new(HANDLE, LINE), Sense::hover());
            let response = ui.interact(handle, id.with(("handle", &option.id)), Sense::drag());
            a11y::describe(&response, Role::Image, &format!("Move option {}", option.name));
            ui.painter().text(
                handle.center(),
                Align2::CENTER_CENTER,
                icons::text("mdi-drag-vertical"),
                icons::font(16.0),
                colours.muted,
            );
            if response.drag_started() {
                editing.moving = Some(Moving { from: index, dy: 0.0 });
            }
            if response.dragged()
                && let Some(moving) = editing.moving.as_mut()
            {
                moving.dy += response.drag_delta().y;
            }
            if response.drag_stopped()
                && let Some(moving) = editing.moving.take()
            {
                let to = moving.to(count);
                if to != moving.from {
                    gestures.push(gesture("moveOption").index(to as i64));
                }
            }

            let (tag, _) = ui.allocate_exact_size(Vec2::new(tag_width, LINE), Sense::hover());
            let tag = Rect::from_center_size(tag.center(), Vec2::new(tag_width, 18.0));
            ui.painter().rect_filled(tag, 3.0, colours.option(&option.color));
            ui.painter().text(
                tag.center(),
                Align2::CENTER_CENTER,
                &option.name,
                font.clone(),
                colours.text,
            );

            let key = option.id.clone();
            let name = editing.names.entry(key.clone()).or_insert_with(|| option.name.clone());
            let field = ui.add_sized(
                Vec2::new(NAME, LINE),
                egui::TextEdit::singleline(name).id(id.with(("name", &option.id))),
            );
            a11y::label(&field, &format!("Name of {}", option.name));
            if field.lost_focus() {
                let next = name.trim().to_owned();
                editing.names.remove(&key);
                if !next.is_empty() && next != option.name {
                    gestures.push(gesture("renameOption").values([next]));
                }
            } else if !field.has_focus() {
                // The model's name is the truth: a rename refused, or made elsewhere, shows here.
                *name = option.name.clone();
            }

            let shown = if OPTION_COLOURS.contains(&option.color.as_str()) {
                option.color.as_str()
            } else {
                "default"
            };
            let choice = egui::ComboBox::from_id_salt(id.with(("colour", &option.id)))
                .width(COLOUR)
                // All ten at once, none scrolled out of reach.
                .height(f32::INFINITY)
                .selected_text(capitalised(shown))
                .show_ui(ui, |ui| {
                    for colour in OPTION_COLOURS {
                        if ui.selectable_label(colour == shown, capitalised(colour)).clicked() && colour != shown {
                            gestures.push(gesture("recolourOption").setting("colour", colour));
                        }
                    }
                });
            a11y::describe(&choice.response, Role::ComboBox, &format!("Colour of {}", option.name));

            let control = |ui: &mut egui::Ui, icon: &str, name: String, enabled: bool| {
                let (place, _) = ui.allocate_exact_size(Vec2::splat(CONTROL), Sense::hover());
                let response = ui.interact(place, id.with((icon, &option.id)), Sense::click());
                let response = response.on_hover_text(&name);
                a11y::describe(&response, Role::Button, &name);
                let colour = if !enabled {
                    colours.muted.gamma_multiply(0.4)
                } else if response.hovered() {
                    colours.text
                } else {
                    colours.muted
                };
                ui.painter().text(
                    place.center(),
                    Align2::CENTER_CENTER,
                    icons::text(icon),
                    icons::font(16.0),
                    colour,
                );
                enabled && response.clicked()
            };
            if control(ui, "mdi-chevron-up", format!("Move {} up", option.name), index > 0) {
                gestures.push(gesture("moveOption").index(index as i64 - 1));
            }
            if control(
                ui,
                "mdi-chevron-down",
                format!("Move {} down", option.name),
                index + 1 < count,
            ) {
                gestures.push(gesture("moveOption").index(index as i64 + 1));
            }
            let uses = uses_of(column, option);
            if control(ui, "mdi-trash-can-outline", format!("Delete {}", option.name), true) {
                if uses > 0 {
                    editing.asking = Some(option.id.clone());
                } else {
                    gestures.push(gesture("deleteOption"));
                }
            }
        });
        if dragged {
            // The option being dragged, raised above the others making way for it.
            ui.painter().rect_stroke(
                line.response.rect.expand(2.0),
                3.0,
                Stroke::new(1.0, colours.focus),
                StrokeKind::Outside,
            );
        }
        if editing.asking.as_deref() == Some(option.id.as_str()) {
            // An option some rows have is not deleted on one press: its author is told how many first.
            let asked = ui.horizontal_wrapped(|ui| {
                ui.label(question(option, uses_of(column, option)));
                if ui.button("Delete").clicked() {
                    editing.asking = None;
                    gestures.push(gesture("deleteOption"));
                }
                if ui.button("Keep").clicked() {
                    editing.asking = None;
                }
            });
            a11y::describe(&asked.response, Role::AlertDialog, &format!("Delete {}?", option.name));
        }
    }
    if editing.moving.is_some() && !ui.input(|i| i.pointer.any_down()) {
        // A drag that ended without its handle hearing of it is no move.
        editing.moving = None;
    }
    if addable {
        ui.horizontal(|ui| {
            let field = ui.add(
                egui::TextEdit::singleline(&mut editing.adding)
                    .id(id.with("new"))
                    .hint_text("Add an option…")
                    .desired_width(HANDLE + tag_width + NAME + 2.0 * GAP),
            );
            a11y::label(&field, "New option");
            let entered = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            let name = editing.adding.trim().to_owned();
            let pressed = ui.add_enabled(!name.is_empty(), egui::Button::new("Add")).clicked();
            if (entered || pressed) && !name.is_empty() {
                gestures.push(TableGesture::new("addOption").column(&column.id).values([name]));
                editing.adding.clear();
            }
        });
    }
    gestures
}
