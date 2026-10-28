//! The validation view (etalii.adp spec 035, story 4, T129; FR-041, FR-044, research R21): a tool description opened
//! with one of its conformance cases. The left pane runs the description's own tool on a working copy of the case's
//! document, held in memory, with its canvas, toolbox, forms, operations and refusals; the right pane lists where the
//! description differs from the case, each difference linked to the construct of the `.atd` that produced it. A save
//! of any file of the description checks it again, and runs the case again when it checks clean; when it does not,
//! the last result stays and the findings show above it. No file of the description or of the case is ever written.

use std::path::{Path, PathBuf};

use adp_atd_check::{Checked, Finding, FsFiles, Severity, check};
use adp_atd_runtime::{RuntimeHost, diagram_tool};
use adp_atd_validate::{CaseResult, cases_of, read_case, validate_description};
use adp_diagram::workspace::{DiagramWorkspace, OpenedDiagram};
use adp_theme::Theme;
use eframe::egui::{self, Frame, Margin, RichText, accesskit::Role};

use crate::a11y;
use crate::canvas::DiagramTab;
use crate::debugger::{self, DebuggerPanel};
use crate::theme::{self, Palette};

/// Where the working copy's documents are named: never read or written.
const WORKING_ROOT: &str = "/validation";

/// A construct of the description a difference or a finding points at: its file and line, from 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub file: PathBuf,
    pub line: usize,
}

/// The working copy of the case's document: its own in-memory workspace and the canvas over it.
struct Working {
    workspace: DiagramWorkspace,
    tab: DiagramTab,
}

/// Which side pane the working copy shows beside its canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pane {
    Toolbox,
    Properties,
}

/// A tab with a description, one of its cases, the working copy and the last result.
pub struct ValidationTab {
    /// The description's root file.
    pub path: PathBuf,
    id: egui::Id,
    /// The cases beside the description, by name, with their folders.
    pub cases: Vec<(String, PathBuf)>,
    /// The case shown, an index into `cases`.
    pub case: usize,
    /// Every file of the description, whose saves are followed.
    files: Vec<PathBuf>,
    /// The last result: the differences of the case shown, from the last version that checked clean.
    pub result: Option<CaseResult>,
    /// The findings of the last check, when it did not check clean: shown above the last result, which stays.
    pub findings: Vec<Finding>,
    /// Why the case could not be run, when it could not.
    pub problem: Option<String>,
    /// How many times the case has run, for a test to see that a save ran it again.
    pub runs: usize,
    working: Option<Working>,
    pane: Pane,
    pub notice: Option<String>,
    /// Whether the debugger shows in place of the working copy (T141).
    pub debugging: bool,
    pub debugger: DebuggerPanel,
}

impl ValidationTab {
    /// Opens the description `root` with its first case; refused when it has none.
    pub fn open(root: &Path) -> Result<ValidationTab, String> {
        let cases: Vec<(String, PathBuf)> = cases_of(root)
            .into_iter()
            .map(|folder| {
                let name = folder
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                (name, folder)
            })
            .collect();
        if cases.is_empty() {
            return Err(format!(
                "{} has no conformance cases in conformance/<type>/<case>/ beside it.",
                root.file_name().unwrap_or_default().to_string_lossy()
            ));
        }
        let id = egui::Id::unique(("validation", root.to_path_buf()));
        let mut tab = ValidationTab {
            path: root.to_path_buf(),
            id,
            cases,
            case: 0,
            files: vec![root.to_path_buf()],
            result: None,
            findings: Vec::new(),
            problem: None,
            runs: 0,
            working: None,
            pane: Pane::Toolbox,
            notice: None,
            debugging: false,
            debugger: DebuggerPanel::new(id.with("debugger")),
        };
        tab.run();
        Ok(tab)
    }

    pub fn id(&self) -> egui::Id {
        self.id
    }

    pub fn name(&self) -> String {
        format!(
            "{} — {}",
            self.path.file_name().unwrap_or_default().to_string_lossy(),
            self.cases[self.case].0
        )
    }

    pub fn title(&self) -> String {
        format!("Validate {}", self.name())
    }

    /// Whether this tab shows the description `path` is the root of.
    pub fn shows(&self, path: &Path) -> bool {
        self.path == path
    }

    /// Shows another case of the description, run at once.
    pub fn choose(&mut self, case: usize) {
        if case < self.cases.len() && case != self.case {
            self.case = case;
            self.result = None;
            self.run();
        }
    }

    /// A file changed on disk: when it is one of the description's, the description is checked again, and the case
    /// run again when it checks clean (FR-044).
    pub fn changed_on_disk(&mut self, path: &Path) {
        let same = |f: &PathBuf| f == path || f.canonicalize().ok() == path.canonicalize().ok();
        if self.files.iter().any(same) || self.cases[self.case].1.starts_with(path.parent().unwrap_or(path)) {
            self.run();
        }
    }

