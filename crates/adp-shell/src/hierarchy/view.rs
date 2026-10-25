//! Drawing the hierarchy tab with egui (etalii.adp spec 036, T028, research R8): the trail, the read-only reason, the
//! findings and the page, ported from the standalone host's `HierarchySurfaceTrail.tsx`, `FindingList.tsx`,
//! `HierarchySurface.tsx` (`NodeView`), `HierarchySurfaceBox.tsx` and `CollapseToggle.tsx` at a7fdf03, in the sizes of
//! `hierarchySurface.css` and VS Code's colours.
//!
//! - A level's own box is as high as what stands to its right: its entries, then its sub-levels, each a box sharing its
//!   borders with its neighbours. As many levels as fit the page's width are shown; a level whose deeper levels do not
//!   fit carries a mark that shows them.
//! - A box's first line is the collapse control, the number and the title. The title wraps under them, its next lines
//!   as far from the box's left as the labels and the description (the hanging indent of research R8): the first line
//!   is laid out at the box's 6 point edge after the control and the number, the rest again at the 12 point edge.
//! - The box's controls are one list: in a row at the end of the first line when the whole line fits beside them, in a
//!   column at the box's right otherwise, from the top down.
//! - On a document that can change (spec 036, T038, [`super::editing`]): the controls that add at the end of every
//!   level's content and of the page, a grip before the collapse control a box is dragged by, the place of the delete
//!   at the box's bottom right, a title or a description typed in place where it stood and as high as it, and the
//!   question a delete asks first. A box keeps its height whether it is selected or typed in.
//! - The filter bar above the findings ([`super::narrowing`], T048), which leaves of the page what its query selects
//!   and orders each level's entries by its sort.
//! - Everything a frame is asked is collected while the page is drawn and carried out after it.

use adp_desl::hierarchy::definition::{
    ADD_DESCRIPTION_ICON, DELETE_ICON, FOCUS_ICON, GRIP_ICON, HIDDEN_LEVELS_ICON, LabelLook,
};
use adp_desl::hierarchy::model::{SurfaceNode, roles};
use adp_theme::Theme;
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2,
    accesskit::Role, text::LayoutJob,
};

use super::editing::{self, Attribute, DropHint, DropKind, OpenField};
use super::layout::{
    BOX_SPACING, ENTRY_MIN_WIDTH, GRIP_WIDTH, HEADER_GAP, HierarchyIndex, ICON_SIZE, IconsLayout, LINE_HEIGHT,
    PAGE_MARGIN, header_width, icons_layout,
};
use super::narrowing::Narrowing;
use super::notes::NoteControl;
use super::{HierarchyTab, is_line_id};
use crate::a11y;
use crate::icons;
use crate::table::view::Colours;
use crate::theme::Palette;

/// The size of the count of several open notes beside their icon (`.hs-note-count`).
const NOTE_COUNT_FONT: f32 = 10.0;
/// The space between the labels' row and the title, and between the tags in it.
const TAG_GAP: f32 = 4.0;
/// A tag's height, its side padding and its font size (`.hs-look .table-option-tag`).
const TAG_HEIGHT: f32 = 18.0;
const TAG_PADDING: f32 = 7.0;
const TAG_FONT: f32 = 11.0;
/// The space above a description and below it.
const DESCRIPTION_TOP: f32 = 4.0;
const DESCRIPTION_BOTTOM: f32 = 6.0;
/// An add control's size, the space between two, the space around a level's row of them and around the page's, and
/// the size of its icon (`.hs-add`, `.hs-add-button`).
const ADD_SIZE: f32 = 22.0;
const ADD_GAP: f32 = 8.0;
const ADD_PADDING: Vec2 = Vec2::new(6.0, 2.0);
const PAGE_ADD_PADDING: f32 = 4.0;
const ADD_ICON: f32 = 16.0;
/// The left padding of the place a dragged node would be dropped at, which shows its number and title (`.hs-drop-slot`).
const SLOT_PADDING: f32 = 40.0;

/// What a frame of the page asked for, carried out once the page is drawn.
enum Act {
    Toggle(String),
    Select(String),
    Focus(String),
    ShowHidden(String),
    Reveal(String),
    /// An add control: the operation, and the level the node goes into, empty for the top.
    Add(String, String),
    Open(String, Attribute),
    Commit,
    Cancel,
    /// Delete on a node; whether to ask first whatever concerns it.
    Delete(String, bool),
    StartDrag(String),
    EndDrag,
    /// The icon at a box's top right: its list of notes opened or closed.
    ToggleNotes(String),
    /// The conflict bar's two sides: drop what was typed, or keep it.
    ShowFile,
    KeepText,
}

