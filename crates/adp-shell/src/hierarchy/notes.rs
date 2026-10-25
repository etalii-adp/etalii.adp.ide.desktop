//! A node's notes (etalii.adp spec 036, T042): the standalone host's `notes/noteMark.ts`, `NoteIcon.tsx` and
//! `NoteList.tsx` at a7fdf03.
//!
//! - The icon at a box's top right is the first of its controls: the kind's own icon for one open note, the icon for
//!   several with their count, and with none, the way to add the first, on a document that can change. A resolved note
//!   is not counted, and a collapsed level's icon counts every note inside it, since nothing inside it shows.
//! - It opens the box's list of notes, drawn above every pane and placed under the box's icons in the window: each
//!   note with its kind, the number it is about, its text and its detail, resolved ones too; each resolved or reopened,
//!   changed or removed by the icons beside it; and a new one added at the bottom, its kind above its three-line text
//!   and Add below them. A gesture is sent as soon as it is made; what the file then says is the page's next model.
//! - Escape drops a change being typed, and otherwise closes the list; so does a press outside it. While it is open,
//!   the page's keys are the list's.
//!
//! It knows no designer type: the kinds, their icons, the operations and their words are the definition's.

use std::collections::{BTreeMap, HashSet};

use adp_desl::hierarchy::definition::{ADD_NOTE_ICON, SurfaceDefinition};
use adp_desl::hierarchy::model::{Note, SurfaceNode, roles};
use eframe::egui::{self, Align2, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2, accesskit::Role, text::LayoutJob};

use super::HierarchyTab;
use super::layout::{HierarchyIndex, LINE_HEIGHT};
use crate::a11y;
use crate::icons;
use crate::table::view::Colours;

/// The icons of a note's controls; what each says is the definition's, as their tooltip.
const RESOLVE_ICON: &str = "mdi-check-circle-outline";
const REOPEN_ICON: &str = "mdi-restore";
const CHANGE_ICON: &str = "mdi-pencil-outline";
const REMOVE_ICON: &str = "mdi-delete-outline";
const CHEVRON_ICON: &str = "mdi-chevron-down";

/// The list's width, the gap between it and the icon it opened from and the window's edges, its padding, a note
/// control's size, the indent of a note's detail and the rows of its text fields (`.hs-note-list`).
const LIST_WIDTH: f32 = 340.0;
const GAP: f32 = 4.0;
const PADDING: f32 = 8.0;
/// The width of the list's border.
const BORDER: f32 = 1.0;
const ACTION_SIZE: f32 = 22.0;
const DETAIL_INDENT: f32 = 22.0;
const TEXT_ROWS: usize = 3;
const DETAIL_ROWS: usize = 2;

/// The icon a node shows for its open notes, and what its tooltip says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteMark {
    pub icon: String,
    pub count: usize,
    pub label: String,
}

/// The icon of a note's kind, or the one for several where its kind has none.
pub fn icon_of<'d>(definition: &'d SurfaceDefinition, kind: &str) -> &'d str {
    definition
        .note_icons
        .iter()
        .find(|(k, _)| k == kind)
        .map_or(definition.several_notes_icon.as_str(), |(_, icon)| icon.as_str())
}

/// The mark of the open notes about `ids`: the kind's own icon for one, the declared icon for several, nothing for none.
/// A resolved note is not counted; it has been answered.
pub fn mark_of(notes: &[Note], ids: &HashSet<String>, definition: &SurfaceDefinition) -> Option<NoteMark> {
    let open: Vec<&Note> = notes
        .iter()
        .filter(|n| !n.resolved && ids.contains(&n.about_id))
        .collect();
    match open.as_slice() {
        [] => None,
        [only] => Some(NoteMark {
            icon: icon_of(definition, &only.kind).to_owned(),
            count: 1,
            label: format!("{}: {}", only.kind, only.text),
        }),
        several => Some(NoteMark {
            icon: definition.several_notes_icon.clone(),
            count: several.len(),
            label: format!("{} open {}", several.len(), definition.notes.words.list.to_lowercase()),
        }),
    }
}

/// The nodes whose notes a box stands for: its own, and with a collapsed level, everything inside it.
pub fn about_ids(index: &HierarchyIndex, node: &SurfaceNode, collapsed: bool) -> HashSet<String> {
    let mut ids = HashSet::from([node.id.clone()]);
    if node.role == roles::LEVEL && collapsed {
        let mut pending = vec![node.id.as_str()];
        while let Some(id) = pending.pop() {
            for child in index.children_of(id) {
                ids.insert(child.id.clone());
                pending.push(&child.id);
            }
        }
    }
    ids
}

