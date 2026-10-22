//! The add dialog (etalii.adp spec 030, Story 5 scenario 1, FR-021, task T068): the explorer's Add… on a folder and
//! Add as diagram… on a file a tool type reads, drawn as the standalone host's client draws it at 8b6ccfc
//! (`ChoicePromptDialog`): the tree of choices, one group per vendor, each tool type with its state; the description
//! of the chosen one; the name field, or the sentence that stands in for it; and Cancel and the confirm button. What
//! it offers and what it writes are [`adp_add`]'s.

use std::collections::HashSet;

use adp_add::{AddOption, AddPrompt, AddTarget, find, first_selectable};
use adp_core::ToolState;
use adp_theme::Theme;
use eframe::egui::{self, Color32, Key, RichText, Sense};

use crate::a11y;
use crate::theme::color;

/// What the person did with the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogOutcome {
    /// The option chosen, and the name typed when the option takes one.
    Submit {
        value: String,
        text: Option<String>,
    },
    Cancel,
}

/// One rendered row of the tree.
#[derive(Clone)]
struct VisibleRow {
    id: String,
    label: String,
    depth: usize,
    /// The group it is under; none at the top.
    parent: Option<String>,
    expandable: bool,
    open: bool,
    selectable: bool,
    unavailable: String,
    state: Option<ToolState>,
    first_child: Option<String>,
}

/// The dialog on screen (`ChoicePromptDialog`): the tree of choices, the description of the chosen one, the name
/// field and the buttons.
pub struct AddDialog {
    pub target: AddTarget,
    pub prompt: AddPrompt,
    pub expanded: HashSet<String>,
    pub selected: Option<String>,
    focused: Option<String>,
    pub name: String,
    /// Whether the person typed the name; until then choosing an option names the file after it.
    touched: bool,
    /// Why the typed name cannot be used.
    pub name_error: Option<String>,
    /// Why the last submission was refused; the dialog stays open with it.
    pub submit_error: Option<String>,
    tree_id: egui::Id,
    /// Whether the keyboard goes to the tree on the next frame, as it does when the dialog opens.
    focus_pending: bool,
    /// Where each row was drawn in the last frame, by option id.
    pub drawn: Vec<(String, egui::Rect)>,
}

impl AddDialog {
    pub fn new(target: AddTarget, prompt: AddPrompt) -> Self {
        // With one top-level group there is nothing to choose between, so it opens.
        let expanded = match prompt.options.as_slice() {
            [only] if !only.children.is_empty() => HashSet::from([only.id.clone()]),
            _ => HashSet::new(),
        };
        let name = if prompt.name_field.is_some() {
            first_selectable(&prompt.options)
                .map(|o| o.suggested_value.clone())
                .unwrap_or_default()
        } else {
            String::new()
        };
        AddDialog {
            target,
            prompt,
            expanded,
            selected: None,
            focused: None,
            name,
            touched: false,
            name_error: None,
            submit_error: None,
            tree_id: egui::Id::unique("add-dialog-tree"),
            focus_pending: true,
            drawn: Vec::new(),
        }
    }

    fn find(&self, id: &str) -> Option<&AddOption> {
        find(&self.prompt.options, id)
    }

    fn name_suppressed(&self) -> bool {
        self.selected
            .as_deref()
            .and_then(|id| self.find(id))
            .is_some_and(|o| !o.name_suppressed_reason.is_empty())
    }

    /// Chooses the option `id`, unless it is unavailable; until the person typed a name it names the file after it.
    pub fn choose(&mut self, id: &str) {
        let Some(option) = self.find(id) else { return };
        if !option.unavailable_reason.is_empty() || !option.selectable {
            return;
        }
        let suggestion = option.suggested_value.clone();
        self.selected = Some(id.to_owned());
        if self.prompt.name_field.is_some() && !self.touched {
            self.name = suggestion;
            self.name_error = None;
        }
    }

    /// The person typed `name`, judged by `validate`.
    pub fn type_name(&mut self, name: &str, validate: impl Fn(&str) -> Result<(), String>) {
        self.touched = true;
        self.name = name.to_owned();
        self.name_error = validate(name).err();
    }