/// Draws the tab: the trail, the read-only reason, the conflict bar, a refusal, the findings and the page.
pub fn show(ui: &mut egui::Ui, tab: &mut HierarchyTab, theme: &Theme, palette: &Palette) {
    tab.drain();
    let colours = Colours::of(theme, palette);
    let font = FontId::proportional(theme.sizes.workbench_font);
    let mut acts = Vec::new();
    ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
    keys(ui, tab, &mut acts);

    trail(ui, tab, &colours, &font, &mut acts);
    if tab.read_only() {
        bar(ui, &colours, |ui| {
            ui.label(
                RichText::new(icons::text("mdi-lock-outline"))
                    .font(icons::font(font.size))
                    .color(colours.muted),
            );
            ui.label(RichText::new(&tab.model.read_only_reason).color(colours.muted));
        });
    }
    if tab.conflict && tab.field.is_some() {
        conflict_bar(ui, &colours, &mut acts);
    }
    if !tab.refusal.is_empty() {
        bar(ui, &colours, |ui| {
            ui.label(RichText::new(&tab.refusal).color(colours.danger));
        });
    }
    if let Some(filters) = tab.definition.filters.clone() {
        bar(ui, &colours, |ui| {
            super::narrowing::filter_bar(ui, tab, &filters, &colours, &font);
        });
    }
    findings(ui, tab, &colours, &font, &mut acts);
    let drawn = page(ui, tab, &colours, &font, &mut acts);
    if tab.dragging.is_some() {
        tab.hint = drop_hint(ui, tab, &drawn);
    }

    for act in acts {
        match act {
            Act::Toggle(id) => tab.toggle(&id),
            Act::Select(id) => tab.select(&id),
            Act::Focus(id) => tab.set_focus(&id),
            Act::ShowHidden(id) => tab.show_hidden(&id),
            Act::Reveal(id) => tab.reveal(&id),
            Act::Add(operation, target) => tab.add(&operation, &target),
            Act::Open(id, attribute) => tab.open_field(&id, attribute),
            Act::Commit => tab.commit_field(),
            Act::Cancel => tab.cancel_field(),
            Act::ShowFile => tab.show_file(),
            Act::ToggleNotes(id) => tab.toggle_notes(&id),
            Act::KeepText => tab.keep_text(),
            Act::Delete(id, ask) => tab.delete(&id, ask),
            Act::StartDrag(id) => {
                tab.dragging = tab.model.nodes.iter().find(|n| n.id == id).cloned();
                tab.hint = None;
            }
            Act::EndDrag => {
                let hint = drop_hint(ui, tab, &drawn);
                tab.dragging = None;
                tab.hint = None;
                if let Some(hint) = hint {
                    tab.move_to(&hint.dragged, hint.drop);
                }
            }
        }
    }
    let anchor = tab
        .notes
        .as_ref()
        .and_then(|n| drawn.note_icons.iter().find(|(id, _)| *id == n.node_id))
        .map(|(_, rect)| *rect);
    super::notes::show(ui, tab, anchor, &colours, &font);
    confirm_delete(ui, tab);
}

/// The page's own keys, while no field and no other widget has the keyboard: Delete on the selected node, and a key
/// the definition sends with Ctrl or Cmd held, Shift too or not, which runs the selected node's action of that key.
fn keys(ui: &mut egui::Ui, tab: &mut HierarchyTab, acts: &mut Vec<Act>) {
    let free = ui.memory(|m| m.focused().is_none_or(|f| f == tab.id()));
    // An open list of notes keeps the keys for itself.
    let busy = tab.field.is_some() || tab.deleting.is_some() || tab.notes.is_some();
    if !free || busy || tab.selected.is_empty() || tab.read_only() {
        return;
    }
    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Delete)) {
        acts.push(Act::Delete(tab.selected.clone(), false));
    }
    for name in tab.definition.shortcut_keys.clone() {
        let Some(key) = egui::Key::from_name(&name) else {
            continue;
        };
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, key)) {
            tab.shortcut(&name, true);
        } else if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, key)) {
            tab.shortcut(&name, false);
        }
    }
}

/// The question a delete asks first, cancel the default (`ConfirmDelete`).
fn confirm_delete(ui: &mut egui::Ui, tab: &mut HierarchyTab) {
    let Some(deleting) = &tab.deleting else {
        return;
    };
    let mut answer = None;
    let id = tab.id().with("delete");
    egui::Modal::new(id)
        .area(egui::Modal::default_area(id).accessible_name("Delete"))
        .show(ui.ctx(), |ui| {
            ui.set_width(420.0);
            ui.heading("Delete");
            ui.label(deleting.question.as_str());
            ui.horizontal(|ui| {
                if ui.button("Delete").clicked() {
                    answer = Some(true);
                }
                if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    answer = Some(false);
                }
            });
        });
    if let Some(delete) = answer {
        tab.answer_delete(delete);
    }
}

/// Where the page's boxes stood this frame, and the place a dragged node would be dropped at.
#[derive(Default)]
struct Drawn {
    boxes: Vec<(String, Rect)>,
    slot: Option<Rect>,
    /// The icon at each box's top right, which its list of notes opens under.
    note_icons: Vec<(String, Rect)>,
}

/// Where the node being dragged would go with the pointer where it is: over the place the drop is shown at, the drop
/// stays, since that place moved the boxes away from under the pointer; over a box, as [`editing::drop_of`] says for
/// its half; elsewhere nowhere.
fn drop_hint(ui: &egui::Ui, tab: &HierarchyTab, drawn: &Drawn) -> Option<DropHint> {
    let dragged = tab.dragging.as_ref()?;
    let pointer = ui.ctx().pointer_latest_pos()?;
    if drawn.slot.is_some_and(|slot| slot.contains(pointer)) {
        return tab.hint.clone();
    }
    let (id, rect) = drawn.boxes.iter().find(|(_, rect)| rect.contains(pointer))?;
    let index = HierarchyIndex::of(&tab.model.nodes);
    let over = index.by_id.get(id.as_str()).copied()?;
    let lower_half = pointer.y >= rect.center().y;
    editing::drop_of(&index, dragged, over, lower_half).map(|drop| DropHint::of(dragged, over, drop))
}

/// A line above the page, padded as the trail is and with a border under it.
fn bar(ui: &mut egui::Ui, colours: &Colours, content: impl FnOnce(&mut egui::Ui)) {
    let response = egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(PAGE_MARGIN as i8, BOX_SPACING as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(content);
        });
    let rect = response.response.rect;
    ui.painter()
        .hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, colours.border));
}

