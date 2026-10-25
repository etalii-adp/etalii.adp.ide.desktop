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
//! On a document that can change it is changed in place ([`editing`], T038): an entry or a level added by the
//! controls at the end of every level and of the page, or by the selected node's actions and their keys, its title
//! then open for typing; a title or a description typed where it stands; a node dragged by its grip; and deleted, asking
//! first when anything concerns it. A box's notes are listed, added, changed, resolved and removed from the icon at its
//! top right ([`notes`], T042). The bar above the page narrows it by a query and sorts each level's entries, writing
//! nothing, and stores the query under a name ([`narrowing`], T048); a finding stays listed whatever the query hides,
//! and selecting it takes out of the query the words that hide its node. A reference in a description stands as a
//! box with its target's number and title and shows the target when selected, and `#` typed in a description opens
//! the lookup it is picked from ([`references`], T046). A label's values are added, taken off and picked from its list,
//! which is edited in the table's options editor ([`labels`], T047). A change from outside while a title or a description is being typed shows the file
//! around the field and asks, on a bar over the page, whether to drop the text or keep it (T033). Each column of levels
//! is as wide as its right edge was dragged to, kept in the registration ([`columns`], T052).
//!
//! It knows no designer type: which type is a level, which an entry, which labels they carry and which operation each
//! gesture sends are the designer's [`SurfaceDefinition`].

pub mod columns;
pub mod editing;
pub mod labels;
pub mod layout;
pub mod narrowing;
pub mod notes;
pub mod references;
pub mod search;
mod view;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use adp_desl::hierarchy::definition::SurfaceDefinition;
use adp_desl::hierarchy::model::{Gesture, SurfaceAction, SurfaceChange, SurfaceModel, SurfaceNode, causes, roles};
use adp_desl::hierarchy::session::HierarchySurfaceSession;
use eframe::egui;

