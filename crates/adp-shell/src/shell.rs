//! The window's frame (etalii.adp spec 030, task T021): the activity bar, the side bar with the explorer, the editor
//! area and the status bar, in VS Code's sizes and colours. The panes fill with their tasks of Story 1; until then the
//! frame shows where they go. The shell drains the core's events once per frame and changes nothing but through the
//! core's calls.

use std::sync::mpsc::Receiver;

use adp_core::{Event, Outcome, Workspace};
use adp_documents::TextFile;
use eframe::egui::{self, Frame, Margin, RichText};

use crate::editor::{Mark, MarkLevel, TextEditor};
use crate::highlight::Highlighter;
use crate::theme::{self, Palette};

/// A text file open in the editor: the file as read, for saving it back, and the component showing it.
struct OpenText {
    file: TextFile,
    editor: TextEditor,
    /// The buffer revision the editor's analysis was made for, and what it says on the status line.
    analysed: Option<u64>,
    status: String,
    /// For Markdown: the outline and the preview, made for the analysed revision.
    markdown: Option<(Vec<adp_editor_markdown::Heading>, Vec<adp_editor_markdown::Block>)>,
}

impl OpenText {
    /// Runs the editor's analysis when the text changed: for JSON, its mistakes and its schema (FR-020).
    fn analyse(&mut self) {
        let revision = self.editor.buffer.revision();
        if self.analysed == Some(revision) {
            return;
        }
        self.analysed = Some(revision);
        let extension = self
            .file
            .path()
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if extension == "md" || extension == "markdown" {
            let text = self.editor.buffer.text();
            self.markdown = Some((
                adp_editor_markdown::headings_of(&text),
                adp_editor_markdown::preview(&text),
            ));
            return;
        }
        if !matches!(extension.as_str(), "json" | "yaml" | "yml" | "xml" | "xsd") {
            return;
        }
        let text = self.editor.buffer.text();
        // Without a project, the file's own folder is the project the schema may be read from.
        let folder = self
            .file
            .path()
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_default();
        let name = self
            .file
            .path()
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let read: adp_editor_json::ReadFile =
            std::sync::Arc::new(move |path: &[String]| std::fs::read_to_string(folder.join(path.join("/"))).ok());
        // Each editor's findings as (byte span, level, message), and its status line.
        let (findings, status): (Vec<(std::ops::Range<usize>, MarkLevel, String)>, String) = if extension == "json" {
            let analysis = adp_editor_json::analyse(&text, &[name], read);
            let findings = analysis
                .findings
                .into_iter()
                .map(|f| {
                    let level = match f.severity {
                        adp_editor_json::Severity::Error => MarkLevel::Error,
                        adp_editor_json::Severity::Warning => MarkLevel::Warning,
                        adp_editor_json::Severity::Info => MarkLevel::Info,
                    };
                    (f.span, level, f.message)
                })
                .collect();
            (findings, analysis.status)
        } else if extension == "xml" || extension == "xsd" {
            let analysis = adp_editor_xml::analyse(&text, &[name], self.file.encoding_name());
            let findings = analysis
                .findings
                .into_iter()
                .map(|f| {
                    let level = match f.severity {
                        adp_editor_xml::Severity::Error => MarkLevel::Error,
                        adp_editor_xml::Severity::Warning => MarkLevel::Warning,
                        adp_editor_xml::Severity::Info => MarkLevel::Info,
                    };
                    (f.span, level, f.message)
                })
                .collect();
            (findings, analysis.status)
        } else {
            let analysis = adp_editor_yaml::analyse(&text, &[name], read);
            let findings = analysis
                .findings
                .into_iter()
                .map(|f| {
                    let level = match f.severity {
                        adp_editor_yaml::Severity::Error => MarkLevel::Error,
                        adp_editor_yaml::Severity::Warning => MarkLevel::Warning,
                        adp_editor_yaml::Severity::Info => MarkLevel::Info,
                    };
                    (f.span, level, f.message)
                })
                .collect();
            (findings, analysis.status)
        };
        let char_at = |byte: usize| text.get(..byte.min(text.len())).map_or(0, |t| t.chars().count());
        self.editor.marks = findings
            .into_iter()
            .map(|(span, level, message)| Mark {
                start: char_at(span.start),
                end: char_at(span.end),
                level,
                message,
            })
            .collect();
        let analysis_status = status;
        let errors = self.editor.marks.iter().filter(|m| m.level == MarkLevel::Error).count();
        let warnings = self
            .editor
            .marks
            .iter()
            .filter(|m| m.level == MarkLevel::Warning)
            .count();
        self.status = format!("{analysis_status} · {errors} errors, {warnings} warnings");
    }
}

