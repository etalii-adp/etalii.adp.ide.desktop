//! The text component (etalii.adp spec 030, research R6, task T029): the editors' view of an [`adp_text::Buffer`],
//! drawn line by line so only visible lines cost anything, with a gutter of line numbers, highlighting, the
//! selection and the cursor in VS Code's colours and font size. It takes the keyboard while it has focus: typing,
//! Enter, Tab, Backspace and Delete, the arrows with Ctrl for words and Shift to select, Home and End, Ctrl+A, copy,
//! cut and paste, and undo and redo inside the editor.

use std::sync::Arc;

use adp_text::{Buffer, Movement};
use adp_theme::{Kind, Theme};
use eframe::egui::{
    self, Color32, FontId, Galley, Key, Modifiers, Pos2, Rect, Sense, Stroke, TextFormat, Vec2, text::LayoutJob,
};

use crate::highlight::{Highlighter, Run};
use crate::theme::color;

/// VS Code inserts four spaces for Tab by default.
const TAB: &str = "    ";

pub struct TextEditor {
    pub buffer: Buffer,
    extension: String,
    id: egui::Id,
    /// The highlighted runs per line, for the buffer revision and theme they were made for.
    runs: Option<(u64, Kind, Vec<Vec<Run>>)>,
    /// The line the cursor was on last frame, to scroll it into view when it moves.
    last_cursor: Option<usize>,
    /// Findings to underline, by character offsets.
    pub marks: Vec<Mark>,
}

/// A finding drawn as a squiggle under its range, in the theme's error, warning or information colour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mark {
    pub start: usize,
    pub end: usize,
    pub level: MarkLevel,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkLevel {
    Error,
    Warning,
    Info,
}

impl TextEditor {
    pub fn new(text: &str, extension: &str, id: egui::Id) -> Self {
        Self {
            buffer: Buffer::new(text),
            extension: extension.to_lowercase(),
            id,
            runs: None,
            last_cursor: None,
            marks: Vec::new(),
        }
    }

    pub fn id(&self) -> egui::Id {
        self.id
    }

