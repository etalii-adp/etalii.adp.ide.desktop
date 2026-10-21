//! The table model: what crosses from the core to the shell for a table, and what the shell's table draws (the
//! standalone host's `EtAlii.Adp.Designer.TableModel`, `Table.cs` and `TableRows.cs`, at 8b6ccfc). It names no
//! designer type; a designer module maps its own document onto it, as a diagram module maps onto a scene.
//!
//! A table as a connection first sees it is a [`TableBaseline`]: everything but the rows, which follow as
//! [`TableChange::Rows`] for the window the shell has in sight. Every line of a view - a row, a group's heading, or
//! the place a new row is added at - is a [`TableRow`], because they share one order: the shell asks for lines by
//! index and draws what is there.

use indexmap::IndexMap;

/// Named settings, in the order they were given.
pub type Settings = IndexMap<String, String>;

/// A table as a connection first sees it: its columns, its views, the active view's settings and how many lines that
/// view has - everything but the rows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableBaseline {
    /// The table's name.
    pub title: String,
    /// Every column, in the active view's order.
    pub columns: Vec<TableColumn>,
    /// Every view, in their order.
    pub views: Vec<TableView>,
    /// The active view's settings.
    pub settings: TableViewSettings,
    /// The lines of the active view: rows, group headings and the lines a row is added at.
    pub row_count: usize,
    /// What the document's reading and the designer's rules reported.
    pub findings: Vec<TableFinding>,
    /// Why the table cannot be edited, or empty when it can.
    pub read_only_reason: String,
}

/// One column of a table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableColumn {
    /// The column's id, stable under renaming and reordering.
    pub id: String,
    /// The name its header shows.
    pub name: String,
    /// The kind of value its cells hold, in the designer's own vocabulary.
    pub kind: String,
    /// The options of a selection, in their order.
    pub options: Vec<TableOption>,
    /// Its width in pixels in the active view, or 0 for the table's default.
    pub width: i64,
    /// Whether the active view shows it.
    pub visible: bool,
    /// Whether it is the column that names a row.
    pub is_title: bool,
    /// Whether its cells wrap in the active view.
    pub wraps: bool,
    /// What else its kind has, by name.
    pub settings: Settings,
}

/// One option of a selection. Its colour is a colour name of the theme, never a colour value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableOption {
    pub id: String,
    pub name: String,
    pub color: String,
}

/// One view of a table, as its tab shows it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableView {
    pub id: String,
    pub name: String,
}

/// What a view does to the table it shows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableViewSettings {
    /// The view these are the settings of.
    pub view_id: String,
    /// The sorts, the first deciding first.
    pub sorts: Vec<TableSort>,
    /// The filter, or none when the view filters nothing.
    pub filter: Option<TableFilterGroup>,
    /// The column the rows are grouped by, or empty.
    pub group_by: String,
    /// Whether a group without rows is left out.
    pub hides_empty_groups: bool,
    /// What else the view stores, by name.
    pub settings: Settings,
}

/// A sort by one column.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableSort {
    pub column_id: String,
    pub descending: bool,
}

/// A filter's item: a condition on a column, or a group of items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableFilterItem {
    Group(TableFilterGroup),
    Condition(TableCondition),
}

/// Items a row passes when all of them hold, or when any does.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableFilterGroup {
    /// True when one holding item is enough.
    pub any: bool,
    pub items: Vec<TableFilterItem>,
}

/// A comparison of a column's value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableCondition {
    pub column_id: String,
    /// The comparison, in the designer's own vocabulary.
    pub comparison: String,
    /// What it is compared with; none for a comparison that needs none.
    pub values: Vec<String>,
}

/// How serious a finding is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum TableFindingSeverity {
    #[default]
    Info,
    Warning,
    Error,
}

/// Something the document's reading or the designer's rules reported.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableFinding {
    pub code: String,
    pub severity: TableFindingSeverity,
    /// The sentence shown.
    pub message: String,
    /// The row of the cell it is about, or empty.
    pub row_id: String,
    /// The column of the cell it is about, or empty.
    pub column_id: String,
}

impl TableFinding {
    pub fn new(code: impl Into<String>, severity: TableFindingSeverity, message: impl Into<String>) -> Self {
        TableFinding {
            code: code.into(),
            severity,
            message: message.into(),
            ..TableFinding::default()
        }
    }

    pub fn at(mut self, row_id: impl Into<String>, column_id: impl Into<String>) -> Self {
        self.row_id = row_id.into();
        self.column_id = column_id.into();
        self
    }
}

