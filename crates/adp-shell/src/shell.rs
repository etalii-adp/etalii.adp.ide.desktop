//! The window's frame (etalii.adp spec 030, task T021): the activity bar, the side bar with the explorer, the editor
//! area and the status bar, in VS Code's sizes and colours. The panes fill with their tasks of Story 1; until then the
//! frame shows where they go. The shell drains the core's events once per frame and changes nothing but through the
//! core's calls.

use std::sync::mpsc::Receiver;

use adp_core::{Event, Outcome, Workspace};
use eframe::egui::{self, Frame, Margin, RichText};

use crate::theme::{self, Palette};

/// The window's state: its core and what the core last told it.
pub struct Shell {
    workspace: Workspace,
    events: Receiver<Event>,
    /// The last notice the core sent, shown in the status bar.
    notice: Option<String>,
    /// Events drained so far; for tests, and a cheap sign of life in the status bar.
    pub drained: usize,
}

impl Default for Shell {
    fn default() -> Self {
        let (workspace, events) = Workspace::open("", Vec::new());
        Self {
            workspace,
            events,
            notice: None,
            drained: 0,
        }
    }
}

impl Shell {
    #[cfg(test)]
    pub fn workspace(&self) -> &Workspace {
        &self.workspace
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
        self.shortcuts(ui);
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
