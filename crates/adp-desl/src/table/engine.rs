//! A view's lines: the rows its filter lets through and its search finds, in the order its sorts give, under the
//! headings of its grouping or nested under their parents - computed on the core side, so the shell draws a window
//! of them and never ten thousand rows (the standalone host's `KnowledgeViewEngine` at 8b6ccfc).
//!
//! - **A value is compared as what its column's type says it is**: a number as a number, a date as a moment, a
//!   choice by its option's place among the options - by the DESL value type's `editor` and `sort` (DESL section
//!   7.5). A cell that still holds a value of a type the column had before has no value of this type, and is
//!   compared as empty.
//! - **A condition that is not finished filters nothing.** A comparison that needs a value and has none yet lets
//!   every row through, so a filter being built does not empty the table.
//! - **A sort is stable**: rows the sorts do not tell apart keep the document's order, and a row without a value
//!   comes after those with one, whichever way the sort runs.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use super::content::{
    Condition, ContentColumn, ContentRow, ContentView, FilterGroup, FilterItem, TableContent, ViewSort,
};
use super::model::{TableCell, TableRow};
use super::values;
use crate::loading::specification::{Editor, SortKind};

/// The key of the group of rows that have no value for the grouping column.
pub const NO_VALUE: &str = "none";

/// What a related row is called, by the relation's column and the row's id; empty for one that is not found.
pub type LabelOf<'a> = &'a dyn Fn(&str, &str) -> String;

/// The cells whose value is an edit not yet written, by row and column.
pub type Unwritten = HashSet<(String, String)>;

/// What a view's lines are made from besides the table: the rows kept in sight, the cells not yet written, whether
/// lines to add a row at are given, how a related row is called, and what the rows are searched for.
#[derive(Clone, Copy, Default)]
pub struct LineOptions<'a> {
    /// Rows shown whatever the filter says: those added while this view was open.
    pub kept: Option<&'a HashSet<String>>,
    pub unwritten: Option<&'a Unwritten>,
    /// Whether a line to add a row at is given, at the bottom and under every group.
    pub editable: bool,
    pub label_of: Option<LabelOf<'a>>,
    /// Only rows with it in one of their values are shown. Empty for all of them.
    pub search: &'a str,
}

impl LineOptions<'_> {
    fn is_kept(&self, row_id: &str) -> bool {
        self.kept.is_some_and(|k| k.contains(row_id))
    }
}

/// The values a row has for a column, as values of the column's own type: none when the cell holds none, or holds
/// one of another type.
pub fn values_of<'r>(row: &'r ContentRow, column: &ContentColumn) -> &'r [String] {
    match row.cell(&column.id) {
        Some(cell) if cell.key == column.kind.cell_key => &cell.values,
        _ => &[],
    }
}

/// One row as a line of the view, holding the cells that have a value (`KnowledgeTableMapper.Row`). A cell among
/// `unwritten` shows an edit that is not in the document yet, and says so; a related row is shown by its title.
pub fn line_of(table: &TableContent, row: &ContentRow, options: &LineOptions<'_>) -> TableRow {
    let cells = row
        .cells
        .iter()
        .filter(|cell| !cell.values.is_empty())
        .map(|cell| {
            let related = table
                .column(&cell.column_id)
                .is_some_and(|c| c.kind.editor == Editor::Rows && c.kind.cell_key == cell.key);
            let labels = match (related, options.label_of) {
                (true, Some(label_of)) => cell.values.iter().map(|v| label_of(&cell.column_id, v)).collect(),
                _ => Vec::new(),
            };
            TableCell {
                column_id: cell.column_id.clone(),
                values: cell.values.clone(),
                labels,
                pending: options
                    .unwritten
                    .is_some_and(|u| u.contains(&(row.id.clone(), cell.column_id.clone()))),
            }
        })
        .collect();
    TableRow {
        id: row.id.clone(),
        cells,
        ..TableRow::default()
    }
}