    /// Draws the editor filling the space it is given; returns whether the text changed this frame.
    pub fn show(&mut self, ui: &mut egui::Ui, theme: &Theme, highlighter: &Highlighter) -> bool {
        let revision = self.buffer.revision();
        let focused = ui.memory(|m| m.has_focus(self.id));
        if focused {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    self.id,
                    egui::EventFilter {
                        tab: true,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: false,
                    },
                )
            });
            self.keyboard(ui);
        }

        if self
            .runs
            .as_ref()
            .is_none_or(|(r, k, _)| *r != self.buffer.revision() || *k != theme.kind)
        {
            let runs = highlighter.highlight(&self.buffer.text(), &self.extension, theme.kind);
            self.runs = Some((self.buffer.revision(), theme.kind, runs));
        }

        let font = FontId::monospace(theme.sizes.editor_font);
        let row_height = ui.fonts_mut(|f| f.row_height(&font)).ceil() + 4.0;
        let foreground = color(theme, &["editor.foreground", "foreground"], Color32::GRAY);
        let background = color(theme, &["editor.background"], Color32::BLACK);
        let gutter_text = color(theme, &["editorLineNumber.foreground"], Color32::DARK_GRAY);
        let gutter_active = color(
            theme,
            &["editorLineNumber.activeForeground", "editor.foreground"],
            foreground,
        );
        let selection = color(theme, &["editor.selectionBackground"], Color32::from_rgb(38, 79, 120));
        let cursor_color = color(theme, &["editorCursor.foreground", "editor.foreground"], foreground);
        let line_highlight = color(
            theme,
            &["editor.lineHighlightBorder", "editor.lineHighlightBackground"],
            Color32::TRANSPARENT,
        );

        let line_count = self.buffer.line_count();
        let digit_width = ui.fonts_mut(|f| f.glyph_width(&font, '0'));
        let gutter = digit_width * (line_count.to_string().len().max(3) as f32) + 28.0;
        let cursor_line = self.buffer.position(self.buffer.cursor()).0;
        let scroll_to_cursor = self.last_cursor != Some(self.buffer.cursor());
        self.last_cursor = Some(self.buffer.cursor());
        let (sel_start, sel_end) = self.buffer.selection();

        egui::Frame::NONE.fill(background).show(ui, |ui| {
            let mut area = egui::ScrollArea::both().auto_shrink([false, false]).id_salt(self.id);
            if scroll_to_cursor && focused {
                let visible = ui.available_height();
                let target = cursor_line as f32 * row_height;
                let offset = ui
                    .ctx()
                    .data(|d| d.get_temp::<f32>(self.id.with("offset")))
                    .unwrap_or(0.0);
                if target < offset {
                    area = area.vertical_scroll_offset(target);
                } else if target + row_height > offset + visible {
                    area = area.vertical_scroll_offset(target + row_height - visible);
                }
            }
            let output = area.show_rows(ui, row_height, line_count, |ui, rows| {
                let width = ui.available_width().max(400.0);
                let (rect, _) =
                    ui.allocate_exact_size(Vec2::new(width, row_height * rows.len() as f32), Sense::hover());
                let response = ui.interact(rect, self.id, Sense::click_and_drag());
                if response.clicked() || response.drag_started() {
                    ui.memory_mut(|m| m.request_focus(self.id));
                }
                let painter = ui.painter_at(rect.expand(2.0));
                let mut galleys: Vec<(usize, Arc<Galley>, Pos2)> = Vec::with_capacity(rows.len());
                for (i, line) in rows.clone().enumerate() {
                    let top = rect.top() + i as f32 * row_height;
                    let text_origin = Pos2::new(rect.left() + gutter, top + 2.0);
                    let line_text = self.buffer.line(line);
                    let line_start = self.buffer.offset(line, 0);
                    if line == cursor_line && focused {
                        painter.rect_stroke(
                            Rect::from_min_size(Pos2::new(rect.left(), top), Vec2::new(width, row_height)),
                            0.0,
                            Stroke::new(theme.sizes.border_width, line_highlight),
                            egui::StrokeKind::Inside,
                        );
                    }
                    let number_colour = if line == cursor_line {
                        gutter_active
                    } else {
                        gutter_text
                    };
                    painter.text(
                        Pos2::new(rect.left() + gutter - 16.0, top + 2.0),
                        egui::Align2::RIGHT_TOP,
                        (line + 1).to_string(),
                        font.clone(),
                        number_colour,
                    );
                    let runs = self
                        .runs
                        .as_ref()
                        .and_then(|(_, _, r)| r.get(line))
                        .cloned()
                        .unwrap_or_default();
                    let galley = painter.layout_job(line_job(&line_text, &runs, &font, foreground));
                    // The selection's part of this line.
                    let line_end = line_start + line_text.chars().count();
                    if sel_start != sel_end && sel_start <= line_end && sel_end >= line_start {
                        let from = sel_start.max(line_start) - line_start;
                        let to = sel_end.min(line_end) - line_start;
                        let left = galley.pos_from_cursor(egui::text::CCursor::new(from)).left();
                        let mut right = galley.pos_from_cursor(egui::text::CCursor::new(to)).left();
                        if sel_end > line_end {
                            right += digit_width; // the line break is selected too
                        }
                        painter.rect_filled(
                            Rect::from_min_max(
                                Pos2::new(text_origin.x + left, top),
                                Pos2::new(text_origin.x + right, top + row_height),
                            ),
                            0.0,
                            selection,
                        );
                    }
                    painter.galley(text_origin, galley.clone(), foreground);
                    for mark in &self.marks {
                        if mark.end < line_start || mark.start > line_end {
                            continue;
                        }
                        let from = mark.start.max(line_start) - line_start;
                        let to = (mark.end.min(line_end) - line_start).max(from + 1);
                        let left = text_origin.x + galley.pos_from_cursor(egui::text::CCursor::new(from)).left();
                        let right = (text_origin.x + galley.pos_from_cursor(egui::text::CCursor::new(to)).left())
                            .max(left + digit_width);
                        let colour = match mark.level {
                            MarkLevel::Error => {
                                color(theme, &["editorError.foreground", "errorForeground"], Color32::RED)
                            }
                            MarkLevel::Warning => color(theme, &["editorWarning.foreground"], Color32::YELLOW),
                            MarkLevel::Info => color(theme, &["editorInfo.foreground"], Color32::LIGHT_BLUE),
                        };
                        squiggle(&painter, left, right, top + row_height - 3.0, colour);
                    }
                    if focused && line == cursor_line {
                        let column = self.buffer.position(self.buffer.cursor()).1;
                        let x = text_origin.x + galley.pos_from_cursor(egui::text::CCursor::new(column)).left();
                        painter.line_segment(
                            [Pos2::new(x, top + 2.0), Pos2::new(x, top + row_height - 2.0)],
                            Stroke::new(2.0, cursor_color),
                        );
                    }
                    galleys.push((line, galley, text_origin));
                }
                if let Some(pointer) = response.interact_pointer_pos()
                    && (response.clicked() || response.dragged() || response.drag_started())
                {
                    let row = ((pointer.y - rect.top()) / row_height).floor().max(0.0) as usize;
                    if let Some((line, galley, origin)) = galleys.get(row.min(galleys.len().saturating_sub(1))) {
                        let column = galley.cursor_from_pos(pointer - *origin).index.0;
                        let offset = self.buffer.offset(*line, column);
                        let extend = response.dragged() && !response.drag_started()
                            || ui.input(|i| i.modifiers.shift) && response.clicked();
                        if extend {
                            let anchor = if self.buffer.selection().0 == self.buffer.cursor() {
                                self.buffer.selection().1
                            } else {
                                self.buffer.selection().0
                            };
                            self.buffer.select(anchor, offset);
                        } else {
                            self.buffer.set_cursor(offset);
                        }
                        self.last_cursor = Some(self.buffer.cursor());
                    }
                }
            });
            ui.ctx()
                .data_mut(|d| d.insert_temp(self.id.with("offset"), output.state.clamped_offset().y));
        });
        self.buffer.revision() != revision
    }

    fn keyboard(&mut self, ui: &egui::Ui) {
        let events = ui.input(|i| i.events.clone());
        for event in events {
            match event {
                egui::Event::Text(text) => self.buffer.insert(&text),
                egui::Event::Paste(text) => self.buffer.insert(&text.replace("\r\n", "\n")),
                egui::Event::Copy => {
                    let text = self.buffer.selected_text();
                    if !text.is_empty() {
                        ui.ctx().copy_text(text);
                    }
                }
                egui::Event::Cut => {
                    let text = self.buffer.selected_text();
                    if !text.is_empty() {
                        ui.ctx().copy_text(text);
                        self.buffer.delete_backward();
                    }
                }
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => self.key(key, modifiers),
                _ => {}
            }
        }
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) {
        let word = modifiers.command || modifiers.alt;
        let extend = modifiers.shift;
        let movement = match key {
            Key::ArrowLeft if word => Some(Movement::WordLeft),
            Key::ArrowRight if word => Some(Movement::WordRight),
            Key::ArrowLeft => Some(Movement::Left),
            Key::ArrowRight => Some(Movement::Right),
            Key::ArrowUp => Some(Movement::Up),
            Key::ArrowDown => Some(Movement::Down),
            Key::Home if modifiers.command => Some(Movement::DocumentStart),
            Key::End if modifiers.command => Some(Movement::DocumentEnd),
            Key::Home => Some(Movement::LineStart),
            Key::End => Some(Movement::LineEnd),
            _ => None,
        };
        if let Some(movement) = movement {
            self.buffer.move_cursor(movement, extend);
            return;
        }
        match key {
            Key::Enter => self.buffer.insert("\n"),
            Key::Tab if !modifiers.shift => self.buffer.insert(TAB),
            Key::Backspace => self.buffer.delete_backward(),
            Key::Delete => self.buffer.delete_forward(),
            Key::A if modifiers.command => self.buffer.select_all(),
            Key::Z if modifiers.command && modifiers.shift => {
                self.buffer.redo();
            }
            Key::Z if modifiers.command => {
                self.buffer.undo();
            }
            Key::Y if modifiers.command => {
                self.buffer.redo();
            }
            _ => {}
        }
    }
}

