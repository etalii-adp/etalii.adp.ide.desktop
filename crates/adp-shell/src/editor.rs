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
    /// Whether a language client serves this editor (only `.atd`, spec 035, T105): its keys then ask for completion,
    /// hover, definition, references, rename and formatting.
    pub language: bool,
    /// What the person asked of the language client since it last looked.
    pub commands: Vec<Command>,
    /// The open completion list.
    pub completion: Option<Completion>,
    /// The language client's colouring over the highlighting, per line, for the buffer revision it was made for.
    pub semantic: Option<(u64, Vec<Vec<SemanticRun>>)>,
    /// Where the cursor was drawn this frame, for the popups beside it.
    pub cursor_rect: Option<Rect>,
    /// The character under the pointer this frame, and the pointer.
    pub pointer: Option<(usize, Pos2)>,
    /// Ctrl+K was pressed: the next key completes the chord.
    chord: bool,
}

/// A request of the person to the language client, by the keys VS Code gives it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Ctrl+Space, or typing `.`.
    Complete,
    /// Ctrl+K Ctrl+I.
    Hover,
    /// F12.
    Definition,
    /// Shift+F12.
    References,
    /// F2.
    Rename,
    /// Shift+Alt+F.
    Format,
    /// Enter or Tab in the completion list: the chosen item.
    Accept(String),
}

/// A completion list: its items, the one selected among those shown, and where the word it completes starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    pub items: Vec<String>,
    pub selected: usize,
    pub start: usize,
}

impl Completion {
    /// The items that start with what was typed since the list opened.
    pub fn shown(&self, buffer: &Buffer) -> Vec<String> {
        let cursor = buffer.cursor();
        let (line, column) = buffer.position(cursor);
        let (start_line, start_column) = buffer.position(self.start);
        if cursor < self.start || line != start_line {
            return Vec::new();
        }
        let typed: String = buffer
            .line(line)
            .chars()
            .skip(start_column)
            .take(column - start_column)
            .collect::<String>()
            .to_lowercase();
        self.items
            .iter()
            .filter(|i| i.to_lowercase().starts_with(&typed))
            .cloned()
            .collect()
    }
}

