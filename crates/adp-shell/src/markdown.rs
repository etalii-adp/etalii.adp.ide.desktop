//! The Markdown editor's outline and preview drawn (etalii.adp spec 030, FR-020, task T030): the headings above the
//! text, each a button that moves the cursor to its line, and the rendered document beside the text, in VS Code's
//! Markdown preview colours.

use adp_editor_markdown::{Block, Heading, Span};
use adp_theme::Theme;
use eframe::egui::{self, Color32, FontFamily, FontId, RichText, Stroke, TextFormat, text::LayoutJob};

use crate::theme::color;

/// Draws the outline; returns the line (from 1) of the heading clicked, if any.
pub fn outline(ui: &mut egui::Ui, headings: &[Heading], theme: &Theme) -> Option<usize> {
    let mut clicked = None;
    let colour = color(theme, &["foreground"], Color32::GRAY);
    egui::ScrollArea::horizontal()
        .id_salt("markdown-outline")
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for heading in headings {
                    let label = format!("{} {}", "#".repeat(heading.level), heading.title);
                    if ui
                        .add(egui::Button::new(RichText::new(label).color(colour)).frame(false))
                        .clicked()
                    {
                        clicked = Some(heading.line);
                    }
                }
            });
        });
    clicked
}

/// Draws the rendered document.
pub fn preview(ui: &mut egui::Ui, blocks: &[Block], theme: &Theme) {
    let body = theme.sizes.workbench_font + 1.0;
    let foreground = color(theme, &["editor.foreground", "foreground"], Color32::GRAY);
    let link = color(theme, &["textLink.foreground"], Color32::LIGHT_BLUE);
    let code_background = color(theme, &["textCodeBlock.background"], Color32::from_gray(40));
    let code_foreground = color(theme, &["textPreformat.foreground"], foreground);
    let rule = color(theme, &["textSeparator.foreground", "panel.border"], Color32::DARK_GRAY);
    let quote_border = color(theme, &["textBlockQuote.border"], rule);
    egui::ScrollArea::vertical()
        .id_salt("markdown-preview")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            for block in blocks {
                match block {
                    Block::Heading { level, spans } => {
                        let size = match level {
                            1 => body * 2.0,
                            2 => body * 1.5,
                            3 => body * 1.25,
                            _ => body,
                        };
                        ui.add_space(4.0);
                        ui.label(job(
                            spans,
                            size,
                            foreground,
                            link,
                            code_foreground,
                            code_background,
                            ui.available_width(),
                        ));
                        if *level <= 2 {
                            let width = ui.available_width();
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 1.0), egui::Sense::hover());
                            ui.painter()
                                .hline(rect.x_range(), rect.center().y, Stroke::new(1.0, rule));
                        }
                    }
                    Block::Paragraph { spans, quote_depth } => {
                        if *quote_depth > 0 {
                            ui.horizontal(|ui| {
                                let (rect, _) =
                                    ui.allocate_exact_size(egui::vec2(4.0, body * 1.6), egui::Sense::hover());
                                ui.painter().rect_filled(rect, 0.0, quote_border);
                                ui.label(job(
                                    spans,
                                    body,
                                    foreground,
                                    link,
                                    code_foreground,
                                    code_background,
                                    ui.available_width(),
                                ));
                            });
                        } else {
                            ui.label(job(
                                spans,
                                body,
                                foreground,
                                link,
                                code_foreground,
                                code_background,
                                ui.available_width(),
                            ));
                        }
                    }
                    Block::Item { depth, number, spans } => {
                        ui.horizontal_wrapped(|ui| {
                            ui.add_space(16.0 * (*depth as f32 + 1.0));
                            let marker = number.map_or_else(|| "•".to_owned(), |n| format!("{n}."));
                            ui.label(RichText::new(marker).color(foreground).size(body));
                            ui.label(job(
                                spans,
                                body,
                                foreground,
                                link,
                                code_foreground,
                                code_background,
                                ui.available_width(),
                            ));
                        });
                    }
                    Block::Code { text, .. } => {
                        egui::Frame::NONE
                            .fill(code_background)
                            .inner_margin(egui::Margin::same(10))
                            .corner_radius(3)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.label(
                                    RichText::new(text)
                                        .font(FontId::new(theme.sizes.editor_font, FontFamily::Monospace))
                                        .color(code_foreground),
                                );
                            });
                    }
                    Block::Rule => {
                        let width = ui.available_width();
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 9.0), egui::Sense::hover());
                        ui.painter()
                            .hline(rect.x_range(), rect.center().y, Stroke::new(1.0, rule));
                    }
                }
            }
        });
}

#[allow(clippy::too_many_arguments)] // One call per block kind; a struct for the colours would only rename them.
fn job(
    spans: &[Span],
    size: f32,
    foreground: Color32,
    link: Color32,
    code_foreground: Color32,
    code_background: Color32,
    width: f32,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = width;
    for span in spans {
        let family = if span.code {
            FontFamily::Monospace
        } else {
            FontFamily::Proportional
        };
        let colour = if span.link.is_some() {
            link
        } else if span.code {
            code_foreground
        } else {
            foreground
        };
        let format = TextFormat {
            font_id: FontId::new(if span.code { size * 0.9 } else { size }, family),
            color: colour,
            italics: span.emphasis,
            background: if span.code {
                code_background
            } else {
                Color32::TRANSPARENT
            },
            underline: if span.link.is_some() {
                Stroke::new(1.0, link)
            } else {
                Stroke::NONE
            },
            ..Default::default()
        };
        job.append(&span.text, 0.0, format);
    }
    job
}