/// The bar over the page when another program changed the file while a title or a description was being typed (the
/// standalone host's `ConflictBar.tsx` at a7fdf03), in the shape of the text editors' conflict bar. It offers both sides
/// and never drops the text by itself; the field does not commit when its focus leaves for the bar.
fn conflict_bar(ui: &mut egui::Ui, colours: &Colours, acts: &mut Vec<Act>) {
    bar(ui, colours, |ui| {
        let sentence = "This file was changed outside the designer.";
        let label = ui.label(RichText::new(sentence).color(colours.text));
        a11y::describe(&label, Role::Alert, sentence);
        if ui.button("Show the file").clicked() {
            acts.push(Act::ShowFile);
        }
        if ui.button("Keep my text").clicked() {
            acts.push(Act::KeepText);
        }
    });
}

/// The trail above the page, always shown: the document's title, then the levels down to the selected node, or to the
/// focused level while nothing is selected. Each is a link that makes it the page's left edge, the title the whole
/// page, except the one that already is.
fn trail(ui: &mut egui::Ui, tab: &HierarchyTab, colours: &Colours, font: &FontId, acts: &mut Vec<Act>) {
    let current = tab.trail_current();
    let title = if tab.model.title.is_empty() {
        "Document".to_owned()
    } else {
        tab.model.title.clone()
    };
    let path: Vec<(String, String)> = tab
        .trail()
        .iter()
        .map(|n| (n.id.clone(), format!("{} {}", n.number, n.title)))
        .collect();
    bar(ui, colours, |ui| {
        trail_step(ui, &title, current == Some(""), "", colours, acts);
        for (id, text) in &path {
            ui.label(
                RichText::new(icons::text("mdi-chevron-right"))
                    .font(icons::font(font.size))
                    .color(colours.muted),
            );
            trail_step(ui, text, current == Some(id.as_str()), id, colours, acts);
        }
    });
}

fn trail_step(ui: &mut egui::Ui, text: &str, current: bool, id: &str, colours: &Colours, acts: &mut Vec<Act>) {
    if current {
        ui.label(RichText::new(text).strong().color(colours.text));
    } else if ui.link(RichText::new(text).color(colours.text)).clicked() {
        acts.push(Act::Focus(id.to_owned()));
    }
}

/// What reading the document reported, above the page: each finding at its line, and a link to the node it stands at
/// when it names one.
fn findings(ui: &mut egui::Ui, tab: &HierarchyTab, colours: &Colours, font: &FontId, acts: &mut Vec<Act>) {
    if tab.model.findings.is_empty() {
        return;
    }
    let response = egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(PAGE_MARGIN as i8, 4))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            for finding in &tab.model.findings {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = BOX_SPACING;
                    let (icon, colour) = match finding.severity.as_str() {
                        "error" => ("mdi-alert-circle-outline", colours.danger),
                        "warning" => ("mdi-alert-outline", colours.warning),
                        _ => ("mdi-information-outline", colours.muted),
                    };
                    let mark = ui.label(
                        RichText::new(icons::text(icon))
                            .font(icons::font(font.size))
                            .color(colour),
                    );
                    a11y::describe(&mark, Role::Image, &finding.severity);
                    if finding.node_id.is_empty() {
                        ui.label(RichText::new(&finding.message).color(colours.text));
                    } else if ui.link(RichText::new(&finding.message).color(colours.text)).clicked() {
                        acts.push(Act::Reveal(finding.node_id.clone()));
                    }
                    if finding.line > 0 {
                        ui.label(RichText::new(format!("line {}", finding.line)).color(colours.muted));
                    }
                });
                ui.add_space(2.0);
            }
        });
    let rect = response.response.rect;
    ui.painter()
        .hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, colours.border));
}

/// The page: one area that scrolls, the boxes laid out from its top left, and the add control at its end.
fn page(ui: &mut egui::Ui, tab: &mut HierarchyTab, colours: &Colours, font: &FontId, acts: &mut Vec<Act>) -> Drawn {
    let mut measured = tab.width;
    // The field's text is typed into this frame's copy, and kept once the page is drawn.
    let mut field = tab.field.clone();
    let mut drawn = Drawn::default();
    egui::ScrollArea::both()
        .id_salt(tab.id().with("page"))
        .auto_shrink(false)
        .show(ui, |ui| {
            measured = (ui.available_width() - 2.0 * PAGE_MARGIN).max(0.0);
            let index = HierarchyIndex::of(&tab.model.nodes);
            let narrowing = Narrowing::of(&tab.model, &index, &tab.words());
            let mut page = Page {
                tab,
                index: &index,
                narrowing: &narrowing,
                colours,
                font,
                fit: tab.fit(),
                root_level: tab.root_level(),
                editable: !tab.read_only(),
                field: &mut field,
                acts,
                drawn: &mut drawn,
                right: 0.0,
            };
            let origin = ui.max_rect().min + Vec2::splat(PAGE_MARGIN);
            let width = measured.max(ENTRY_MIN_WIDTH);
            let mut y = origin.y;
            // The file no longer has the node the field is open on, so its box is gone; the text is kept above the
            // page instead, and committing it is for the file to refuse (R10).
            let orphan = page
                .field
                .as_ref()
                .is_some_and(|f| !index.by_id.contains_key(f.node_id.as_str()));
            if orphan {
                let inner = width - 2.0 * BOX_SPACING;
                let height = page.field_height(ui.painter(), inner);
                let place = Rect::from_min_size(
                    Pos2::new(origin.x + BOX_SPACING, y + BOX_SPACING),
                    Vec2::new(inner, height),
                );
                page.text_field(ui, place);
                y += height + 2.0 * BOX_SPACING;
            }
            let mut roots = tab.roots();
            roots.retain(|n| narrowing.shows(n));
            let hint = tab
                .hint
                .as_ref()
                .filter(|h| tab.focus.id.is_empty() && h.drop.into.is_empty());
            for shown in with_slot(&roots, hint) {
                let height = match shown {
                    Some(root) => page.node(ui, root, 0, Pos2::new(origin.x, y), width),
                    None => page.slot(ui, Pos2::new(origin.x, y), width),
                };
                y += height - 1.0;
            }
            if page.editable && tab.focus.id.is_empty() {
                let add = &tab.definition.operations.add_top_level;
                let at = Pos2::new(origin.x, y + 1.0 + PAGE_ADD_PADDING);
                page.adds(ui, at, &[add], "", "");
                y += 1.0 + 2.0 * PAGE_ADD_PADDING + ADD_SIZE;
            }
            let size = Vec2::new(
                page.right.max(origin.x + width) - ui.max_rect().min.x,
                y - ui.max_rect().min.y,
            ) + Vec2::splat(PAGE_MARGIN);
            ui.allocate_rect(Rect::from_min_size(ui.max_rect().min, size), Sense::hover());
        });
    tab.width = measured;
    tab.field = field;
    tab.field_opened = false;
    tab.field_refocus = false;
    drawn
}