/// A run of a line coloured by the language client, by character offsets within the line.
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticRun {
    pub start: usize,
    pub end: usize,
    pub color: Color32,
    pub italic: bool,
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
            language: false,
            commands: Vec::new(),
            completion: None,
            semantic: None,
            cursor_rect: None,
            pointer: None,
            chord: false,
        }
    }

    pub fn id(&self) -> egui::Id {
        self.id
    }

    /// Draws the editor filling the space it is given; returns whether the text changed this frame.
    pub fn show(&mut self, ui: &mut egui::Ui, theme: &Theme, highlighter: &Highlighter) -> bool {
        let revision = self.buffer.revision();
        self.cursor_rect = None;
        self.pointer = None;
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
                    let mut runs = self
                        .runs
                        .as_ref()
                        .and_then(|(_, _, r)| r.get(line))
                        .cloned()
                        .unwrap_or_default();
                    if let Some((made_for, lines)) = &self.semantic
                        && *made_for == self.buffer.revision()
                        && let Some(semantic) = lines.get(line).filter(|s| !s.is_empty())
                    {
                        runs = overlay(&runs, &line_text, semantic);
                    }
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
                        self.cursor_rect = Some(Rect::from_min_max(
                            Pos2::new(x, top),
                            Pos2::new(x + 1.0, top + row_height),
                        ));
                        painter.line_segment(
                            [Pos2::new(x, top + 2.0), Pos2::new(x, top + row_height - 2.0)],
                            Stroke::new(2.0, cursor_color),
                        );
                    }
                    galleys.push((line, galley, text_origin));
                }
                if let Some(pointer) = response.hover_pos() {
                    let row = ((pointer.y - rect.top()) / row_height).floor().max(0.0) as usize;
                    if let Some((line, galley, origin)) = galleys.get(row) {
                        let at = pointer - *origin;
                        let column = galley.cursor_from_pos(at).index.0;
                        if at.x >= 0.0 && at.x < galley.rect.right() {
                            self.pointer = Some((
                                self.buffer.offset(
                                    *line,
                                    column.saturating_sub(usize::from(
                                        galley.pos_from_cursor(egui::text::CCursor::new(column)).left() > at.x,
                                    )),
                                ),
                                pointer,
                            ));
                        }
                    }
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
                egui::Event::Text(text) => {
                    self.buffer.insert(&text);
                    if self.language && text == "." {
                        self.commands.push(Command::Complete);
                    }
                }
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
        if self.language && self.language_key(key, modifiers) {
            return;
        }
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

impl TextEditor {
    /// The keys of the language client and of an open completion list; returns whether the key was one of them.
    fn language_key(&mut self, key: Key, modifiers: Modifiers) -> bool {
        if let Some(completion) = &mut self.completion {
            let shown = completion.shown(&self.buffer);
            match key {
                Key::ArrowDown => {
                    completion.selected = (completion.selected + 1) % shown.len().max(1);
                    return true;
                }
                Key::ArrowUp => {
                    completion.selected = (completion.selected + shown.len().max(1) - 1) % shown.len().max(1);
                    return true;
                }
                Key::Enter | Key::Tab => {
                    if let Some(label) = shown.get(completion.selected) {
                        self.commands.push(Command::Accept(label.clone()));
                        self.completion = None;
                        return true;
                    }
                    self.completion = None;
                }
                Key::Escape => {
                    self.completion = None;
                    return true;
                }
                _ => {}
            }
        }
        let chord = std::mem::take(&mut self.chord);
        let command = match key {
            Key::Space if modifiers.ctrl => Command::Complete,
            Key::K if modifiers.command => {
                self.chord = true;
                return true;
            }
            Key::I if chord && modifiers.command => Command::Hover,
            Key::F12 if modifiers.shift => Command::References,
            Key::F12 => Command::Definition,
            Key::F2 => Command::Rename,
            Key::F if modifiers.shift && modifiers.alt => Command::Format,
            _ => return false,
        };
        self.commands.push(command);
        true
    }
}

/// The highlighted runs of a line with the language client's runs over them, by byte range.
fn overlay(runs: &[Run], text: &str, semantic: &[SemanticRun]) -> Vec<Run> {
    let mut styles: Vec<Option<(Color32, bool)>> = vec![None; text.len()];
    for run in runs {
        for style in styles
            .iter_mut()
            .take(run.range.end.min(text.len()))
            .skip(run.range.start)
        {
            *style = Some((run.color, run.italic));
        }
    }
    let byte_of = |char_offset: usize| text.char_indices().nth(char_offset).map_or(text.len(), |(i, _)| i);
    for run in semantic {
        let (from, to) = (byte_of(run.start), byte_of(run.end));
        for style in styles.iter_mut().take(to).skip(from) {
            *style = Some((run.color, run.italic));
        }
    }
    let mut out: Vec<Run> = Vec::new();
    let mut at = 0;
    while at < styles.len() {
        let style = styles[at];
        let mut end = at + 1;
        while end < styles.len() && styles[end] == style {
            end += 1;
        }
        if let Some((color, italic)) = style {
            out.push(Run {
                range: at..end,
                color,
                italic,
            });
        }
        at = end;
    }
    out
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
    fn the_language_keys_become_commands_and_steer_the_completion_list() {
        let ctx = egui::Context::default();
        let highlighter = Highlighter::default();
        let mut editor = TextEditor::new("atd 0.1\n", "atd", egui::Id::unique("atd-editor"));
        editor.language = true;
        run_frame(&ctx, Vec::new(), &mut editor, &highlighter);
        ctx.memory_mut(|m| m.request_focus(editor.id()));
        run_frame(&ctx, Vec::new(), &mut editor, &highlighter);
        editor.buffer.set_cursor(8);
        run_frame(
            &ctx,
            vec![
                key(Key::Space, Modifiers::CTRL),
                key(Key::F12, Modifiers::NONE),
                key(Key::F12, Modifiers::SHIFT),
                key(Key::F2, Modifiers::NONE),
                key(Key::K, Modifiers::COMMAND),
                key(Key::I, Modifiers::COMMAND),
                egui::Event::Text(".".into()),
            ],
            &mut editor,
            &highlighter,
        );
        assert_eq!(
            editor.commands,
            vec![
                Command::Complete,
                Command::Definition,
                Command::References,
                Command::Rename,
                Command::Hover,
                Command::Complete
            ]
        );
        editor.commands.clear();
        editor.completion = Some(Completion {
            items: vec!["diagram".into(), "designer".into(), "node".into()],
            selected: 0,
            start: editor.buffer.cursor(),
        });
        run_frame(
            &ctx,
            vec![
                egui::Event::Text("d".into()),
                key(Key::ArrowDown, Modifiers::NONE),
                key(Key::Enter, Modifiers::NONE),
            ],
            &mut editor,
            &highlighter,
        );
        assert_eq!(editor.commands, vec![Command::Accept("designer".into())]);
        assert!(editor.completion.is_none());
    }

    #[test]
    fn semantic_runs_take_the_place_of_the_highlighting_by_characters() {
        let runs = vec![Run {
            range: 0..6,
            color: Color32::RED,
            italic: false,
        }];
        let out = overlay(
            &runs,
            "é node",
            &[SemanticRun {
                start: 2,
                end: 6,
                color: Color32::BLUE,
                italic: true,
            }],
        );
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].range, 0..3);
        assert_eq!(out[1].range, 3..7);
        assert_eq!(out[1].color, Color32::BLUE);
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
