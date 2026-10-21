//! The window (etalii.adp spec 030, Story 1, tasks T021, T034, T035): the project list when no project is open; with
//! one open, the activity bar, the explorer, the tabs and their editors, the errors and warnings panel and the status
//! bar, in VS Code's sizes and colours. The shell drains the core's events once per frame, follows the disk, and
//! asks before anything it cannot undo.

use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;

use adp_core::state::StateFolder;
use adp_core::{Event, Outcome, Workspace};
use adp_hierarchy::{Change, Hierarchy, Kind, Watcher, validate_name};
use adp_problems::{ProblemSet, ProblemStore, Scope};
use adp_projects::{Project, ProjectList};
use eframe::egui::{self, Frame, Margin, RichText};

use crate::documents::OpenText;
use crate::explorer::{Explorer, Request};
use crate::highlight::Highlighter;
use crate::theme::{self, Palette};

/// A project open in the window: its tree, kept in step with the disk, and its findings.
struct OpenProject {
    project: Project,
    tree: Hierarchy,
    watcher: Option<Watcher>,
    explorer: Explorer,
    problems: Option<ProblemStore>,
    problem_set: Option<ProblemSet>,
}

/// A question the window is waiting on the person to answer.
enum Pending {
    NewFolder {
        parent: String,
        name: String,
        error: Option<String>,
    },
    Rename {
        path: String,
        name: String,
        error: Option<String>,
    },
    Delete {
        path: String,
        question: String,
    },
    CloseUnsaved {
        index: usize,
    },
}

/// The window's state.
pub struct Shell {
    workspace: Workspace,
    events: Receiver<Event>,
    /// The last notice, shown in the status bar.
    notice: Option<String>,
    /// Events drained so far; for tests.
    pub drained: usize,
    highlighter: Option<Highlighter>,
    state: Option<StateFolder>,
    projects: Option<ProjectList>,
    project: Option<OpenProject>,
    tabs: Vec<OpenText>,
    active: usize,
    pending: Option<Pending>,
}

impl Default for Shell {
    /// A window without per-user state, as the tests use it.
    fn default() -> Self {
        let (workspace, events) = Workspace::open("", Vec::new());
        Self {
            workspace,
            events,
            notice: None,
            drained: 0,
            highlighter: None,
            state: None,
            projects: None,
            project: None,
            tabs: Vec::new(),
            active: 0,
            pending: None,
        }
    }
}

impl Shell {
    /// A window keeping its project list and caches in `state`.
    pub fn with_state(state: StateFolder) -> Self {
        let projects = match ProjectList::load(state.clone()) {
            Ok(list) => Some(list),
            Err(reason) => {
                return Self {
                    notice: Some(reason.0),
                    state: Some(state),
                    ..Self::default()
                };
            }
        };
        Self {
            projects,
            state: Some(state),
            ..Self::default()
        }
    }

    #[cfg(test)]
    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    /// Opens what the command line names: a folder as a project, a file in a tab.
    pub fn open_path(&mut self, path: &Path) {
        if path.is_dir() {
            let added = match &mut self.projects {
                Some(list) => list
                    .projects()
                    .into_iter()
                    .find(|p| p.path.canonicalize().ok() == path.canonicalize().ok())
                    .map(Ok)
                    .unwrap_or_else(|| list.add(path)),
                None => Ok(Project {
                    id: "command-line".into(),
                    name: path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    path: path.to_path_buf(),
                    added_at: 0,
                    opened_at: None,
                }),
            };
            match added {
                Ok(project) => self.open_project(project),
                Err(reason) => self.notice = Some(reason.0),
            }
        } else {
            let root = self
                .root()
                .unwrap_or_else(|| path.parent().map(Path::to_path_buf).unwrap_or_default());
            self.open_text(path, &root);
        }
    }

    /// Opens a text file in a tab, or focuses the tab it already has.
    pub fn open_text(&mut self, path: &Path, root: &Path) {
        if let Some(index) = self.tabs.iter().position(|t| t.file.path() == path) {
            self.active = index;
            return;
        }
        match OpenText::open(path, root) {
            Ok(open) => {
                self.notice = Some(format!(
                    "{} · {}",
                    open.file.encoding_name(),
                    open.file.line_ending_style()
                ));
                self.tabs.push(open);
                self.active = self.tabs.len() - 1;
            }
            Err(reason) => self.notice = Some(reason),
        }
    }

