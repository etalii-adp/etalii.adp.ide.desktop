//! The ribbon (etalii.adp spec 030, P-010, task T103): the standalone host's `RibbonBar.tsx` across the top of the
//! workspace. File's Save and New, the history's Undo and Redo, View's Zoom In, Zoom Out and Fit to View for the open
//! diagram, and then the contextual groups: the menu of what is selected on the canvas, one group per group of it, an
//! entry with children a drop-down of them. A button that cannot run now is disabled and says why. The ribbon only says
//! what was asked; the shell does it. Beside View stands a button per export a tool type's module offers (etalii.adp spec
//! 074), which the ribbon knows by what the module says of it and not by the tool type.

use adp_context::{Action, ActionGroup, Shortcut};
use adp_theme::Theme;
use eframe::egui::{self, RichText};

use crate::a11y;

/// What the person asked of the ribbon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Save,
    New,
    Undo,
    Redo,
    ZoomIn,
    ZoomOut,
    FitToView,
    /// An action of the selection's menu, by its id.
    Run(String),
    /// A file the active document's module writes, by the export's id (etalii.adp spec 074).
    Export(String),
}

/// One export's button: what it is called and says, and why it cannot run now, or none when it can.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportButton {
    pub id: String,
    pub label: String,
    pub hint: String,
    pub unavailable: Option<String>,
}

/// What the ribbon shows this frame, as the shell knows it.
pub struct Situation {
    /// Why Save cannot run now, or none when it can.
    pub save: Option<String>,
    /// Why Undo and Redo cannot run now, each, or none when they can.
    pub undo: Option<String>,
    pub redo: Option<String>,
    /// Whether a diagram is open, which View zooms and fits.
    pub diagram: bool,
    /// What is selected on the canvas, as a key that changes with it, and its menu.
    pub selection: Option<String>,
    pub actions: Vec<ActionGroup>,
    /// A button per export any tool type offers (etalii.adp spec 074, FR-001, FR-002).
    pub exports: Vec<ExportButton>,
}

/// The sentence a View button says with no diagram open (`RibbonViewGroup`).
pub const NO_DIAGRAM: &str = "Open a diagram to use this.";

/// The ribbon, which keeps the contextual groups it last showed.
#[derive(Default)]
pub struct Ribbon {
    shown: Vec<ActionGroup>,
    shown_for: Option<String>,
}

impl Ribbon {
    /// Draws the ribbon; what it was asked, if anything.
    pub fn show(&mut self, ui: &mut egui::Ui, theme: &Theme, situation: &Situation) -> Option<Command> {
        // A contextual button, once shown, stays where it is when the selection goes away, disabled, rather than the
        // ribbon changing shape under the pointer (`RibbonContextualGroups`).
        let current =
            situation.selection.is_some() && (!situation.actions.is_empty() || self.shown_for == situation.selection);
        if current {
            self.shown = situation.actions.clone();
            self.shown_for = situation.selection.clone();
        }
        let size = theme.sizes.workbench_font - 1.0;
        let mut asked = None;
        ui.horizontal(|ui| {
            let mut button =
                |ui: &mut egui::Ui, label: &str, hint: &str, unavailable: Option<&str>, command: Command| {
                    let response = ui.add_enabled(
                        unavailable.is_none(),
                        egui::Button::new(RichText::new(label).size(size)),
                    );
                    if let Some(reason) = unavailable {
                        a11y::describe_state(&response, reason);
                    }
                    let response = response
                        .on_hover_text(hint)
                        .on_disabled_hover_text(unavailable.unwrap_or(""));
                    if response.clicked() {
                        asked = Some(command);
                    }
                };
            button(ui, "Save", "Save (Ctrl+S)", situation.save.as_deref(), Command::Save);
            button(ui, "New", "New…", None, Command::New);
            ui.separator();
            button(ui, "Undo", "Undo (Ctrl+Z)", situation.undo.as_deref(), Command::Undo);
            button(ui, "Redo", "Redo (Ctrl+Y)", situation.redo.as_deref(), Command::Redo);
            ui.separator();
            let view = (!situation.diagram).then_some(NO_DIAGRAM);
            button(ui, "Zoom In", "Zoom In", view, Command::ZoomIn);
            button(ui, "Zoom Out", "Zoom Out", view, Command::ZoomOut);
            button(ui, "Fit to View", "Fit to View", view, Command::FitToView);
            if !situation.exports.is_empty() {
                ui.separator();
            }
            for export in &situation.exports {
                button(
                    ui,
                    &export.label,
                    &export.hint,
                    export.unavailable.as_deref(),
                    Command::Export(export.id.clone()),
                );
            }
            for group in self.shown.iter().filter(|g| !g.actions.is_empty()) {
                ui.separator();
                for action in &group.actions {
                    if let Some(id) = contextual(ui, action, current, size) {
                        asked = Some(Command::Run(id));
                    }
                }
            }
        });
        asked
    }
}

/// One contextual entry: a button, or a drop-down of its children; the id of what was chosen.
fn contextual(ui: &mut egui::Ui, action: &Action, current: bool, size: f32) -> Option<String> {
    let tooltip = tooltip(action);
    if action.items.is_empty() {
        let enabled = current && action.is_available();
        let response = ui.add_enabled(enabled, egui::Button::new(RichText::new(&action.label).size(size)));
        if !action.is_available() {
            a11y::describe_state(&response, &action.unavailable_reason);
        }
        let response = response.on_hover_text(&tooltip).on_disabled_hover_text(&tooltip);
        return response.clicked().then(|| action.id.clone());
    }
    // A disabled parent may still open its children, so only a held-over one is inert.
    let response = ui.add_enabled(
        current,
        egui::Button::new(RichText::new(format!("{} ⏷", action.label)).size(size)),
    );
    a11y::describe(&response, egui::accesskit::Role::Button, &action.label);
    response
        .ctx
        .accesskit_node_builder(response.id, |node| node.set_has_popup(egui::accesskit::HasPopup::Menu));
    let response = response.on_hover_text(&tooltip);
    let mut chosen = None;
    egui::Popup::menu(&response)
        .accessible_name(&action.label)
        .show(|ui| crate::canvas::menu_entries(ui, &action.items, response.id, &mut chosen));
    chosen
}

/// A button's tooltip: the reason when it cannot run, else its label with its shortcut (`tooltipFor`).
fn tooltip(action: &Action) -> String {
    if !action.is_available() && !action.unavailable_reason.is_empty() {
        return action.unavailable_reason.clone();
    }
    match &action.shortcut {
        Some(shortcut) => format!("{} ({})", action.label, shortcut_text(shortcut)),
        None => action.label.clone(),
    }
}

/// "Ctrl+Shift+X": a shortcut as a tooltip reads it (`formatShortcut`).
fn shortcut_text(shortcut: &Shortcut) -> String {
    let mut parts = Vec::new();
    if shortcut.primary {
        parts.push("Ctrl".to_owned());
    }
    if shortcut.alt {
        parts.push("Alt".to_owned());
    }
    if shortcut.shift {
        parts.push("Shift".to_owned());
    }
    parts.push(match shortcut.key.as_str() {
        " " => "Space".to_owned(),
        "ArrowUp" => "Up".to_owned(),
        "ArrowDown" => "Down".to_owned(),
        "ArrowLeft" => "Left".to_owned(),
        "ArrowRight" => "Right".to_owned(),
        key if key.chars().count() == 1 => key.to_uppercase(),
        key => key.to_owned(),
    });
    parts.join("+")
}
