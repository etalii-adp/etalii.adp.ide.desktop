//! The widths of the columns of levels (the standalone host's `columns/useColumnWidths.ts` and `columns/ColumnEdge.tsx`
//! at a7fdf03; DESL section 7.7.1; etalii.adp spec 036, T052): those the document keeps, the one being dragged, and the
//! page's own for the rest.
//!
//! - A level box's right edge is dragged to change its column's width: every box of that depth follows the pointer,
//!   and one gesture is sent when it is let go. The width it showed stays until the document's next model, or until
//!   the gesture is refused, which puts the column back.
//! - A double click, or Home on a focused edge, gives a kept column the page's own width again; the arrows move a
//!   focused edge by a step.
//! - Each column's width is its own, so changing one while another's gesture is answered leaves the other as sent.

use std::collections::BTreeMap;

use adp_desl::hierarchy::definition::DOCUMENT_TARGET;
use eframe::egui::{self, Rect, Sense, Stroke, Vec2, accesskit};

use super::HierarchyTab;
use super::layout::LEVEL_WIDTH;
use crate::a11y;
use crate::table::view::Colours;

/// How far an arrow key moves a column's edge, in points.
pub const STEP: f32 = 16.0;
/// How wide the edge is to the pointer, half on each side of the box's border (`.hs-column-edge`).
const EDGE_WIDTH: f32 = 8.0;
/// How wide its line is drawn while it is hovered, dragged or has the keyboard.
const LINE_WIDTH: f32 = 2.0;

/// What a column's edge asked for in a frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EdgeAct {
    /// Show the column at a level at a width while its edge is dragged, sending nothing.
    Preview(u32, f32),
    /// Keep a width for the column at a level: its first, or a change of the kept one.
    Commit(u32, f32),
    /// Give the column at a level the page's own width again.
    Reset(u32),
}

/// The widths shown before the document has them, by depth, and the column whose edge is being dragged.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ColumnWidths {
    changing: BTreeMap<u32, f32>,
    dragging: Option<u32>,
}

impl ColumnWidths {
    /// The width shown for the column at `level` before the document has it, if any.
    pub fn shown(&self, level: u32) -> Option<f32> {
        self.changing.get(&level).copied()
    }

    /// The column whose edge is being dragged, which stays in sight however wide it is made: the levels that fit are
    /// counted with its width, and a column that hid itself would lose the pointer that drags it.
    pub fn dragging(&self) -> Option<u32> {
        self.dragging
    }

    /// The model that answers a gesture holds the width it kept; what was shown is then the document's.
    pub fn answered(&mut self) {
        self.changing.clear();
    }
}

impl HierarchyTab {
    /// The range a width is held to: the definition's, or the page's own width when it keeps none.
    pub fn width_range(&self) -> (f32, f32) {
        self.definition
            .widths
            .as_ref()
            .map_or((LEVEL_WIDTH, LEVEL_WIDTH), |w| (w.min as f32, w.max as f32))
    }

    /// Whether the columns' edges are offered: a document that can change, with a definition that keeps widths.
    pub fn resizable(&self) -> bool {
        !self.read_only() && self.definition.widths.is_some()
    }

    /// The width the document keeps for the column at `level`, with its id, if any.
    fn kept(&self, level: u32) -> Option<(String, u32)> {
        self.model
            .widths
            .iter()
            .find(|c| c.level == level)
            .map(|c| (c.id.clone(), c.width))
    }

    /// What a column's edge asked for.
    pub fn edge(&mut self, act: EdgeAct) {
        match act {
            EdgeAct::Preview(level, width) => {
                let (min, max) = self.width_range();
                self.columns.changing.insert(level, width.clamp(min, max));
                self.columns.dragging = Some(level);
            }
            EdgeAct::Commit(level, width) => {
                self.columns.dragging = None;
                self.commit_width(level, width);
            }
            EdgeAct::Reset(level) => self.reset_width(level),
        }
    }