    fn root(&self) -> Option<PathBuf> {
        self.project.as_ref().map(|p| p.project.path.clone())
    }

    fn open_project(&mut self, project: Project) {
        let project = match &mut self.projects {
            Some(list) => match list.open(&project.id) {
                Ok(opened) => opened,
                Err(reason) => {
                    self.notice = Some(reason.0);
                    return;
                }
            },
            None => project,
        };
        let tree = match Hierarchy::scan(&project.path) {
            Ok(tree) => tree,
            Err(e) => {
                self.notice = Some(format!("The folder {} cannot be read: {e}", project.path.display()));
                return;
            }
        };
        let watcher = Watcher::start(&project.path).ok();
        let problems = self
            .state
            .clone()
            .map(|state| ProblemStore::load(state, &project.id, &project.path));
        let problem_set = problems.as_ref().map(ProblemStore::get);
        self.notice = Some(format!("Opened {}", project.name));
        self.project = Some(OpenProject {
            project,
            tree,
            watcher,
            explorer: Explorer::default(),
            problems,
            problem_set,
        });
    }

    fn close_project(&mut self) {
        if self.tabs.iter().any(|t| t.editor.buffer.is_modified()) {
            self.notice = Some("Save or close the tabs with unsaved changes first.".into());
            return;
        }
        self.tabs.clear();
        self.project = None;
    }

    /// Follows changes on disk: the tree is read again, and an open tab without unsaved changes reloads its file.
    fn follow_disk(&mut self) {
        let Some(project) = &mut self.project else { return };
        let Some(watcher) = &project.watcher else { return };
        let changes: Vec<Change> = watcher.changes().try_iter().collect();
        if changes.is_empty() {
            return;
        }
        if let Ok(tree) = Hierarchy::scan(&project.project.path) {
            project.tree = tree;
        }
        let root = project.project.path.clone();
        for change in changes {
            let (Change::Modified(path) | Change::Added(path)) = change else {
                continue;
            };
            if let Some(index) = self.tabs.iter().position(|t| t.file.path() == path)
                && !self.tabs[index].editor.buffer.is_modified()
                && let Ok(reloaded) = OpenText::open(&path, &root)
                && reloaded.file.content() != self.tabs[index].file.content()
            {
                self.tabs[index] = reloaded;
            }
        }
    }

    fn handle(&mut self, request: Request) {
        let Some(root) = self.root() else { return };
        let name_of = |path: &str| path.rsplit('/').next().unwrap_or(path).to_owned();
        match request {
            Request::Open(path) | Request::OpenAsText(path) => self.open_text(&root.join(&path), &root),
            Request::NewFolder(parent) => {
                self.pending = Some(Pending::NewFolder {
                    parent,
                    name: String::new(),
                    error: None,
                });
            }
            Request::Rename(path) => {
                self.pending = Some(Pending::Rename {
                    name: name_of(&path),
                    path,
                    error: None,
                });
            }
            Request::Delete(path) => {
                let question = self.delete_question(&path);
                self.pending = Some(Pending::Delete { path, question });
            }
            Request::Validate(path) => self.validate(if root.join(&path).is_dir() {
                Scope::Folder(path)
            } else {
                Scope::File(path)
            }),
            Request::ValidateAll => self.validate(Scope::Project),
        }
    }

    /// The standalone host's question before a delete, which says that it cannot be undone.
    fn delete_question(&self, path: &str) -> String {
        let name = path.rsplit('/').next().unwrap_or(path);
        let node = self.project.as_ref().and_then(|p| p.tree.find(path));
        match node {
            Some(n) if n.kind == Kind::Folder => {
                format!("Delete the folder '{name}' and everything inside it from disk? This cannot be undone.")
            }
            Some(n) => match n.children.len() {
                0 => format!("Delete '{name}' from disk? This cannot be undone."),
                1 => format!("Delete '{name}' and the 1 diagram registered on it from disk? This cannot be undone."),
                count => format!(
                    "Delete '{name}' and the {count} diagrams registered on it from disk? This cannot be undone."
                ),
            },
            None => format!("Delete '{name}' from disk? This cannot be undone."),
        }
    }