use editing::{Attribute, Deleting, DropHint, EntryDrop, OpenField};
use layout::HierarchyIndex;
use narrowing::FilterBar;
use notes::NoteList;

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
    /// The title or description open for typing, the one thing on the page that can be unsaved.
    pub field: Option<OpenField>,
    /// Whether the field opened since the last frame, which gives it the keyboard with its text selected.
    pub field_opened: bool,
    /// Whether another program changed the file while the field was open, which asks what to do with its text (the
    /// standalone host's `ConflictBar.tsx` at a7fdf03; spec 029 R10, R11, FR-023). The page is already the file's.
    pub conflict: bool,
    /// Whether the field is to take the keyboard back, its text as it was typed.
    pub field_refocus: bool,
    /// The node being dragged by its grip, and where it would go now.
    pub dragging: Option<SurfaceNode>,
    pub hint: Option<DropHint>,
    /// A delete waiting on its question's answer.
    pub deleting: Option<Deleting>,
    /// The list of notes open on a box, from its icon.
    pub notes: Option<NoteList>,
    /// The list of a label open on a box, from its field, its picker or its control.
    pub label_list: Option<labels::LabelList>,
    /// The query the filter bar applies, as typed: the page's own, never sent until it is stored.
    pub query: String,
    pub filter_bar: FilterBar,
    /// The widths of the columns of levels shown before the document has them.
    pub columns: columns::ColumnWidths,
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
            field: None,
            field_opened: false,
            conflict: false,
            field_refocus: false,
            dragging: None,
            hint: None,
            deleting: None,
            notes: None,
            label_list: None,
            query: String::new(),
            filter_bar: FilterBar::default(),
            columns: columns::ColumnWidths::default(),
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
                SurfaceChange::Model(model) => {
                    // A change from outside while a field is open asks what to do with the text.
                    if model.cause == causes::OUTSIDE && self.field.is_some() {
                        self.conflict = true;
                    }
                    self.model = *model;
                    self.columns.answered();
                }
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
        if self
            .deleting
            .as_ref()
            .is_some_and(|d| !self.model.nodes.iter().any(|n| n.id == d.node_id))
        {
            self.deleting = None;
        }
        if self
            .notes
            .as_ref()
            .is_some_and(|n| !self.model.nodes.iter().any(|node| node.id == n.node_id))
        {
            self.notes = None;
        }
    }

    /// The icon at a box's top right: opens the box's list of notes, or closes it when it is the one open.
    pub fn toggle_notes(&mut self, node_id: &str) {
        if self.notes.as_ref().is_some_and(|n| n.node_id == node_id) {
            self.notes = None;
        } else {
            self.notes = Some(NoteList::on(node_id, &self.definition));
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

    /// Sends an add, from a control or a key: the node it brings is shown, where the page's width hides it too,
    /// selected, and its title is open for typing. A refused add opens nothing.
    pub fn add(&mut self, operation: &str, target: &str) {
        let before = self.model.nodes.clone();
        self.gesture(operation, target, BTreeMap::new());
        if !self.refusal.is_empty() {
            return;
        }
        let Some(added) = editing::added_node(&before, &self.model.nodes).cloned() else {
            return;
        };
        self.reveal(&added.id);
        self.field = Some(OpenField::on(&added, Attribute::Title));
        self.field_opened = true;
    }

    /// The actions the selected node offers with their keys, for the ribbon; none on a document that cannot change.
    pub fn actions(&self) -> Vec<SurfaceAction> {
        if self.read_only() || self.selected.is_empty() {
            return Vec::new();
        }
        self.session.actions(&self.selected)
    }

    /// Runs the selected node's action `id`: its operation sent for the node, as an add.
    pub fn run_action(&mut self, id: &str) {
        let Some(action) = self.actions().into_iter().find(|a| a.id == id) else {
            return;
        };
        if !action.unavailable.is_empty() {
            self.refusal = action.unavailable;
            return;
        }
        let target = self.selected.clone();
        self.add(&action.operation, &target);
    }

    /// A key pressed with Ctrl or Cmd on the selected node: the action it runs, if the definition sends that key.
    pub fn shortcut(&mut self, key: &str, shift: bool) {
        if !self.definition.shortcut_keys.iter().any(|k| k == key) {
            return;
        }
        if let Some(action) = self.actions().into_iter().find(|a| a.key == key && a.shift == shift) {
            self.run_action(&action.id);
        }
    }

    /// Opens `attribute` of `node_id` for typing.
    pub fn open_field(&mut self, node_id: &str, attribute: Attribute) {
        if self.read_only() {
            return;
        }
        if let Some(node) = self.model.nodes.iter().find(|n| n.id == node_id) {
            self.field = Some(OpenField::on(node, attribute));
            self.field_opened = true;
        }
    }

    /// Keeps what the field holds: a gesture, unless nothing changed. A node the file no longer has is still sent:
    /// refusing it is the file's to say.
    pub fn commit_field(&mut self) {
        let Some(field) = self.field.take() else {
            return;
        };
        self.conflict = false;
        let node = self.model.nodes.iter().find(|n| n.id == field.node_id);
        if node.is_some_and(|n| field.attribute.of(n) == field.text) {
            return;
        }
        let operation = match field.attribute {
            Attribute::Title => self.definition.operations.set_title.clone(),
            Attribute::Description => self.definition.operations.set_description.clone(),
        };
        let params = BTreeMap::from([(field.attribute.param().to_owned(), field.text)]);
        self.gesture(&operation, &field.node_id, params);
    }

    /// Drops what the field holds.
    pub fn cancel_field(&mut self) {
        self.field = None;
        self.conflict = false;
    }

    /// "Show the file" on the conflict's bar: the field closes and what was typed in it is dropped.
    pub fn show_file(&mut self) {
        self.cancel_field();
    }

    /// "Keep my text" on the conflict's bar: the field stays open with what was typed, and committing it is a new
    /// edit of the file as it is now.
    pub fn keep_text(&mut self) {
        self.conflict = false;
        self.field_refocus = self.field.is_some();
    }

    /// Delete on a node: by the key, at once when nothing concerns it, otherwise after the definition's question; by the
    /// control on its box (`ask`), always after a question that says what it takes. A node without a number of its own,
    /// or one of a document that cannot change, is not deleted.
    pub fn delete(&mut self, node_id: &str, ask: bool) {
        if self.read_only() || is_line_id(node_id) {
            return;
        }
        let index = HierarchyIndex::of(&self.model.nodes);
        let Some(node) = index.by_id.get(node_id).copied() else {
            return;
        };
        let count = editing::delete_count(&self.model, &index, node_id);
        let question = if count >= self.definition.deletion.threshold {
            self.definition.deletion.question(&node.number, count)
        } else if ask {
            format!("Delete {} {}? Nothing else concerns it.", node.number, node.title)
        } else {
            let operation = self.definition.operations.delete.clone();
            self.gesture(&operation, node_id, BTreeMap::new());
            return;
        };
        self.deleting = Some(Deleting {
            node_id: node_id.to_owned(),
            question,
        });
    }

    /// The answer to a delete's question: deleted, or nothing.
    pub fn answer_delete(&mut self, delete: bool) {
        let Some(deleting) = self.deleting.take() else {
            return;
        };
        if delete {
            let operation = self.definition.operations.delete.clone();
            self.gesture(&operation, &deleting.node_id, BTreeMap::new());
        }
    }

    /// Drops `dragged` where `drop` says: one move, with everything in it.
    pub fn move_to(&mut self, dragged: &SurfaceNode, drop: EntryDrop) {
        let operation = if dragged.role == roles::LEVEL {
            self.definition.operations.move_level.clone()
        } else {
            self.definition.operations.move_entry.clone()
        };
        let params = BTreeMap::from([
            (self.definition.move_params.into.clone(), drop.into),
            (self.definition.move_params.after.clone(), drop.after),
        ]);
        self.gesture(&operation, &dragged.id, params);
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

    /// Shows the node a finding names: the query without the words that hide it, the focus that keeps it in sight,
    /// every level above it expanded, selected.
    pub fn reveal(&mut self, id: &str) {
        let index = HierarchyIndex::of(&self.model.nodes);
        let Some(plan) = index.reveal_of(id, &self.focus.id, self.fit(), |n| self.is_collapsed(n)) else {
            return;
        };
        let words = self.words();
        let unhidden = narrowing::unhiding(&self.model, &index, &words, id);
        if unhidden.len() != words.len() {
            self.query = narrowing::query_text(&unhidden);
        }
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

    /// The width of the column of levels at `depth`, 1 for the top: as it is being changed, as the file keeps it, or
    /// the default.
    pub fn column_width(&self, depth: u32) -> f32 {
        if let Some(shown) = self.columns.shown(depth) {
            return shown;
        }
        self.model
            .widths
            .iter()
            .find(|c| c.level == depth)
            .map(|c| c.width as f32)
            .unwrap_or(layout::LEVEL_WIDTH)
    }

    /// How many levels fit the page as the last frame measured it, and at least those down to the column whose edge
    /// is being dragged.
    pub fn fit(&self) -> usize {
        let root = self.root_level();
        let fit = layout::levels_that_fit(self.width, |column| self.column_width(root + column as u32));
        match self.columns.dragging() {
            Some(level) if level >= root => fit.max((level - root + 1) as usize),
            _ => fit,
        }
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