/// One line of a view: a row, the heading of a group of rows, or the place a new row is added at.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableRow {
    /// A row's id, or a group's key; empty for the line a row is added at the bottom of the table.
    pub id: String,
    /// How deep the row is nested under parent rows.
    pub depth: usize,
    /// A row's cells that hold a value; an empty cell is not sent.
    pub cells: Vec<TableCell>,
    /// Whether this line is a group's heading.
    pub is_group: bool,
    /// A group heading's text.
    pub label: String,
    /// A group heading's number of rows.
    pub count: usize,
    /// Whether the group, or the rows under this row, are folded away.
    pub collapsed: bool,
    /// Whether rows are nested under this row.
    pub has_children: bool,
    /// Whether this line is where a new row is added: the bottom of the table, or of the group whose key is its id.
    pub is_new_row: bool,
}

impl TableRow {
    /// A group's heading.
    pub fn group(key: impl Into<String>, label: impl Into<String>, count: usize, collapsed: bool) -> Self {
        TableRow {
            id: key.into(),
            is_group: true,
            label: label.into(),
            count,
            collapsed,
            ..TableRow::default()
        }
    }

    /// The line a row is added at: the bottom of the table for an empty key, else of the group of that key.
    pub fn new_row(key: impl Into<String>) -> Self {
        TableRow {
            id: key.into(),
            is_new_row: true,
            ..TableRow::default()
        }
    }

    /// The cell of a column, when the row holds a value for it.
    pub fn cell(&self, column_id: &str) -> Option<&TableCell> {
        self.cells.iter().find(|c| c.column_id == column_id)
    }
}

/// A row's value for one column.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableCell {
    pub column_id: String,
    /// The value in its written form; several for a kind that holds several.
    pub values: Vec<String>,
    /// What to show per value where that is not the value itself, e.g. a related row's title.
    pub labels: Vec<String>,
    /// Whether the value shown is an edit that has not been written yet.
    pub pending: bool,
}

/// The columns, the views or the active view's settings changed: all of them, anew.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableStructure {
    pub title: String,
    pub columns: Vec<TableColumn>,
    pub views: Vec<TableView>,
    pub settings: TableViewSettings,
    pub row_count: usize,
    pub read_only_reason: String,
}

/// Something that changed in an open table, pushed to the connection showing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableChange {
    /// The lines of the window, replacing what the shell holds there, and the view's total, which an edit or a
    /// filter may have changed.
    Rows {
        first: usize,
        rows: Vec<TableRow>,
        row_count: usize,
    },
    Structure(Box<TableStructure>),
    /// The findings: all of them, replacing what the shell holds.
    Findings(Vec<TableFinding>),
    /// What became of an edit that was accepted: written, or refused - and then every later edit that waited on it
    /// is taken back with it.
    EditSettled {
        edit_id: String,
        written: bool,
        error: String,
        taken_back: Vec<String>,
    },
}

/// What the author did, in the table's own words. The kinds are a session's to define and the shell's table to
/// raise; the host carries a gesture and knows none.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableGesture {
    pub kind: String,
    pub row_id: String,
    pub column_id: String,
    pub view_id: String,
    /// The value or values given, in their written form.
    pub values: Vec<String>,
    /// A position, for a gesture that places something.
    pub index: i64,
    /// A second thing it names: what something is placed beside.
    pub target_id: String,
    pub settings: Settings,
}

impl TableGesture {
    pub fn new(kind: impl Into<String>) -> Self {
        TableGesture {
            kind: kind.into(),
            ..TableGesture::default()
        }
    }

    pub fn row(mut self, row_id: impl Into<String>) -> Self {
        self.row_id = row_id.into();
        self
    }

    pub fn column(mut self, column_id: impl Into<String>) -> Self {
        self.column_id = column_id.into();
        self
    }

    pub fn view(mut self, view_id: impl Into<String>) -> Self {
        self.view_id = view_id.into();
        self
    }

    pub fn values<S: Into<String>>(mut self, values: impl IntoIterator<Item = S>) -> Self {
        self.values = values.into_iter().map(Into::into).collect();
        self
    }

    pub fn index(mut self, index: i64) -> Self {
        self.index = index;
        self
    }

    pub fn target(mut self, target_id: impl Into<String>) -> Self {
        self.target_id = target_id.into();
        self
    }

    pub fn setting(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.settings.insert(name.into(), value.into());
        self
    }

    /// A named setting, or empty.
    pub fn setting_of(&self, name: &str) -> &str {
        self.settings.get(name).map(String::as_str).unwrap_or_default()
    }
}
