//! A table's content as a designer module reads it out of its document: its columns, its views with every setting
//! they have, and its rows (the standalone host's `KnowledgeTable` and the records beside it in `KnowledgeModel.cs`,
//! at 8b6ccfc, made host-neutral). A column carries what its DESL value type says of it ([`ValueKind`]), so the view
//! engine compares, sorts and groups by the specification rather than by a designer's own type names.

use crate::loading::specification::{DeslValueType, Editor, SortKind};

/// What a column's value type says of how its values are held, compared and sorted (DESL section 7.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueKind {
    pub editor: Editor,
    pub sort: SortKind,
    /// The comparisons a filter offers on it, besides `is-empty` and `is-not-empty`.
    pub comparisons: Vec<String>,
    /// The key a cell holds a value of this type under, as the document's reader names it. A cell under any other
    /// key holds a value of another type - one its column had before - and has no value of this one.
    pub cell_key: String,
    pub many: bool,
}

impl ValueKind {
    /// What a value type of the specification says, with the key its cells are read under.
    pub fn of(value_type: &DeslValueType, cell_key: impl Into<String>) -> Self {
        ValueKind {
            editor: value_type.editor,
            sort: value_type.sort,
            comparisons: value_type.comparisons.clone(),
            cell_key: cell_key.into(),
            many: value_type.many,
        }
    }

    /// A value type the specification does not have: its values are text held under the type's own name, sorted
    /// as text, and only `is-empty` and `is-not-empty` compare them.
    pub fn unknown(value_type: &str) -> Self {
        ValueKind {
            editor: Editor::Text,
            sort: SortKind::Text,
            comparisons: Vec::new(),
            cell_key: value_type.to_owned(),
            many: false,
        }
    }

    /// Whether a filter can compare a value of this type in this way.
    pub fn compares(&self, comparison: &str) -> bool {
        matches!(comparison, "is-empty" | "is-not-empty") || self.comparisons.iter().any(|c| c == comparison)
    }
}

/// One column: its value type and what its type has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentColumn {
    pub id: String,
    pub name: String,
    /// The value type, a key of the surface's `valueTypes`, as the document writes it.
    pub value_type: String,
    pub kind: ValueKind,
    pub is_title: bool,
    pub options: Vec<ContentOption>,
    /// The document a column of rows relates to, `.` for the document itself (DESL section 5.4).
    pub target_file: String,
    /// How many rows a column of rows holds: `one` or `none` (no limit).
    pub limit: String,
    /// The other side of a two-way relation, by its column's id in the target.
    pub counterpart: String,
    /// Whether its values are read from the other side rather than held.
    pub is_computed: bool,
    /// Whether its values are the row's parent within the same document, for nesting (DESL section 7.3).
    pub is_parent: bool,
}

impl ContentColumn {
    /// A column of `value_type` with what its kind says, and nothing else.
    pub fn new(id: impl Into<String>, name: impl Into<String>, value_type: impl Into<String>, kind: ValueKind) -> Self {
        ContentColumn {
            id: id.into(),
            name: name.into(),
            value_type: value_type.into(),
            kind,
            is_title: false,
            options: Vec::new(),
            target_file: String::new(),
            limit: "none".into(),
            counterpart: String::new(),
            is_computed: false,
            is_parent: false,
        }
    }
}

/// One option of a choice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentOption {
    pub id: String,
    pub name: String,
    pub colour: String,
}

impl ContentOption {
    pub fn new(id: impl Into<String>, name: impl Into<String>, colour: impl Into<String>) -> Self {
        ContentOption {
            id: id.into(),
            name: name.into(),
            colour: colour.into(),
        }
    }
}

/// What a view says about one column: whether it shows, how wide, whether it wraps, and what the line under it
/// calculates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewColumn {
    pub column_id: String,
    pub visible: bool,
    pub width: i64,
    pub wrap: bool,
    pub calculate: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewSort {
    pub column_id: String,
    pub descending: bool,
}

/// A filter's item: a condition, or a group of items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterItem {
    Group(FilterGroup),
    Condition(Condition),
}

/// Items joined by *all* or by *any*. A view's own filter is the outermost group.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FilterGroup {
    pub any: bool,
    pub items: Vec<FilterItem>,
}

impl FilterGroup {
    pub fn all(items: Vec<FilterItem>) -> Self {
        FilterGroup { any: false, items }
    }

