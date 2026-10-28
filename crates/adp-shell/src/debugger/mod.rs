//! The debugger panel of the validation view (etalii.adp spec 035, story 6, T141; FR-070 to FR-072, contracts/debugger.md):
//! the description's text with a gutter of breakpoints, each with an optional CEL condition; a toolbar to start the
//! case under the debugger, continue, step over, into and out, and stop; and panes for the call stack, the scopes of
//! the selected frame, the transaction and the budget. The case runs on a worker thread on the description's own tool,
//! as validation runs it, so the window stays responsive while it is paused; a stop discards what the gesture did, and
//! no file of the description or of the case is written.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_atd_check::{FsFiles, Severity, check};
use adp_atd_runtime::RuntimeHost;
use adp_atd_script::debug::{
    Breakpoint, DebugControl, DebugEvent, Frame, PauseReason, Scope, ScopeKind, Session, StopReason, Variable,
    statement_lines, verify,
};
use adp_atd_validate::{read_case, validate_description};
use adp_theme::Theme;
use eframe::egui::{self, Frame as Panel, Key, KeyboardShortcut, Margin, Modifiers, RichText, accesskit::Role};

use crate::a11y;
use crate::theme::{self, Palette};

/// A file of the description: its name as spans give it, where it is, and its lines.
pub struct SourceFile {
    pub name: String,
    pub path: PathBuf,
    pub lines: Vec<String>,
}

/// Where the case under the debugger is.
#[derive(Debug, Clone, PartialEq)]
pub enum RunState {
    /// Not started, or ended.
    Idle,
    Running,
    Paused {
        reason: PauseReason,
        frames: Vec<Frame>,
        scopes: Vec<Scope>,
    },
}

/// A control of the toolbar: what it sends, its name, and its key.
struct Control {
    control: DebugControl,
    name: &'static str,
    shortcut: KeyboardShortcut,
}

const CONTROLS: [Control; 5] = [
    Control {
        control: DebugControl::Continue,
        name: "Continue",
        shortcut: KeyboardShortcut::new(Modifiers::NONE, Key::F5),
    },
    Control {
        control: DebugControl::StepOver,
        name: "Step Over",
        shortcut: KeyboardShortcut::new(Modifiers::NONE, Key::F10),
    },
    Control {
        control: DebugControl::StepInto,
        name: "Step Into",
        shortcut: KeyboardShortcut::new(Modifiers::NONE, Key::F11),
    },
    Control {
        control: DebugControl::StepOut,
        name: "Step Out",
        shortcut: KeyboardShortcut::new(Modifiers::SHIFT, Key::F11),
    },
    Control {
        control: DebugControl::Stop,
        name: "Stop",
        shortcut: KeyboardShortcut::new(Modifiers::SHIFT, Key::F5),
    },
];

/// Starts the case under the debugger: the platform's modifier key with F5.
const START: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::F5);

/// The debugger of one description and case.
pub struct DebuggerPanel {
    id: egui::Id,
    pub files: Vec<SourceFile>,
    /// The file the source pane shows, an index into `files`.
    pub shown: usize,
    pub breakpoints: Vec<Breakpoint>,
    pub state: RunState,
    /// The frame whose scopes show, an id of the paused call stack.
    pub selected: usize,
    /// What the run said: `notify`, conditions that did not evaluate, how it ended.
    pub output: Vec<String>,
    session: Option<Session>,
    /// The lines of the description's statements, by file: where a breakpoint is verified.
    statements: BTreeSet<(String, usize)>,
    /// The line the source pane scrolls to on the next frame: where the run paused.
    scroll_to: Option<usize>,
}

impl DebuggerPanel {
    pub fn new(id: egui::Id) -> Self {
        DebuggerPanel {
            id,
            files: Vec::new(),
            shown: 0,
            breakpoints: Vec::new(),
            state: RunState::Idle,
            selected: 0,
            output: Vec::new(),
            session: None,
            statements: BTreeSet::new(),
            scroll_to: None,
        }
    }

