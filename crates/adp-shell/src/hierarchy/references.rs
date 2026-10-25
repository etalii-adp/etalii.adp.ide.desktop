//! References between the nodes of a document (etalii.adp spec 036, T046), ported from the standalone host's
//! `hierarchySurface/library/references/{ReferenceBox,ReferenceLookup}.tsx` and the `#` of `editing/EntryTextField.tsx`
//! at a7fdf03: a reference in a description stands as a box with its target's number and current title, which shows
//! the target when it is selected ([`super::HierarchyTab::reveal`], from `reveal.ts`); one that names nothing keeps
//! its text as written and is marked broken. Typing `#` in a description opens the lookup under the field, of every
//! numbered level and entry of the document, narrowed by number or title; the pick is written as its bracketed number.

use adp_desl::hierarchy::model::{Reference, SurfaceNode};
use eframe::egui::{self, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2, accesskit::Role};

use super::is_line_id;
use crate::a11y;
use crate::table::view::Colours;

/// The lookup's width, and the most rows it shows before it scrolls.
const LOOKUP_WIDTH: f32 = 300.0;
const LOOKUP_HEIGHT: f32 = 240.0;

/// A part of a text: as it is written, or a reference, with the node it names, empty for none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Part {
    Text(String),
    Reference { written: String, target_id: String },
}

/// The character `offset` UTF-16 code units into `text` stands at, as a byte index; none past its end or inside a
/// character.
fn byte_at(text: &str, offset: usize) -> Option<usize> {
    let mut units = 0;
    for (at, c) in text.char_indices() {
        if units == offset {
            return Some(at);
        }
        units += c.len_utf16();
    }
    (units == offset).then_some(text.len())
}

/// A text with each reference in it as a part of its own, and the text between them as it is written. The
/// references are the model's, by their offsets in the text; one that does not fit the text, or overlaps the one
/// before it, is left as text.
pub fn parts(text: &str, references: &[Reference]) -> Vec<Part> {
    let mut sorted: Vec<&Reference> = references.iter().collect();
    sorted.sort_by_key(|r| r.start);
    let mut parts = Vec::new();
    let mut at = 0;
    for reference in sorted {
        let (Some(start), Some(end)) = (
            byte_at(text, reference.start),
            byte_at(text, reference.start + reference.length),
        ) else {
            continue;
        };
        if start < at {
            continue;
        }
        parts.push(Part::Text(text[at..start].to_owned()));
        parts.push(Part::Reference {
            written: text[start..end].to_owned(),
            target_id: reference.target_id.clone(),
        });
        at = end;
    }
    parts.push(Part::Text(text[at..].to_owned()));
    parts.retain(|p| *p != Part::Text(String::new()));
    parts
}

/// Where a `#` was typed into `before` to give `after`, in characters: one `#` and nothing else changed.
pub fn hash_typed(before: &str, after: &str) -> Option<usize> {
    let (before, after): (Vec<char>, Vec<char>) = (before.chars().collect(), after.chars().collect());
    if after.len() != before.len() + 1 {
        return None;
    }
    let at = before.iter().zip(&after).take_while(|(b, a)| b == a).count();
    (after[at] == '#' && after[at + 1..] == before[at..]).then_some(at)
}

/// The levels and entries a reference can name: every node of the document that has a number, as the lookup lists
/// it, number and title.
pub fn candidates(nodes: &[SurfaceNode]) -> Vec<(String, String)> {
    nodes
        .iter()
        .filter(|n| !is_line_id(&n.id))
        .map(|n| (n.number.clone(), format!("{} {}", n.number, n.title)))
        .collect()
}

/// The candidates whose label holds the typed text, case ignored; all of them for no text.
pub fn matching<'c>(candidates: &'c [(String, String)], typed: &str) -> Vec<&'c (String, String)> {
    let wanted = typed.trim().to_lowercase();
    candidates
        .iter()
        .filter(|(_, label)| wanted.is_empty() || label.to_lowercase().contains(&wanted))
        .collect()
}

/// The text a pick leaves: the `#` at `at`, in characters, becomes the bracketed `number`; and where the caret goes.
pub fn picked(text: &str, at: usize, number: &str) -> (String, usize) {
    let chars: Vec<char> = text.chars().collect();
    let written = format!("[{number}]");
    let before: String = chars[..at.min(chars.len())].iter().collect();
    let after: String = chars.get(at + 1..).map(|c| c.iter().collect()).unwrap_or_default();
    (format!("{before}{written}{after}"), at + written.chars().count())
}

