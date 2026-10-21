//! The designer tab (etalii.adp spec 030, Story 4, FR-022, SC-006, task T065): a Knowledge designer document drawn as
//! a windowed table. The tab holds the document's [`KnowledgeSession`] - the connection the standalone host's
//! `useTableStream` opens - asks it only for the lines in sight, and draws what it is given; every gesture goes to the
//! session, which shows it at once and writes it behind, and undo and redo are the session's.
//!
//! Ported from the standalone host at `8b6ccfc`: `src/client/src/table/library/TableSurface.tsx` (one row height, the
//! window of lines in sight and a margin either side, reported only when it becomes other lines; the sticky header;
//! group headings and the line a row is added at; the active cell, the arrows, Enter, Tab, Escape and Delete; a
//! refusal shown on the table's own line and an editor that stays open on it; the question a delete is answered
//! with; "changes are being saved"), `rows/windowing.ts` (`windowOf`, `DEFAULT_ROW_HEIGHT` 33, `DEFAULT_MARGIN` 10),
//! `cells/CellEditor.tsx` (an input per written kind, a searchable list for an option, several options or related
//! rows, a value that was not changed sending nothing), `views/ViewTabs.tsx` and `views/ViewBar.tsx` (switching is the
//! connection's own, everything else an edit), `definition/tableDefinition.ts` (`DEFAULT_COLUMN_WIDTH` 180) and the
//! Knowledge designer's `client/knowledgeDefinition.ts` and `KnowledgePanel.tsx` (each kind's editor, every gesture
//! said in the view it was made in, one selected row the selection of the window). Its look is `table.css` in VS
//! Code's colours: every colour is a theme colour, but for the option tags, whose ten colours are the standalone
//! host's own.
//!
//! The shell's tab, its property grid and its explorer use this module through [`TableTab`]; it draws with egui and
//! knows no file format.

mod view;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use adp_context::PropertyRow;
use adp_designer_knowledge::{KnowledgeSession, project, row_context};
use adp_desl::table::model::{
    TableChange, TableColumn, TableFinding, TableGesture, TableRow, TableView, TableViewSettings,
};
use adp_desl::table::session::DesignerSession;
use eframe::egui;

pub use view::show;

/// The height of every line, in points (`DEFAULT_ROW_HEIGHT`).
pub const ROW_HEIGHT: f32 = 33.0;
/// The lines kept ready either side of what is in sight (`DEFAULT_MARGIN`).
pub const MARGIN: usize = 10;
/// A column's width when the view gives none (`DEFAULT_COLUMN_WIDTH`).
pub const COLUMN_WIDTH: f32 = 180.0;
/// What answers that start with it are: a question to confirm, not a refusal (`CONFIRM_PREFIX`).
pub const CONFIRM_PREFIX: &str = "confirm:";

/// Whether a Knowledge designer document is at `path`: a knowledge file's data file, or a registration of one.
pub fn claims(path: &Path) -> bool {
    project::body_of(path).is_some()
}

/// How a kind of value is edited (`TableEditorKind`), as the Knowledge designer's definition says for each of its
/// nine property types; none for a kind that is shown and never edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorKind {
    Text,
    Number,
    Checkbox,
    Date,
    DateTime,
    Time,
    /// One option, from a searchable list.
    Option,
    /// Several options, from a searchable list.
    Options,
    /// Rows of a table, from a searchable list.
    Rows,
}

impl EditorKind {
    /// The editor of a kind, as `knowledgeDefinition.ts` declares it.
    pub fn of(kind: &str) -> Option<EditorKind> {
        Some(match kind {
            "text" => EditorKind::Text,
            "number" => EditorKind::Number,
            "checkbox" => EditorKind::Checkbox,
            "date" => EditorKind::Date,
            "dateTime" => EditorKind::DateTime,
            "time" => EditorKind::Time,
            "selection" => EditorKind::Option,
            "multipleSelection" => EditorKind::Options,
            "relation" => EditorKind::Rows,
            _ => return None,
        })
    }

    /// Whether a value is chosen from a list rather than typed.
    pub fn is_list(self) -> bool {
        matches!(self, EditorKind::Option | EditorKind::Options | EditorKind::Rows)
    }