    /// The description's files as `root` checks now, keeping the breakpoints on them.
    pub fn load(&mut self, root: &Path) {
        let checked = check(&FsFiles, root);
        self.files = checked
            .description
            .files
            .iter()
            .map(|unit| SourceFile {
                name: unit.name.clone(),
                path: unit.path.clone(),
                lines: unit.text.lines().map(str::to_owned).collect(),
            })
            .collect();
        self.statements = checked.description.members().flat_map(statement_lines).collect();
        verify(&mut self.breakpoints, &self.statements);
        self.shown = self.shown.min(self.files.len().saturating_sub(1));
    }

    /// Sets a breakpoint at `line` of `file`, or removes the one there.
    pub fn toggle(&mut self, file: &str, line: usize) {
        match self.breakpoints.iter().position(|b| b.file == file && b.line == line) {
            Some(index) => {
                self.breakpoints.remove(index);
            }
            None => self.breakpoints.push(Breakpoint::new(file, line)),
        }
        verify(&mut self.breakpoints, &self.statements);
    }

    pub fn is_running(&self) -> bool {
        self.session.is_some()
    }

    /// Runs the case in `case` on the description `root` under the debugger, on a worker thread.
    pub fn start(&mut self, root: &Path, case: &Path) {
        self.load(root);
        self.output.clear();
        self.selected = 0;
        let (root, case) = (root.to_path_buf(), case.to_path_buf());
        self.session = Some(Session::start(self.breakpoints.clone(), false, move |debugger| {
            let output = debugger.output();
            let checked = check(&FsFiles, &root);
            if checked.findings.iter().any(|f| f.severity == Severity::Error) {
                output.send("The description does not check, so the case does not run.");
                return Ok(());
            }
            let case = match read_case(&case) {
                Ok(case) => case,
                Err(problem) => {
                    output.send(problem);
                    return Ok(());
                }
            };
            let host = RuntimeHost {
                debugger: Some(Rc::new(debugger)),
                ..RuntimeHost::default()
            };
            match validate_description(&checked, &host, std::slice::from_ref(&case)) {
                Ok(results) => {
                    for result in results {
                        output.send(if result.passed() {
                            format!("The case '{}' passes.", result.name)
                        } else {
                            format!(
                                "The case '{}' differs in {} place(s).",
                                result.name,
                                result.differences.len()
                            )
                        });
                    }
                }
                Err(problem) => output.send(problem),
            }
            Ok(())
        }));
        self.state = RunState::Running;
    }

    /// Sends a control of the toolbar; a stop ends the run, discarding what its gesture did.
    pub fn control(&mut self, control: DebugControl) {
        let Some(session) = &self.session else {
            return;
        };
        if control == DebugControl::Stop {
            // Dropping the session stops the gesture and waits for the worker to end.
            self.session = None;
            self.state = RunState::Idle;
            self.output
                .push("Stopped: what the gesture did is discarded.".to_owned());
            return;
        }
        if matches!(self.state, RunState::Paused { .. }) {
            session.control(control);
            self.state = RunState::Running;
        }
    }

    /// Takes what the worker sent since the last frame; whether anything came.
    pub fn poll(&mut self) -> bool {
        let Some(session) = &self.session else {
            return false;
        };
        let mut any = false;
        let mut ended = false;
        while let Some(event) = session.try_event() {
            any = true;
            match event {
                DebugEvent::Paused { reason, frames, scopes } => {
                    self.selected = 0;
                    if let Some(index) = frames
                        .first()
                        .and_then(|f| self.files.iter().position(|s| s.name == f.file))
                    {
                        self.shown = index;
                    }
                    self.scroll_to = frames.first().map(|f| f.line);
                    self.state = RunState::Paused { reason, frames, scopes };
                }
                DebugEvent::Continued => self.state = RunState::Running,
                DebugEvent::Output { text } => self.output.push(text),
                DebugEvent::ConditionFailed { file, line, message } => self
                    .output
                    .push(format!("The condition at {file}:{line} does not evaluate: {message}")),
                DebugEvent::Finished { committed } => {
                    ended = true;
                    if !committed {
                        self.output.push("The gesture did not land.".to_owned());
                    }
                }
                DebugEvent::Stopped { reason } => {
                    ended = true;
                    self.output.push(
                        match reason {
                            StopReason::User => "Stopped.",
                            StopReason::Budget => "Stopped: the gesture ran past its budget.",
                            StopReason::Recursion => "Stopped: a function recursed past its bound.",
                        }
                        .to_owned(),
                    );
                }
            }
        }
        if ended {
            self.session = None;
            self.state = RunState::Idle;
        }
        any
    }