/// The window's state: its core and what the core last told it.
pub struct Shell {
    workspace: Workspace,
    events: Receiver<Event>,
    /// The last notice the core sent, shown in the status bar.
    notice: Option<String>,
    /// Events drained so far; for tests, and a cheap sign of life in the status bar.
    pub drained: usize,
    open: Option<OpenText>,
    highlighter: Option<Highlighter>,
}

impl Default for Shell {
    fn default() -> Self {
        let (workspace, events) = Workspace::open("", Vec::new());
        Self {
            workspace,
            events,
            notice: None,
            drained: 0,
            open: None,
            highlighter: None,
        }
    }
}

impl Shell {
    #[cfg(test)]
    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    /// Opens a text file in the editor, or shows why it cannot be opened.
    pub fn open_text(&mut self, path: &std::path::Path) {
        match TextFile::open(path) {
            Ok(file) => {
                let extension = path
                    .extension()
                    .map(|e| e.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let editor = TextEditor::new(file.content(), &extension, egui::Id::unique("text-editor"));
                self.notice = Some(format!("{} · {}", file.encoding_name(), file.line_ending_style()));
                self.open = Some(OpenText {
                    file,
                    editor,
                    analysed: None,
                    status: String::new(),
                    markdown: None,
                });
            }
            Err(reason) => self.notice = Some(reason),
        }
    }

    /// Saves the open text with Ctrl+S (Cmd+S on macOS), keeping its encoding and line endings.
    fn save(&mut self, ui: &egui::Ui) {
        if !ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
            return;
        }
        if let Some(open) = &mut self.open {
            let text = open.editor.buffer.text();
            match open.file.save(&text) {
                Ok(()) => {
                    open.editor.buffer.mark_saved();
                    self.notice = Some(format!("Saved {}", open.file.path().display()));
                }
                Err(reason) => self.notice = Some(reason),
            }
        }
    }