/// A wavy underline from `left` to `right` at `y`, as VS Code draws a finding.
fn squiggle(painter: &egui::Painter, left: f32, right: f32, y: f32, colour: Color32) {
    let mut points = Vec::new();
    let mut x = left;
    let mut up = true;
    while x <= right {
        points.push(Pos2::new(x, if up { y - 1.5 } else { y + 1.5 }));
        x += 2.0;
        up = !up;
    }
    if points.len() > 1 {
        painter.add(egui::Shape::line(points, Stroke::new(1.0, colour)));
    }
}

/// One line as a layout job: the highlighted runs over the line's text, the rest in the editor's foreground.
fn line_job(text: &str, runs: &[Run], font: &FontId, foreground: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mut at = 0;
    for run in runs {
        let end = run.range.end.min(text.len());
        if run.range.start >= end || run.range.start < at {
            continue;
        }
        if run.range.start > at {
            job.append(
                &text[at..run.range.start],
                0.0,
                TextFormat::simple(font.clone(), foreground),
            );
        }
        let format = TextFormat {
            italics: run.italic,
            ..TextFormat::simple(font.clone(), run.color)
        };
        job.append(&text[run.range.start..end], 0.0, format);
        at = end;
    }
    if at < text.len() {
        job.append(&text[at..], 0.0, TextFormat::simple(font.clone(), foreground));
    }
    job
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_frame(
        ctx: &egui::Context,
        events: Vec<egui::Event>,
        editor: &mut TextEditor,
        highlighter: &Highlighter,
    ) -> bool {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 600.0))),
            events,
            ..Default::default()
        };
        let mut changed = false;
        let theme = Theme::dark();
        let output = ctx.run_ui(input, |ui| changed = editor.show(ui, &theme, highlighter));
        let mut deltas = output.textures_delta;
        deltas.clear();
        changed
    }

    fn key(key: Key, modifiers: Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    #[test]
    fn a_focused_editor_takes_typing_keys_and_its_own_undo() {
        let ctx = egui::Context::default();
        let highlighter = Highlighter::default();
        let mut editor = TextEditor::new("{\n}\n", "json", egui::Id::unique("test-editor"));
        run_frame(&ctx, Vec::new(), &mut editor, &highlighter);
        ctx.memory_mut(|m| m.request_focus(editor.id()));
        run_frame(&ctx, Vec::new(), &mut editor, &highlighter);
        editor.buffer.set_cursor(1);
        assert!(run_frame(
            &ctx,
            vec![key(Key::Enter, Modifiers::NONE), egui::Event::Text("\"a\": 1".into())],
            &mut editor,
            &highlighter
        ));
        assert_eq!(editor.buffer.text(), "{\n\"a\": 1\n}\n");
        run_frame(&ctx, vec![key(Key::Z, Modifiers::COMMAND)], &mut editor, &highlighter);
        assert_ne!(editor.buffer.text(), "{\n\"a\": 1\n}\n");
        run_frame(
            &ctx,
            vec![key(Key::A, Modifiers::COMMAND), key(Key::Delete, Modifiers::NONE)],
            &mut editor,
            &highlighter,
        );
        assert_eq!(editor.buffer.text(), "");
    }

    #[test]
    fn an_editor_without_focus_ignores_typing() {
        let ctx = egui::Context::default();
        let highlighter = Highlighter::default();
        let mut editor = TextEditor::new("x", "txt", egui::Id::unique("other-editor"));
        assert!(!run_frame(
            &ctx,
            vec![egui::Event::Text("y".into())],
            &mut editor,
            &highlighter
        ));
        assert_eq!(editor.buffer.text(), "x");
    }
}