    /// What a typed value looks like, shown in an empty field.
    pub fn hint(self) -> &'static str {
        match self {
            EditorKind::Date => "year-month-day",
            EditorKind::DateTime => "year-month-day hours:minutes",
            EditorKind::Time => "hours:minutes",
            _ => "",
        }
    }
}

/// The kind's name and the mark its header shows (`knowledgeDefinition.ts`'s labels; the standalone host draws
/// Material Design icons, which egui has no font for).
pub fn kind_label(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "text" => ("Text", "Aa"),
        "number" => ("Number", "#"),
        "checkbox" => ("Checkbox", "✔"),
        "date" => ("Date", "📅"),
        "dateTime" => ("Date and time", "📅"),
        "time" => ("Time", "⏰"),
        "selection" => ("Selection", "⏷"),
        "multipleSelection" => ("Multiple selection", "☰"),
        "relation" => ("Relation", "↗"),
        _ => ("Unknown type", "?"),
    }
}

/// Whether a view can be grouped by a kind (`groupable`).
pub fn groupable(kind: &str) -> bool {
    matches!(kind, "checkbox" | "selection" | "multipleSelection" | "relation")
}

/// The cell being edited: where, and what its editor holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellEditing {
    pub line: usize,
    pub column: usize,
    pub row_id: String,
    pub column_id: String,
    pub editor: EditorKind,
    /// What the field holds, for a typed value; what the list is searched for, for a chosen one.
    pub text: String,
    /// What the cell held when the editor opened.
    pub initial: String,
    /// The backend's refusal of the last value sent, shown under the editor, which stays open.
    pub reason: String,
    /// Whether the field has been given the focus.
    pub focused: bool,
}

/// What a frame of the table panel was asked to show besides the rows: which of the view bar's panels is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewPanel {
    Sort,
    Filter,
    Group,
}

/// One open Knowledge designer document, in a tab.
pub struct TableTab {
    /// What was opened: the data file or its registration.
    pub path: PathBuf,
    pub session: KnowledgeSession,
    pub title: String,
    pub columns: Vec<TableColumn>,
    pub views: Vec<TableView>,
    pub settings: TableViewSettings,
    pub row_count: usize,
    pub findings: Vec<TableFinding>,
    pub read_only_reason: String,
    /// The window last asked for, by its first line and how many.
    pub window: Option<(usize, usize)>,
    /// The lines the session gave for that window, by their index in the view: never more than the window.
    pub held: HashMap<usize, TableRow>,
    /// The cell the keyboard is on, by line and visible column.
    pub active: Option<(usize, usize)>,
    pub editing: Option<CellEditing>,
    /// The row that is selected: the window's selection, whose values the property grid shows.
    pub selected: Option<String>,
    /// The selection last told to the window.
    pub reported: Option<Option<String>>,
    /// The sentence a gesture made outside an editor was refused with, shown on the table's own line.
    pub refusal: String,
    /// A gesture answered with a question, and the question, until its author answers it.
    pub asking: Option<(TableGesture, String)>,
    /// What to tell the status bar.
    pub notice: Option<String>,
    /// Where the rows are to be scrolled to on the next frame, in points from the top.
    pub scroll_to: Option<f32>,
    pub panel: Option<ViewPanel>,
    /// A view being renamed, and the name its field holds.
    pub renaming: Option<(String, String)>,
    /// The text of a filter condition's value field being edited, by its place.
    pub filter_text: HashMap<usize, String>,
    /// The text of a property grid field being edited, by property, for the row it was opened on.
    pub grid_editing: HashMap<String, String>,
    pub grid_row: Option<String>,
    next_edit: u64,
    id: egui::Id,
}

impl TableTab {
    /// Opens the document at `path` - a data file or its registration - in the project at `root`.
    pub fn open(path: &Path, root: &Path) -> Result<TableTab, String> {
        let body = project::body_of(path).ok_or_else(|| format!("{} is not a knowledge table.", path.display()))?;
        let session = KnowledgeSession::open_for_editing(&body, root);
        Ok(TableTab::with_session(path, session))
    }