    /// Keeps `width` for the column at `level`: one gesture, none when it is the width the column has.
    fn commit_width(&mut self, level: u32, width: f32) {
        let (min, max) = self.width_range();
        let wanted = width.round().clamp(min, max);
        let stored = self.kept(level);
        let Some(widths) = self.definition.widths.clone() else {
            self.columns.changing.remove(&level);
            return;
        };
        if wanted == stored.as_ref().map_or(LEVEL_WIDTH, |(_, w)| *w as f32) {
            self.columns.changing.remove(&level);
            return;
        }
        let width = (wanted as u32).to_string();
        match stored {
            None => {
                let params = BTreeMap::from([(widths.level_param, level.to_string()), (widths.width_param, width)]);
                self.send_width(level, &widths.add, DOCUMENT_TARGET, params);
            }
            Some((id, _)) => {
                let params = BTreeMap::from([(widths.width_param, width)]);
                self.send_width(level, &widths.set, &id, params);
            }
        }
    }

    /// Gives the column at `level` the page's own width again, when it has a kept one.
    fn reset_width(&mut self, level: u32) {
        if let (Some(widths), Some((id, _))) = (self.definition.widths.clone(), self.kept(level)) {
            self.send_width(level, &widths.reset, &id, BTreeMap::new());
        }
    }

    /// Sends a gesture on the width of the column at `level`. The session answers at once: the model it sends shows
    /// the width the document now keeps, and a refusal leaves the document as it was, so the column goes back to it.
    fn send_width(&mut self, level: u32, operation: &str, target: &str, params: BTreeMap<String, String>) {
        self.gesture(operation, target, params);
        if !self.refusal.is_empty() {
            self.columns.changing.remove(&level);
        }
    }
}

/// Draws the right edge of the column at `level` along `rect`'s right side, its width `width`; what it asked for. Each
/// box of the column has one, named for it by `key`.
pub fn edge(
    ui: &mut egui::Ui,
    tab: &HierarchyTab,
    level: u32,
    key: &str,
    rect: Rect,
    colours: &Colours,
) -> Option<EdgeAct> {
    let width = tab.column_width(level);
    let (min, max) = tab.width_range();
    let place = Rect::from_center_size(
        egui::pos2(rect.right(), rect.center().y),
        Vec2::new(EDGE_WIDTH, rect.height()),
    );
    let id = tab.id().with(("column edge", level, key));
    let response = ui
        .interact(place, id, Sense::click_and_drag())
        .on_hover_cursor(egui::CursorIcon::ResizeHorizontal)
        .on_hover_text("Drag to change the column's width; double-click for the default");
    a11y::describe(
        &response,
        accesskit::Role::Splitter,
        &format!("Width of column {level}"),
    );
    response.ctx.accesskit_node_builder(response.id, |node| {
        node.set_orientation(accesskit::Orientation::Vertical);
        node.set_numeric_value(f64::from(width));
        node.set_min_numeric_value(f64::from(min));
        node.set_max_numeric_value(f64::from(max));
    });
    if response.clicked() || response.drag_started() {
        // A pressed edge takes the keyboard, as a focusable element of the page does.
        response.request_focus();
    }
    let focused = response.has_focus();
    if response.hovered() || response.dragged() || focused {
        ui.painter().vline(
            place.center().x,
            place.y_range(),
            Stroke::new(LINE_WIDTH, colours.focus),
        );
    }
    let mut asked = None;
    // The width the edge was grabbed at, kept while it is dragged.
    if response.drag_started() {
        ui.data_mut(|d| d.insert_temp(id, width));
    }
    if response.drag_stopped() {
        // Let go where the last frame showed the column: the press it travelled from is over by now.
        ui.data_mut(|d| d.remove::<f32>(id));
        asked = Some(EdgeAct::Commit(level, width));
    } else if response.dragged() {
        let grabbed = ui.data(|d| d.get_temp::<f32>(id)).unwrap_or(width);
        let travelled = ui.input(|i| match (i.pointer.press_origin(), i.pointer.interact_pos()) {
            (Some(from), Some(to)) => to.x - from.x,
            _ => 0.0,
        });
        asked = Some(EdgeAct::Preview(level, grabbed + travelled));
    } else if response.double_clicked() {
        asked = Some(EdgeAct::Reset(level));
    }
    if focused {
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                id,
                egui::EventFilter {
                    horizontal_arrows: true,
                    ..Default::default()
                },
            )
        });
        ui.input_mut(|i| {
            if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight) {
                asked = Some(EdgeAct::Commit(level, width + STEP));
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft) {
                asked = Some(EdgeAct::Commit(level, width - STEP));
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::Home) {
                asked = Some(EdgeAct::Reset(level));
            }
        });
    }
    asked
}