/// `shown` with the place a drop shows, as none, where `hint` would put the dragged node among them.
fn with_slot<'n>(shown: &[&'n SurfaceNode], hint: Option<&DropHint>) -> Vec<Option<&'n SurfaceNode>> {
    let mut all: Vec<Option<&SurfaceNode>> = shown.iter().map(|n| Some(*n)).collect();
    if let Some(at) = hint.and_then(|h| editing::slot_index(shown, h)) {
        all.insert(at.min(all.len()), None);
    }
    all
}

/// One frame's drawing of the page.
struct Page<'a, 'm> {
    tab: &'a HierarchyTab,
    index: &'a HierarchyIndex<'m>,
    /// What the filter bar's query leaves of the page, and the order of a level's entries.
    narrowing: &'a Narrowing,
    colours: &'a Colours,
    font: &'a FontId,
    fit: usize,
    root_level: u32,
    /// Whether the document can change, which offers the controls that change it.
    editable: bool,
    field: &'a mut Option<OpenField>,
    acts: &'a mut Vec<Act>,
    drawn: &'a mut Drawn,
    /// The rightmost edge drawn, for the page's width.
    right: f32,
}

impl Page<'_, '_> {
    /// Draws `node` at `at`, `width` wide, with what stands to its right; its height.
    fn node(&mut self, ui: &mut egui::Ui, node: &SurfaceNode, depth: usize, at: Pos2, width: f32) -> f32 {
        if node.role != roles::LEVEL {
            return self.boxed(ui, node, Rect::from_min_size(at, Vec2::new(width, 0.0)), 0);
        }
        let collapsed = self.tab.is_collapsed(node);
        let level_width = self.tab.column_width(self.root_level + depth as u32);
        let shows_sub_levels = depth + 1 < self.fit;
        let hidden = if shows_sub_levels {
            0
        } else {
            self.index.levels_below(&node.id)
        };
        let children: Vec<&SurfaceNode> = self
            .index
            .children_of(&node.id)
            .iter()
            .filter(|c| self.narrowing.shows(c))
            .copied()
            .collect();
        let mut content: Vec<&SurfaceNode> = Vec::new();
        if !collapsed {
            let entries = children.iter().filter(|c| c.role != roles::LEVEL).copied().collect();
            content.extend(self.narrowing.order(entries));
            if shows_sub_levels {
                content.extend(children.iter().filter(|c| c.role == roles::LEVEL));
            }
        }
        // The content keeps the entries' least width, and the page scrolls sideways when the window is narrower.
        let content_width = (width - level_width + 1.0).max(ENTRY_MIN_WIDTH);
        let content_x = at.x + level_width - 1.0;
        let hint = self.tab.hint.as_ref().filter(|h| h.drop.into == node.id);
        let shown = with_slot(&content, hint);
        let mut y = at.y;
        for child in &shown {
            let height = match child {
                Some(child) => {
                    let child_depth = if child.role == roles::LEVEL { depth + 1 } else { depth };
                    self.node(ui, child, child_depth, Pos2::new(content_x, y), content_width)
                }
                None => self.slot(ui, Pos2::new(content_x, y), content_width),
            };
            y += height - 1.0;
        }
        let mut content_height = if shown.is_empty() { 0.0 } else { y - at.y + 1.0 };
        if self.editable && !collapsed {
            let operations = &self.tab.definition.operations;
            let top = if shown.is_empty() { at.y } else { y + 1.0 };
            self.adds(
                ui,
                Pos2::new(content_x, top) + ADD_PADDING,
                &[&operations.add_entry, &operations.add_level],
                &node.number,
                &node.id,
            );
            content_height = top - at.y + ADD_SIZE + 2.0 * ADD_PADDING.y;
        }
        let own = Rect::from_min_size(at, Vec2::new(level_width, content_height));
        self.boxed(ui, node, own, hidden).max(content_height)
    }

