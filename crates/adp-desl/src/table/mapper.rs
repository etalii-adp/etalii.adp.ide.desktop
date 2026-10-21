//! A table's content onto the table model, for the view a connection shows (the standalone host's
//! `KnowledgeTableMapper` at 8b6ccfc, the part of it that names no designer type): the columns in the view's order,
//! the view's settings, and the filter in the table model's words. What a column says of its kind beyond that - a
//! designer's calculations, its relations' targets - is the designer module's, added through [`ColumnExtras`].

use serde::Serialize;

use super::content::{ContentColumn, ContentView, FilterGroup, FilterItem, TableContent, ViewColumn};
use super::engine::LabelOf;
use super::groups;
use super::model::{
    Settings, TableColumn, TableCondition, TableFilterGroup, TableFilterItem, TableOption, TableSort, TableStructure,
    TableView, TableViewSettings,
};

/// The setting a view lists its groups under: each with its key, what it is called and whether it is hidden.
pub const GROUPS: &str = "groups";

/// The setting a view names the column under that its columns are frozen up to.
pub const FREEZE: &str = "freeze";

/// The setting that says what the connection searches the rows for. It is the connection's own, and in no document.
pub const SEARCH: &str = "search";

/// The setting that says how many groups the view hides: what there is to show again.
pub const HIDDEN_GROUPS: &str = "hiddenGroups";

/// What a designer module adds to a column: settings of its kind, and the options to offer where they are not the
/// column's own (a relation offers its target's rows).
pub type ColumnExtras<'a> = &'a dyn Fn(&ContentColumn, Option<&ViewColumn>, &mut Settings) -> Option<Vec<TableOption>>;

/// Everything about the table but its rows, for the view `view_id`.
pub fn structure(
    table: &TableContent,
    read_only_reason: &str,
    view_id: &str,
    row_count: usize,
    extras: Option<ColumnExtras<'_>>,
    label_of: Option<LabelOf<'_>>,
    search: &str,
) -> TableStructure {
    let view = table.view_or_default(view_id);
    TableStructure {
        title: table.name.clone(),
        columns: columns(table, view, extras),
        views: table
            .views
            .iter()
            .map(|v| TableView {
                id: v.id.clone(),
                name: v.name.clone(),
            })
            .collect(),
        settings: settings(table, view, label_of, search),
        row_count,
        read_only_reason: read_only_reason.to_owned(),
    }
}

/// The columns in the view's order: the columns the view lists, as it lists them, then those it does not mention,
/// which are shown - a column added outside ADP appears.
pub fn columns(table: &TableContent, view: Option<&ContentView>, extras: Option<ColumnExtras<'_>>) -> Vec<TableColumn> {
    let mut placed: Vec<&str> = Vec::new();
    let mut columns = Vec::with_capacity(table.columns.len());
    for listed in view.map(|v| v.columns.as_slice()).unwrap_or_default() {
        // A view naming a column that is gone still opens, without that setting.
        if let Some(column) = table.column(&listed.column_id)
            && !placed.contains(&column.id.as_str())
        {
            placed.push(&column.id);
            columns.push(column_of(column, Some(listed), extras));
        }
    }
    for column in &table.columns {
        if !placed.contains(&column.id.as_str()) {
            columns.push(column_of(column, None, extras));
        }
    }
    columns
}

fn column_of(column: &ContentColumn, listed: Option<&ViewColumn>, extras: Option<ColumnExtras<'_>>) -> TableColumn {
    let mut settings = Settings::new();
    let mut options: Vec<TableOption> = column
        .options
        .iter()
        .map(|o| TableOption {
            id: o.id.clone(),
            name: o.name.clone(),
            color: o.colour.clone(),
        })
        .collect();
    if let Some(extras) = extras
        && let Some(offered) = extras(column, listed, &mut settings)
    {
        options = offered;
    }
    TableColumn {
        id: column.id.clone(),
        name: column.name.clone(),
        kind: column.value_type.clone(),
        options,
        width: listed.map_or(0, |l| l.width),
        // The column that names a row is always shown, whatever a document says.
        visible: column.is_title || listed.is_none_or(|l| l.visible),
        is_title: column.is_title,
        wraps: listed.is_some_and(|l| l.wrap),
        settings,
    }
}

#[derive(Serialize)]
struct ListedGroup<'a> {
    key: &'a str,
    label: &'a str,
    hidden: bool,
}

/// The view's settings, or those of a table without a view.
pub fn settings(
    table: &TableContent,
    view: Option<&ContentView>,
    label_of: Option<LabelOf<'_>>,
    search: &str,
) -> TableViewSettings {
    let Some(view) = view else {
        let mut settings = Settings::new();
        if !search.is_empty() {
            settings.insert(SEARCH.into(), search.into());
        }
        return TableViewSettings {
            settings,
            ..TableViewSettings::default()
        };
    };
    let mut settings = Settings::new();
    // A freeze naming a column that is gone is a setting the view opens without.
    if table.column(&view.freeze).is_some() {
        settings.insert(FREEZE.into(), view.freeze.clone());
    }
    if !search.is_empty() {
        settings.insert(SEARCH.into(), search.into());
    }
    if !view.hidden_groups.is_empty() {
        settings.insert(HIDDEN_GROUPS.into(), view.hidden_groups.len().to_string());
    }
    // A view that nests its rows by the parent relation has no groups to list.
    if let Some(grouped) = table.column(&view.group_by).filter(|c| !c.is_parent) {
        let listed = groups::of(table, view, grouped, label_of);
        let listed: Vec<ListedGroup<'_>> = listed
            .iter()
            .map(|(key, label)| ListedGroup {
                key,
                label,
                hidden: view.hidden_groups.contains(key),
            })
            .collect();
        settings.insert(
            GROUPS.into(),
            serde_json::to_string(&listed).expect("a list of groups serialises"),
        );
    }
    TableViewSettings {
        view_id: view.id.clone(),
        sorts: view
            .sorts
            .iter()
            .map(|s| TableSort {
                column_id: s.column_id.clone(),
                descending: s.descending,
            })
            .collect(),
        filter: (!view.filter.items.is_empty()).then(|| filter(&view.filter)),
        group_by: view.group_by.clone(),
        hides_empty_groups: view.hide_empty_groups,
        settings,
    }
}

/// A view's filter in the table model's words. The two comparisons every type has are the table's own, under its
/// names for them.
pub fn filter(group: &FilterGroup) -> TableFilterGroup {
    TableFilterGroup {
        any: group.any,
        items: group
            .items
            .iter()
            .map(|item| match item {
                FilterItem::Group(nested) => TableFilterItem::Group(filter(nested)),
                FilterItem::Condition(condition) => TableFilterItem::Condition(TableCondition {
                    column_id: condition.column_id.clone(),
                    comparison: match condition.operator.as_str() {
                        "is-empty" => "isEmpty".into(),
                        "is-not-empty" => "isNotEmpty".into(),
                        other => other.into(),
                    },
                    values: if condition.value.is_empty() {
                        Vec::new()
                    } else {
                        vec![condition.value.clone()]
                    },
                }),
            })
            .collect(),
    }
}