/// The lines of `view`, or of a table without one - every row, in the document's order - when it is none.
pub fn lines(table: &TableContent, view: Option<&ContentView>, options: &LineOptions<'_>) -> Vec<TableRow> {
    let shown = in_sight(table, view, options);
    let line = |row: &ContentRow, depth: usize, has_children: bool, collapsed: bool| TableRow {
        depth,
        has_children,
        collapsed,
        ..line_of(table, row, options)
    };

    // A table with a parent relation shows its rows under their parents in every view, as Notion shows sub-items:
    // in a view without groups, and under each heading of one with them.
    let parent = table.parent_column();
    let mut lines = Vec::new();
    match view {
        None => lines.extend(shown.iter().map(|row| line(row, 0, false, false))),
        Some(view) => match table.column(&view.group_by).filter(|g| !g.is_parent) {
            None => match parent {
                None => lines.extend(shown.iter().map(|row| line(row, 0, false, false))),
                Some(parent) => nest(&shown, parent, view, &line, &mut lines),
            },
            Some(grouped) => {
                group(&shown, grouped, view, &line, &mut lines, options, parent);
                return lines;
            }
        },
    }
    if options.editable {
        lines.push(TableRow::new_row(""));
    }
    lines
}

/// The rows a view has in sight, in its order: those its filter lets through and the search finds. They are what
/// its lines are made of, and what the line under a column calculates over.
pub fn in_sight<'t>(
    table: &'t TableContent,
    view: Option<&ContentView>,
    options: &LineOptions<'_>,
) -> Vec<&'t ContentRow> {
    let columns: HashMap<&str, &ContentColumn> = table.columns.iter().map(|c| (c.id.as_str(), c)).collect();
    let sought = options.search.trim();
    let rows = table.rows.iter().filter(|row| {
        sought.is_empty() || options.is_kept(&row.id) || mentions(row, sought, &columns, options.label_of)
    });
    match view {
        None => rows.collect(),
        Some(view) => {
            let passing: Vec<&ContentRow> = rows
                .filter(|row| options.is_kept(&row.id) || passes(&view.filter, row, &columns))
                .collect();
            sorted(passing, &view.sorts, &columns)
        }
    }
}

/// Whether a row has the text in what one of its cells shows: a value as written, an option by its name, a related
/// row by its title. A checkbox shows no text, and is never what a search finds.
fn mentions(
    row: &ContentRow,
    sought: &str,
    columns: &HashMap<&str, &ContentColumn>,
    label_of: Option<LabelOf<'_>>,
) -> bool {
    row.cells.iter().any(|cell| {
        columns.get(cell.column_id.as_str()).is_some_and(|column| {
            column.kind.editor != Editor::Checkbox
                && values_of(row, column)
                    .iter()
                    .any(|value| values::contains_ignoring_case(&shown(column, value, label_of), sought))
        })
    })
}

/// A value as its cell shows it.
fn shown(column: &ContentColumn, value: &str, label_of: Option<LabelOf<'_>>) -> String {
    match column.kind.editor {
        Editor::Choice | Editor::Choices => column
            .options
            .iter()
            .find(|o| o.id == value)
            .map(|o| o.name.clone())
            .unwrap_or_default(),
        Editor::Rows => label_of.map(|l| l(&column.id, value)).unwrap_or_default(),
        _ => value.to_owned(),
    }
}

// ---- the filter ----

fn passes(group: &FilterGroup, row: &ContentRow, columns: &HashMap<&str, &ContentColumn>) -> bool {
    if group.items.is_empty() {
        return true;
    }
    let holds = |item: &FilterItem| match item {
        FilterItem::Group(nested) => passes(nested, row, columns),
        FilterItem::Condition(condition) => condition_holds(condition, row, columns),
    };
    if group.any {
        group.items.iter().any(holds)
    } else {
        group.items.iter().all(holds)
    }
}