    fn paused(&self) -> Option<(&[Frame], &[Scope])> {
        match &self.state {
            RunState::Paused { frames, scopes, .. } => Some((frames, scopes)),
            _ => None,
        }
    }
}

/// Draws the panel; `case` is the folder of the case to run, `root` the description's root file.
pub fn show(ui: &mut egui::Ui, panel: &mut DebuggerPanel, root: &Path, case: &Path, theme: &Theme, palette: &Palette) {
    if panel.poll() || panel.is_running() {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
    }
    if panel.files.is_empty() {
        panel.load(root);
    }
    let sizes = theme.sizes;
    let muted = theme::color(theme, &["descriptionForeground"], palette.foreground);
    let current_line = theme::color(
        theme,
        &["editor.stackFrameHighlightBackground", "editor.lineHighlightBackground"],
        egui::Color32::from_rgba_unmultiplied(255, 255, 0, 40),
    );
    let breakpoint_color = theme::color(
        theme,
        &["debugIcon.breakpointForeground", "errorForeground"],
        egui::Color32::RED,
    );

    // The keys, the toolbar's and the one that starts.
    let mut pressed = None;
    let mut start = false;
    ui.input_mut(|input| {
        if input.consume_shortcut(&START) {
            start = true;
        }
        for control in &CONTROLS {
            if input.consume_shortcut(&control.shortcut) {
                pressed = Some(control.control);
            }
        }
    });

    // The toolbar.
    let toolbar = a11y::region(Role::Toolbar, "Debug");
    ui.scope_builder(toolbar, |ui| {
        ui.horizontal(|ui| {
            let running = panel.is_running();
            let label = if running { "Debugging" } else { "Debug Case" };
            let shortcut = ui.ctx().format_shortcut(&START);
            let response = ui
                .add_enabled(!running, egui::Button::new(label))
                .on_hover_text(format!("Run the case under the debugger ({shortcut})"));
            a11y::button(&response, label, &shortcut);
            if response.clicked() {
                start = true;
            }
            let paused = panel.paused().is_some();
            for control in &CONTROLS {
                let enabled = if control.control == DebugControl::Stop {
                    running
                } else {
                    paused
                };
                let shortcut = ui.ctx().format_shortcut(&control.shortcut);
                let response = ui
                    .add_enabled(enabled, egui::Button::new(control.name))
                    .on_hover_text(format!("{} ({shortcut})", control.name));
                a11y::button(&response, control.name, &shortcut);
                if response.clicked() {
                    pressed = Some(control.control);
                }
            }
            let state = match &panel.state {
                RunState::Idle => "Not running".to_owned(),
                RunState::Running => "Running".to_owned(),
                RunState::Paused { reason, .. } => format!("Paused on {}", reason_name(*reason)),
            };
            let response = ui.label(RichText::new(&state).color(muted));
            a11y::describe(&response, Role::Status, &state);
        });
    });
    if start && !panel.is_running() {
        panel.start(root, case);
    }
    if let Some(control) = pressed {
        panel.control(control);
    }

    // The call stack, scopes, transaction and budget, right of the text.
    egui::Panel::right(panel.id.with("debug-panes"))
        .default_size(320.0)
        .min_size(200.0)
        .resizable(true)
        .frame(Panel::NONE.inner_margin(Margin::symmetric(8, 4)))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt((panel.id, "debug-panes-scroll"))
                .auto_shrink([false, false])
                .show(ui, |ui| panes(ui, panel, sizes.workbench_font, palette, muted));
        });

    // The text with its gutter.
    egui::CentralPanel::default()
        .frame(Panel::NONE.fill(palette.editor_background))
        .show(ui, |ui| {
            if panel.files.is_empty() {
                ui.label(RichText::new("The description has no files to show.").color(muted));
                return;
            }
            ui.horizontal(|ui| {
                let shown = panel.files[panel.shown].name.clone();
                let combo = egui::ComboBox::from_id_salt((panel.id, "file"))
                    .selected_text(&shown)
                    .show_ui(ui, |ui| {
                        for index in 0..panel.files.len() {
                            let name = panel.files[index].name.clone();
                            if ui.selectable_label(index == panel.shown, &name).clicked() {
                                panel.shown = index;
                            }
                        }
                    });
                a11y::describe(&combo.response, Role::ComboBox, &format!("File: {shown}"));
            });
            source(ui, panel, current_line, breakpoint_color, muted);
        });
}

