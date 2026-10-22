//! The window (etalii.adp spec 030, Story 1, tasks T021, T034, T035): the project list when no project is open; with
//! one open, the activity bar, the explorer, the tabs and their editors, the errors and warnings panel and the status
//! bar, in VS Code's sizes and colours. The shell drains the core's events once per frame, follows the disk, and
//! asks before anything it cannot undo. A diagram opens on its canvas (task T055), with the toolbox and the property
//! grid beside it where the standalone host has them (T057): right of the tabs, as two tabs of one pane. A Knowledge
//! designer document opens as its table (T065), with the selected row's values in the property pane and the
//! explorer's entry that adds a relation (T064).

use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, OnceLock};

use adp_add::AddTarget;
use adp_context::{Reference, SelectMode, SelectionSource};
use adp_core::state::StateFolder;
use adp_core::{Event, Outcome, Workspace};
use adp_documents::reconcile::CONFLICT_NOTICE;
use adp_documents::{Reconciliation, Resolution, TextFile, reconcile_text};
use adp_hierarchy::{Change, Hierarchy, Kind, Watcher, validate_name};
use adp_problems::{ProblemSet, ProblemStore, Scope};
use adp_projects::{Project, ProjectList};
use eframe::egui::{self, Frame, Margin, RichText, accesskit::Role};

use crate::a11y;
use crate::add_dialog::{AddDialog, DialogOutcome};
use crate::canvas::{DiagramHost, DiagramTab};
use crate::documents::OpenText;
use crate::explorer::{Explorer, Request};
use crate::highlight::Highlighter;
use crate::table::TableTab;
use crate::theme::{self, Palette};

/// A project open in the window: its tree, kept in step with the disk, and its findings.
struct OpenProject {
    project: Project,
    /// The project's folder as the platform names it, when it is opened through a link: the watcher may report
    /// changes by this name, and they are mapped back to the folder as it was opened.
    real_root: Option<PathBuf>,
    tree: Hierarchy,
    watcher: Option<Watcher>,
    explorer: Explorer,
    problems: Option<ProblemStore>,
    problem_set: Option<ProblemSet>,
}

/// What a tab shows: a text in its editor, or a diagram on its canvas.
// A window holds a handful of tabs, so the text editor's size is not worth a box of its own.
#[allow(clippy::large_enum_variant)]
pub enum Tab {
    Text(OpenText),
    Diagram(Box<DiagramTab>),
    Table(Box<TableTab>),
}

impl Tab {
    pub fn text(&self) -> Option<&OpenText> {
        match self {
            Tab::Text(open) => Some(open),
            _ => None,
        }
    }

    pub fn text_mut(&mut self) -> Option<&mut OpenText> {
        match self {
            Tab::Text(open) => Some(open),
            _ => None,
        }
    }

    pub fn diagram(&self) -> Option<&DiagramTab> {
        match self {
            Tab::Diagram(tab) => Some(tab),
            _ => None,
        }
    }

    pub fn diagram_mut(&mut self) -> Option<&mut DiagramTab> {
        match self {
            Tab::Diagram(tab) => Some(tab),
            _ => None,
        }
    }

    pub fn table(&self) -> Option<&TableTab> {
        match self {
            Tab::Table(tab) => Some(tab),
            _ => None,
        }
    }

    pub fn table_mut(&mut self) -> Option<&mut TableTab> {
        match self {
            Tab::Table(tab) => Some(tab),
            _ => None,
        }
    }

    /// The file the tab was opened on.
    pub fn path(&self) -> &Path {
        match self {
            Tab::Text(open) => open.file.path(),
            Tab::Diagram(tab) => &tab.path,
            Tab::Table(tab) => &tab.path,
        }
    }

    pub fn name(&self) -> String {
        match self {
            Tab::Text(open) => open.name(),
            Tab::Diagram(tab) => tab.name(),
            Tab::Table(tab) => tab.name(),
        }
    }

    pub fn title(&self) -> String {
        match self {
            Tab::Text(open) => open.title(),
            Tab::Diagram(tab) => tab.title(),
            Tab::Table(tab) => tab.title(),
        }
    }

    /// Whether the tab holds changes that are not on disk; a diagram writes every edit as it is made, and a table
    /// writes its edits behind, by the next frame.
    pub fn is_modified(&self) -> bool {
        self.text().is_some_and(|t| t.editor.buffer.is_modified())
    }
}