    /// The add controls at `at`, each its icon alone with its label as the tooltip, named for a screen reader with the
    /// number of the level the node goes into, `within`, and sent for the level `target`.
    fn adds(
        &mut self,
        ui: &mut egui::Ui,
        at: Pos2,
        adds: &[&adp_desl::hierarchy::definition::AddDefinition],
        within: &str,
        target: &str,
    ) {
        for (i, add) in adds.iter().enumerate() {
            let place = Rect::from_min_size(
                at + Vec2::new(i as f32 * (ADD_SIZE + ADD_GAP), 0.0),
                Vec2::splat(ADD_SIZE),
            );
            let name = if within.is_empty() {
                add.label.clone()
            } else {
                format!("{} in {within}", add.label)
            };
            let response = ui.interact(place, self.tab.id().with(("add", target, i)), Sense::click());
            a11y::describe(&response, Role::Button, &name);
            let colour = if response.hovered() {
                ui.painter().rect_filled(place, 3.0, self.colours.raised);
                self.colours.text
            } else {
                self.colours.muted
            };
            ui.painter().text(
                place.center(),
                Align2::CENTER_CENTER,
                icons::text(&add.icon),
                icons::font(ADD_ICON),
                colour,
            );
            if response.on_hover_text(&add.label).clicked() {
                self.acts.push(Act::Add(add.operation.clone(), target.to_owned()));
            }
        }
    }