/// What the icon at a box's top right is: the mark of its open notes, or the way to add the first.
pub struct NoteControl {
    pub icon: String,
    pub label: String,
    pub count: usize,
    pub mark: bool,
}

impl NoteControl {
    /// The icon of `node`'s box, or none when it has nothing to show and nothing can be added.
    pub fn of(tab: &HierarchyTab, index: &HierarchyIndex, node: &SurfaceNode, collapsed: bool) -> Option<NoteControl> {
        let ids = about_ids(index, node, collapsed);
        match mark_of(&tab.model.notes, &ids, &tab.definition) {
            Some(mark) => Some(NoteControl {
                icon: mark.icon,
                label: mark.label,
                count: mark.count,
                mark: true,
            }),
            None if !tab.read_only() => Some(NoteControl {
                icon: ADD_NOTE_ICON.into(),
                label: format!("{} on {}", tab.definition.notes.words.add, node.number),
                count: 0,
                mark: false,
            }),
            None => None,
        }
    }
}

/// A note as it is being written: its kind, its text and the lines below it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Draft {
    pub kind: String,
    pub text: String,
    pub detail: String,
}

/// The list open on one box.
#[derive(Debug, Clone, PartialEq)]
pub struct NoteList {
    /// The node whose box it opened from, which an add is about.
    pub node_id: String,
    /// The new note at its bottom.
    pub adding: Draft,
    /// The note being changed, by its id, and what it is changed to.
    pub changing: Option<(String, Draft)>,
    /// Whether the change's text is to take the keyboard.
    pub focus_change: bool,
    /// The list's height as the last frame drew it, which it is placed by.
    pub height: f32,
}

impl NoteList {
    pub fn on(node_id: &str, definition: &SurfaceDefinition) -> NoteList {
        NoteList {
            node_id: node_id.to_owned(),
            adding: Draft {
                kind: definition
                    .note_icons
                    .first()
                    .map(|(k, _)| k.clone())
                    .unwrap_or_default(),
                ..Draft::default()
            },
            changing: None,
            focus_change: false,
            height: 0.0,
        }
    }
}

/// What a frame of the list asked for.
enum Asked {
    Gesture(String, String, BTreeMap<String, String>),
    Change(String, Draft),
    DropChange,
    Close,
}

