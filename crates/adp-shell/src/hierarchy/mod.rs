//! The hierarchy tab (etalii.adp spec 036, Story 1, T028 and T029): a designer document of the `hierarchy` kind drawn
//! as one page of nested boxes. The tab holds the document's [`HierarchySurfaceSession`], drains the changes it queues
//! once a frame as the table tab does, and draws the model it was last given; every gesture goes to the session, which
//! writes it and sends the model as the file now is.
//!
//! Ported from the standalone host's `src/client/src/hierarchySurface/library` at `a7fdf03`: `HierarchySurface.tsx`
//! (the page and its state: focus, the selection, a collapse kept on the page alone, the levels that fit), and with
//! [`view`] `HierarchySurfaceBox.tsx`, `CollapseToggle.tsx`, `HierarchySurfaceTrail.tsx` and `FindingList.tsx`; the
//! layout rules are [`layout`]. Its look is `hierarchySurface.css` in VS Code's colours, the tags the table's own.
//!
//! It knows no designer type: which type is a level, which an entry, which labels they carry and which operation each
//! gesture sends are the designer's [`SurfaceDefinition`].

pub mod layout;
mod view;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use adp_desl::hierarchy::definition::SurfaceDefinition;
use adp_desl::hierarchy::model::{Gesture, SurfaceChange, SurfaceModel, SurfaceNode, roles};
use adp_desl::hierarchy::session::HierarchySurfaceSession;
use eframe::egui;

use layout::HierarchyIndex;

pub use view::show;

/// Whether a requirement document is at `path`: a Markdown file with the envelope, or a registration of one.
pub fn claims(path: &Path) -> bool {
    adp_designer_hierarchical_requirement::documents::document_at(path).is_some()
}

/// Whether a node's id is its number, which a gesture can name; `line/<n>` ids last only as long as one model.
fn is_line_id(id: &str) -> bool {
    id.starts_with("line/")
}

/// The page's left edge: a level, or none for the whole page, and whether it shows what is inside that level rather
/// than the level itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Focus {
    pub id: String,
    pub inside: bool,
}

/// One open document of the hierarchy kind, in a tab.
pub struct HierarchyTab {
    /// What was opened: the body or its registration.
    pub path: PathBuf,
    /// The Markdown file the tab shows.
    pub body: PathBuf,
    session: Box<dyn HierarchySurfaceSession>,
    pub definition: SurfaceDefinition,
    /// The model the session sent last.
    pub model: SurfaceModel,
    pub focus: Focus,
    /// The node selected, or empty.
    pub selected: String,
    /// The nodes whose collapse is the page's alone: those with no number of their own, and any while the document is
    /// read-only, since the document has nothing to keep it by.
    pub local_collapse: HashSet<String>,
    /// The page's width as the last frame measured it, 0 before the first.
    pub width: f32,
    /// The sentence the last gesture was refused with, shown above the page until the next one.
    pub refusal: String,
    /// What to tell the status bar.
    pub notice: Option<String>,
    /// The gestures sent whose outcome has not arrived.
    pub pending: usize,
    next_edit: u64,
    id: egui::Id,
}

impl HierarchyTab {
    /// Opens the requirement document at `path` - its body or its registration.
    pub fn open(path: &Path) -> Result<HierarchyTab, String> {
        let (body, registration) = adp_designer_hierarchical_requirement::documents::document_at(path)
            .ok_or_else(|| format!("{} is not a requirement document.", path.display()))?;
        let session = adp_designer_hierarchical_requirement::session::RequirementSession::open(&body, registration);
        Ok(HierarchyTab::with_session(
            path,
            &body,
            Box::new(session),
            adp_designer_hierarchical_requirement::surface::definition(),
        ))
    }

    /// A tab over a session already opened.
    pub fn with_session(
        path: &Path,
        body: &Path,
        session: Box<dyn HierarchySurfaceSession>,
        definition: SurfaceDefinition,
    ) -> HierarchyTab {
        let model = session.model();
        let mut tab = HierarchyTab {
            path: path.to_path_buf(),
            body: body.to_path_buf(),
            session,
            definition,
            model,
            focus: Focus::default(),
            selected: String::new(),
            local_collapse: HashSet::new(),
            width: 0.0,
            refusal: String::new(),
            notice: None,
            pending: 0,
            next_edit: 0,
            id: egui::Id::unique(("hierarchy", body.to_path_buf())),
        };
        tab.drain();
        tab
    }

    /// Whether the tab shows the document at `path`: its body or the registration it was opened at.
    pub fn shows(&self, path: &Path) -> bool {
        let same = adp_designer_knowledge::relations::same_path;
        same(path, &self.path) || same(path, &self.body)
    }