/// Whether one condition holds for a row (DESL section 7.5's comparisons).
pub fn condition_holds(condition: &Condition, row: &ContentRow, columns: &HashMap<&str, &ContentColumn>) -> bool {
    // A condition naming a column that is gone is a setting the view opens without.
    let Some(column) = columns.get(condition.column_id.as_str()) else {
        return true;
    };
    // A comparison its column's type does not have - written by hand, or left from a type the column had before -
    // is a setting the view opens without, too.
    if !column.kind.compares(&condition.operator) {
        return true;
    }
    let values = values_of(row, column);
    let value = values.first().map(String::as_str).unwrap_or_default();
    let given = condition.value.as_str();
    match condition.operator.as_str() {
        "is-empty" => return values.is_empty(),
        "is-not-empty" => return !values.is_empty(),
        "is-checked" => return value == "true",
        "is-not-checked" => return value != "true",
        _ => {}
    }
    // Every other comparison is with a value. Without one the condition is not finished, and filters nothing.
    if given.is_empty() {
        return true;
    }
    let operator = condition.operator.as_str();
    match column.kind.editor {
        Editor::Number => {
            let Some(wanted) = values::parse_number(given) else {
                return true;
            };
            let Some(number) = values::parse_number(value) else {
                // A row without a number is not equal to one, and is neither more nor less.
                return operator == "does-not-equal";
            };
            match operator {
                "equals" => number == wanted || (number.is_nan() && wanted.is_nan()),
                "does-not-equal" => !(number == wanted || (number.is_nan() && wanted.is_nan())),
                "greater-than" => number > wanted,
                "less-than" => number < wanted,
                "at-least" => number >= wanted,
                "at-most" => number <= wanted,
                _ => true,
            }
        }
        editor if editor.is_moment() => {
            let Some(asked) = values::moment(editor, given) else {
                return true;
            };
            let Some(moment) = values::moment(editor, value) else {
                return false;
            };
            match operator {
                "is" => moment == asked,
                "is-before" => moment < asked,
                "is-after" => moment > asked,
                "is-on-or-before" => moment <= asked,
                "is-on-or-after" => moment >= asked,
                _ => true,
            }
        }
        Editor::Choices | Editor::Rows => match operator {
            "contains" => values.iter().any(|v| v == given),
            "does-not-contain" => !values.iter().any(|v| v == given),
            _ => true,
        },
        Editor::Choice => match operator {
            "is" => value == given,
            "is-not" => value != given,
            _ => true,
        },
        _ => match operator {
            "is" => values::equals_ignoring_case(value, given),
            "is-not" => !values::equals_ignoring_case(value, given),
            "contains" => values::contains_ignoring_case(value, given),
            "does-not-contain" => !values::contains_ignoring_case(value, given),
            "starts-with" => values::starts_with_ignoring_case(value, given),
            "ends-with" => values::ends_with_ignoring_case(value, given),
            _ => true,
        },
    }
}

// ---- the sorts ----

/// What a row is sorted by for one column: a number, or a text, or nothing.
#[derive(Debug, Clone)]
struct SortKey {
    number: f64,
    text: String,
    has_value: bool,
}

impl SortKey {
    const EMPTY: SortKey = SortKey {
        number: 0.0,
        text: String::new(),
        has_value: false,
    };

    fn number(number: f64) -> Self {
        SortKey {
            number,
            text: String::new(),
            has_value: true,
        }
    }

    fn compare(left: &SortKey, right: &SortKey, descending: bool) -> Ordering {
        if !left.has_value || !right.has_value {
            // Without a value is last, in either direction.
            return match (left.has_value, right.has_value) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => Ordering::Equal,
            };
        }
        let mut order = values::compare_numbers(left.number, right.number);
        if order == Ordering::Equal {
            order = values::compare_ignoring_case(&left.text, &right.text);
        }
        if descending { order.reverse() } else { order }
    }
}