    pub fn any(items: Vec<FilterItem>) -> Self {
        FilterGroup { any: true, items }
    }

    /// Every condition, at any depth, in order.
    pub fn conditions(&self) -> Vec<&Condition> {
        let mut found = Vec::new();
        for item in &self.items {
            match item {
                FilterItem::Group(group) => found.extend(group.conditions()),
                FilterItem::Condition(condition) => found.push(condition),
            }
        }
        found
    }
}

/// One comparison of a column's value. The value is in its written form, empty for a comparison that takes none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Condition {
    pub column_id: String,
    pub operator: String,
    pub value: String,
}

impl Condition {
    pub fn new(column_id: impl Into<String>, operator: impl Into<String>, value: impl Into<String>) -> Self {
        Condition {
            column_id: column_id.into(),
            operator: operator.into(),
            value: value.into(),
        }
    }
}

/// One view, with everything it stores.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContentView {
    pub id: String,
    pub name: String,
    pub columns: Vec<ViewColumn>,
    pub sorts: Vec<ViewSort>,
    pub filter: FilterGroup,
    pub group_by: String,
    pub hide_empty_groups: bool,
    pub group_order: Vec<String>,
    pub hidden_groups: Vec<String>,
    /// The groups, and the parent rows, that are folded away.
    pub collapsed: Vec<String>,
    /// The column the view's columns are frozen up to, or empty.
    pub freeze: String,
}

impl ContentView {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        ContentView {
            id: id.into(),
            name: name.into(),
            ..ContentView::default()
        }
    }
}

/// A row's value for one column, in written form: one value, or several for a kind that holds several.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentCell {
    pub column_id: String,
    pub values: Vec<String>,
    /// The key the document holds the value under: the key of the type the value was written as, which after a
    /// change of type need not be its column's; empty for a cell without a value.
    pub key: String,
}

impl ContentCell {
    pub fn new<S: Into<String>>(
        column_id: impl Into<String>,
        values: impl IntoIterator<Item = S>,
        key: impl Into<String>,
    ) -> Self {
        ContentCell {
            column_id: column_id.into(),
            values: values.into_iter().map(Into::into).collect(),
            key: key.into(),
        }
    }
}

/// One row and the cells it holds. A column without a cell is empty in this row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentRow {
    pub id: String,
    pub cells: Vec<ContentCell>,
}

impl ContentRow {
    pub fn new(id: impl Into<String>, cells: Vec<ContentCell>) -> Self {
        ContentRow { id: id.into(), cells }
    }

    /// The row's cell for a column, whatever key it holds its value under.
    pub fn cell(&self, column_id: &str) -> Option<&ContentCell> {
        self.cells.iter().find(|c| c.column_id == column_id)
    }
}

/// A table's content: its name, the view it was left in, and its columns, views and rows in the document's order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableContent {
    pub name: String,
    /// The view the table was last left in, or empty for the first.
    pub active_view_id: String,
    pub columns: Vec<ContentColumn>,
    pub views: Vec<ContentView>,
    /// The rows in the document's order: their order where no sort applies.
    pub rows: Vec<ContentRow>,
}

impl TableContent {
    /// The view a connection shows when it names none: the one the document was left in, else the first.
    pub fn view_or_default(&self, view_id: &str) -> Option<&ContentView> {
        self.views
            .iter()
            .find(|v| v.id == view_id)
            .or_else(|| self.views.iter().find(|v| v.id == self.active_view_id))
            .or_else(|| self.views.first())
    }

    pub fn column(&self, id: &str) -> Option<&ContentColumn> {
        self.columns.iter().find(|c| c.id == id)
    }

    pub fn view(&self, id: &str) -> Option<&ContentView> {
        self.views.iter().find(|v| v.id == id)
    }

    pub fn row(&self, id: &str) -> Option<&ContentRow> {
        self.rows.iter().find(|r| r.id == id)
    }

    /// The column that names a row, when there is one.
    pub fn title_column(&self) -> Option<&ContentColumn> {
        self.columns.iter().find(|c| c.is_title)
    }

    /// The column whose values are a row's parent, when there is one.
    pub fn parent_column(&self) -> Option<&ContentColumn> {
        self.columns.iter().find(|c| c.is_parent)
    }
}