fn reason_name(reason: PauseReason) -> &'static str {
    match reason {
        PauseReason::Breakpoint => "a breakpoint",
        PauseReason::Step => "a step",
        PauseReason::Entry => "entry",
        PauseReason::Pause => "pause",
    }
}

/// The shown file's lines, each with its gutter: a click there sets or removes a breakpoint.
fn source(
    ui: &mut egui::Ui,
    panel: &mut DebuggerPanel,
    current_line: egui::Color32,
    breakpoint_color: egui::Color32,
    muted: egui::Color32,
) {
    let file = &panel.files[panel.shown];
    let name = file.name.clone();
    let at = panel
        .paused()
        .and_then(|(frames, _)| frames.get(panel.selected).filter(|f| f.file == name).map(|f| f.line));
    let mut toggled = None;
    let rows = file.lines.len();
    let height = ui.text_style_height(&egui::TextStyle::Monospace);
    let row = height + ui.spacing().item_spacing.y;
    let lines = a11y::region(Role::List, &format!("Lines of {name}"));
    let scroll_to = panel.scroll_to.take();
    ui.scope_builder(lines, |ui| {
        let mut area = egui::ScrollArea::vertical()
            .id_salt((panel.id, "source", panel.shown))
            .auto_shrink([false, false]);
        if let Some(line) = scroll_to {
            area = area.vertical_scroll_offset(line.saturating_sub(6) as f32 * row);
        }
        area.show_rows(ui, height, rows, |ui, range| {
            for index in range {
                let line = index + 1;
                let text = &panel.files[panel.shown].lines[index];
                let breakpoint = panel.breakpoints.iter().find(|b| b.file == name && b.line == line);
                ui.horizontal(|ui| {
                    let mark = match breakpoint {
                        Some(b) if b.condition.is_some() => "◆",
                        Some(_) => "●",
                        None => " ",
                    };
                    let gutter = ui.add(
                        egui::Label::new(
                            RichText::new(format!("{mark}{line:>5}"))
                                .monospace()
                                .color(if breakpoint.is_some() { breakpoint_color } else { muted }),
                        )
                        .sense(egui::Sense::click()),
                    );
                    let state = match breakpoint {
                        Some(b) if !b.verified => ", breakpoint not on a statement",
                        Some(b) if b.condition.is_some() => ", conditional breakpoint",
                        Some(_) => ", breakpoint",
                        None => "",
                    };
                    let label = format!("Line {line}{state}");
                    a11y::describe(&gutter, Role::Button, &label);
                    if gutter.clicked() || gutter.has_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        toggled = Some(line);
                    }
                    let mut row = RichText::new(text).monospace();
                    if at == Some(line) {
                        row = row.background_color(current_line);
                    }
                    let response = ui.label(row);
                    if at == Some(line) {
                        a11y::current(&response);
                    }
                });
            }
        });
    });
    if let Some(line) = toggled {
        panel.toggle(&name, line);
    }
}

