//! The errors and warnings panel (etalii.adp spec 030, FR-019, task T034): the project's findings from its problem
//! cache, validated by the same analyses the editors run, each row opening its file at its line.

use std::path::Path;

use adp_problems::{Finding, ProblemSet, SetState, Severity, Validator};
use adp_theme::Theme;
use eframe::egui::{self, Color32, RichText};

use crate::documents::{analyse, analysed, document_path, extension_of};
use crate::editor::MarkLevel;
use crate::theme::color;

/// The editors' analyses as a validator of the project's files.
pub struct EditorValidator;

impl Validator for EditorValidator {
    fn applies_to(&self, file: &str) -> bool {
        analysed(&extension_of(Path::new(file)))
    }

    fn validate(&self, root: &Path, file: &str) -> Vec<Finding> {
        let path = root.join(file);
        let Ok(text) = adp_documents::TextFile::open(&path) else {
            return Vec::new();
        };
        let content = text.content();
        let (found, _) = analyse(
            &extension_of(&path),
            content,
            root,
            &document_path(root, &path),
            text.encoding_name(),
        );
        found
            .into_iter()
            .map(|f| {
                let before = &content[..f.span.start.min(content.len())];
                let line = before.matches('\n').count() as u32 + 1;
                let column = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) as u32 + 1;
                Finding {
                    severity: match f.level {
                        MarkLevel::Error => Severity::Error,
                        MarkLevel::Warning => Severity::Warning,
                        MarkLevel::Info => Severity::Info,
                    },
                    message: f.message,
                    file: file.to_owned(),
                    line: Some(line),
                    column: Some(column),
                    element: None,
                    rule: format!("editor.{}", extension_of(&path)),
                }
            })
            .collect()
    }
}

/// Draws the panel; returns the file and line the person double-clicked.
pub fn show(ui: &mut egui::Ui, set: &ProblemSet, theme: &Theme) -> Option<(String, u32)> {
    let error = color(theme, &["editorError.foreground", "errorForeground"], Color32::RED);
    let warning = crate::theme::warning(theme);
    let info = color(theme, &["editorInfo.foreground"], Color32::LIGHT_BLUE);
    let muted = color(theme, &["descriptionForeground"], Color32::GRAY);
    let mut open = None;
    if set.state == SetState::NeverValidated {
        ui.label(
            RichText::new("This project has not been validated yet: right-click the explorer and choose Validate All.")
                .color(muted),
        );
        return None;
    }
    if set.problems.is_empty() {
        ui.label(RichText::new("No problems have been detected in the workspace.").color(muted));
        return None;
    }
    egui::ScrollArea::vertical()
        .id_salt("problems")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for reported in &set.problems {
                let finding = &reported.finding;
                let (mark, colour) = match finding.severity {
                    Severity::Error => ("Error", error),
                    Severity::Warning => ("Warning", warning),
                    _ => ("Info", info),
                };
                let place = match (finding.line, finding.column) {
                    (Some(l), Some(c)) => format!(" [Ln {l}, Col {c}]"),
                    _ => String::new(),
                };
                let stale = if reported.stale {
                    " (changed since it was checked)"
                } else {
                    ""
                };
                let response = ui.add(
                    egui::Label::new(
                        RichText::new(format!("{mark}: {}  {}{place}{stale}", finding.message, finding.file))
                            .color(colour),
                    )
                    .sense(egui::Sense::click()),
                );
                crate::a11y::describe(
                    &response,
                    egui::accesskit::Role::ListItem,
                    &format!("{mark}: {}  {}{place}{stale}", finding.message, finding.file),
                );
                if response.double_clicked() {
                    open = Some((finding.file.clone(), finding.line.unwrap_or(1)));
                }
            }
            if set.truncated_at > 0 {
                ui.label(RichText::new(format!("Only the first {} are shown.", set.truncated_at)).color(muted));
            }
        });
    open
}