/// Draws the list open on the tab, under `anchor`, the icon it opened from; closes it when its box is not drawn.
pub fn show(ui: &mut egui::Ui, tab: &mut HierarchyTab, anchor: Option<Rect>, colours: &Colours, font: &FontId) {
    let Some(list) = tab.notes.as_ref() else {
        return;
    };
    let (Some(anchor), Some(node)) = (anchor, tab.model.nodes.iter().find(|n| n.id == list.node_id)) else {
        tab.notes = None;
        return;
    };
    let ctx = ui.ctx().clone();
    let popup_open = egui::Popup::is_any_open(&ctx);
    let mut asked = Vec::new();
    // Escape drops a change being typed, and otherwise closes the list, wherever the focus is; a kind's choice open
    // closes first, by itself.
    if !popup_open && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        asked.push(if list.changing.is_some() {
            Asked::DropChange
        } else {
            Asked::Close
        });
    }

    let index = HierarchyIndex::of(&tab.model.nodes);
    let ids = about_ids(&index, node, tab.is_collapsed(node));
    let notes: Vec<Note> = tab
        .model
        .notes
        .iter()
        .filter(|n| ids.contains(&n.about_id))
        .cloned()
        .collect();
    let screen = ctx.content_rect();
    let width = LIST_WIDTH.min(screen.width() - 2.0 * GAP);
    // Under the box's icons, its right edge on theirs, kept in the window; above them where the window has no room.
    let left = (anchor.right() - width).min(screen.right() - width - GAP).max(GAP);
    let below = anchor.bottom() + GAP;
    let top = if below + list.height > screen.bottom() - GAP && anchor.top() - GAP - list.height >= GAP {
        anchor.top() - GAP - list.height
    } else {
        below
    };
    let number = node.number.clone();
    let node_id = node.id.clone();
    let editable = !tab.read_only();
    let mut drawn = Drawn {
        definition: &tab.definition,
        colours,
        font,
        list: tab.notes.as_mut().expect("the list is open"),
        asked: &mut asked,
        id: tab.id.with("notes"),
    };
    let area = egui::Area::new(drawn.id)
        .order(egui::Order::Foreground)
        .fixed_pos(Pos2::new(left, top))
        .show(&ctx, |ui| {
            egui::Frame::popup(ui.style())
                .fill(colours.surface)
                .stroke(Stroke::new(BORDER, colours.border))
                .inner_margin(PADDING)
                .show(ui, |ui| {
                    ui.set_width(width - 2.0 * (PADDING + BORDER));
                    ui.spacing_mut().item_spacing = Vec2::new(4.0, 2.0);
                    for note in &notes {
                        drawn.note(ui, note, editable);
                    }
                    if editable {
                        drawn.new_note(ui, &number, &node_id);
                    }
                })
                .response
        });
    let frame = area.inner;
    a11y::describe(
        &frame,
        Role::Dialog,
        &format!("{} of {}", tab.definition.notes.words.list, number),
    );
    if let Some(list) = tab.notes.as_mut() {
        list.height = area.response.rect.height();
        list.focus_change = false;
    }
    // A press outside it closes it, but not one on the icon it opened from, which closes it by itself, nor one in a
    // kind's choice, which stands outside it.
    let pressed = ctx.input(|i| i.pointer.any_pressed().then(|| i.pointer.interact_pos()).flatten());
    if let Some(at) = pressed
        && !popup_open
        && !area.response.rect.contains(at)
        && !anchor.contains(at)
    {
        asked.push(Asked::Close);
    }

    for ask in asked {
        match ask {
            Asked::Gesture(operation, target, params) => tab.gesture(&operation, &target, params),
            Asked::Change(id, draft) => {
                if let Some(list) = tab.notes.as_mut() {
                    list.changing = Some((id, draft));
                    list.focus_change = true;
                }
            }
            Asked::DropChange => {
                if let Some(list) = tab.notes.as_mut() {
                    list.changing = None;
                }
            }
            Asked::Close => tab.notes = None,
        }
    }
}

/// One frame of the list: what it draws with, and what it is asked.
struct Drawn<'a> {
    definition: &'a SurfaceDefinition,
    colours: &'a Colours,
    font: &'a FontId,
    list: &'a mut NoteList,
    asked: &'a mut Vec<Asked>,
    id: egui::Id,
}