    /// Whether the confirm button is enabled: an available option is chosen and its name, when it takes one, is fine.
    pub fn can_submit(&self) -> bool {
        let Some(selected) = self.selected.as_deref() else {
            return false;
        };
        let available = self.find(selected).is_some_and(|o| o.unavailable_reason.is_empty());
        let name_ok = self.prompt.name_field.is_none()
            || self.name_suppressed()
            || if self.touched {
                self.name_error.is_none()
            } else {
                !self.name.is_empty()
            };
        available && name_ok
    }

    /// The submission of what is chosen, without the name when the option takes none.
    pub fn submission(&self) -> Option<DialogOutcome> {
        let value = self.selected.clone()?;
        let text = (self.prompt.name_field.is_some() && !self.name_suppressed()).then(|| self.name.clone());
        Some(DialogOutcome::Submit { value, text })
    }

    /// The rows on screen, top to bottom: a closed group's children are not among them.
    fn rows(&self) -> Vec<VisibleRow> {
        fn walk(
            nodes: &[AddOption],
            depth: usize,
            parent: Option<&str>,
            expanded: &HashSet<String>,
            rows: &mut Vec<VisibleRow>,
        ) {
            for option in nodes {
                let open = !option.children.is_empty() && expanded.contains(&option.id);
                rows.push(VisibleRow {
                    id: option.id.clone(),
                    label: option.label.clone(),
                    depth,
                    parent: parent.map(str::to_owned),
                    expandable: !option.children.is_empty(),
                    open,
                    selectable: option.selectable,
                    unavailable: option.unavailable_reason.clone(),
                    state: option.state,
                    first_child: option.children.first().map(|c| c.id.clone()),
                });
                if open {
                    walk(&option.children, depth + 1, Some(&option.id), expanded, rows);
                }
            }
        }
        let mut rows = Vec::new();
        walk(&self.prompt.options, 0, None, &self.expanded, &mut rows);
        rows
    }

    fn toggle(&mut self, id: &str) {
        if !self.expanded.remove(id) {
            self.expanded.insert(id.to_owned());
        }
    }