    fn validate(&mut self, scope: Scope) {
        let Some(project) = &mut self.project else { return };
        let Some(store) = &mut project.problems else {
            self.notice = Some("Findings are kept only for projects in the project list.".into());
            return;
        };
        match store.validate(&scope, &[&crate::problems::EditorValidator]) {
            Ok(()) => {
                let set = store.get();
                self.notice = Some(format!("Validated: {} errors, {} warnings", set.errors, set.warnings));
                project.problem_set = Some(set);
            }
            Err(e) => self.notice = Some(format!("Validation stopped: {e}")),
        }
    }

    /// Carries out an answered question; returns the error to show when the answer cannot be used.
    fn answer(&mut self, pending: Pending) -> Result<(), (Pending, String)> {
        let Some(root) = self.root() else { return Ok(()) };
        match pending {
            Pending::NewFolder { parent, name, error } => {
                let folder = root.join(&parent);
                if let Err(reason) = validate_name(&name, &folder, None) {
                    return Err((Pending::NewFolder { parent, name, error }, reason));
                }
                std::fs::create_dir(folder.join(name.trim())).map_err(|e| {
                    (
                        Pending::NewFolder {
                            parent: parent.clone(),
                            name: name.clone(),
                            error: None,
                        },
                        e.to_string(),
                    )
                })?;
                if let Some(project) = &mut self.project {
                    project.explorer.expanded.insert(parent);
                }
            }
            Pending::Rename { path, name, error } => {
                let from = root.join(&path);
                let folder = from.parent().map(Path::to_path_buf).unwrap_or_else(|| root.clone());
                let current = from
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if let Err(reason) = validate_name(&name, &folder, Some(&current)) {
                    return Err((Pending::Rename { path, name, error }, reason));
                }
                let to = folder.join(name.trim());
                std::fs::rename(&from, &to).map_err(|e| {
                    (
                        Pending::Rename {
                            path: path.clone(),
                            name: name.clone(),
                            error: None,
                        },
                        e.to_string(),
                    )
                })?;
                // The registrations named after the file follow it, as in the standalone host.
                let registrations: Vec<String> = self
                    .project
                    .as_ref()
                    .and_then(|p| p.tree.find(&path))
                    .map(|n| n.children.iter().map(|c| c.name.clone()).collect())
                    .unwrap_or_default();
                for registration in registrations {
                    if let Some(rest) = registration.strip_prefix(&current) {
                        let _ = std::fs::rename(
                            folder.join(&registration),
                            folder.join(format!("{}{rest}", name.trim())),
                        );
                    }
                }
                for tab in &mut self.tabs {
                    if tab.file.path() == from
                        && let Ok(reopened) = OpenText::open(&to, &root)
                    {
                        *tab = reopened;
                    }
                }
            }
            Pending::Delete { path, .. } => {
                let target = root.join(&path);
                let registrations: Vec<String> = self
                    .project
                    .as_ref()
                    .and_then(|p| p.tree.find(&path))
                    .map(|n| {
                        n.children
                            .iter()
                            .filter(|c| c.kind == Kind::Registration)
                            .map(|c| c.path.clone())
                            .collect()
                    })
                    .unwrap_or_default();
                let result = if target.is_dir() {
                    std::fs::remove_dir_all(&target)
                } else {
                    std::fs::remove_file(&target)
                };
                if let Err(e) = result {
                    self.notice = Some(format!("'{path}' could not be deleted: {e}"));
                    return Ok(());
                }
                for registration in registrations {
                    let _ = std::fs::remove_file(root.join(registration));
                }
                self.tabs.retain(|t| !t.file.path().starts_with(&target));
                self.active = self.active.min(self.tabs.len().saturating_sub(1));
            }
            Pending::CloseUnsaved { index } => {
                if index < self.tabs.len() {
                    self.tabs.remove(index);
                    self.active = self.active.min(self.tabs.len().saturating_sub(1));
                }
            }
        }
        if let Some(project) = &mut self.project
            && let Ok(tree) = Hierarchy::scan(&project.project.path)
        {
            project.tree = tree;
        }
        Ok(())
    }