/// Which pane beside a diagram is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidePane {
    Toolbox,
    Properties,
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
    /// The explorer's entry that adds a relation to a knowledge file (T064): the question it asks now, committed
    /// under `action_id`, and what has been answered.
    Relate {
        path: PathBuf,
        action_id: String,
        prompt: Box<adp_context::Prompt>,
        value: String,
        text: String,
        error: Option<String>,
    },
    /// The add dialog (T068): a document added to a folder, or a file registered.
    Add(Box<AddDialog>),
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
    tabs: Vec<Tab>,
    active: usize,
    /// The diagram tool types and each project's diagrams, loaded when a file is first opened.
    diagrams: Option<DiagramHost>,
    side_pane: SidePane,
    pending: Option<Pending>,
    /// The window's title as last sent: sending it asks for a new frame, so it is sent only when it changes.
    title: Option<String>,
    /// The window, known from its first frame, which the watcher wakes when a change on disk arrives.
    window: Arc<OnceLock<egui::Context>>,
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
            diagrams: None,
            side_pane: SidePane::Toolbox,
            pending: None,
            title: None,
            window: Arc::new(OnceLock::new()),
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
            let path = self.as_opened(path);
            let root = self
                .root()
                .unwrap_or_else(|| path.parent().map(Path::to_path_buf).unwrap_or_default());
            self.open_document(&path, &root);
        }
    }

    fn host(&mut self) -> &mut DiagramHost {
        self.diagrams
            .get_or_insert_with(|| DiagramHost::with_vendor(&crate::canvas::vendor_folder()))
    }

    /// Opens a file in a tab, or focuses the tab it already has: a diagram that a usable tool type claims, by its
    /// body or its registration, on its canvas, anything else in the text editor.
    pub fn open_document(&mut self, path: &Path, root: &Path) {
        if let Some(index) = self
            .tabs
            .iter()
            .position(|t| t.diagram().is_some_and(|d| d.shows(path)) || t.table().is_some_and(|d| d.shows(path)))
        {
            self.active = index;
            return;
        }
        // A Knowledge designer document - a knowledge file or its registration - opens as its table.
        if crate::table::claims(path) {
            match TableTab::open(path, root) {
                Ok(tab) => {
                    self.notice = Some(format!("Opened {} as {}", tab.name(), adp_designer_knowledge::ORIGIN));
                    self.tabs.push(Tab::Table(Box::new(tab)));
                    self.active = self.tabs.len() - 1;
                    return;
                }
                Err(reason) => self.notice = Some(reason),
            }
        }
        if self.host().claims(path) {
            match self.host().open(path, root) {
                Ok(tab) => {
                    self.notice = Some(format!("Opened {} as {}", tab.name(), tab.opened.origin));
                    self.tabs.push(Tab::Diagram(Box::new(tab)));
                    self.active = self.tabs.len() - 1;
                    return;
                }
                Err(reason) => self.notice = Some(reason),
            }
        }
        self.open_text(path, root);
    }

    /// Opens a text file in a tab, or focuses the text tab it already has.
    pub fn open_text(&mut self, path: &Path, root: &Path) {
        if let Some(index) = self
            .tabs
            .iter()
            .position(|t| t.text().is_some_and(|t| t.file.path() == path))
        {
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
                self.tabs.push(Tab::Text(open));
                self.active = self.tabs.len() - 1;
            }
            Err(reason) => self.notice = Some(reason),
        }
    }

    fn root(&self) -> Option<PathBuf> {
        self.project.as_ref().map(|p| p.project.path.clone())
    }

    /// `path` by the open project's folder as it was opened, so the tabs, the explorer and the watcher name a file
    /// alike when the folder was opened through a link; a path outside the project as it is.
    fn as_opened(&self, path: &Path) -> PathBuf {
        let Some(project) = &self.project else {
            return path.to_path_buf();
        };
        let root = &project.project.path;
        if path.starts_with(root) {
            return path.to_path_buf();
        }
        let real_root = project.real_root.as_deref().unwrap_or(root);
        if let Ok(rest) = path.strip_prefix(real_root) {
            return root.join(rest);
        }
        // The path itself may come through a link: its folder as the platform names it, the file's own name kept.
        let real = match (path.parent().and_then(|p| p.canonicalize().ok()), path.file_name()) {
            (Some(parent), Some(name)) => parent.join(name),
            _ => return path.to_path_buf(),
        };
        match real.strip_prefix(real_root) {
            Ok(rest) => root.join(rest),
            Err(_) => path.to_path_buf(),
        }
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
        let window = Arc::clone(&self.window);
        let watcher = Watcher::start_waking(&project.path, move || {
            if let Some(ctx) = window.get() {
                ctx.request_repaint_of(egui::ViewportId::ROOT);
            }
        })
        .ok();
        let real_root = project.path.canonicalize().ok().filter(|real| *real != project.path);
        let problems = self
            .state
            .clone()
            .map(|state| ProblemStore::load(state, &project.id, &project.path));
        let problem_set = problems.as_ref().map(ProblemStore::get);
        self.notice = Some(format!("Opened {}", project.name));
        let mut explorer = Explorer::default();
        explorer.root = project.path.clone();
        self.project = Some(OpenProject {
            project,
            real_root,
            tree,
            watcher,
            explorer,
            problems,
            problem_set,
        });
    }

    fn close_project(&mut self) {
        if self.tabs.iter().any(Tab::is_modified) {
            self.notice = Some("Save or close the tabs with unsaved changes first.".into());
            return;
        }
        self.tabs.clear();
        self.project = None;
    }

    /// Follows changes on disk: the tree is read again, and each open tab of a changed file is reconciled as the
    /// standalone host does (T058): without unsaved edits the new text takes over, and unsaved edits that differ
    /// from it wait for the person to choose a side.
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
            let change = match change {
                Change::Modified(path) => Change::Modified(self.as_opened(&path)),
                Change::Added(path) => Change::Added(self.as_opened(&path)),
                Change::Removed(path) => Change::Removed(self.as_opened(&path)),
                other => other,
            };
            let path = match change {
                Change::Modified(path) | Change::Added(path) => {
                    self.reconcile(&path, &root);
                    path
                }
                Change::Removed(path) => path,
                _ => continue,
            };
            if let Some(host) = &mut self.diagrams {
                let changed = host.reconcile(&path);
                self.diagrams_changed(&changed);
            }
            // A table reads its file again when it changed, and a table it relates to when that one did.
            for table in self.tabs.iter_mut().filter_map(Tab::table_mut) {
                table.changed_on_disk(&path);
            }
        }
    }

    /// The diagram tabs of bodies whose documents changed draw them again.
    fn diagrams_changed(&mut self, bodies: &[PathBuf]) {
        for tab in self.tabs.iter_mut().filter_map(Tab::diagram_mut) {
            if bodies.iter().any(|b| tab.shows(b)) {
                tab.changed();
            }
        }
    }

    /// Reconciles the open tab of `path`, if any, with what the file holds now.
    fn reconcile(&mut self, path: &Path, root: &Path) {
        let Some(index) = self
            .tabs
            .iter()
            .position(|t| t.text().is_some_and(|t| t.file.path() == path))
        else {
            return;
        };
        let Ok(disk) = TextFile::open(path) else {
            return;
        };
        let Some(open) = self.tabs[index].text_mut() else {
            return;
        };
        let edited = open.editor.buffer.is_modified().then(|| open.editor.buffer.text());
        match reconcile_text(open.file.content(), edited.as_deref(), Some(disk.content())) {
            Reconciliation::Unchanged | Reconciliation::Deleted => {}
            Reconciliation::TakeOver => {
                if let Ok(reloaded) = OpenText::open(path, root) {
                    self.tabs[index] = Tab::Text(reloaded);
                }
            }
            Reconciliation::Conflict => open.disk = Some(disk),
        }
    }

    /// Settles a tab whose file changed on disk under unsaved edits.
    fn resolve_conflict(&mut self, index: usize, resolution: Resolution) {
        let Some(open) = self.tabs.get_mut(index).and_then(Tab::text_mut) else {
            return;
        };
        let Some(disk) = open.disk.take() else { return };
        match resolution {
            Resolution::LoadDisk => {
                if let Ok(reloaded) = OpenText::open(disk.path(), &open.root) {
                    self.tabs[index] = Tab::Text(reloaded);
                }
            }
            Resolution::KeepMine => open.file = disk,
        }
    }

    fn handle(&mut self, request: Request) {
        let Some(root) = self.root() else { return };
        let name_of = |path: &str| path.rsplit('/').next().unwrap_or(path).to_owned();
        match request {
            Request::Open(path) => self.open_document(&root.join(&path), &root),
            Request::OpenAsText(path) => self.open_text(&root.join(&path), &root),
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
            Request::Relate(path) => self.start_relate(&root, &root.join(&path)),
            Request::Add(path) => self.start_add(&root, &path),
        }
    }

    /// Opens the add dialog on the entry at `path`: a folder to add a document to, or a file to register.
    fn start_add(&mut self, root: &Path, path: &str) {
        let Some(project) = &mut self.project else { return };
        let adds = project.explorer.adds();
        let full = root.join(path);
        let target = if full.is_dir() {
            AddTarget::Folder(full)
        } else {
            AddTarget::File(full)
        };
        match adds.execute(&target) {
            Ok(prompt) => self.pending = Some(Pending::Add(Box::new(AddDialog::new(target, prompt)))),
            Err(reason) => self.notice = Some(reason),
        }
    }

    /// Creates what the add dialog chose; then the explorer selects the registration, its folder and its file opened
    /// on the way, and it is opened as a double click opens it.
    #[allow(clippy::result_large_err)]
    fn add(&mut self, root: &Path, dialog: Box<AddDialog>) -> Result<(), (Pending, String)> {
        let Some(project) = &mut self.project else {
            return Ok(());
        };
        let adds = project.explorer.adds();
        let Some(DialogOutcome::Submit { value, text }) = dialog.submission() else {
            return Err((Pending::Add(dialog), "Choose one first.".to_owned()));
        };
        let created = match adds.commit(&dialog.target, &value, text.as_deref().unwrap_or_default()) {
            Ok(created) => created,
            Err(reason) => return Err((Pending::Add(dialog), reason)),
        };
        if let Ok(tree) = Hierarchy::scan(&project.project.path) {
            project.tree = tree;
        }
        let relative = created
            .strip_prefix(root)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let mut chain = Vec::new();
        if ancestors_of(&project.tree.children, &relative, &mut chain) {
            project.explorer.expanded.extend(chain);
        }
        project.explorer.selected = Some(relative);
        self.open_document(&created, root);
        Ok(())
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
    // The question is handed back whole, to be asked again with the reason; it is answered once per press.
    #[allow(clippy::result_large_err)]
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
                    if tab.text().is_some_and(|t| t.file.path() == from)
                        && let Ok(reopened) = OpenText::open(&to, &root)
                    {
                        *tab = Tab::Text(reopened);
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
                self.tabs.retain(|t| !t.path().starts_with(&target));
                self.active = self.active.min(self.tabs.len().saturating_sub(1));
            }
            Pending::CloseUnsaved { index } => {
                if index < self.tabs.len() {
                    self.tabs.remove(index);
                    self.active = self.active.min(self.tabs.len().saturating_sub(1));
                }
            }
            relate @ Pending::Relate { .. } => return self.relate(&root, relate),
            Pending::Add(dialog) => return self.add(&root, dialog),
        }
        if let Some(project) = &mut self.project
            && let Ok(tree) = Hierarchy::scan(&project.project.path)
        {
            project.tree = tree;
        }
        Ok(())
    }

    /// Starts the explorer's entry that adds a relation to the knowledge file at `path` (the standalone host's
    /// `KnowledgeContextActionProvider`): its first question is the table to relate to.
    fn start_relate(&mut self, root: &Path, path: &Path) {
        match adp_designer_knowledge::context_actions::execute(root, path) {
            Ok(prompt) => {
                self.pending = Some(Pending::Relate {
                    path: path.to_path_buf(),
                    action_id: adp_designer_knowledge::context_actions::RELATE.to_owned(),
                    prompt: Box::new(prompt),
                    value: String::new(),
                    text: String::new(),
                    error: None,
                })
            }
            Err(reason) => self.notice = Some(reason),
        }
    }

    /// Commits one answer of the entry that adds a relation: the next question, or the relation made through the
    /// table's tab - opened when it is not - as one of its edits, shown at once and undone with Ctrl+Z.
    #[allow(clippy::result_large_err)]
    fn relate(&mut self, root: &Path, pending: Pending) -> Result<(), (Pending, String)> {
        use adp_designer_knowledge::context_actions::{self, Commit};
        let Pending::Relate {
            path,
            action_id,
            prompt,
            value,
            text,
            error,
        } = pending
        else {
            return Ok(());
        };
        let checked = if value == context_actions::TWO_WAY {
            context_actions::validate(root, &action_id, &text)
        } else if value.is_empty() {
            Err("Choose one first.".to_owned())
        } else {
            context_actions::validate(root, &action_id, &value)
        };
        if let Err(reason) = checked {
            let back = Pending::Relate {
                path,
                action_id,
                prompt,
                value,
                text,
                error,
            };
            return Err((back, reason));
        }
        match context_actions::commit(root, &path, &action_id, &value, &text) {
            Commit::Asks {
                prompt: next,
                next_action_id,
            } => {
                let text = match &next {
                    adp_context::Prompt::Choice {
                        name_field: Some(field),
                        ..
                    } => field.initial_value.clone(),
                    _ => String::new(),
                };
                self.pending = Some(Pending::Relate {
                    path,
                    action_id: next_action_id,
                    prompt: Box::new(next),
                    value: String::new(),
                    text,
                    error: None,
                });
            }
            Commit::Make { body_path, gesture } => {
                self.open_document(&path, root);
                match self.tabs.get_mut(self.active).and_then(Tab::table_mut) {
                    Some(table) if table.shows(&body_path) => table.raise(gesture),
                    _ => {
                        if let Err(reason) = context_actions::make(&body_path, &gesture) {
                            self.notice = Some(reason);
                        }
                    }
                }
            }
            Commit::Failed(reason) => {
                let back = Pending::Relate {
                    path,
                    action_id,
                    prompt,
                    value,
                    text,
                    error,
                };
                return Err((back, reason));
            }
        }
        Ok(())
    }

    /// Ctrl+S (Cmd+S on macOS) saves the active tab, keeping its encoding and line endings.
    fn save(&mut self, ui: &egui::Ui) {
        if !ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
            return;
        }
        if let Some(open) = self.tabs.get_mut(self.active).and_then(Tab::text_mut) {
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
        if !undo && !redo {
            return;
        }
        // A table's edits are its session's.
        if let Some(table) = self.tabs.get_mut(self.active).and_then(Tab::table_mut) {
            let outcome = if undo { table.undo() } else { table.redo() };
            if let Err(reason) = outcome {
                self.notice = Some(reason);
            }
            return;
        }
        // A diagram's edits are on its project's diagram history.
        if let Some(root) = self
            .tabs
            .get(self.active)
            .and_then(Tab::diagram)
            .map(|d| d.root.clone())
        {
            let workspace = self.host().workspace(&root);
            let outcome = if undo { workspace.undo() } else { workspace.redo() };
            if let adp_history::Outcome::Refused(reason) = outcome {
                self.notice = Some(reason.0);
            }
            let changed = self.host().take_changed();
            self.diagrams_changed(&changed);
            if let Some(tab) = self.tabs.get_mut(self.active).and_then(Tab::diagram_mut) {
                tab.changed();
            }
            return;
        }
        let outcome = if undo {
            self.workspace.undo()
        } else {
            self.workspace.redo()
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
        self.window.get_or_init(|| ui.ctx().clone());
        self.drain();
        self.follow_disk();
        self.save(ui);
        if let Some(open) = self.tabs.get_mut(self.active).and_then(Tab::text_mut) {
            open.analyse();
        }
        // A text field has the keyboard: the editor, or a field over or beside a canvas.
        let editor_focused = match self.tabs.get(self.active) {
            Some(Tab::Text(open)) => ui.memory(|m| m.has_focus(open.editor.id())),
            Some(Tab::Diagram(tab)) => ui.memory(|m| m.focused().is_some_and(|f| f != tab.canvas_id())),
            Some(Tab::Table(tab)) => ui.memory(|m| m.focused().is_some_and(|f| f != tab.grid_id())),
            None => false,
        };
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
        if self.title.as_ref() != Some(&title) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = Some(title);
        }

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
                    a11y::describe(&response, Role::Button, "Back to projects");
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
                        if let Some(open) = self
                            .tabs
                            .get(self.active)
                            .and_then(Tab::text)
                            .filter(|o| !o.status.is_empty())
                        {
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
        }
        self.diagram_panes(ui, theme, palette);
        if let Some(project) = &mut self.project {
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
                    // To a screen reader the panel is a list named Problems, each finding an item of it (T088).
                    let list = egui::UiBuilder::new()
                        .accessibility_role(Role::List)
                        .accessibility_label("Problems");
                    ui.scope_builder(list, |ui| {
                        if let Some(set) = &project.problem_set {
                            jump = crate::problems::show(ui, set, theme);
                        } else {
                            ui.label("Findings are kept only for projects in the project list.");
                        }
                    });
                });
            if let Some((file, line)) = jump {
                let root = project.project.path.clone();
                self.open_text(&root.join(&file), &root);
                if let Some(open) = self.tabs.get_mut(self.active).and_then(Tab::text_mut) {
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
                let list = egui::UiBuilder::new()
                    .accessibility_role(Role::TabList)
                    .accessibility_label("Open editors");
                ui.scope_builder(list, |ui| {
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
                                    let name = open.name();
                                    let tab = ui.add(
                                        egui::Label::new(RichText::new(open.title()).color(foreground))
                                            .sense(egui::Sense::click()),
                                    );
                                    a11y::tab(&tab, &name, active);
                                    if open.is_modified() {
                                        a11y::describe_state(&tab, "Unsaved changes");
                                    }
                                    if tab.clicked() {
                                        self.active = index;
                                    }
                                    let closer =
                                        ui.add(egui::Button::new(RichText::new("×").color(foreground)).frame(false));
                                    a11y::describe(&closer, Role::Button, &format!("Close {name}"));
                                    if closer.on_hover_text("Close").clicked() {
                                        close = Some(index);
                                    }
                                });
                        }
                    })
                });
            });
        if let Some(index) = close {
            if self.tabs[index].is_modified() {
                self.pending = Some(Pending::CloseUnsaved { index });
            } else {
                self.tabs.remove(index);
                self.active = self.active.min(self.tabs.len().saturating_sub(1));
            }
        }
        if self
            .tabs
            .get(self.active)
            .and_then(Tab::text)
            .is_some_and(|t| t.disk.is_some())
        {
            let mut chosen = None;
            egui::Panel::top("conflict")
                .frame(
                    Frame::NONE
                        .fill(palette.notification_background)
                        .inner_margin(Margin::symmetric(12, 6)),
                )
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(CONFLICT_NOTICE).color(palette.notification_foreground));
                        for resolution in [Resolution::LoadDisk, Resolution::KeepMine] {
                            if ui.button(resolution.label()).clicked() {
                                chosen = Some(resolution);
                            }
                        }
                    });
                });
            if let Some(resolution) = chosen {
                self.resolve_conflict(self.active, resolution);
            }
        }
        if let Some(Tab::Table(tab)) = self.tabs.get_mut(self.active) {
            egui::CentralPanel::default()
                .frame(Frame::NONE.fill(palette.editor_background))
                .show(ui, |ui| crate::table::show(ui, tab, theme, palette));
            self.after_table();
            return;
        }
        if let Some(Tab::Diagram(tab)) = self.tabs.get_mut(self.active) {
            let workspace = self
                .diagrams
                .get_or_insert_with(|| DiagramHost::with_vendor(&crate::canvas::vendor_folder()))
                .workspace(&tab.root);
            egui::CentralPanel::default()
                .frame(Frame::NONE.fill(palette.editor_background))
                .show(ui, |ui| tab.show(ui, workspace, theme, palette));
            self.after_canvas();
            return;
        }
        let highlighter = self.highlighter.get_or_insert_with(Highlighter::default);
        let Some(open) = self.tabs.get_mut(self.active).and_then(Tab::text_mut) else {
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

    /// What a frame of a canvas changed: its notice in the status bar, its selection the window's, and every other
    /// view of a body it edited drawn again.
    fn after_canvas(&mut self) {
        let Some(tab) = self.tabs.get_mut(self.active).and_then(Tab::diagram_mut) else {
            return;
        };
        // An edit that went through takes away the refusal the status bar still shows.
        if std::mem::take(&mut tab.cleared) {
            self.notice = None;
        }
        if let Some(notice) = tab.notice.take() {
            self.notice = Some(notice);
        }
        let document = tab.opened.body_path.display().to_string();
        let references: Vec<Reference> = tab
            .selection
            .iter()
            .map(|element| Reference::Element {
                document: document.clone(),
                element: element.clone(),
            })
            .collect();
        if tab.reported.as_ref() != Some(&tab.selection) {
            tab.reported = Some(tab.selection.clone());
            self.workspace
                .select(references, SelectMode::Replace, SelectionSource::Canvas);
        }
        if let Some(host) = &mut self.diagrams {
            let changed = host.take_changed();
            let active = self.active;
            for (index, tab) in self.tabs.iter_mut().enumerate() {
                if index != active
                    && let Some(tab) = tab.diagram_mut()
                    && changed.iter().any(|b| tab.shows(b))
                {
                    tab.changed();
                }
            }
        }
    }

    /// What a frame of a table changed: its notice in the status bar, and its selected row the window's selection.
    fn after_table(&mut self) {
        let Some(tab) = self.tabs.get_mut(self.active).and_then(Tab::table_mut) else {
            return;
        };
        if let Some(notice) = tab.notice.take() {
            self.notice = Some(notice);
        }
        if tab.reported.as_ref() != Some(&tab.selected) {
            tab.reported = Some(tab.selected.clone());
            let references = tab
                .selected
                .iter()
                .map(|row| Reference::Element {
                    document: tab.body_path().display().to_string(),
                    element: row.clone(),
                })
                .collect();
            self.workspace
                .select(references, SelectMode::Replace, SelectionSource::Canvas);
        }
    }

    /// The property grid of the active table: the selected row's values, each edited as the table edits it.
    fn table_pane(&mut self, ui: &mut egui::Ui, theme: &adp_theme::Theme, palette: &Palette) {
        let Some(Tab::Table(tab)) = self.tabs.get_mut(self.active) else {
            return;
        };
        let sizes = theme.sizes;
        egui::Panel::right("table-pane")
            .default_size(280.0)
            .min_size(180.0)
            .resizable(true)
            .frame(
                Frame::NONE
                    .fill(palette.side_bar_background)
                    .stroke(egui::Stroke::new(sizes.border_width, palette.border))
                    .inner_margin(Margin::symmetric(12, 8)),
            )
            .show(ui, |ui| {
                ui.label(
                    RichText::new("PROPERTIES")
                        .size(sizes.workbench_font - 2.0)
                        .color(palette.side_bar_title),
                );
                ui.add_space(8.0);
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| crate::properties::show_row(ui, tab, theme, palette));
            });
    }

    /// The toolbox and the property grid of the active diagram, as two tabs of one pane right of the editor.
    fn diagram_panes(&mut self, ui: &mut egui::Ui, theme: &adp_theme::Theme, palette: &Palette) {
        if matches!(self.tabs.get(self.active), Some(Tab::Table(_))) {
            self.table_pane(ui, theme, palette);
            return;
        }
        let Some(Tab::Diagram(tab)) = self.tabs.get_mut(self.active) else {
            return;
        };
        let sizes = theme.sizes;
        let workspace = self
            .diagrams
            .get_or_insert_with(|| DiagramHost::with_vendor(&crate::canvas::vendor_folder()))
            .workspace(&tab.root);
        let side_pane = &mut self.side_pane;
        egui::Panel::right("diagram-panes")
            .default_size(280.0)
            .min_size(180.0)
            .resizable(true)
            .frame(
                Frame::NONE
                    .fill(palette.side_bar_background)
                    .stroke(egui::Stroke::new(sizes.border_width, palette.border))
                    .inner_margin(Margin::symmetric(12, 8)),
            )
            .show(ui, |ui| {
                let list = egui::UiBuilder::new()
                    .accessibility_role(Role::TabList)
                    .accessibility_label("Diagram panes");
                ui.scope_builder(list, |ui| {
                    ui.horizontal(|ui| {
                        for (pane, label, name) in [
                            (SidePane::Toolbox, "TOOLBOX", "Toolbox"),
                            (SidePane::Properties, "PROPERTIES", "Properties"),
                        ] {
                            let active = *side_pane == pane;
                            let colour = if active {
                                palette.side_bar_title
                            } else {
                                palette.side_bar_title.gamma_multiply(0.6)
                            };
                            let response = ui.add(
                                egui::Label::new(RichText::new(label).size(sizes.workbench_font - 2.0).color(colour))
                                    .sense(egui::Sense::click()),
                            );
                            a11y::tab(&response, name, active);
                            if active {
                                let r = response.rect;
                                ui.painter().line_segment(
                                    [
                                        r.left_bottom() + egui::vec2(0.0, 2.0),
                                        r.right_bottom() + egui::vec2(0.0, 2.0),
                                    ],
                                    egui::Stroke::new(
                                        1.0,
                                        theme::color(theme, &["panelTitle.activeBorder"], palette.focus),
                                    ),
                                );
                            }
                            if response.clicked() {
                                *side_pane = pane;
                            }
                            ui.add_space(12.0);
                        }
                    })
                });
                ui.add_space(8.0);
                egui::ScrollArea::vertical()
                    .auto_shrink(false)
                    .show(ui, |ui| match *side_pane {
                        SidePane::Toolbox => crate::canvas::toolbox::show(ui, tab, workspace, theme, palette),
                        SidePane::Properties => crate::properties::show(ui, tab, workspace, theme, palette),
                    });
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
            Pending::Relate { .. } => "Add relation",
            Pending::Add(_) => "Add",
        };
        // The dialog is named by its title, as a screen reader announces it when it opens (T088).
        let name = match &pending {
            Pending::Add(dialog) => dialog.prompt.title.clone(),
            _ => title.to_owned(),
        };
        let theme = theme::current(ctx);
        let adds = self.project.as_mut().map(|p| p.explorer.adds());
        let id = egui::Id::unique("prompt");
        egui::Modal::new(id)
            .area(egui::Modal::default_area(id).accessible_name(name))
            .show(ctx, |ui| {
                ui.set_width(420.0);
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                cancelled = ui.input(|i| i.key_pressed(egui::Key::Escape));
                match &mut pending {
                    Pending::NewFolder { name, error, .. } | Pending::Rename { name, error, .. } => {
                        ui.heading(title);
                        let field = ui.text_edit_singleline(name).accessible_name(match title {
                            "Rename" => "New name",
                            _ => "Folder name",
                        });
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
                    Pending::Relate {
                        prompt,
                        value,
                        text,
                        error,
                        ..
                    } => {
                        let confirm = relate_prompt(ui, prompt, value, text);
                        if let Some(error) = error {
                            ui.colored_label(ui.visuals().error_fg_color, error.as_str());
                        }
                        ui.horizontal(|ui| {
                            answered = ui.button(confirm).clicked();
                            cancelled |= ui.button("Cancel").clicked();
                        });
                    }
                    Pending::Add(dialog) => {
                        let target = dialog.target.clone();
                        let validate = |name: &str| adds.as_ref().map_or(Ok(()), |a| a.validate(&target, name));
                        match dialog.show(ui, &theme, validate) {
                            Some(DialogOutcome::Submit { .. }) => answered = true,
                            Some(DialogOutcome::Cancel) => cancelled = true,
                            None => {}
                        }
                    }
                    Pending::CloseUnsaved { index } => {
                        let name = self.tabs.get(*index).map(Tab::name).unwrap_or_default();
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
            match &mut back {
                Pending::NewFolder { error, .. } | Pending::Rename { error, .. } | Pending::Relate { error, .. } => {
                    *error = Some(reason)
                }
                Pending::Add(dialog) => dialog.submit_error = Some(reason),
                _ => {}
            }
            self.pending = Some(back);
        }
    }
}

/// Collects the paths of the entries above `path` in `nodes`, its folders and the file it is registered on; whether
/// it was found.
fn ancestors_of(nodes: &[adp_hierarchy::Node], path: &str, chain: &mut Vec<String>) -> bool {
    for node in nodes {
        if node.path == path {
            return true;
        }
        chain.push(node.path.clone());
        if ancestors_of(&node.children, path, chain) {
            return true;
        }
        chain.pop();
    }
    false
}

/// The body of one question of the entry that adds a relation: the files or the options to choose from, and the
/// name field where the option chosen takes a name. The label of the button that answers it.
fn relate_prompt(ui: &mut egui::Ui, prompt: &adp_context::Prompt, value: &mut String, text: &mut String) -> String {
    use adp_context::Prompt;
    match prompt {
        Prompt::File {
            title,
            confirm_label,
            files,
            pinned,
            empty_message,
            ..
        } => {
            ui.heading(title);
            for option in pinned.iter().chain(files) {
                let label = if option.label == option.id {
                    option.label.clone()
                } else {
                    format!("{} ({})", option.label, option.id)
                };
                if ui.selectable_label(*value == option.id, label).clicked() {
                    *value = option.id.clone();
                }
            }
            if files.is_empty() {
                ui.label(RichText::new(empty_message).small());
            }
            confirm_label.clone()
        }
        Prompt::Choice {
            title,
            confirm_label,
            options,
            name_field,
            ..
        } => {
            ui.heading(title);
            for option in options {
                if ui.radio(*value == option.id, &option.label).clicked() {
                    *value = option.id.clone();
                    if text.is_empty() && !option.suggested_value.is_empty() {
                        *text = option.suggested_value.clone();
                    }
                }
            }
            let chosen = options.iter().find(|o| o.id == *value);
            if let Some(field) = name_field {
                match chosen.filter(|o| !o.name_suppressed_reason.is_empty()) {
                    Some(option) => {
                        ui.label(RichText::new(&option.name_suppressed_reason).small());
                    }
                    None => {
                        let label = ui.label(&field.label);
                        ui.text_edit_singleline(text).labelled_by(label.id);
                    }
                }
            }
            confirm_label.clone()
        }
        _ => "OK".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::MarkLevel;
    use adp_context::{Reference, SelectMode, SelectionSource};

    impl Shell {
        /// The text tab at `index`.
        fn text(&mut self, index: usize) -> &mut OpenText {
            self.tabs[index].text_mut().expect("a text tab")
        }
    }

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
        let open = shell.text(0);
        open.analyse();
        assert!(
            open.editor.marks.iter().any(|m| m.level == MarkLevel::Error),
            "{:?}",
            open.editor.marks
        );
        assert!(!open.status.starts_with("No schema"), "{}", open.status);
    }

    #[test]
    fn a_file_changed_on_disk_takes_over_a_clean_tab_and_waits_for_a_choice_under_unsaved_edits() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("notes.md");
        std::fs::write(
            &path, "one
",
        )
        .unwrap();
        let mut shell = Shell::default();
        shell.open_path(&path);
        let root = shell.text(0).root.clone();

        std::fs::write(
            &path, "two
",
        )
        .unwrap();
        shell.reconcile(&path, &root);
        assert_eq!(
            shell.text(0).editor.buffer.text(),
            "two
",
            "a clean tab takes the new text over"
        );

        shell.text(0).editor.buffer.insert("mine ");
        std::fs::write(
            &path, "three
",
        )
        .unwrap();
        shell.reconcile(&path, &root);
        assert!(shell.text(0).disk.is_some(), "unsaved edits wait for the person");
        shell.resolve_conflict(0, Resolution::KeepMine);
        assert!(shell.text(0).editor.buffer.is_modified(), "the edits stay, unsaved");
        assert_eq!(
            shell.text(0).file.content(),
            "three
",
            "over the new text"
        );

        std::fs::write(
            &path, "four
",
        )
        .unwrap();
        shell.reconcile(&path, &root);
        shell.resolve_conflict(0, Resolution::LoadDisk);
        assert_eq!(
            shell.text(0).editor.buffer.text(),
            "four
"
        );
        assert!(!shell.text(0).editor.buffer.is_modified());

        std::fs::write(
            &path, "four
",
        )
        .unwrap();
        shell.reconcile(&path, &root);
        assert_eq!(
            shell.text(0).editor.buffer.text(),
            "four
",
            "a clean tab takes over other line endings, so its next save keeps them"
        );
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
    fn a_diagram_opens_on_its_canvas_its_text_a_choice_away_and_ctrl_z_undoes_its_edits() {
        let fixtures =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../adp-diagram/tests/fixtures/dependency-graph/example-1");
        let folder = tempfile::tempdir().unwrap();
        for name in ["services.adp", "services.dgr"] {
            std::fs::copy(fixtures.join(name), folder.path().join(name)).unwrap();
        }
        let body = folder.path().join("services.dgr");
        let original = std::fs::read_to_string(&body).unwrap();
        let mut shell = Shell::default();
        shell.open_path(folder.path());
        shell.handle(Request::Open("services.dgr".into()));
        shell.handle(Request::Open("services.adp".into()));
        assert_eq!(shell.tabs.len(), 1, "the body and its registration are one diagram");
        assert!(shell.tabs[0].diagram().is_some(), "a claimed body opens on a canvas");
        shell.handle(Request::OpenAsText("services.dgr".into()));
        assert_eq!(shell.tabs.len(), 2);
        assert!(shell.tabs[1].text().is_some(), "and as text when asked");
        shell.active = 0;

        let ctx = egui::Context::default();
        theme::install(&ctx);
        frame(&ctx, &mut shell);
        let root = shell.tabs[0].diagram().unwrap().root.clone();
        let tab = shell.tabs[0].diagram_mut().unwrap();
        tab.select(vec!["api-gateway".into()], SelectMode::Replace);
        let workspace = shell.diagrams.as_mut().unwrap().workspace(&root);
        let label = crate::properties::rows(tab, workspace)
            .into_iter()
            .find(|r| r.label == "Label")
            .unwrap()
            .id;
        crate::properties::set(tab, workspace, &label, "Edge gateway");
        assert_ne!(std::fs::read_to_string(&body).unwrap(), original, "{:?}", tab.notice);
        frame(&ctx, &mut shell);

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
        assert_eq!(
            std::fs::read_to_string(&body).unwrap(),
            original,
            "Ctrl+Z undoes the diagram's edit"
        );
    }

    /// A copy of the Knowledge designer's cities example and the table it relates to, each with its registration.
    fn knowledge_project(folder: &Path) {
        let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../modules/designers/knowledge/tests/examples");
        for name in ["cities.yaml", "provinces.yaml"] {
            std::fs::copy(examples.join(name), folder.join(name)).unwrap();
        }
        std::fs::write(folder.join("cities.adp"), "etalii/knowledge\r\nbody: cities.yaml\r\n").unwrap();
    }

    fn ctrl(key: egui::Key) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0))),
            events: vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            }],
            ..Default::default()
        }
    }

    #[test]
    fn a_knowledge_document_opens_as_its_table_its_edits_are_written_and_ctrl_z_undoes_them() {
        let folder = tempfile::tempdir().unwrap();
        knowledge_project(folder.path());
        let body = folder.path().join("cities.yaml");
        let original = std::fs::read(&body).unwrap();
        let mut shell = Shell::default();
        shell.open_path(folder.path());
        shell.handle(Request::Open("cities.adp".into()));
        shell.handle(Request::Open("cities.yaml".into()));
        assert_eq!(shell.tabs.len(), 1, "the registration and its data file are one table");
        assert!(shell.tabs[0].table().is_some(), "a knowledge file opens as its table");

        let ctx = egui::Context::default();
        theme::install(&ctx);
        frame(&ctx, &mut shell);
        let table = shell.tabs[0].table_mut().unwrap();
        assert!(!table.held.is_empty(), "the lines in sight are drawn");
        table.select_row(Some("r1".into()));
        table.set_property("p1", "Mokum");
        frame(&ctx, &mut shell);
        frame(&ctx, &mut shell);
        assert!(std::fs::read_to_string(&body).unwrap().contains("text: Mokum"));
        assert_eq!(
            shell.tabs[0].table().unwrap().reported,
            Some(Some("r1".to_owned())),
            "the selected row is told to the window as its selection"
        );

        ctx.run_ui(ctrl(egui::Key::Z), |ui| shell.show(ui))
            .textures_delta
            .clear();
        assert_eq!(
            std::fs::read(&body).unwrap(),
            original,
            "Ctrl+Z undoes the table's edit"
        );
        ctx.run_ui(ctrl(egui::Key::Y), |ui| shell.show(ui))
            .textures_delta
            .clear();
        assert!(
            std::fs::read_to_string(&body).unwrap().contains("text: Mokum"),
            "Ctrl+Y makes it again"
        );
    }

    #[test]
    fn the_explorers_add_relation_asks_three_questions_and_makes_the_relation_in_the_open_table() {
        use adp_designer_knowledge::context_actions::{MANY, TWO_WAY};
        let folder = tempfile::tempdir().unwrap();
        knowledge_project(folder.path());
        let mut shell = Shell::default();
        shell.open_path(folder.path());
        shell.handle(Request::Relate("cities.adp".into()));
        let mut answers = vec![("provinces.yaml", ""), (MANY, ""), (TWO_WAY, "Cities nearby")].into_iter();
        while let Some(Pending::Relate { .. }) = &shell.pending {
            let Some(Pending::Relate {
                path,
                action_id,
                prompt,
                error,
                ..
            }) = shell.pending.take()
            else {
                unreachable!()
            };
            let (value, text) = answers.next().expect("three questions");
            let answered = shell.answer(Pending::Relate {
                path,
                action_id,
                prompt,
                value: value.into(),
                text: text.into(),
                error,
            });
            assert!(answered.is_ok(), "{:?}", answered.err().map(|e| e.1));
        }
        assert_eq!(answers.next(), None, "every question was asked");
        let table = shell
            .tabs
            .iter_mut()
            .find_map(Tab::table_mut)
            .expect("the table is open");
        assert!(
            table
                .columns
                .iter()
                .any(|c| c.name == "Provinces" && c.kind == "relation"),
            "{}",
            table.refusal
        );
        table.write_behind();
        let provinces = std::fs::read_to_string(folder.path().join("provinces.yaml")).unwrap();
        assert!(provinces.contains("Cities nearby"), "the other table has its side");
        table.undo().expect("undone");
        let provinces = std::fs::read_to_string(folder.path().join("provinces.yaml")).unwrap();
        assert!(!provinces.contains("Cities nearby"), "one undo takes both sides back");
    }

    #[test]
    fn the_explorers_add_writes_the_document_selects_its_registration_and_opens_it() {
        let folder = tempfile::tempdir().unwrap();
        std::fs::create_dir(folder.path().join("plans")).unwrap();
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let mut shell = Shell::default();
        shell.open_path(folder.path());

        shell.handle(Request::Add("plans".into()));
        frame(&ctx, &mut shell);
        let Some(Pending::Add(mut dialog)) = shell.pending.take() else {
            panic!("the add dialog is asked")
        };
        assert_eq!(dialog.prompt.title, "Add diagram");
        dialog.choose("generic/timeline");
        assert_eq!(dialog.name, "timeline");
        dialog.type_name("roadmap", |_| Ok(()));
        assert!(shell.answer(Pending::Add(dialog)).is_ok());

        let plans = folder.path().join("plans");
        assert_eq!(
            std::fs::read(plans.join("roadmap.adp")).unwrap(),
            b"generic/timeline\r\n"
        );
        assert_eq!(
            std::fs::read(plans.join("roadmap.tml")).unwrap(),
            b"timeline: 1\r\nelements: []\r\n"
        );
        let explorer = &shell.project.as_ref().unwrap().explorer;
        assert_eq!(explorer.selected.as_deref(), Some("plans/roadmap.adp"));
        assert!(explorer.expanded.contains("plans"));
        assert_eq!(shell.tabs[shell.active].path(), plans.join("roadmap.adp"));
        assert!(shell.tabs[shell.active].diagram().is_some(), "it opens on its canvas");

        // The same name again is refused, and the dialog stays open with the reason.
        shell.handle(Request::Add("plans".into()));
        let Some(Pending::Add(mut dialog)) = shell.pending.take() else {
            panic!("the add dialog is asked")
        };
        dialog.choose("generic/timeline");
        dialog.type_name("roadmap", |_| Ok(()));
        let Err((Pending::Add(_), reason)) = shell.answer(Pending::Add(dialog)) else {
            panic!("refused")
        };
        assert_eq!(reason, "An item named 'roadmap.adp' already exists in this folder.");
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

    /// One headless frame, and how long it asks the window to wait before the next: `Duration::MAX` when it waits
    /// for input.
    fn frame_asking(ctx: &egui::Context, shell: &mut Shell) -> std::time::Duration {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0))),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| shell.show(ui));
        output.textures_delta.clear();
        output.viewport_output[&egui::ViewportId::ROOT].repaint_delay
    }

    /// Frames without input until the window waits for input; whether it did within `frames`.
    fn settles(ctx: &egui::Context, shell: &mut Shell, frames: usize) -> bool {
        (0..frames).any(|_| frame_asking(ctx, shell) == std::time::Duration::MAX)
    }

    /// The window's wake-ups from outside a frame: every repaint asked for without delay, as eframe hears of them.
    fn wakes(ctx: &egui::Context) -> Arc<std::sync::atomic::AtomicUsize> {
        let wakes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counted = Arc::clone(&wakes);
        ctx.set_request_repaint_callback(move |info| {
            if info.delay.is_zero() {
                counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        });
        wakes
    }

    /// Draws a frame each time the window is woken, and only then, until `done` holds; whether it did within 5 s.
    fn drawn_when_woken(
        ctx: &egui::Context,
        shell: &mut Shell,
        wakes: &std::sync::atomic::AtomicUsize,
        done: impl Fn(&mut Shell) -> bool,
    ) -> bool {
        use std::sync::atomic::Ordering;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            if wakes.swap(0, Ordering::SeqCst) > 0 {
                frame_asking(ctx, shell);
                if done(shell) {
                    return true;
                }
            } else {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
        false
    }

    fn explorer_has(shell: &Shell, name: &str) -> bool {
        shell
            .project
            .as_ref()
            .is_some_and(|p| p.tree.children.iter().any(|n| n.name == name))
    }

    #[test]
    fn an_idle_window_waits_for_input_and_a_change_on_disk_wakes_it() {
        let folder = tempfile::tempdir().unwrap();
        let root = folder.path().canonicalize().unwrap();
        std::fs::write(root.join("notes.md"), "# One\n").unwrap();
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let mut shell = Shell::default();
        shell.open_path(&root);
        shell.handle(Request::Open("notes.md".into()));
        assert!(
            settles(&ctx, &mut shell, 20),
            "an idle window asks for no frame after the first few"
        );
        assert_eq!(
            frame_asking(&ctx, &mut shell),
            std::time::Duration::MAX,
            "nor for one in every frame"
        );
        // The watcher's first events, of the file just written, come and go.
        std::thread::sleep(std::time::Duration::from_millis(500));
        assert!(settles(&ctx, &mut shell, 20));

        let wakes = wakes(&ctx);
        std::fs::write(root.join("notes.md"), "# Two\n").unwrap();
        std::fs::write(root.join("added.md"), "added\n").unwrap();
        assert!(
            drawn_when_woken(&ctx, &mut shell, &wakes, |shell| {
                shell.text(0).editor.buffer.text() == "# Two\n" && explorer_has(shell, "added.md")
            }),
            "the watcher wakes the window, and the open tab and the explorer follow the disk"
        );
        assert!(settles(&ctx, &mut shell, 20), "and the window waits again");
    }

    /// Makes `link` a link to the folder `target`; false where the platform does not let this process make one.
    fn link_folder(target: &Path, link: &Path) -> bool {
        #[cfg(unix)]
        let made = std::os::unix::fs::symlink(target, link);
        #[cfg(windows)]
        let made = std::os::windows::fs::symlink_dir(target, link);
        made.is_ok()
    }

    #[test]
    fn a_project_opened_through_a_link_has_its_tabs_and_explorer_follow_the_disk() {
        let folder = tempfile::tempdir().unwrap();
        let real = folder.path().canonicalize().unwrap().join("real");
        std::fs::create_dir(&real).unwrap();
        std::fs::write(real.join("notes.md"), "# One\n").unwrap();
        let link = folder.path().canonicalize().unwrap().join("link");
        if !link_folder(&real, &link) {
            eprintln!("skipped: this process may not make a link to a folder here");
            return;
        }
        let ctx = egui::Context::default();
        theme::install(&ctx);
        let mut shell = Shell::default();
        shell.open_path(&link);
        shell.handle(Request::Open("notes.md".into()));
        assert_eq!(
            shell.tabs[0].path(),
            link.join("notes.md"),
            "the tab is named by the folder as opened"
        );
        // A file named by the folder's real path is the file the tab has open.
        assert_eq!(shell.as_opened(&real.join("notes.md")), link.join("notes.md"));
        shell.open_path(&real.join("notes.md"));
        assert_eq!(shell.tabs.len(), 1, "the file is not opened twice by its other name");
        assert!(settles(&ctx, &mut shell, 20));
        std::thread::sleep(std::time::Duration::from_millis(500));
        assert!(settles(&ctx, &mut shell, 20));

        let wakes = wakes(&ctx);
        std::fs::write(real.join("notes.md"), "# Two\n").unwrap();
        std::fs::write(real.join("added.md"), "added\n").unwrap();
        assert!(
            drawn_when_woken(&ctx, &mut shell, &wakes, |shell| {
                shell.text(0).editor.buffer.text() == "# Two\n" && explorer_has(shell, "added.md")
            }),
            "the open tab follows the disk as the explorer does"
        );
    }
}