    pub fn name(&self) -> String {
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    pub fn title(&self) -> String {
        self.name()
    }

    /// The id the page takes the keyboard's focus under.
    pub fn id(&self) -> egui::Id {
        self.id
    }

    pub fn read_only(&self) -> bool {
        !self.model.read_only_reason.is_empty()
    }

    /// Takes every change the session has queued: a model replaces the page's, an outcome ends a gesture's wait and
    /// shows its refusal.
    pub fn drain(&mut self) {
        for change in self.session.take_changes() {
            match change {
                SurfaceChange::Model(model) => self.model = *model,
                SurfaceChange::Outcome(outcome) => {
                    self.pending = self.pending.saturating_sub(1);
                    if !outcome.written && !outcome.error.is_empty() {
                        self.refusal = outcome.error;
                    }
                }
            }
        }
        // A focus or a selection on a node the file no longer has falls back to the whole page and to nothing.
        if !self.focus.id.is_empty() && !self.model.nodes.iter().any(|n| n.id == self.focus.id) {
            self.focus = Focus::default();
        }
        if !self.selected.is_empty() && !self.model.nodes.iter().any(|n| n.id == self.selected) {
            self.selected.clear();
        }
    }

    /// Sends one gesture: an operation of the specification, the node it is about, and its parameters.
    pub fn gesture(&mut self, operation: &str, target: &str, params: BTreeMap<String, String>) {
        self.next_edit += 1;
        self.refusal.clear();
        let gesture = Gesture {
            edit_id: format!("edit-{}", self.next_edit),
            operation: operation.to_owned(),
            target_id: target.to_owned(),
            params,
        };
        match self.session.gesture(gesture) {
            Ok(()) => self.pending += 1,
            Err(reason) => self.refusal = reason,
        }
        self.drain();
    }

    /// Whether `node` is shown collapsed: as the file says, or turned on the page alone.
    pub fn is_collapsed(&self, node: &SurfaceNode) -> bool {
        node.collapsed != self.local_collapse.contains(&node.id)
    }

    /// The control to the left of a title: the definition's operation for a numbered node of an editable document,
    /// which the file keeps; the page's alone for any other.
    pub fn toggle(&mut self, id: &str) {
        if is_line_id(id) || self.read_only() {
            if !self.local_collapse.remove(id) {
                self.local_collapse.insert(id.to_owned());
            }
            return;
        }
        let operation = self.definition.operations.toggle_collapsed.clone();
        self.gesture(&operation, id, BTreeMap::new());
    }

    /// Makes `id` the page's left edge, or the whole page for "".
    pub fn set_focus(&mut self, id: &str) {
        self.focus = Focus {
            id: id.to_owned(),
            inside: false,
        };
    }

    /// The mark of a level's hidden levels shows them: the level becomes the left edge, or, where only one level fits,
    /// its sub-levels do.
    pub fn show_hidden(&mut self, id: &str) {
        self.focus = Focus {
            id: id.to_owned(),
            inside: self.fit() < 2,
        };
    }

    pub fn select(&mut self, id: &str) {
        self.selected = id.to_owned();
    }

    /// Shows the node a finding names: the focus that keeps it in sight, every level above it expanded, selected.
    pub fn reveal(&mut self, id: &str) {
        let index = HierarchyIndex::of(&self.model.nodes);
        let Some(plan) = index.reveal_of(id, &self.focus.id, self.fit(), |n| self.is_collapsed(n)) else {
            return;
        };
        self.set_focus(&plan.focus);
        for level in plan.expand {
            self.toggle(&level);
        }
        self.select(id);
    }

    /// The depth of the page's left column, 1 for the top levels: a column's width is kept by its depth.
    pub fn root_level(&self) -> u32 {
        if self.focus.id.is_empty() {
            return 1;
        }
        let index = HierarchyIndex::of(&self.model.nodes);
        index.ancestors_of(&self.focus.id).len() as u32 + if self.focus.inside { 2 } else { 1 }
    }

    /// The width of the column of levels at `depth`, 1 for the top: the one the file keeps, or the default.
    pub fn column_width(&self, depth: u32) -> f32 {
        self.model
            .widths
            .iter()
            .find(|c| c.level == depth)
            .map(|c| c.width as f32)
            .unwrap_or(layout::LEVEL_WIDTH)
    }

    /// How many levels fit the page as the last frame measured it.
    pub fn fit(&self) -> usize {
        let root = self.root_level();
        layout::levels_that_fit(self.width, |column| self.column_width(root + column as u32))
    }

    /// The nodes the page starts at: the top, the focused level, or, focused inside, its sub-levels.
    pub fn roots(&self) -> Vec<&SurfaceNode> {
        let index = HierarchyIndex::of(&self.model.nodes);
        if self.focus.id.is_empty() {
            return index.children_of("").to_vec();
        }
        if self.focus.inside {
            return index
                .children_of(&self.focus.id)
                .iter()
                .filter(|n| n.role == roles::LEVEL)
                .copied()
                .collect();
        }
        index.by_id.get(self.focus.id.as_str()).copied().into_iter().collect()
    }

    /// The trail: the levels down to the one the selection stands in, or the focus while nothing is selected.
    pub fn trail(&self) -> Vec<&SurfaceNode> {
        let index = HierarchyIndex::of(&self.model.nodes);
        let anchor = index
            .by_id
            .get(self.selected.as_str())
            .or_else(|| index.by_id.get(self.focus.id.as_str()))
            .copied();
        match anchor {
            None => Vec::new(),
            Some(node) if node.role == roles::LEVEL => {
                let mut path = index.ancestors_of(&node.id);
                path.push(node);
                path
            }
            Some(node) => index.ancestors_of(&node.id),
        }
    }

    /// The level the trail marks as the page's left edge: empty for the whole page, none while it shows what is inside
    /// a level.
    pub fn trail_current(&self) -> Option<&str> {
        (!self.focus.inside).then_some(self.focus.id.as_str())
    }

    pub fn undo(&mut self) -> Result<(), String> {
        let outcome = self.session.undo();
        self.drain();
        outcome
    }

    pub fn redo(&mut self) -> Result<(), String> {
        let outcome = self.session.redo();
        self.drain();
        outcome
    }

    /// The watcher saw `path` change: the session reads its files again when it is one of them.
    pub fn changed_on_disk(&mut self, path: &Path) {
        self.session.file_changed(path);
        self.drain();
    }
}