    /// Undo and redo from the keyboard: the platform's Z with and without Shift, and Y, as in the standalone host.
    fn shortcuts(&mut self, ui: &egui::Ui) {
        let (undo, redo) = ui.input_mut(|input| {
            let redo = input.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z)
                || input.consume_key(egui::Modifiers::COMMAND, egui::Key::Y);
            let undo = input.consume_key(egui::Modifiers::COMMAND, egui::Key::Z);
            (undo, redo)
        });
        let outcome = if undo {
            self.workspace.undo()
        } else if redo {
            self.workspace.redo()
        } else {
            return;
        };
        if let Outcome::Refused(reason) = outcome {
            self.notice = Some(reason);
        }
    }

    /// Takes every event the core has sent since the last frame, in order.
    pub fn drain(&mut self) {
        while let Ok(event) = self.events.try_recv() {
            self.drained += 1;
            if let Event::Notice { message } = event {
                self.notice = Some(message);
            }
        }
    }

    /// Draws one frame.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.drain();
        self.save(ui);
        if let Some(open) = &mut self.open {
            open.analyse();
        }
        let editor_focused = self
            .open
            .as_ref()
            .is_some_and(|o| ui.memory(|m| m.has_focus(o.editor.id())));
        if !editor_focused {
            self.shortcuts(ui);
        }
        let theme = theme::current(ui.ctx());
        let palette = Palette::of(&theme);
        let sizes = theme.sizes;

        egui::Panel::bottom("status-bar")
            .exact_size(sizes.status_bar_height)
            .resizable(false)
            .frame(
                Frame::NONE
                    .fill(palette.status_bar_background)
                    .inner_margin(Margin::symmetric(10, 0)),
            )
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    let text = self.notice.as_deref().unwrap_or("No project open");
                    ui.label(
                        RichText::new(text)
                            .color(palette.status_bar_foreground)
                            .size(sizes.workbench_font - 1.0),
                    );
                    if let Some(open) = self.open.as_ref().filter(|o| !o.status.is_empty()) {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new(&open.status)
                                    .color(palette.status_bar_foreground)
                                    .size(sizes.workbench_font - 1.0),
                            );
                        });
                    }
                });
            });

        egui::Panel::left("activity-bar")
            .exact_size(sizes.activity_bar_width)
            .resizable(false)
            .frame(Frame::NONE.fill(palette.activity_bar_background))
            .show(ui, |_ui| {});

        egui::Panel::left("side-bar")
            .default_size(260.0)
            .resizable(true)
            .frame(
                Frame::NONE
                    .fill(palette.side_bar_background)
                    .stroke(egui::Stroke::new(sizes.border_width, palette.border))
                    .inner_margin(Margin::symmetric(12, 8)),
            )
            .show(ui, |ui| {
                ui.label(
                    RichText::new("EXPLORER")
                        .color(palette.side_bar_title)
                        .size(sizes.workbench_font - 2.0),
                );
                ui.add_space(8.0);
                ui.label(RichText::new("No project open.").color(palette.side_bar_foreground));
            });

        if let Some(open) = &mut self.open {
            let highlighter = self.highlighter.get_or_insert_with(Highlighter::default);
            egui::Panel::top("tabs")
                .exact_size(sizes.tab_height)
                .resizable(false)
                .frame(Frame::NONE.fill(theme::color(
                    &theme,
                    &["editorGroupHeader.tabsBackground"],
                    palette.side_bar_background,
                )))
                .show(ui, |ui| {
                    let name = open
                        .file
                        .path()
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    let title = if open.editor.buffer.is_modified() {
                        format!("● {name}")
                    } else {
                        name
                    };
                    let tab = theme::color(&theme, &["tab.activeBackground"], palette.editor_background);
                    Frame::NONE
                        .fill(tab)
                        .inner_margin(Margin::symmetric(12, 8))
                        .show(ui, |ui| {
                            ui.label(RichText::new(title).color(theme::color(
                                &theme,
                                &["tab.activeForeground"],
                                palette.foreground,
                            )));
                        });
                });
            egui::CentralPanel::default()
                .frame(Frame::NONE.fill(palette.editor_background))
                .show(ui, |ui| match &open.markdown {
                    Some((headings, blocks)) => {
                        if !headings.is_empty()
                            && let Some(line) = crate::markdown::outline(ui, headings, &theme)
                        {
                            let offset = open.editor.buffer.offset(line - 1, 0);
                            open.editor.buffer.set_cursor(offset);
                            ui.memory_mut(|m| m.request_focus(open.editor.id()));
                        }
                        let blocks = blocks.clone();
                        ui.columns(2, |columns| {
                            open.editor.show(&mut columns[0], &theme, highlighter);
                            egui::Frame::NONE
                                .inner_margin(Margin::symmetric(16, 8))
                                .show(&mut columns[1], |ui| {
                                    crate::markdown::preview(ui, &blocks, &theme);
                                });
                        });
                    }
                    None => {
                        open.editor.show(ui, &theme, highlighter);
                    }
                });
        } else {
            egui::CentralPanel::default()
                .frame(
                    Frame::NONE
                        .fill(palette.editor_background)
                        .inner_margin(Margin::same(24)),
                )
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(ui.available_height() / 3.0);
                        ui.heading("ADP");
                        ui.label(format!("Desktop host {}", env!("CARGO_PKG_VERSION")));
                    });
                });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adp_context::{Reference, SelectMode, SelectionSource};

    /// One headless frame, as the window would draw it.
    fn frame(ctx: &egui::Context, shell: &mut Shell) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0))),
            ..Default::default()
        };
        ctx.run_ui(input, |ui| shell.show(ui)).textures_delta.clear();
    }

    #[test]
    fn a_frame_draws_and_drains_the_cores_events_in_order() {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let mut shell = Shell::default();
        frame(&ctx, &mut shell);
        assert_eq!(shell.drained, 0);
        shell
            .workspace()
            .select(vec![Reference::Project], SelectMode::Replace, SelectionSource::Explorer);
        // The event is sent before the call returns, so the next frame sees it.
        frame(&ctx, &mut shell);
        assert_eq!(shell.drained, 1);
    }

    #[test]
    fn undo_with_nothing_to_undo_says_so_in_the_status_bar() {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let mut shell = Shell::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0))),
            events: vec![egui::Event::Key {
                key: egui::Key::Z,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            }],
            ..Default::default()
        };
        ctx.run_ui(input, |ui| shell.show(ui)).textures_delta.clear();
        assert_eq!(shell.notice.as_deref(), Some("There is nothing to undo."));
    }

    #[test]
    fn a_json_file_is_checked_against_its_schema_and_its_findings_are_marked() {
        let examples =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../modules/editors/json/tests/examples");
        let mut shell = Shell::default();
        shell.open_text(&examples.join("service-broken.json"));
        let open = shell.open.as_mut().unwrap();
        open.analyse();
        assert!(
            open.editor.marks.iter().any(|m| m.level == MarkLevel::Error),
            "{:?}",
            open.editor.marks
        );
        assert!(!open.status.starts_with("No schema"), "{}", open.status);
    }

    #[test]
    fn both_themes_are_installed() {
        let ctx = egui::Context::default();
        theme::install(&ctx);
        for (kind, expected) in [
            (egui::Theme::Dark, adp_theme::Theme::dark()),
            (egui::Theme::Light, adp_theme::Theme::light()),
        ] {
            ctx.set_theme(kind);
            assert_eq!(
                ctx.global_style().visuals.panel_fill,
                theme::color32(expected.color("editor.background").unwrap())
            );
        }
    }
}