    /// Ctrl+S (Cmd+S on macOS) saves the active tab, keeping its encoding and line endings.
    fn save(&mut self, ui: &egui::Ui) {
        if !ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
            return;
        }
        if let Some(open) = self.tabs.get_mut(self.active) {
            self.notice = Some(match open.save() {
                Ok(()) => format!("Saved {}", open.file.path().display()),
                Err(reason) => reason,
            });
        }
    }

    /// Undo and redo from the keyboard, when no editor has the focus: Z with and without Shift, and Y.
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
        self.follow_disk();
        self.save(ui);
        if let Some(open) = self.tabs.get_mut(self.active) {
            open.analyse();
        }
        let editor_focused = self
            .tabs
            .get(self.active)
            .is_some_and(|o| ui.memory(|m| m.has_focus(o.editor.id())));
        if !editor_focused {
            self.shortcuts(ui);
        }
        let theme = theme::current(ui.ctx());
        let palette = Palette::of(&theme);
        let sizes = theme.sizes;
        let title = match &self.project {
            Some(p) => format!("{} - ADP", p.project.name),
            None => "ADP".to_owned(),
        };
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Title(title));

        self.status_bar(ui, &theme, &palette);
        egui::Panel::left("activity-bar")
            .exact_size(sizes.activity_bar_width)
            .resizable(false)
            .frame(Frame::NONE.fill(palette.activity_bar_background))
            .show(ui, |ui| {
                ui.add_space(8.0);
                if self.project.is_some() {
                    // Back to the projects, drawn as a left chevron rather than taken from a font.
                    let (rect, response) =
                        ui.allocate_exact_size(egui::vec2(sizes.activity_bar_width, 40.0), egui::Sense::click());
                    let colour = if response.hovered() {
                        palette.foreground
                    } else {
                        palette.side_bar_foreground.gamma_multiply(0.7)
                    };
                    let c = rect.center();
                    ui.painter().line(
                        vec![
                            c + egui::vec2(4.0, -8.0),
                            c + egui::vec2(-4.0, 0.0),
                            c + egui::vec2(4.0, 8.0),
                        ],
                        egui::Stroke::new(2.0, colour),
                    );
                    if response.on_hover_text("Back to projects").clicked() {
                        self.close_project();
                    }
                }
            });

        if self.project.is_none() && self.tabs.is_empty() {
            self.projects_page(ui, &theme, &palette);
        } else {
            self.workspace_view(ui, &theme, &palette);
        }
        self.prompt(ui.ctx());
    }

    fn status_bar(&mut self, ui: &mut egui::Ui, theme: &adp_theme::Theme, palette: &Palette) {
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
                    let text =
                        self.notice
                            .as_deref()
                            .unwrap_or(if self.project.is_some() { "" } else { "No project open" });
                    let small = |t: &str| {
                        RichText::new(t.to_owned())
                            .color(palette.status_bar_foreground)
                            .size(sizes.workbench_font - 1.0)
                    };
                    ui.label(small(text));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(open) = self.tabs.get(self.active).filter(|o| !o.status.is_empty()) {
                            ui.label(small(&open.status));
                        }
                        if let Some(set) = self.project.as_ref().and_then(|p| p.problem_set.as_ref()) {
                            ui.label(small(&format!("{} errors, {} warnings", set.errors, set.warnings)));
                        }
                    });
                });
            });
    }

    fn projects_page(&mut self, ui: &mut egui::Ui, theme: &adp_theme::Theme, palette: &Palette) {
        let mut open = None;
        let mut remove = None;
        let mut add = false;
        egui::CentralPanel::default()
            .frame(
                Frame::NONE
                    .fill(palette.editor_background)
                    .inner_margin(Margin::same(48)),
            )
            .show(ui, |ui| {
                ui.heading(RichText::new("ADP").size(theme.sizes.workbench_font * 2.0));
                ui.label(format!("Desktop host {}", env!("CARGO_PKG_VERSION")));
                ui.add_space(24.0);
                ui.label(
                    RichText::new("PROJECTS")
                        .size(theme.sizes.workbench_font - 2.0)
                        .color(palette.side_bar_title),
                );
                ui.add_space(6.0);
                match &self.projects {
                    None => {
                        ui.label("Open a folder from the command line: adp-desktop <folder>.");
                    }
                    Some(list) => {
                        let projects = list.projects();
                        if projects.is_empty() {
                            ui.label("No projects yet.");
                        }
                        for project in projects {
                            ui.horizontal(|ui| {
                                if ui
                                    .link(&project.name)
                                    .on_hover_text(project.path.display().to_string())
                                    .clicked()
                                {
                                    open = Some(project.clone());
                                }
                                ui.label(RichText::new(project.path.display().to_string()).color(theme::color(
                                    theme,
                                    &["descriptionForeground"],
                                    palette.foreground,
                                )));
                                if ui
                                    .small_button("Remove")
                                    .on_hover_text("Remove from the list; the folder is not touched.")
                                    .clicked()
                                {
                                    remove = Some(project.id.clone());
                                }
                            });
                        }
                        ui.add_space(12.0);
                        add = ui.button("Add project…").clicked();
                    }
                }
            });
        if add && let Some(folder) = rfd::FileDialog::new().set_title("Add a project folder").pick_folder() {
            self.open_path(&folder);
        }
        if let Some(project) = open {
            self.open_project(project);
        }
        if let (Some(id), Some(list)) = (remove, &mut self.projects)
            && let Err(reason) = list.remove(&id)
        {
            self.notice = Some(reason.0);
        }
    }

    fn workspace_view(&mut self, ui: &mut egui::Ui, theme: &adp_theme::Theme, palette: &Palette) {
        let sizes = theme.sizes;
        let mut requests = Vec::new();
        if let Some(project) = &mut self.project {
            egui::Panel::left("side-bar")
                .default_size(260.0)
                .min_size(170.0)
                .resizable(true)
                .frame(
                    Frame::NONE
                        .fill(palette.side_bar_background)
                        .stroke(egui::Stroke::new(sizes.border_width, palette.border))
                        .inner_margin(Margin::symmetric(0, 8)),
                )
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new("EXPLORER")
                                .color(palette.side_bar_title)
                                .size(sizes.workbench_font - 2.0),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new(project.project.name.to_uppercase())
                                .strong()
                                .size(sizes.workbench_font - 2.0),
                        );
                    });
                    requests = project.explorer.show(ui, &project.tree, theme);
                });
            let mut jump = None;
            egui::Panel::bottom("problems")
                .default_size(140.0)
                .resizable(true)
                .frame(
                    Frame::NONE
                        .fill(theme::color(theme, &["panel.background"], palette.editor_background))
                        .inner_margin(Margin::symmetric(12, 6)),
                )
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("PROBLEMS")
                            .size(sizes.workbench_font - 2.0)
                            .color(palette.side_bar_title),
                    );
                    if let Some(set) = &project.problem_set {
                        jump = crate::problems::show(ui, set, theme);
                    } else {
                        ui.label("Findings are kept only for projects in the project list.");
                    }
                });
            if let Some((file, line)) = jump {
                let root = project.project.path.clone();
                self.open_text(&root.join(&file), &root);
                if let Some(open) = self.tabs.get_mut(self.active) {
                    let offset = open.editor.buffer.offset(line.saturating_sub(1) as usize, 0);
                    open.editor.buffer.set_cursor(offset);
                }
            }
        }
        for request in requests {
            self.handle(request);
        }
        self.tabs_and_editor(ui, theme, palette);
    }

    fn tabs_and_editor(&mut self, ui: &mut egui::Ui, theme: &adp_theme::Theme, palette: &Palette) {
        let sizes = theme.sizes;
        if self.tabs.is_empty() {
            egui::CentralPanel::default()
                .frame(Frame::NONE.fill(palette.editor_background))
                .show(ui, |ui| {
                    ui.centered_and_justified(|ui| {
                        ui.label(RichText::new("Open a file from the explorer.").color(theme::color(
                            theme,
                            &["descriptionForeground"],
                            palette.foreground,
                        )));
                    });
                });
            return;
        }
        let mut close = None;
        egui::Panel::top("tabs")
            .exact_size(sizes.tab_height)
            .resizable(false)
            .frame(Frame::NONE.fill(theme::color(
                theme,
                &["editorGroupHeader.tabsBackground"],
                palette.side_bar_background,
            )))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    for (index, open) in self.tabs.iter().enumerate() {
                        let active = index == self.active;
                        let (background, foreground) = if active {
                            (
                                theme::color(theme, &["tab.activeBackground"], palette.editor_background),
                                theme::color(theme, &["tab.activeForeground"], palette.foreground),
                            )
                        } else {
                            (
                                theme::color(theme, &["tab.inactiveBackground"], palette.side_bar_background),
                                theme::color(theme, &["tab.inactiveForeground"], palette.foreground),
                            )
                        };
                        Frame::NONE
                            .fill(background)
                            .inner_margin(Margin::symmetric(12, 9))
                            .show(ui, |ui| {
                                if ui
                                    .add(
                                        egui::Label::new(RichText::new(open.title()).color(foreground))
                                            .sense(egui::Sense::click()),
                                    )
                                    .clicked()
                                {
                                    self.active = index;
                                }
                                if ui
                                    .add(egui::Button::new(RichText::new("×").color(foreground)).frame(false))
                                    .on_hover_text("Close")
                                    .clicked()
                                {
                                    close = Some(index);
                                }
                            });
                    }
                });
            });
        if let Some(index) = close {
            if self.tabs[index].editor.buffer.is_modified() {
                self.pending = Some(Pending::CloseUnsaved { index });
            } else {
                self.tabs.remove(index);
                self.active = self.active.min(self.tabs.len().saturating_sub(1));
            }
        }
        let highlighter = self.highlighter.get_or_insert_with(Highlighter::default);
        let Some(open) = self.tabs.get_mut(self.active) else {
            return;
        };
        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(palette.editor_background))
            .show(ui, |ui| match &open.markdown {
                Some((headings, blocks)) => {
                    if !headings.is_empty()
                        && let Some(line) = crate::markdown::outline(ui, headings, theme)
                    {
                        let offset = open.editor.buffer.offset(line - 1, 0);
                        open.editor.buffer.set_cursor(offset);
                        ui.memory_mut(|m| m.request_focus(open.editor.id()));
                    }
                    let blocks = blocks.clone();
                    ui.columns(2, |columns| {
                        open.editor.show(&mut columns[0], theme, highlighter);
                        Frame::NONE
                            .inner_margin(Margin::symmetric(16, 8))
                            .show(&mut columns[1], |ui| {
                                crate::markdown::preview(ui, &blocks, theme);
                            });
                    });
                }
                None => {
                    open.editor.show(ui, theme, highlighter);
                }
            });
    }

    /// Shows the question being asked, if any, as a modal dialog.
    fn prompt(&mut self, ctx: &egui::Context) {
        let Some(mut pending) = self.pending.take() else { return };
        let mut answered = false;
        let mut cancelled = false;
        let title = match &pending {
            Pending::NewFolder { .. } => "New folder",
            Pending::Rename { .. } => "Rename",
            Pending::Delete { .. } => "Delete",
            Pending::CloseUnsaved { .. } => "Unsaved changes",
        };
        egui::Modal::new(egui::Id::unique("prompt")).show(ctx, |ui| {
            ui.set_width(420.0);
            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
            cancelled = ui.input(|i| i.key_pressed(egui::Key::Escape));
            match &mut pending {
                Pending::NewFolder { name, error, .. } | Pending::Rename { name, error, .. } => {
                    ui.heading(title);
                    let field = ui.text_edit_singleline(name);
                    field.request_focus();
                    if let Some(error) = error {
                        ui.colored_label(ui.visuals().error_fg_color, error.as_str());
                    }
                    ui.horizontal(|ui| {
                        answered = ui.button("OK").clicked() || enter;
                        cancelled |= ui.button("Cancel").clicked();
                    });
                }
                Pending::Delete { question, .. } => {
                    ui.heading("Delete");
                    ui.label(question.as_str());
                    ui.horizontal(|ui| {
                        answered = ui.button("Delete").clicked();
                        cancelled |= ui.button("Cancel").clicked();
                    });
                }
                Pending::CloseUnsaved { index } => {
                    let name = self.tabs.get(*index).map(OpenText::name).unwrap_or_default();
                    ui.heading("Unsaved changes");
                    ui.label(format!(
                        "'{name}' has changes that are not saved. Close it and lose them?"
                    ));
                    ui.horizontal(|ui| {
                        answered = ui.button("Close without saving").clicked();
                        cancelled |= ui.button("Cancel").clicked();
                    });
                }
            }
        });
        if cancelled {
            return;
        }
        if !answered {
            self.pending = Some(pending);
            return;
        }
        if let Err((mut back, reason)) = self.answer(pending) {
            if let Pending::NewFolder { error, .. } | Pending::Rename { error, .. } = &mut back {
                *error = Some(reason);
            }
            self.pending = Some(back);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::MarkLevel;
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
        let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../modules/editors/json/tests/examples");
        let mut shell = Shell::default();
        shell.open_path(&examples.join("service-broken.json"));
        let open = &mut shell.tabs[0];
        open.analyse();
        assert!(
            open.editor.marks.iter().any(|m| m.level == MarkLevel::Error),
            "{:?}",
            open.editor.marks
        );
        assert!(!open.status.starts_with("No schema"), "{}", open.status);
    }

    #[test]
    fn a_project_opens_with_its_tree_and_its_files_open_in_tabs_once() {
        let state = tempfile::tempdir().unwrap();
        let folder = tempfile::tempdir().unwrap();
        std::fs::write(folder.path().join("a.md"), "# A\n").unwrap();
        std::fs::create_dir(folder.path().join("docs")).unwrap();
        let mut shell = Shell::with_state(StateFolder::at(state.path()));
        shell.open_path(folder.path());
        let names: Vec<String> = shell
            .project
            .as_ref()
            .unwrap()
            .tree
            .children
            .iter()
            .map(|n| n.name.clone())
            .collect();
        assert_eq!(names, ["docs", "a.md"]);
        shell.handle(Request::Open("a.md".into()));
        shell.handle(Request::Open("a.md".into()));
        assert_eq!(shell.tabs.len(), 1);
        // The project is in the list for the next start.
        assert_eq!(
            ProjectList::load(StateFolder::at(state.path()))
                .unwrap()
                .projects()
                .len(),
            1
        );
    }

    #[test]
    fn new_folder_rename_and_delete_follow_the_standalone_hosts_rules() {
        let folder = tempfile::tempdir().unwrap();
        std::fs::write(folder.path().join("ideas.mm"), "<map/>").unwrap();
        std::fs::write(folder.path().join("ideas.mm.adp"), "freeplane/mindmap\n").unwrap();
        let mut shell = Shell::default();
        shell.open_path(folder.path());

        assert!(
            shell
                .answer(Pending::NewFolder {
                    parent: String::new(),
                    name: "a/b".into(),
                    error: None
                })
                .is_err()
        );
        assert!(
            shell
                .answer(Pending::NewFolder {
                    parent: String::new(),
                    name: "notes".into(),
                    error: None
                })
                .is_ok()
        );
        assert!(folder.path().join("notes").is_dir());

        assert!(
            shell
                .answer(Pending::Rename {
                    path: "ideas.mm".into(),
                    name: "plans.mm".into(),
                    error: None
                })
                .is_ok()
        );
        assert!(folder.path().join("plans.mm").is_file());
        assert!(
            folder.path().join("plans.mm.adp").is_file(),
            "the registration follows its file"
        );

        assert_eq!(
            shell.delete_question("plans.mm"),
            "Delete 'plans.mm' and the 1 diagram registered on it from disk? This cannot be undone."
        );
        assert!(
            shell
                .answer(Pending::Delete {
                    path: "plans.mm".into(),
                    question: String::new()
                })
                .is_ok()
        );
        assert!(!folder.path().join("plans.mm").exists());
        assert!(!folder.path().join("plans.mm.adp").exists());
    }

    #[test]
    fn validate_all_fills_the_problems_panel() {
        let state = tempfile::tempdir().unwrap();
        let folder = tempfile::tempdir().unwrap();
        std::fs::write(folder.path().join("broken.json"), "{ \"a\": 1, }").unwrap();
        let mut shell = Shell::with_state(StateFolder::at(state.path()));
        shell.open_path(folder.path());
        shell.handle(Request::ValidateAll);
        let set = shell.project.as_ref().unwrap().problem_set.as_ref().unwrap();
        assert_eq!(set.errors, 1);
        assert_eq!(set.problems[0].finding.file, "broken.json");
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