/// The breakpoints with their conditions, the call stack, the scopes of the selected frame, the transaction and the
/// budget, and what the run said.
fn panes(ui: &mut egui::Ui, panel: &mut DebuggerPanel, font: f32, palette: &Palette, muted: egui::Color32) {
    let heading = |ui: &mut egui::Ui, text: &str| {
        ui.add_space(6.0);
        ui.label(RichText::new(text).size(font - 2.0).color(palette.side_bar_title));
    };

    heading(ui, "BREAKPOINTS");
    if panel.breakpoints.is_empty() {
        ui.label(RichText::new("Click a line's number to set one.").color(muted));
    }
    let mut removed = None;
    for (index, breakpoint) in panel.breakpoints.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            let place = format!("{}:{}", breakpoint.file, breakpoint.line);
            let mut text = RichText::new(&place).monospace();
            if !breakpoint.verified {
                text = text.color(muted);
            }
            ui.label(text);
            let mut condition = breakpoint.condition.clone().unwrap_or_default();
            let field = ui.add(
                egui::TextEdit::singleline(&mut condition)
                    .hint_text("condition (CEL)")
                    .desired_width(140.0),
            );
            a11y::label(&field, &format!("Condition of the breakpoint at {place}"));
            if field.changed() {
                breakpoint.condition = (!condition.trim().is_empty()).then_some(condition);
            }
            let remove = ui.small_button("×");
            a11y::button(&remove, &format!("Remove the breakpoint at {place}"), "");
            if remove.clicked() {
                removed = Some(index);
            }
        });
    }
    if let Some(index) = removed {
        panel.breakpoints.remove(index);
    }

    let Some((frames, scopes)) = panel.paused().map(|(f, s)| (f.to_vec(), s.to_vec())) else {
        heading(ui, "OUTPUT");
        for line in &panel.output {
            ui.label(line);
        }
        return;
    };

    heading(ui, "CALL STACK");
    ui.scope_builder(a11y::region(Role::List, "Call stack"), |ui| {
        for frame in &frames {
            let name = format!("{}  {}:{}", frame.name, frame.file, frame.line);
            let selected = frame.id == panel.selected;
            let response = ui.selectable_label(selected, &name);
            a11y::describe(&response, Role::ListItem, &name);
            response
                .ctx
                .accesskit_node_builder(response.id, |node| node.set_selected(selected));
            if response.clicked() {
                panel.selected = frame.id;
            }
        }
    });

    for (kind, title) in [
        (ScopeKind::Locals, "LOCALS"),
        (ScopeKind::Arguments, "ARGUMENTS"),
        (ScopeKind::Elements, "ELEMENTS"),
        (ScopeKind::Transaction, "TRANSACTION"),
        (ScopeKind::Budget, "BUDGET"),
    ] {
        // The gesture's scopes are given with the innermost frame; a frame's own with it.
        let frame = if matches!(kind, ScopeKind::Locals | ScopeKind::Arguments) {
            panel.selected
        } else {
            0
        };
        let Some(scope) = scopes.iter().find(|s| s.kind == kind && s.frame == frame) else {
            continue;
        };
        heading(ui, title);
        let name = title.to_lowercase();
        ui.scope_builder(a11y::region(Role::Tree, &name), |ui| {
            if scope.variables.is_empty() {
                ui.label(RichText::new("(none)").color(muted));
            }
            for variable in &scope.variables {
                tree(ui, variable, 1, (panel.id, title));
            }
        });
    }

    heading(ui, "OUTPUT");
    for line in &panel.output {
        ui.label(line);
    }
}

/// A variable, its children under a fold.
fn tree(ui: &mut egui::Ui, variable: &Variable, level: usize, salt: (egui::Id, &str)) {
    let text = if variable.type_name.is_empty() {
        format!("{}: {}", variable.name, variable.value)
    } else {
        format!("{}: {} ({})", variable.name, variable.value, variable.type_name)
    };
    if variable.children.is_empty() {
        let response = ui.label(&text);
        a11y::tree_item(&response, &text, level, false, None);
        return;
    }
    let id = ui.make_persistent_id((salt, level, &variable.name, &variable.value));
    let state = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false);
    let open = state.is_open();
    state
        .show_header(ui, |ui| {
            let response = ui.label(&text);
            a11y::tree_item(&response, &text, level, false, Some(open));
        })
        .body(|ui| {
            for child in &variable.children {
                tree(ui, child, level + 1, salt);
            }
        });
}

#[cfg(test)]
mod tests;