    /// Draws the dialog's body and its buttons; what the person did, if anything.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        theme: &Theme,
        validate: impl Fn(&str) -> Result<(), String>,
    ) -> Option<DialogOutcome> {
        let mut outcome = None;
        if std::mem::take(&mut self.focus_pending) {
            self.focus_tree(ui.ctx());
        }
        // The tree keeps its arrows while it has the keyboard.
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                self.tree_id,
                egui::EventFilter {
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    ..Default::default()
                },
            );
        });
        ui.heading(&self.prompt.title);
        if self.prompt.options.is_empty() {
            ui.label(&self.prompt.empty_message);
        } else {
            if ui.memory(|m| m.has_focus(self.tree_id)) {
                outcome = self.keyboard(ui);
            }
            if let Some(confirmed) = self.tree(ui, theme) {
                outcome = Some(confirmed);
            }
            // What the chosen type is for; drawn even when empty so the dialog keeps its height.
            let description = self
                .selected
                .as_deref()
                .and_then(|id| self.find(id))
                .map(|o| o.description.clone());
            ui.label(RichText::new(description.unwrap_or_default()).small());
            if self.prompt.name_field.is_some() {
                let suppression = self
                    .selected
                    .as_deref()
                    .and_then(|id| self.find(id))
                    .map(|o| o.name_suppressed_reason.clone())
                    .filter(|r| !r.is_empty());
                match suppression {
                    Some(reason) => {
                        ui.label(reason);
                    }
                    None => {
                        let label = ui.label(self.prompt.name_field.clone().unwrap_or_default());
                        let mut name = self.name.clone();
                        let field = ui
                            .add(egui::TextEdit::singleline(&mut name).desired_width(f32::INFINITY))
                            .labelled_by(label.id);
                        if field.changed() {
                            self.type_name(&name, &validate);
                        }
                        if field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) && self.can_submit() {
                            outcome = self.submission();
                        }
                        if let Some(error) = &self.name_error {
                            ui.colored_label(ui.visuals().error_fg_color, error.as_str());
                        }
                    }
                }
            }
        }
        if let Some(error) = &self.submit_error {
            ui.colored_label(ui.visuals().error_fg_color, error.as_str());
        }
        ui.horizontal(|ui| {
            if ui.button("Cancel").clicked() {
                outcome = Some(DialogOutcome::Cancel);
            }
            let confirm = ui.add_enabled(self.can_submit(), egui::Button::new(&self.prompt.confirm_label));
            if confirm.clicked() {
                outcome = self.submission();
            }
        });
        outcome
    }

    /// The tree, a row per visible option: a chevron on a group, the label, an unavailable option's reason greyed and
    /// the tool type's state at the right. A double click on a choice confirms it.
    fn tree(&mut self, ui: &mut egui::Ui, theme: &Theme) -> Option<DialogOutcome> {
        let row_height = theme.sizes.row_height;
        let foreground = color(theme, &["foreground"], Color32::GRAY);
        let muted = color(theme, &["descriptionForeground"], foreground);
        let disabled = color(theme, &["disabledForeground"], muted);
        let selection = color(theme, &["list.activeSelectionBackground"], Color32::DARK_BLUE);
        let hover = color(theme, &["list.hoverBackground"], Color32::from_gray(40));
        let focus = color(theme, &["list.focusOutline", "focusBorder"], Color32::LIGHT_BLUE);
        let font = egui::FontId::proportional(theme.sizes.workbench_font);
        let has_focus = ui.memory(|m| m.has_focus(self.tree_id));
        let mut clicked: Option<(String, bool)> = None;
        let mut double_clicked: Option<String> = None;
        let mut drawn = Vec::new();
        let rows = self.rows();
        // To a screen reader the choices are a tree named as the dialog is, each row a tree item (T088).
        let tree = egui::UiBuilder::new()
            .accessibility_role(egui::accesskit::Role::Tree)
            .accessibility_label(self.prompt.title.clone());
        let scrolled = ui
            .scope_builder(tree, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt(self.tree_id)
                    .max_height(row_height * 14.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        for VisibleRow {
                            id,
                            label,
                            depth,
                            expandable,
                            open,
                            selectable,
                            unavailable,
                            state,
                            ..
                        } in &rows
                        {
                            let width = ui.available_width();
                            let (rect, response) =
                                ui.allocate_exact_size(egui::vec2(width, row_height), Sense::click());
                            drawn.push((id.clone(), rect));
                            let chosen = self.selected.as_deref() == Some(id);
                            a11y::tree_item(&response, label, depth + 1, chosen, expandable.then_some(*open));
                            // What the row says beside its name: why it cannot be chosen, and the tool type's state.
                            let besides: Vec<&str> = [unavailable.as_str(), state.map_or("", |s| s.label())]
                                .into_iter()
                                .filter(|t| !t.is_empty())
                                .collect();
                            if !besides.is_empty() {
                                a11y::describe_state(&response, &besides.join(", "));
                            }
                            if !unavailable.is_empty() {
                                response
                                    .ctx
                                    .accesskit_node_builder(response.id, |node| node.set_disabled());
                            }
                            if self.selected.as_deref() == Some(id) {
                                ui.painter().rect_filled(rect, 0.0, selection);
                            } else if response.hovered() && unavailable.is_empty() {
                                ui.painter().rect_filled(rect, 0.0, hover);
                            }
                            if has_focus && self.focused.as_deref() == Some(id) {
                                ui.painter().rect_stroke(
                                    rect.shrink(0.5),
                                    0.0,
                                    egui::Stroke::new(theme.sizes.focus_outline_width, focus),
                                    egui::StrokeKind::Inside,
                                );
                            }
                            let indent = 8.0 + *depth as f32 * 16.0;
                            if *expandable {
                                let centre = egui::pos2(rect.left() + indent + 4.0, rect.center().y);
                                let points = if *open {
                                    vec![
                                        centre + egui::vec2(-4.0, -2.0),
                                        centre + egui::vec2(4.0, -2.0),
                                        centre + egui::vec2(0.0, 3.0),
                                    ]
                                } else {
                                    vec![
                                        centre + egui::vec2(-2.0, -4.0),
                                        centre + egui::vec2(3.0, 0.0),
                                        centre + egui::vec2(-2.0, 4.0),
                                    ]
                                };
                                ui.painter()
                                    .add(egui::Shape::convex_polygon(points, foreground, egui::Stroke::NONE));
                            }
                            let text_colour = if unavailable.is_empty() { foreground } else { disabled };
                            let drawn = ui.painter().text(
                                egui::pos2(rect.left() + indent + 14.0, rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                label,
                                font.clone(),
                                text_colour,
                            );
                            if !unavailable.is_empty() {
                                ui.painter().text(
                                    egui::pos2(drawn.right() + 8.0, rect.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    unavailable,
                                    font.clone(),
                                    muted,
                                );
                            }
                            if let Some(state) = state {
                                ui.painter().text(
                                    egui::pos2(rect.right() - 8.0, rect.center().y),
                                    egui::Align2::RIGHT_CENTER,
                                    state.label(),
                                    font.clone(),
                                    muted,
                                );
                            }
                            if response.double_clicked() && unavailable.is_empty() {
                                double_clicked = Some(id.clone());
                            } else if response.clicked() {
                                let on_chevron = response
                                    .interact_pointer_pos()
                                    .is_some_and(|p| p.x < rect.left() + indent + 12.0);
                                clicked = Some((id.clone(), *expandable && (on_chevron || !*selectable)));
                            }
                        }
                    })
            })
            .inner;
        // The tree is one widget to the keyboard, which keeps it focused from frame to frame.
        ui.interact(scrolled.inner_rect, self.tree_id, Sense::focusable_noninteractive());
        self.drawn = drawn;
        if let Some((id, toggles)) = clicked {
            ui.memory_mut(|m| m.request_focus(self.tree_id));
            self.focused = Some(id.clone());
            if toggles {
                self.toggle(&id);
            } else {
                self.choose(&id);
            }
        }
        let id = double_clicked?;
        let option = self.find(&id)?;
        if !option.selectable {
            if !option.children.is_empty() {
                self.toggle(&id);
            }
            return None;
        }
        self.choose(&id);
        (self.prompt.name_field.is_none() || self.name_suppressed() || !self.name.is_empty())
            .then(|| self.submission())
            .flatten()
    }

    /// The tree's keys (`handleKeyDown`): the arrows move, open and close, Enter and Space choose, and Enter on the
    /// chosen option confirms it.
    fn keyboard(&mut self, ui: &egui::Ui) -> Option<DialogOutcome> {
        let pressed = |key: Key| ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key));
        let rows = self.rows();
        let index = self.focused.as_ref().and_then(|f| rows.iter().position(|r| &r.id == f));
        if pressed(Key::ArrowDown) {
            let next = index.map_or(0, |i| (i + 1).min(rows.len().saturating_sub(1)));
            self.focused = rows.get(next).map(|r| r.id.clone());
        }
        if pressed(Key::ArrowUp) {
            let next = index.map_or(0, |i| i.saturating_sub(1));
            self.focused = rows.get(next).map(|r| r.id.clone());
        }
        let VisibleRow {
            id,
            parent,
            expandable,
            open,
            selectable,
            first_child,
            ..
        } = index.and_then(|i| rows.get(i)).cloned()?;
        if pressed(Key::ArrowRight) && expandable {
            if open {
                self.focused = first_child;
            } else {
                self.toggle(&id);
            }
        } else if pressed(Key::ArrowLeft) {
            if expandable && open {
                self.toggle(&id);
            } else if parent.is_some() {
                self.focused = parent;
            }
        } else if pressed(Key::Enter) {
            if selectable {
                if self.selected.as_deref() == Some(id.as_str()) && self.can_submit() {
                    return self.submission();
                }
                self.choose(&id);
            } else if expandable {
                self.toggle(&id);
            }
        } else if pressed(Key::Space) {
            if selectable {
                self.choose(&id);
            } else if expandable {
                self.toggle(&id);
            }
        }
        None
    }

    /// Moves the keyboard to the tree, on the first row when none is focused.
    fn focus_tree(&mut self, ctx: &egui::Context) {
        ctx.memory_mut(|m| m.request_focus(self.tree_id));
        if self.focused.is_none() {
            self.focused = self.prompt.options.first().map(|o| o.id.clone());
        }
    }
}

#[cfg(test)]
mod tests;