/// The lookup `#` opened in a description: where the `#` stands, what is typed in it and the row the arrows are on.
#[derive(Debug, Clone, PartialEq)]
pub struct Lookup {
    pub at: usize,
    pub typed: String,
    pub active: usize,
    /// Whether it is yet to take the keyboard, as it does once it opens.
    pub opened: bool,
    /// Where it stood in the last frame, so that a press on it does not take the keyboard from its field.
    pub rect: Option<Rect>,
}

impl Lookup {
    pub fn at(at: usize) -> Lookup {
        Lookup {
            at,
            typed: String::new(),
            active: 0,
            opened: true,
            rect: None,
        }
    }
}

/// What became of the lookup in a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Looked {
    /// A node was picked; its number is what the reference writes.
    Picked(String),
    /// Escape: back to the text, with what was typed kept.
    Closed,
    /// The keyboard left both the lookup and its field.
    Left,
}

/// Draws the lookup under `anchor`, the field it belongs to, in front of the page: a field typed in to narrow the
/// candidates, and a row for each, picked by Enter or a press; the arrows move between them and Escape closes it.
pub fn lookup(
    ui: &egui::Ui,
    lookup: &mut Lookup,
    candidates: &[(String, String)],
    anchor: Rect,
    id: egui::Id,
    colours: &Colours,
    font: &FontId,
) -> Option<Looked> {
    let ctx = ui.ctx().clone();
    let field = id.with("search");
    let rows = matching(candidates, &lookup.typed);
    let mut looked = None;
    // Asked until it has it: an area is drawn unseen in its first frame, where its field cannot take the keyboard.
    if lookup.opened {
        ctx.memory_mut(|m| m.request_focus(field));
    }
    if ctx.memory(|m| m.has_focus(field)) {
        ctx.input_mut(|i| {
            if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
                lookup.active = (lookup.active + 1).min(rows.len().saturating_sub(1));
            }
            if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
                lookup.active = lookup.active.saturating_sub(1);
            }
            if i.consume_key(egui::Modifiers::NONE, egui::Key::Enter) {
                looked = rows
                    .get(lookup.active)
                    .map(|(number, _)| Looked::Picked(number.clone()));
            }
            if i.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
                looked = Some(Looked::Closed);
            }
        });
    }
    // A press on the lookup keeps the keyboard in its field, as the field of a completion keeps it.
    let pointer_on = lookup
        .rect
        .zip(ctx.pointer_interact_pos())
        .is_some_and(|(rect, at)| rect.contains(at));
    let surrender = ctx.options(|o| o.input_options.surrender_focus_on);
    if pointer_on {
        ctx.options_mut(|o| o.input_options.surrender_focus_on = egui::SurrenderFocusOn::Never);
    }
    let screen = ctx.content_rect();
    let width = LOOKUP_WIDTH.min(anchor.width().max(LOOKUP_WIDTH * 0.5));
    let left = anchor.left().min(screen.right() - width).max(screen.left());
    let area = egui::Area::new(id)
        .order(egui::Order::Foreground)
        .fixed_pos(Pos2::new(left, anchor.bottom() + 4.0))
        .show(&ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(colours.surface)
                .stroke(Stroke::new(1.0, colours.border))
                .show(ui, |ui| {
                    ui.set_width(width - 16.0);
                    let edit = egui::TextEdit::singleline(&mut lookup.typed)
                        .id(field)
                        .hint_text("Number or title")
                        .font(font.clone())
                        .desired_width(f32::INFINITY);
                    let response = ui.add(edit);
                    response.ctx.accesskit_node_builder(response.id, |node| {
                        node.set_role(Role::ComboBox);
                        node.set_label("Refer to");
                    });
                    if response.changed() {
                        lookup.active = 0;
                    }
                    if response.has_focus() {
                        lookup.opened = false;
                    }
                    if response.lost_focus() && looked.is_none() {
                        looked = Some(Looked::Left);
                    }
                    egui::ScrollArea::vertical().max_height(LOOKUP_HEIGHT).show(ui, |ui| {
                        if rows.is_empty() {
                            ui.label(RichText::new("Nothing matches.").color(colours.muted));
                        }
                        for (i, (number, label)) in rows.iter().enumerate() {
                            let row = ui.add(
                                egui::Button::selectable(i == lookup.active, label.as_str())
                                    .min_size(Vec2::new(ui.available_width(), 0.0))
                                    .sense(Sense::click()),
                            );
                            a11y::describe(&row, Role::ListBoxOption, label);
                            if row.hovered() {
                                lookup.active = i;
                            }
                            if row.clicked() {
                                looked = Some(Looked::Picked(number.clone()));
                            }
                        }
                    });
                })
        });
    ctx.options_mut(|o| o.input_options.surrender_focus_on = surrender);
    a11y::describe(&area.response, Role::ListBox, "Refer to");
    lookup.rect = Some(area.response.rect);
    looked
}