fn key_of(row: &ContentRow, column: &ContentColumn) -> SortKey {
    let Some(value) = values_of(row, column).first() else {
        return SortKey::EMPTY;
    };
    match column.kind.sort {
        SortKind::Number => values::parse_number(value).map_or(SortKey::EMPTY, SortKey::number),
        // Unticked first.
        SortKind::FalseFirst => SortKey::number(if value == "true" { 1.0 } else { 0.0 }),
        SortKind::Chronological => {
            values::moment(column.kind.editor, value).map_or(SortKey::EMPTY, |m| SortKey::number(m as f64))
        }
        // By the option's place among the options, not by its name.
        SortKind::OptionOrder => column
            .options
            .iter()
            .position(|o| &o.id == value)
            .map_or(SortKey::EMPTY, |place| SortKey::number(place as f64)),
        // A related row sorts by what it is written as, as the standalone host sorts it.
        SortKind::Text | SortKind::Title => SortKey {
            number: 0.0,
            text: value.clone(),
            has_value: true,
        },
    }
}

/// The rows by the sorts in turn, the first deciding first.
fn sorted<'t>(
    rows: Vec<&'t ContentRow>,
    sorts: &[ViewSort],
    columns: &HashMap<&str, &ContentColumn>,
) -> Vec<&'t ContentRow> {
    let applied: Vec<(&ContentColumn, bool)> = sorts
        .iter()
        .filter_map(|s| columns.get(s.column_id.as_str()).map(|c| (*c, s.descending)))
        .collect();
    if applied.is_empty() {
        return rows;
    }
    let mut keyed: Vec<(Vec<SortKey>, &ContentRow)> = rows
        .into_iter()
        .map(|row| (applied.iter().map(|(c, _)| key_of(row, c)).collect(), row))
        .collect();
    keyed.sort_by(|(left, _), (right, _)| {
        for (index, (_, descending)) in applied.iter().enumerate() {
            let order = SortKey::compare(&left[index], &right[index], *descending);
            if order != Ordering::Equal {
                return order;
            }
        }
        Ordering::Equal
    });
    keyed.into_iter().map(|(_, row)| row).collect()
}

// ---- groups ----

type Line<'l> = dyn Fn(&ContentRow, usize, bool, bool) -> TableRow + 'l;

fn group(
    rows: &[&ContentRow],
    grouped: &ContentColumn,
    view: &ContentView,
    line: &Line<'_>,
    lines: &mut Vec<TableRow>,
    options: &LineOptions<'_>,
    parent: Option<&ContentColumn>,
) {
    // The groups there are, in their natural order: an option each, in the options' order; unticked and ticked; a
    // related row each, as they are first met. The rows without a value come last.
    let mut groups: Vec<(String, String)> = match grouped.kind.editor {
        Editor::Choice | Editor::Choices => grouped.options.iter().map(|o| (o.id.clone(), o.name.clone())).collect(),
        Editor::Checkbox => vec![
            ("false".into(), "Not checked".into()),
            ("true".into(), "Checked".into()),
        ],
        _ => {
            let mut seen = HashSet::new();
            rows.iter()
                .flat_map(|row| values_of(row, grouped).iter())
                .filter(|id| seen.insert(id.as_str()))
                .map(|id| {
                    // A related row is a group under its title; one that is not found, under the id it is named by.
                    let title = options.label_of.map(|l| l(&grouped.id, id)).unwrap_or_default();
                    (id.clone(), if title.is_empty() { id.clone() } else { title })
                })
                .collect()
        }
    };

    // A checkbox has no row without a value: unticked is one. Every other grouping has a group for them.
    let checkbox = grouped.kind.editor == Editor::Checkbox;
    let mut members: HashMap<String, Vec<&ContentRow>> = groups.iter().map(|(k, _)| (k.clone(), Vec::new())).collect();
    if !checkbox {
        groups.push((NO_VALUE.into(), format!("No {}", grouped.name)));
        members.insert(NO_VALUE.into(), Vec::new());
    }
    for row in rows {
        let values = values_of(row, grouped);
        if checkbox {
            let key = if values.first().is_some_and(|v| v == "true") {
                "true"
            } else {
                "false"
            };
            members.get_mut(key).expect("the checkbox's groups").push(row);
            continue;
        }
        // A row with several values is under each of them. One naming a group that is not there has no group.
        let mut placed = false;
        let mut distinct = HashSet::new();
        for value in values.iter().filter(|v| distinct.insert(v.as_str())) {
            if value != NO_VALUE
                && let Some(group) = members.get_mut(value)
            {
                group.push(row);
                placed = true;
            }
        }
        if !placed {
            members.get_mut(NO_VALUE).expect("the group without a value").push(row);
        }
    }

    // The order the view gives its groups first, then every other group in its natural order.
    let mut order: Vec<String> = Vec::new();
    for key in view.group_order.iter().filter(|k| members.contains_key(*k)) {
        if !order.contains(key) {
            order.push(key.clone());
        }
    }
    for (key, _) in &groups {
        if !order.contains(key) {
            order.push(key.clone());
        }
    }
    let labels: HashMap<&str, &str> = groups.iter().map(|(k, l)| (k.as_str(), l.as_str())).collect();
    for key in order {
        let under = &members[&key];
        if view.hidden_groups.contains(&key) || (view.hide_empty_groups && under.is_empty()) {
            continue;
        }
        let collapsed = view.collapsed.contains(&key);
        lines.push(TableRow::group(
            key.clone(),
            labels[key.as_str()],
            under.len(),
            collapsed,
        ));
        if collapsed {
            continue;
        }
        match parent {
            None => lines.extend(under.iter().map(|row| line(row, 0, false, false))),
            Some(parent) => nest(under, parent, view, line, lines),
        }
        if options.editable {
            lines.push(TableRow::new_row(key));
        }
    }
}