    /// The place the dragged node would be dropped at, drawn as it would stand: its number and title in a dashed box,
    /// as wide as a level's column for a level; its height.
    fn slot(&mut self, ui: &mut egui::Ui, at: Pos2, width: f32) -> f32 {
        let Some(hint) = &self.tab.hint else {
            return 0.0;
        };
        let dragged = &hint.dragged;
        let width = if dragged.role == roles::LEVEL {
            self.tab.column_width(self.root_level)
        } else {
            width
        };
        let rect = Rect::from_min_size(at, Vec2::new(width, LINE_HEIGHT + 2.0 * BOX_SPACING));
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, self.colours.focus.gamma_multiply(0.1));
        painter.rect_stroke(rect, 0.0, Stroke::new(2.0, self.colours.focus), StrokeKind::Inside);
        painter.text(
            Pos2::new(rect.left() + SLOT_PADDING, rect.center().y),
            Align2::LEFT_CENTER,
            format!("{} {}", dragged.number, dragged.title),
            self.font.clone(),
            self.colours.text,
        );
        self.drawn.slot = Some(rect);
        rect.height()
    }

    /// Draws one node's own box in `rect`'s width, at least as high as `rect`; its height.
    fn boxed(&mut self, ui: &mut egui::Ui, node: &SurfaceNode, rect: Rect, hidden: usize) -> f32 {
        let tab = self.tab;
        let colours = self.colours;
        let level = node.role == roles::LEVEL;
        let collapsed = tab.is_collapsed(node);
        let selected = tab.selected == node.id;
        let painter = ui.painter().clone();
        let background = painter.add(egui::Shape::Noop);
        let id = tab.id().with(("box", &node.id));
        let typing = self
            .field
            .as_ref()
            .filter(|f| f.node_id == node.id)
            .map(|f| f.attribute);
        let deletable = self.editable && !is_line_id(&node.id);
        // The place of adding a description is kept on every box that could offer it, so selecting one moves nothing.
        let offers_description = self.editable && !collapsed && node.description.is_empty();
        // Each control's icon, its name, its tooltip, what it asks for, and whether it shows only where a person is
        // looking: the hidden levels' mark is always shown.
        let mut controls: Vec<(&'static str, String, String, Act, bool)> = Vec::new();
        // The notes' icon first, so that it stands at the corner whatever else a box offers.
        let notes = NoteControl::of(tab, self.index, node, collapsed);
        if let Some(control) = &notes {
            controls.push((
                "",
                control.label.clone(),
                control.label.clone(),
                Act::ToggleNotes(node.id.clone()),
                !control.mark,
            ));
        }
        if hidden > 0 {
            let levels = if hidden == 1 { "level" } else { "levels" };
            controls.push((
                HIDDEN_LEVELS_ICON,
                format!(
                    "{hidden} more {levels}: show the levels of {} {}",
                    node.number, node.title
                ),
                format!("{hidden} more {levels}"),
                Act::ShowHidden(node.id.clone()),
                false,
            ));
        }
        if level {
            let name = format!("Focus on {} {}", node.number, node.title);
            controls.push((FOCUS_ICON, name.clone(), name, Act::Focus(node.id.clone()), true));
        }
        if offers_description {
            let name = "Add a description".to_owned();
            controls.push((
                ADD_DESCRIPTION_ICON,
                name.clone(),
                name,
                Act::Open(node.id.clone(), Attribute::Description),
                true,
            ));
        }

        let number = painter.layout_no_wrap(node.number.clone(), self.font.clone(), colours.muted);
        let title_width = painter
            .layout_no_wrap(node.title.clone(), self.font.clone(), colours.text)
            .size()
            .x;
        let header = header_width(self.editable, number.size().x, title_width);
        let layout = icons_layout(Some(rect.width()), Some(header), controls.len());
        // The column at the right: the controls when they stand in a column, and the place of the delete.
        let side = match layout {
            IconsLayout::Column => !controls.is_empty() || deletable,
            IconsLayout::Row => deletable,
        };
        let left = rect.left() + BOX_SPACING;
        let text_right = rect.right() - BOX_SPACING - if side { ICON_SIZE + BOX_SPACING } else { 0.0 };
        let text_width = (text_right - left).max(ICON_SIZE);
        let top = rect.top() + BOX_SPACING;

        // The first line: the grip, the collapse control, the number and as much of the title as fits after them.
        let grip = Rect::from_min_size(Pos2::new(left, top), Vec2::new(GRIP_WIDTH, LINE_HEIGHT));
        let toggle_left = if self.editable { grip.right() + HEADER_GAP } else { left };
        let toggle = Rect::from_min_size(Pos2::new(toggle_left, top), Vec2::splat(ICON_SIZE));
        let number_at = Pos2::new(toggle.right() + HEADER_GAP, top + (LINE_HEIGHT - number.size().y) / 2.0);
        let number_right = number_at.x + number.size().x;
        let mut y = top + LINE_HEIGHT;
        let mut title_field = None;
        if typing == Some(Attribute::Title) {
            // Typed after the number, whose space it keeps, one line high until its text wraps.
            let x = number_right + HEADER_GAP;
            let width = (text_right - x).max(ICON_SIZE);
            let height = self.field_height(&painter, width);
            title_field = Some(Rect::from_min_size(Pos2::new(x, top), Vec2::new(width, height)));
            y = top + height;
        } else {
            let lead = toggle.right() - left + HEADER_GAP + (number_right - number_at.x) + HEADER_GAP;
            let first = painter.layout_job(self.title_job(&node.title, lead, text_width, Some(1)));
            let shown: usize = first.rows.first().map_or(0, |r| r.char_count_excluding_newline().0);
            let rest: String = node
                .title
                .chars()
                .skip(shown)
                .collect::<String>()
                .trim_start()
                .to_owned();
            painter.galley(Pos2::new(left, top), first, colours.text);
            if !rest.is_empty() {
                let more = painter.layout_job(self.title_job(&rest, 0.0, text_width - BOX_SPACING, None));
                let height = more.size().y;
                painter.galley(Pos2::new(left + BOX_SPACING, y), more, colours.text);
                y += height;
            }
        }
        painter.galley(number_at, number, colours.muted);
        let title_bottom = y;

        y = self.labels(&painter, node, Pos2::new(left + BOX_SPACING, y), text_right);
        let mut description_field = None;
        let mut description_area = None;
        if !collapsed {
            let at = Pos2::new(left + BOX_SPACING, y + DESCRIPTION_TOP);
            let width = text_width - BOX_SPACING;
            if typing == Some(Attribute::Description) {
                // Where the text stood and as high as it, ending short of the controls.
                let height = self.field_height(&painter, width);
                description_field = Some(Rect::from_min_size(at, Vec2::new(width, height)));
                y += DESCRIPTION_TOP + height + DESCRIPTION_BOTTOM - BOX_SPACING;
            } else if !node.description.is_empty() {
                let galley = painter.layout_job(self.text_job(&node.description, width));
                let height = galley.size().y;
                description_area = Some(Rect::from_min_size(at, Vec2::new(width, height)));
                painter.galley(at, galley, colours.text);
                y += DESCRIPTION_TOP + height + DESCRIPTION_BOTTOM - BOX_SPACING;
            }
        }
        // The delete stands under the controls, in a row or a column: a box of one line keeps the room for both.
        let side_height = match layout {
            IconsLayout::Column => controls.len() as f32 * ICON_SIZE,
            IconsLayout::Row if deletable && !controls.is_empty() => ICON_SIZE,
            IconsLayout::Row => 0.0,
        } + if deletable { ICON_SIZE } else { 0.0 };
        let height = (y + BOX_SPACING - rect.top())
            .max(side_height + 2.0 * BOX_SPACING)
            .max(rect.height());
        let outline = Rect::from_min_size(rect.min, Vec2::new(rect.width(), height));
        self.right = self.right.max(outline.right());
        self.drawn.boxes.push((node.id.clone(), outline));

        let drop_into = tab
            .hint
            .as_ref()
            .is_some_and(|h| h.node_id == node.id && h.kind == DropKind::Into);
        let fill = if level || drop_into {
            colours.raised
        } else {
            colours.surface
        };
        painter.set(background, egui::Shape::rect_filled(outline, CornerRadius::ZERO, fill));
        painter.rect_stroke(outline, 0.0, Stroke::new(1.0, colours.border), StrokeKind::Inside);
        if selected || drop_into {
            painter.rect_stroke(outline, 0.0, Stroke::new(2.0, colours.focus), StrokeKind::Inside);
        }
        let response = ui.interact(outline, id, Sense::click());
        a11y::tree_item(
            &response,
            &format!("{} {}", node.number, node.title),
            self.index.ancestors_of(&node.id).len() + 1,
            selected,
            Some(!collapsed),
        );
        if response.clicked() {
            self.acts.push(Act::Select(node.id.clone()));
        }
        if self.editable && response.double_clicked() {
            let pointer = ui.ctx().pointer_interact_pos().unwrap_or_default();
            if pointer.y < title_bottom {
                self.acts.push(Act::Open(node.id.clone(), Attribute::Title));
            } else if description_area.is_some_and(|area| area.contains(pointer)) {
                self.acts.push(Act::Open(node.id.clone(), Attribute::Description));
            }
        }

        // The collapse control, and the box's controls: shown under the pointer and on the selected box.
        // The pointer over the box, its controls included: over a control the box itself is not hovered.
        let hovered = ui.rect_contains_pointer(outline) || selected;
        let chevron = if collapsed {
            "mdi-chevron-right"
        } else {
            "mdi-chevron-down"
        };
        let verb = if collapsed { "Expand" } else { "Collapse" };
        let name = format!("{verb} {} {}", node.number, node.title);
        if self.icon_button(ui, toggle, chevron, &name, &name, id.with("toggle")) {
            self.acts.push(Act::Toggle(node.id.clone()));
        }
        if self.editable {
            self.grip(ui, node, grip, id.with("grip"));
        }
        for (i, (icon, label, tooltip, act, looking)) in controls.into_iter().enumerate() {
            let at = match layout {
                IconsLayout::Column => Pos2::new(rect.right() - BOX_SPACING - ICON_SIZE, top + i as f32 * ICON_SIZE),
                IconsLayout::Row => Pos2::new(rect.right() - BOX_SPACING - (i + 1) as f32 * ICON_SIZE, top),
            };
            let place = Rect::from_min_size(at, Vec2::splat(ICON_SIZE));
            if let (Act::ToggleNotes(_), Some(control)) = (&act, &notes) {
                // The mark always, the way to add where a person is looking or while its list is open.
                let open = tab.notes.as_ref().is_some_and(|n| n.node_id == node.id);
                self.drawn.note_icons.push((node.id.clone(), place));
                if (!looking || hovered || open) && self.note_icon(ui, place, control, id.with("notes"), open) {
                    self.acts.push(act);
                }
                continue;
            }
            // Focus where a person is looking, a description's add on the selected box alone, each kept in its place.
            let visible = match icon {
                ADD_DESCRIPTION_ICON => selected && typing.is_none(),
                _ => !looking || hovered,
            };
            if visible && self.icon_button(ui, place, icon, &label, &tooltip, id.with(icon)) {
                self.acts.push(act);
            }
        }
        if deletable && selected {
            let place = Rect::from_min_size(
                Pos2::new(
                    rect.right() - BOX_SPACING - ICON_SIZE,
                    outline.bottom() - BOX_SPACING - ICON_SIZE,
                ),
                Vec2::splat(ICON_SIZE),
            );
            let name = format!("Delete {} {}", node.number, node.title);
            if self.icon_button(ui, place, DELETE_ICON, &name, "Delete", id.with("delete")) {
                self.acts.push(Act::Delete(node.id.clone(), true));
            }
        }
        if let Some(place) = title_field.or(description_field) {
            self.text_field(ui, place);
        }
        if tab.dragging.as_ref().is_some_and(|d| d.id == node.id) {
            // The node being dragged stays in its place, faded.
            painter.rect_filled(outline, 0.0, colours.surface.gamma_multiply(0.55));
        }
        height
    }

    /// The grip a box is dragged by, with everything in it.
    fn grip(&mut self, ui: &mut egui::Ui, node: &SurfaceNode, place: Rect, id: egui::Id) {
        let response = ui.interact(place, id, Sense::drag());
        a11y::describe(&response, Role::Button, &format!("Drag {}", node.number));
        ui.painter().text(
            place.center(),
            Align2::CENTER_CENTER,
            icons::text(GRIP_ICON),
            icons::font(GRIP_WIDTH),
            self.colours.muted,
        );
        if response.drag_started() {
            self.acts.push(Act::StartDrag(node.id.clone()));
        }
        if response.drag_stopped() {
            self.acts.push(Act::EndDrag);
        }
        response.on_hover_text(format!("Drag {}", node.number));
    }

    /// The height the open field's text wraps to at `width`, a line at least.
    fn field_height(&self, painter: &egui::Painter, width: f32) -> f32 {
        let text = self.field.as_ref().map(|f| f.text.as_str()).unwrap_or_default();
        painter.layout_job(self.text_job(text, width)).size().y.max(LINE_HEIGHT)
    }

    /// The open field in `place`, in the font of what it replaced: Enter keeps a title, Ctrl+Enter either,
    /// leaving it keeps either, and Escape drops it. A title is one line: a line break typed or pasted is taken out.
    fn text_field(&mut self, ui: &mut egui::Ui, place: Rect) {
        let Some(field) = self.field.as_mut() else {
            return;
        };
        let id = self.tab.id().with(("field", &field.node_id));
        let title = field.attribute == Attribute::Title;
        if self.tab.field_refocus {
            ui.memory_mut(|m| m.request_focus(id));
        }
        if self.tab.field_opened {
            ui.memory_mut(|m| m.request_focus(id));
            let mut state = egui::text_edit::TextEditState::load(ui.ctx(), id).unwrap_or_default();
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::select_all_str(&field.text)));
            state.store(ui.ctx(), id);
        }
        let mut ended = false;
        // Escape is egui's own end of a focus too, so it is no leaving that keeps the text, whoever has the focus.
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            self.acts.push(Act::Cancel);
            ended = true;
        } else if ui.memory(|m| m.has_focus(id)) {
            let commit = |i: &mut egui::InputState| {
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter)
                    || (title && i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
            };
            if ui.input_mut(commit) {
                self.acts.push(Act::Commit);
                ended = true;
            }
            if ended {
                ui.memory_mut(|m| m.surrender_focus(id));
            }
        }
        let font = self.font.clone();
        let colour = self.colours.text;
        let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap: f32| {
            let mut job = LayoutJob::simple(text.as_str().to_owned(), font.clone(), colour, wrap);
            job.sections[0].format.line_height = Some(LINE_HEIGHT);
            ui.painter().layout_job(job)
        };
        let mut edit = egui::TextEdit::multiline(&mut field.text)
            .id(id)
            .frame(egui::Frame::NONE)
            .margin(egui::Margin::ZERO)
            .desired_width(place.width())
            .desired_rows(1)
            .min_size(place.size())
            .layouter(&mut layouter);
        if title {
            edit = edit.return_key(None);
        }
        let response = ui.put(place, edit);
        if title && field.text.contains(['\r', '\n']) {
            field.text = field.text.replace("\r\n", " ").replace(['\n', '\r'], " ");
        }
        response.ctx.accesskit_node_builder(response.id, |node| {
            if title {
                node.set_role(Role::TextInput);
            }
            node.set_label(field.label.as_str());
        });
        ui.painter().rect_stroke(
            place.expand2(Vec2::new(5.0, 2.0)),
            3.0,
            Stroke::new(1.0, self.colours.border),
            StrokeKind::Inside,
        );
        // Leaving for the conflict's bar is no leaving that keeps the text: the bar decides on it.
        if response.lost_focus() && !ended && !self.tab.conflict {
            self.acts.push(Act::Commit);
        }
    }

    /// A text's layout `width` wide, in the page's font and line height.
    fn text_job(&self, text: &str, width: f32) -> LayoutJob {
        let mut job = LayoutJob::simple(text.to_owned(), self.font.clone(), self.colours.text, width);
        job.sections[0].format.line_height = Some(LINE_HEIGHT);
        job
    }

    /// A title's layout: `lead` points of the first row left for what stands before it, `rows` at most.
    fn title_job(&self, text: &str, lead: f32, width: f32, rows: Option<usize>) -> LayoutJob {
        let mut job = LayoutJob::default();
        let mut format = egui::TextFormat::simple(self.font.clone(), self.colours.text);
        format.line_height = Some(LINE_HEIGHT);
        job.append(text, lead, format);
        job.wrap.max_width = width.max(1.0);
        if let Some(rows) = rows {
            job.wrap.max_rows = rows;
            job.wrap.overflow_character = None;
        }
        job
    }

    /// The labels' values as tags below the title, in the definition's order and each in its look; the bottom of the
    /// row, or `at`'s top when the node carries none.
    fn labels(&self, painter: &egui::Painter, node: &SurfaceNode, at: Pos2, right: f32) -> f32 {
        let colours = self.colours;
        let font = FontId::proportional(TAG_FONT);
        let mut tags: Vec<(LabelLook, String, Color32)> = Vec::new();
        for label in self
            .tab
            .definition
            .labels
            .iter()
            .filter(|l| l.on.contains(&node.type_name))
        {
            let Some(values) = node.labels.iter().find(|v| v.label == label.label) else {
                continue;
            };
            for value in &values.values {
                tags.push((
                    label.look(),
                    value.clone(),
                    colours.option(&self.colour_of(&label.label, value)),
                ));
            }
        }
        if tags.is_empty() {
            return at.y;
        }
        let mut x = at.x;
        let mut y = at.y + TAG_GAP;
        for (look, value, fill) in tags {
            let text = if look == LabelLook::Tag {
                format!("#{value}")
            } else {
                value
            };
            let galley = painter.layout_no_wrap(text, font.clone(), colours.text);
            let dot = if look == LabelLook::Status { 6.0 + 5.0 } else { 0.0 };
            let width = galley.size().x + 2.0 * TAG_PADDING + dot;
            if x > at.x && x + width > right {
                x = at.x;
                y += TAG_HEIGHT + TAG_GAP;
            }
            let tag = Rect::from_min_size(Pos2::new(x, y), Vec2::new(width, TAG_HEIGHT));
            let radius = if look == LabelLook::Badge {
                3.0
            } else {
                TAG_HEIGHT / 2.0
            };
            painter.rect_filled(tag, radius, fill);
            if dot > 0.0 {
                painter.circle_filled(
                    Pos2::new(tag.left() + TAG_PADDING + 3.0, tag.center().y),
                    3.0,
                    colours.text.gamma_multiply(0.6),
                );
            }
            painter.galley(
                Pos2::new(tag.left() + TAG_PADDING + dot, tag.center().y - galley.size().y / 2.0),
                galley,
                colours.text,
            );
            x = tag.right() + TAG_GAP;
        }
        y + TAG_HEIGHT
    }

    /// The colour the option list of `label` gives `value`, matched by name or id as a person types it.
    fn colour_of(&self, label: &str, value: &str) -> String {
        let wanted = value.to_lowercase();
        self.tab
            .model
            .option_lists
            .iter()
            .find(|l| l.label == label)
            .and_then(|l| {
                l.options
                    .iter()
                    .find(|o| o.name.to_lowercase() == wanted || o.id.to_lowercase() == wanted)
            })
            .map(|o| o.colour.clone())
            .unwrap_or_else(|| "default".to_owned())
    }

    /// One of a box's controls: its icon in `place`, named `label` for a screen reader and showing `tooltip` under the
    /// pointer; whether it was pressed. A control not `visible` keeps its place and takes no press.
    #[allow(clippy::too_many_arguments)]
    /// The notes' icon in `place`: the mark of the open notes in the warning colour, with their count when there are
    /// several, or the way to add the first.
    fn note_icon(&self, ui: &mut egui::Ui, place: Rect, control: &NoteControl, id: egui::Id, open: bool) -> bool {
        let response = ui.interact(place, id, Sense::click());
        a11y::describe(&response, Role::Button, &control.label);
        response
            .ctx
            .accesskit_node_builder(response.id, |node| node.set_expanded(open));
        let colour = if control.mark {
            self.colours.warning
        } else if response.hovered() {
            self.colours.text
        } else {
            self.colours.muted
        };
        let painter = ui.painter();
        painter.text(
            place.center(),
            Align2::CENTER_CENTER,
            icons::text(&control.icon),
            icons::font(self.font.size),
            colour,
        );
        if control.count > 1 {
            painter.text(
                place.right_bottom() + Vec2::new(1.0, 1.0),
                Align2::RIGHT_BOTTOM,
                control.count.to_string(),
                FontId::proportional(NOTE_COUNT_FONT),
                colour,
            );
        }
        response.on_hover_text(&control.label).clicked()
    }

    fn icon_button(
        &self,
        ui: &mut egui::Ui,
        place: Rect,
        icon: &str,
        label: &str,
        tooltip: &str,
        id: egui::Id,
    ) -> bool {
        let response = ui.interact(place, id, Sense::click());
        a11y::describe(&response, Role::Button, label);
        let colour = if response.hovered() {
            self.colours.text
        } else {
            self.colours.muted
        };
        ui.painter().text(
            place.center(),
            Align2::CENTER_CENTER,
            icons::text(icon),
            icons::font(self.font.size),
            colour,
        );
        response.on_hover_text(tooltip).clicked()
    }
}