    /// Checks the description and, when it checks clean, runs the case shown on a fresh working copy.
    pub fn run(&mut self) {
        let checked = check(&FsFiles, &self.path);
        self.files = checked.description.files.iter().map(|f| f.path.clone()).collect();
        if self.files.is_empty() {
            self.files.push(self.path.clone());
        }
        let errors: Vec<Finding> = checked
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .cloned()
            .collect();
        if !errors.is_empty() {
            // The last result stays, from the last version that checked clean, with what keeps this one from running.
            self.findings = errors;
            return;
        }
        self.findings.clear();
        let case = match read_case(&self.cases[self.case].1) {
            Ok(case) => case,
            Err(problem) => {
                self.problem = Some(problem);
                return;
            }
        };
        let host = RuntimeHost::default();
        match validate_description(&checked, &host, std::slice::from_ref(&case)) {
            Ok(mut results) => {
                self.problem = None;
                self.result = results.pop();
                self.runs += 1;
            }
            Err(problem) => {
                self.problem = Some(problem);
                return;
            }
        }
        self.working = working_copy(&checked, &host, &case)
            .map_err(|p| self.notice = Some(p))
            .ok();
    }

    /// The place a difference's construct is in, as a file of the description and a line.
    fn place(&self, file: &str, line: usize) -> Place {
        let folder = self.path.parent().unwrap_or(Path::new("."));
        let file = self
            .files
            .iter()
            .find(|f| f.file_name().is_some_and(|n| n.to_string_lossy() == file) || f.ends_with(file))
            .cloned()
            .unwrap_or_else(|| folder.join(file));
        Place { file, line }
    }
}

/// The case's document, opened from its bytes in a workspace of its own, on the description's own tool.
fn working_copy(checked: &Checked, host: &RuntimeHost, case: &adp_atd_validate::Case) -> Result<Working, String> {
    let symbols = checked.symbols.as_ref().ok_or("The description was not checked.")?;
    let runtime = diagram_tool(&checked.description, symbols, host)?;
    let inputs = &case.fixture["inputs"];
    let body_name = inputs["body"].as_str().ok_or("The case names no body.")?;
    let registration_name = inputs["registration"].as_str();
    let body_path = Path::new(WORKING_ROOT).join(body_name);
    let registration_path = registration_name.map(|r| Path::new(WORKING_ROOT).join(r));
    let mut workspace = DiagramWorkspace::in_memory(WORKING_ROOT);
    let session = workspace.open_bytes(
        runtime.tool.clone(),
        &body_path,
        registration_path.as_deref(),
        case.files.get(body_name).cloned().unwrap_or_default(),
        registration_name.and_then(|r| case.files.get(r).cloned()),
    );
    let opened = OpenedDiagram {
        origin: checked.description.origin().unwrap_or_default().to_owned(),
        body_path: body_path.clone(),
        registration_path,
    };
    let tab = DiagramTab::new(Path::new(WORKING_ROOT), &body_path, opened, session)?;
    Ok(Working { workspace, tab })
}