    /// A tab over a session already opened.
    pub fn with_session(path: &Path, session: KnowledgeSession) -> TableTab {
        let baseline = session.baseline();
        let id = egui::Id::unique(("table", session.body_path().to_path_buf()));
        TableTab {
            path: path.to_path_buf(),
            session,
            title: baseline.title,
            columns: baseline.columns,
            views: baseline.views,
            settings: baseline.settings,
            row_count: baseline.row_count,
            findings: baseline.findings,
            read_only_reason: baseline.read_only_reason,
            window: None,
            held: HashMap::new(),
            active: None,
            editing: None,
            selected: None,
            reported: None,
            refusal: String::new(),
            asking: None,
            notice: None,
            scroll_to: None,
            panel: None,
            renaming: None,
            filter_text: HashMap::new(),
            grid_editing: HashMap::new(),
            grid_row: None,
            next_edit: 0,
            id,
        }
    }

    /// The data file the tab shows.
    pub fn body_path(&self) -> &Path {
        self.session.body_path()
    }

    /// Whether the tab shows the document at `path`: its data file or the registration it was opened at.
    pub fn shows(&self, path: &Path) -> bool {
        let same = adp_designer_knowledge::relations::same_path;
        same(path, &self.path) || same(path, self.body_path())
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

    /// The id the grid takes the keyboard's focus under.
    pub fn grid_id(&self) -> egui::Id {
        self.id.with("grid")
    }

    pub fn id(&self) -> egui::Id {
        self.id
    }

    /// Whether the table can be edited.
    pub fn editable(&self) -> bool {
        self.read_only_reason.is_empty()
    }

    /// The columns the active view shows, in its order.
    pub fn visible_columns(&self) -> Vec<&TableColumn> {
        self.columns.iter().filter(|c| c.visible).collect()
    }

    /// A visible column's width in points.
    pub fn width_of(column: &TableColumn) -> f32 {
        if column.width > 0 {
            column.width as f32
        } else {
            COLUMN_WIDTH
        }
    }

    /// What the session gave since it was last asked, taken into what the tab shows.
    pub fn drain(&mut self) {
        for change in self.session.take_changes() {
            match change {
                TableChange::Rows { first, rows, row_count } => {
                    self.row_count = row_count;
                    // The lines of the window replace what was held: nothing outside it is kept.
                    self.held = rows.into_iter().enumerate().map(|(i, r)| (first + i, r)).collect();
                }
                TableChange::Structure(structure) => {
                    self.title = structure.title;
                    self.columns = structure.columns;
                    self.views = structure.views;
                    self.settings = structure.settings;
                    self.row_count = structure.row_count;
                    self.read_only_reason = structure.read_only_reason;
                }
                TableChange::Findings(findings) => self.findings = findings,
                TableChange::EditSettled { written, error, .. } => {
                    if !written && !error.is_empty() {
                        self.refusal = error;
                    }
                }
            }
        }
        if let Some((line, column)) = self.active {
            let columns = self.visible_columns().len();
            if self.row_count == 0 || columns == 0 {
                self.active = None;
            } else if line >= self.row_count || column >= columns {
                self.active = Some((line.min(self.row_count - 1), column.min(columns - 1)));
            }
        }
    }

    /// The window for the lines from `first_in_sight` to `last_in_sight`, widened by the margin and kept inside the
    /// view (`windowOf`).
    pub fn window_of(first_in_sight: usize, last_in_sight: usize, row_count: usize) -> (usize, usize) {
        if row_count == 0 {
            return (0, 0);
        }
        let first = first_in_sight.saturating_sub(MARGIN).min(row_count - 1);
        let last = (last_in_sight + MARGIN).max(first).min(row_count - 1);
        (first, last - first + 1)
    }

    /// Asks the session for the lines from `first_in_sight` to `last_in_sight` and a margin, when that is other lines
    /// than it was last asked for: a scroll within the window asks for nothing.
    pub fn want(&mut self, first_in_sight: usize, last_in_sight: usize) {
        let window = TableTab::window_of(first_in_sight, last_in_sight, self.row_count);
        if self.window != Some(window) {
            self.window = Some(window);
            self.session.set_window(window.0, window.1);
            self.drain();
        }
    }

    /// Writes the edits that wait, and takes what that gave: run once a frame, after the frame that showed them.
    pub fn write_behind(&mut self) {
        if self.session.has_pending_writes() {
            self.session.write_pending();
        }
        self.drain();
    }

    /// The file changed on disk, or a file a relation points at was written: read again.
    pub fn changed_on_disk(&mut self, path: &Path) {
        self.session.written(path);
        self.drain();
    }

    /// Sends one gesture, said in the view it was made in. The answer: empty when it was accepted, else the sentence
    /// it was refused with, or the question it was answered with.
    pub fn edit(&mut self, mut gesture: TableGesture) -> String {
        if gesture.view_id.is_empty() {
            gesture.view_id = self.settings.view_id.clone();
        }
        self.next_edit += 1;
        let edit_id = format!("edit-{}", self.next_edit);
        let answer = self.session.edit(&edit_id, &gesture);
        self.drain();
        answer
    }

    /// Sends a gesture made outside an editor: a refusal is shown on the table's own line, and a question is put.
    pub fn raise(&mut self, gesture: TableGesture) {
        self.refusal.clear();
        let answer = self.edit(gesture.clone());
        if let Some(question) = answer.strip_prefix(CONFIRM_PREFIX) {
            self.asking = Some((gesture, question.to_owned()));
        } else {
            self.refusal = answer;
        }
    }

    /// Makes a gesture that was answered with a question again, confirmed.
    pub fn confirm(&mut self) {
        if let Some((gesture, _)) = self.asking.take() {
            self.raise(gesture.setting("confirmed", "true"));
        }
    }

    /// Shows another view: this connection's own, and remembered in the file as the view it was left in.
    pub fn show_view(&mut self, view_id: &str) {
        self.editing = None;
        self.active = None;
        self.window = None;
        self.session.set_active_view(view_id);
        self.scroll_to = Some(0.0);
        self.drain();
    }

    /// Undoes the latest edit written through this tab.
    pub fn undo(&mut self) -> Result<(), String> {
        self.session.write_pending();
        let result = self.session.undo();
        self.drain();
        result
    }

    /// Makes the latest undone edit again.
    pub fn redo(&mut self) -> Result<(), String> {
        let result = self.session.redo();
        self.drain();
        result
    }

    /// The line at an index of the view, when the session has given it.
    pub fn line(&self, index: usize) -> Option<&TableRow> {
        self.held.get(&index)
    }

    /// The row, column and editor at a cell, when that cell can be edited now.
    pub fn editable_at(&self, line: usize, column: usize) -> Option<(&TableRow, &TableColumn, EditorKind)> {
        let row = self.line(line)?;
        let col = *self.visible_columns().get(column)?;
        let editor = EditorKind::of(&col.kind)?;
        if row.is_group || row.is_new_row || !self.editable() {
            return None;
        }
        Some((row, col, editor))
    }

    /// Opens the editor of a cell, or for a tick sends the other value at once: a tick has no editor.
    pub fn start_edit(&mut self, line: usize, column: usize, replace: Option<String>) {
        let Some((row, col, editor)) = self.editable_at(line, column) else {
            return;
        };
        let (row_id, column_id) = (row.id.clone(), col.id.clone());
        let values = row.cell(&column_id).map(|c| c.values.clone()).unwrap_or_default();
        self.refusal.clear();
        self.active = Some((line, column));
        if editor == EditorKind::Checkbox {
            let next = if values.first().is_some_and(|v| v == "true") {
                "false"
            } else {
                "true"
            };
            let answer = self.edit(
                TableGesture::new("setCell")
                    .row(row_id)
                    .column(column_id)
                    .values([next]),
            );
            self.refusal = answer;
            return;
        }
        let initial = if editor.is_list() {
            String::new()
        } else {
            values.first().cloned().unwrap_or_default()
        };
        self.editing = Some(CellEditing {
            line,
            column,
            row_id,
            column_id,
            editor,
            text: replace.unwrap_or_else(|| initial.clone()),
            initial,
            reason: String::new(),
            focused: false,
        });
    }

    /// Sends the value of the cell being edited: a value that was not changed is no edit. The editor closes when
    /// it is accepted and stays open with the reason when it is refused; true when it closed.
    pub fn commit_edit(&mut self) -> bool {
        let Some(editing) = self.editing.clone() else {
            return true;
        };
        if editing.editor.is_list() || editing.text == editing.initial {
            self.editing = None;
            return true;
        }
        let values: Vec<String> = if editing.text.is_empty() {
            Vec::new()
        } else {
            vec![editing.text.clone()]
        };
        let answer = self.edit(
            TableGesture::new("setCell")
                .row(&editing.row_id)
                .column(&editing.column_id)
                .values(values),
        );
        if answer.is_empty() {
            self.editing = None;
            true
        } else {
            if let Some(open) = &mut self.editing {
                open.reason = answer;
            }
            false
        }
    }

    /// Picks one entry of a list editor: an option or a related row. One choice replaces the one there was and
    /// closes the list; several are added to or taken from what the cell holds.
    pub fn pick(&mut self, id: &str) {
        let Some(editing) = self.editing.clone() else {
            return;
        };
        let column = self.columns.iter().find(|c| c.id == editing.column_id);
        let several = editing.editor == EditorKind::Options
            || (editing.editor == EditorKind::Rows && column.is_some_and(|c| c.setting_of_limit() != "one"));
        let chosen: Vec<String> = self
            .held
            .get(&editing.line)
            .and_then(|r| r.cell(&editing.column_id))
            .map(|c| c.values.clone())
            .unwrap_or_default();
        let has = chosen.iter().any(|v| v == id);
        let values: Vec<String> = if several {
            if has {
                chosen.into_iter().filter(|v| v != id).collect()
            } else {
                chosen.into_iter().chain([id.to_owned()]).collect()
            }
        } else if has {
            Vec::new()
        } else {
            vec![id.to_owned()]
        };
        let answer = self.edit(
            TableGesture::new("setCell")
                .row(&editing.row_id)
                .column(&editing.column_id)
                .values(values),
        );
        if let Some(open) = &mut self.editing {
            open.reason = answer.clone();
        }
        if answer.is_empty() && !several {
            self.editing = None;
        }
    }

    /// Makes an option of the list editor's text and gives it to the cell (`newOption`).
    pub fn create_option(&mut self) {
        let Some(editing) = self.editing.clone() else {
            return;
        };
        let name = editing.text.trim().to_owned();
        if name.is_empty() || editing.editor == EditorKind::Rows {
            return;
        }
        let several = editing.editor == EditorKind::Options;
        let chosen: Vec<String> = if several {
            self.held
                .get(&editing.line)
                .and_then(|r| r.cell(&editing.column_id))
                .map(|c| c.values.clone())
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let answer = self.edit(
            TableGesture::new("setCell")
                .row(&editing.row_id)
                .column(&editing.column_id)
                .values(chosen)
                .setting("newOption", name),
        );
        if let Some(open) = &mut self.editing {
            open.reason = answer.clone();
            open.text.clear();
        }
        if answer.is_empty() && !several {
            self.editing = None;
        }
    }

    /// Adds a row at the line a row is added at: the bottom of the table, or of the group whose key it carries.
    pub fn add_row(&mut self, group: &str) {
        self.raise(TableGesture::new("addRow").setting("group", group));
    }

    /// Selects a row: the window's selection, whose values the property grid shows.
    pub fn select_row(&mut self, row_id: Option<String>) {
        self.selected = row_id;
    }

    /// The property grid's rows for the selected row, as the table shows it now.
    pub fn property_rows(&self) -> Vec<PropertyRow> {
        match &self.selected {
            Some(row) => row_context::properties(self.session.shown(), &self.read_only_reason, row),
            None => Vec::new(),
        }
    }

    /// What the selected row is called.
    pub fn selected_title(&self) -> Option<String> {
        let row = self.selected.as_ref()?;
        row_context::describe_in(self.session.shown(), row).map(|d| d.title)
    }

    /// Sets one of the selected row's values from the property grid: the same edit it is in the table.
    pub fn set_property(&mut self, property_id: &str, value: &str) {
        let Some(row) = self.selected.clone() else {
            return;
        };
        match row_context::gesture(self.session.shown(), &self.read_only_reason, &row, property_id, value) {
            Ok(gesture) => self.raise(gesture),
            Err(reason) => self.refusal = reason,
        }
    }
}

/// What a column's settings say of how many rows a relation's cell holds.
trait LimitOf {
    fn setting_of_limit(&self) -> &str;
}

impl LimitOf for TableColumn {
    fn setting_of_limit(&self) -> &str {
        self.settings.get("limit").map(String::as_str).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests;