// ---- nesting ----

/// The rows nested under their parents, to any depth. A row whose parent is not among the rows shown - filtered
/// away, or not in the table - stands at the top; so does one that is its own ancestor, which a document made
/// outside ADP can hold, so that no row is lost and none is shown twice.
fn nest(rows: &[&ContentRow], parent: &ContentColumn, view: &ContentView, line: &Line<'_>, lines: &mut Vec<TableRow>) {
    let shown: HashSet<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    let mut children: HashMap<&str, Vec<&ContentRow>> = HashMap::new();
    let mut top = Vec::new();
    for row in rows {
        let under = values_of(row, parent).first().map(String::as_str).unwrap_or_default();
        if !under.is_empty() && under != row.id && shown.contains(under) {
            children.entry(under).or_default().push(row);
        } else {
            top.push(*row);
        }
    }

    struct Nesting<'n, 'r> {
        children: &'n HashMap<&'r str, Vec<&'r ContentRow>>,
        emitted: HashSet<&'r str>,
        view: &'n ContentView,
    }

    impl<'r> Nesting<'_, 'r> {
        fn emit(&mut self, row: &'r ContentRow, depth: usize, line: &Line<'_>, lines: &mut Vec<TableRow>) {
            if !self.emitted.insert(row.id.as_str()) {
                return;
            }
            let under = self.children.get(row.id.as_str()).filter(|u| !u.is_empty());
            let collapsed = under.is_some() && self.view.collapsed.contains(&row.id);
            lines.push(line(row, depth, under.is_some(), collapsed));
            if let Some(under) = under {
                if collapsed {
                    // Folded away, and not to be taken for rows that were never reached.
                    self.mark(under);
                } else {
                    for child in under {
                        self.emit(child, depth + 1, line, lines);
                    }
                }
            }
        }

        fn mark(&mut self, under: &[&'r ContentRow]) {
            for child in under {
                if self.emitted.insert(child.id.as_str())
                    && let Some(deeper) = self.children.get(child.id.as_str())
                {
                    self.mark(deeper);
                }
            }
        }
    }

    let mut nesting = Nesting {
        children: &children,
        emitted: HashSet::new(),
        view,
    };
    for row in top {
        nesting.emit(row, 0, line, lines);
    }
    // Rows that name each other as parent, round in a circle, are under nobody that was reached.
    for row in rows {
        if !nesting.emitted.contains(row.id.as_str()) {
            nesting.emit(row, 0, line, lines);
        }
    }
}