/// Draws the view; the construct the person asked to go to, if any.
pub fn show(ui: &mut egui::Ui, tab: &mut ValidationTab, theme: &Theme, palette: &Palette) -> Option<Place> {
    let sizes = theme.sizes;
    let muted = theme::color(theme, &["descriptionForeground"], palette.foreground);
    let error = theme::color(
        theme,
        &["editorError.foreground", "errorForeground"],
        egui::Color32::RED,
    );
    let passed = theme::color(
        theme,
        &["testing.iconPassed", "terminal.ansiGreen"],
        egui::Color32::GREEN,
    );
    let mut go_to = None;
    let mut chosen = None;

    // The case picker, above both panes.
    egui::Frame::NONE.inner_margin(Margin::symmetric(8, 4)).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("CASE").size(sizes.workbench_font - 2.0).color(muted));
            let current = tab.cases[tab.case].0.clone();
            let combo = egui::ComboBox::from_id_salt((tab.id, "case"))
                .selected_text(&current)
                .show_ui(ui, |ui| {
                    for (index, (name, _)) in tab.cases.iter().enumerate() {
                        if ui.selectable_label(index == tab.case, name).clicked() {
                            chosen = Some(index);
                        }
                    }
                });
            a11y::describe(&combo.response, Role::ComboBox, &format!("Case: {current}"));
            if ui
                .button("Run Again")
                .on_hover_text("Check the description and run the case again")
                .clicked()
            {
                tab.run();
            }
            let label = if tab.debugging { "Hide Debugger" } else { "Debugger" };
            let response = ui
                .selectable_label(tab.debugging, label)
                .on_hover_text("Run the case under the debugger, with breakpoints in the description");
            a11y::describe(&response, Role::Button, "Debugger");
            response
                .ctx
                .accesskit_node_builder(response.id, |node| node.set_toggled(tab.debugging.into()));
            if response.clicked() {
                tab.debugging = !tab.debugging;
            }
        });
    });
    if let Some(index) = chosen {
        tab.choose(index);
    }
    if tab.debugging {
        let case = tab.cases[tab.case].1.clone();
        let root = tab.path.clone();
        egui::CentralPanel::default().frame(Frame::NONE).show(ui, |ui| {
            debugger::show(ui, &mut tab.debugger, &root, &case, theme, palette)
        });
        return go_to;
    }

    // The differences, right.
    egui::Panel::right(tab.id.with("differences"))
        .default_size(380.0)
        .min_size(220.0)
        .resizable(true)
        .frame(
            Frame::NONE
                .fill(palette.side_bar_background)
                .stroke(egui::Stroke::new(sizes.border_width, palette.border))
                .inner_margin(Margin::symmetric(12, 8)),
        )
        .show(ui, |ui| {
            ui.label(
                RichText::new("DIFFERENCES")
                    .size(sizes.workbench_font - 2.0)
                    .color(palette.side_bar_title),
            );
            let list = egui::UiBuilder::new()
                .accessibility_role(Role::List)
                .accessibility_label("Differences");
            ui.scope_builder(list, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt((tab.id, "differences-list"))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if !tab.findings.is_empty() {
                            ui.label(
                                RichText::new("The description does not check, so the last result below is kept:")
                                    .color(error),
                            );
                            for finding in tab.findings.clone() {
                                let text =
                                    format!("{}:{} {} {}", finding.file, finding.line, finding.code, finding.message);
                                let response = ui.add(
                                    egui::Label::new(RichText::new(&text).color(error)).sense(egui::Sense::click()),
                                );
                                a11y::describe(&response, Role::ListItem, &text);
                                if response.clicked()
                                    || response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                {
                                    go_to = Some(tab.place(&finding.file, finding.line));
                                }
                            }
                            ui.separator();
                        }
                        if let Some(problem) = &tab.problem {
                            ui.label(RichText::new(problem).color(error));
                        }
                        let Some(result) = tab.result.clone() else {
                            ui.label(RichText::new("The case has not run.").color(muted));
                            return;
                        };
                        if result.passed() {
                            ui.label(RichText::new(format!("The case '{}' passes.", result.name)).color(passed));
                            return;
                        }
                        ui.label(
                            RichText::new(format!(
                                "The case '{}' differs in {} place(s).",
                                result.name,
                                result.differences.len()
                            ))
                            .color(muted),
                        );
                        for difference in &result.differences {
                            let step = difference.step.map_or("read".to_owned(), |s| format!("step {s}"));
                            let at = difference
                                .at
                                .as_ref()
                                .map(|a| format!("{}:{}", a.file, a.line))
                                .unwrap_or_default();
                            ui.add_space(4.0);
                            let heading = format!("{step}, {}", difference.part);
                            let response = ui.add(
                                egui::Label::new(RichText::new(format!("{heading}  {at}")).strong())
                                    .sense(egui::Sense::click()),
                            );
                            a11y::describe(
                                &response,
                                Role::ListItem,
                                &format!(
                                    "{heading}, at {at}: expected {}, got {}",
                                    difference.expected, difference.actual
                                ),
                            );
                            let activated = response.clicked()
                                || response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                            if activated && let Some(a) = &difference.at {
                                go_to = Some(tab.place(&a.file, a.line));
                            }
                            ui.label(
                                RichText::new(format!("expected {}", difference.expected))
                                    .color(muted)
                                    .small(),
                            );
                            ui.label(
                                RichText::new(format!("got      {}", difference.actual))
                                    .color(muted)
                                    .small(),
                            );
                        }
                    });
            });
        });

    // The working copy, left: its toolbox and properties beside its canvas.
    let Some(working) = tab.working.as_mut() else {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.centered_and_justified(|ui| ui.label(RichText::new("The case's document has not opened.").color(muted)));
        });
        return go_to;
    };
    let pane = &mut tab.pane;
    egui::Panel::right(tab.id.with("working-panes"))
        .default_size(240.0)
        .min_size(160.0)
        .resizable(true)
        .frame(
            Frame::NONE
                .fill(palette.side_bar_background)
                .inner_margin(Margin::symmetric(8, 8)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for (which, label, name) in [
                    (Pane::Toolbox, "TOOLBOX", "Toolbox"),
                    (Pane::Properties, "PROPERTIES", "Properties"),
                ] {
                    let active = *pane == which;
                    let response = ui.add(
                        egui::Label::new(RichText::new(label).size(sizes.workbench_font - 2.0).color(if active {
                            palette.side_bar_title
                        } else {
                            palette.side_bar_title.gamma_multiply(0.6)
                        }))
                        .sense(egui::Sense::click()),
                    );
                    a11y::tab(&response, name, active);
                    if response.clicked() {
                        *pane = which;
                    }
                    ui.add_space(12.0);
                }
            });
            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| match *pane {
                    Pane::Toolbox => crate::canvas::toolbox::show(ui, &working.tab, &working.workspace, theme, palette),
                    Pane::Properties => {
                        crate::properties::show(ui, &mut working.tab, &mut working.workspace, theme, palette)
                    }
                });
        });
    egui::CentralPanel::default()
        .frame(Frame::NONE.fill(palette.editor_background))
        .show(ui, |ui| working.tab.show(ui, &mut working.workspace, theme, palette));
    if let Some(notice) = working.tab.notice.take() {
        tab.notice = Some(notice);
    }
    go_to
}

#[cfg(test)]
mod tests;