impl Drawn<'_> {
    /// One note with its controls: beside it, one above the other, when it takes several lines, and under it, side by
    /// side, when it takes one, both at its right. The note being changed is its kind, text and detail to type.
    fn note(&mut self, ui: &mut egui::Ui, note: &Note, editable: bool) {
        if self.list.changing.as_ref().is_some_and(|(id, _)| *id == note.id) {
            self.changing(ui, note);
            separator(ui, self.colours);
            return;
        }
        let words = &self.definition.notes.words;
        let width = ui.available_width();
        let text_width = width - DETAIL_INDENT - ACTION_SIZE - 4.0;
        let tall = !note.detail.is_empty()
            || ui
                .painter()
                .layout(note.text.clone(), self.font.clone(), self.colours.text, text_width)
                .rows
                .len()
                > 1;
        let actions: Vec<(&str, &str, Asked)> = if editable {
            let o = &self.definition.notes.operations;
            vec![
                if note.resolved {
                    (REOPEN_ICON, words.reopen.as_str(), resolve(&o.resolve, note))
                } else {
                    (RESOLVE_ICON, words.resolve.as_str(), resolve(&o.resolve, note))
                },
                (
                    CHANGE_ICON,
                    words.change.as_str(),
                    Asked::Change(
                        note.id.clone(),
                        Draft {
                            kind: note.kind.clone(),
                            text: note.text.clone(),
                            detail: note.detail.clone(),
                        },
                    ),
                ),
                (
                    REMOVE_ICON,
                    words.remove.as_str(),
                    Asked::Gesture(o.remove.clone(), note.id.clone(), BTreeMap::new()),
                ),
            ]
        } else {
            Vec::new()
        };
        let row = ui.scope(|ui| {
            if tall && !actions.is_empty() {
                ui.horizontal_top(|ui| {
                    ui.allocate_ui(Vec2::new(width - ACTION_SIZE - 4.0, 0.0), |ui| self.body(ui, note));
                    ui.vertical(|ui| self.actions(ui, actions));
                });
            } else {
                self.body(ui, note);
                if !actions.is_empty() {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        // Laid out from the right: the last first.
                        let mut actions = actions;
                        actions.reverse();
                        self.actions(ui, actions);
                    });
                }
            }
        });
        let response = ui.interact(row.response.rect, self.id.with(("note", &note.id)), Sense::hover());
        a11y::describe(
            &response,
            Role::ListItem,
            &format!("{} {}: {}", note.about, note.kind, note.text),
        );
        if note.resolved {
            a11y::describe_state(&response, "resolved");
        }
        separator(ui, self.colours);
    }

    /// A note's kind, the number it is about and its text, and its detail under them.
    fn body(&self, ui: &mut egui::Ui, note: &Note) {
        let muted = self.colours.muted;
        let colour = if note.resolved { muted } else { self.colours.text };
        ui.horizontal_top(|ui| {
            let icon = ui.label(
                RichText::new(icons::text(icon_of(self.definition, &note.kind)))
                    .font(icons::font(self.font.size))
                    .color(self.colours.warning),
            );
            a11y::describe(&icon, Role::Image, &note.kind);
            icon.on_hover_text(&note.kind);
            ui.label(RichText::new(&note.about).font(self.font.clone()).color(muted));
            let mut text = RichText::new(&note.text).font(self.font.clone()).color(colour);
            if note.resolved {
                text = text.strikethrough();
            }
            ui.add(egui::Label::new(text).wrap());
        });
        if !note.detail.is_empty() {
            ui.horizontal_top(|ui| {
                ui.add_space(DETAIL_INDENT);
                ui.add(egui::Label::new(RichText::new(&note.detail).font(self.font.clone()).color(colour)).wrap());
            });
        }
    }

    fn actions(&mut self, ui: &mut egui::Ui, actions: Vec<(&str, &str, Asked)>) {
        for (icon, label, ask) in actions {
            let (rect, response) = ui.allocate_exact_size(Vec2::splat(ACTION_SIZE), Sense::click());
            a11y::describe(&response, Role::Button, label);
            if response.hovered() {
                ui.painter().rect_filled(rect, 3.0, self.colours.raised);
            }
            let colour = if response.hovered() {
                self.colours.text
            } else {
                self.colours.muted
            };
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                icons::text(icon),
                icons::font(self.font.size),
                colour,
            );
            if response.on_hover_text(label).clicked() {
                self.asked.push(ask);
            }
        }
    }

    /// The note being changed: its kind, its one-line text, its detail, then Cancel and the change.
    fn changing(&mut self, ui: &mut egui::Ui, note: &Note) {
        let Some((id, mut draft)) = self.list.changing.clone() else {
            return;
        };
        let words = &self.definition.notes.words;
        self.kind_choice(ui, &mut draft.kind, &format!("Kind of {}", note.about), ("change", &id));
        let text_id = self.id.with(("change-text", &id));
        if self.list.focus_change {
            ui.memory_mut(|m| m.request_focus(text_id));
        }
        let save = self.text(
            ui,
            &mut draft.text,
            text_id,
            &format!("Text about {}", note.about),
            TEXT_ROWS,
            "",
        );
        let detail = egui::TextEdit::multiline(&mut draft.detail)
            .id(self.id.with(("change-detail", &id)))
            .desired_rows(DETAIL_ROWS)
            .desired_width(f32::INFINITY)
            .font(self.font.clone());
        let response = ui.add(detail);
        a11y::describe(
            &response,
            Role::MultilineTextInput,
            &format!("Detail about {}", note.about),
        );
        let mut cancel = false;
        let mut change = save;
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            change |= ui
                .add_enabled(!draft.text.trim().is_empty(), egui::Button::new(&words.change))
                .clicked();
            cancel = ui.button("Cancel").clicked();
        });
        if cancel {
            self.asked.push(Asked::DropChange);
        } else if change && !draft.text.trim().is_empty() {
            let p = &self.definition.notes.params;
            let params = BTreeMap::from([
                (p.kind.clone(), draft.kind.clone()),
                (p.text.clone(), draft.text.clone()),
                (p.detail.clone(), draft.detail.clone()),
            ]);
            self.asked.push(Asked::Gesture(
                self.definition.notes.operations.change.clone(),
                id,
                params,
            ));
            self.asked.push(Asked::DropChange);
        } else {
            self.list.changing = Some((id, draft));
        }
    }

    /// The new note at the list's bottom: its kind, its text over three lines, and Add below them on the right.
    fn new_note(&mut self, ui: &mut egui::Ui, number: &str, node_id: &str) {
        ui.add_space(PADDING - 2.0);
        let mut draft = self.list.adding.clone();
        self.kind_choice(
            ui,
            &mut draft.kind,
            &format!("Kind of the new one about {number}"),
            ("new", ""),
        );
        let words = &self.definition.notes.words;
        let label = format!("{} about {number}", words.add);
        let mut add = self.text(
            ui,
            &mut draft.text,
            self.id.with("new-text"),
            &label,
            TEXT_ROWS,
            &words.add,
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            add |= ui
                .add_enabled(!draft.text.trim().is_empty(), egui::Button::new("Add"))
                .clicked();
        });
        if add && !draft.text.trim().is_empty() {
            let p = &self.definition.notes.params;
            let params = BTreeMap::from([
                (p.kind.clone(), draft.kind.clone()),
                (p.text.clone(), draft.text.clone()),
            ]);
            self.asked.push(Asked::Gesture(
                self.definition.notes.operations.add.clone(),
                node_id.to_owned(),
                params,
            ));
            draft.text.clear();
        }
        self.list.adding = draft;
    }

    /// A note's one-line text, over `rows` lines: Enter keeps it, and a line break typed or pasted is taken out.
    /// Whether Enter was pressed in it.
    fn text(&self, ui: &mut egui::Ui, text: &mut String, id: egui::Id, label: &str, rows: usize, hint: &str) -> bool {
        let enter =
            ui.memory(|m| m.has_focus(id)) && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        let edit = egui::TextEdit::multiline(text)
            .id(id)
            .desired_rows(rows)
            .min_size(Vec2::new(0.0, rows as f32 * LINE_HEIGHT))
            .desired_width(f32::INFINITY)
            .hint_text(hint)
            .font(self.font.clone());
        let response = ui.add(edit);
        if text.contains(['\r', '\n']) {
            *text = text.replace("\r\n", " ").replace(['\n', '\r'], " ");
        }
        a11y::describe(&response, Role::MultilineTextInput, label);
        enter
    }

    /// A note's kind, each with its icon to its left: a button that opens the kinds.
    fn kind_choice(&self, ui: &mut egui::Ui, value: &mut String, label: &str, salt: (&str, &str)) {
        let shown = self.kind_job(ui, value, true);
        let choice = egui::ComboBox::from_id_salt(self.id.with(salt))
            .selected_text(shown)
            .width(140.0)
            .icon(|_, _, _, _| {})
            .show_ui(ui, |ui| {
                for (kind, _) in &self.definition.note_icons {
                    let job = self.kind_job(ui, kind, false);
                    let option = ui.selectable_label(kind == value, job);
                    a11y::describe(&option, Role::ListBoxOption, kind);
                    if option.clicked() {
                        value.clone_from(kind);
                    }
                }
            });
        a11y::describe(&choice.response, Role::ComboBox, label);
    }

    /// A kind's icon and name, with the chevron of its choice when it is the choice's button.
    fn kind_job(&self, ui: &egui::Ui, kind: &str, chevron: bool) -> LayoutJob {
        let mut job = LayoutJob::default();
        let icon = |colour| egui::TextFormat::simple(icons::font(self.font.size), colour);
        job.append(
            &icons::text(icon_of(self.definition, kind)),
            0.0,
            icon(self.colours.warning),
        );
        job.append(
            kind,
            6.0,
            egui::TextFormat::simple(self.font.clone(), ui.visuals().text_color()),
        );
        if chevron {
            job.append(&icons::text(CHEVRON_ICON), 8.0, icon(self.colours.muted));
        }
        job
    }
}

fn resolve(operation: &str, note: &Note) -> Asked {
    Asked::Gesture(operation.to_owned(), note.id.clone(), BTreeMap::new())
}

fn separator(ui: &mut egui::Ui, colours: &Colours) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 5.0), Sense::hover());
    ui.painter()
        .hline(rect.x_range(), rect.center().y, Stroke::new(1.0, colours.border));
}
