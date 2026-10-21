//! A row of a knowledge table as something that can be selected, and its values as the property grid's rows (the
//! standalone host's `KnowledgeRowContext.cs` at 8b6ccfc: `KnowledgeRowSourceResolver` and
//! `KnowledgeRowPropertyProvider`; knowledge-designer Requirement 4.6). A row is found under its table's file by its
//! id and named by its title; each property is one row of the grid, edited there as its type is, and an edit made
//! there is the same step it is in the table: the same `setCell` gesture, planned by [`crate::edits`].
//!
//! The standalone host resolves a selection and runs the edit on the project's history; here the shell holds the
//! table's session, so [`gesture`] gives the gesture to send through it - shown at once, written behind and undone as
//! any other edit of the table - and [`set`] carries an edit out on the file directly, for a table no tab shows.

use std::path::Path;

use adp_context::{PropertyEditor, PropertyRow};
use adp_desl::table::model::TableGesture;

use crate::command::{KnowledgeEditCommand, KnowledgeRestore};
use crate::definition;
use crate::edits;
use crate::model::{KnowledgeProperty, KnowledgeTable};
use crate::relations;
use crate::store;

/// Why a relation is not edited in the grid.
pub const RELATION_STAYS: &str = "A relation is filled in in the table, where its rows can be chosen.";

/// Why the computed side of a relation is not edited anywhere.
pub const COMPUTED_STAYS: &str = "This side of the relation is filled in from the other table.";

/// Why a file is not a table whose rows can be selected.
pub const NOT_A_TABLE: &str = "The selected file is not a knowledge table.";

/// Why a row cannot be selected.
pub const ROW_GONE: &str = "That row is no longer in this table.";

/// What the payload a selected row's values are carried under is called: a change of them is a change of the
/// selection's detail, which is what the grid watches.
pub const PAYLOAD_TYPE: &str = "etalii/knowledge+row";

/// A selected row as the selection chain shows it: its title, and its values as one text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowDescription {
    pub title: String,
    /// `property=value,value;` for every cell, in the row's order.
    pub values: String,
}

/// The row of `row_id` in `table`: its title, or its id when it has none, and its values; none for a row that is not
/// there.
pub fn describe_in(table: &KnowledgeTable, row_id: &str) -> Option<RowDescription> {
    let row = table.row(row_id)?;
    let title = relations::title_of(table, row);
    let mut values = String::new();
    for cell in &row.cells {
        values += &format!("{}={};", cell.column_id, cell.values.join(","));
    }
    Some(RowDescription {
        title: if title.is_empty() { row_id.to_owned() } else { title },
        values,
    })
}

/// A row selected under a file - a knowledge file's data file or its registration - as the file holds it now: the
/// sentence a selection is rejected with when the file is no table or the row is not in it.
pub fn describe(entry: &Path, row_id: &str) -> Result<(std::path::PathBuf, RowDescription), String> {
    let body_path = crate::project::body_of(entry).ok_or_else(|| NOT_A_TABLE.to_owned())?;
    let read = store::read(&body_path);
    let Some(body) = read.body.filter(|b| b.unreadable().is_empty()) else {
        return Err(ROW_GONE.into());
    };
    let described = describe_in(body.table(), row_id).ok_or_else(|| ROW_GONE.to_owned())?;
    Ok((body_path, described))
}

/// The grid's rows for one row of a table: one per property, in the table's order, each with the editor its type is
/// edited with. `read_only_reason` is the table's own: a file that cannot be changed shows its values and why.
pub fn properties(table: &KnowledgeTable, read_only_reason: &str, row_id: &str) -> Vec<PropertyRow> {
    let Some(row) = table.row(row_id) else {
        return Vec::new();
    };
    table
        .columns
        .iter()
        .map(|property| {
            let values: Vec<String> = row.cell(&property.id).map(|c| c.values.clone()).unwrap_or_default();
            property_row(property, &values, read_only_reason)
        })
        .collect()
}

/// The grid's rows for one row of the file at `body_path`, as it is on disk; none when the file or the row is not
/// there.
pub fn properties_of_file(body_path: &Path, row_id: &str) -> Vec<PropertyRow> {
    match store::read(body_path).body.filter(|b| b.unreadable().is_empty()) {
        Some(body) => properties(body.table(), &body.read_only_reason(), row_id),
        None => Vec::new(),
    }
}

fn property_row(property: &KnowledgeProperty, values: &[String], read_only_reason: &str) -> PropertyRow {
    let name_of = |option_id: &str| {
        property
            .options
            .iter()
            .find(|o| o.id == option_id)
            .map(|o| o.name.clone())
            .unwrap_or_else(|| option_id.to_owned())
    };
    let first = values.first().cloned().unwrap_or_default();
    let (editor, value, reason, candidates) = match property.value_type.as_str() {
        "checkbox" => (
            PropertyEditor::Toggle,
            if first == "true" { "true" } else { "false" }.to_owned(),
            read_only_reason.to_owned(),
            Vec::new(),
        ),
        "selection" => (
            PropertyEditor::Choice,
            if values.is_empty() {
                String::new()
            } else {
                name_of(&first)
            },
            read_only_reason.to_owned(),
            std::iter::once(String::new())
                .chain(property.options.iter().map(|o| o.name.clone()))
                .collect(),
        ),
        "multipleSelection" => (
            PropertyEditor::Tags,
            values.iter().map(|v| name_of(v)).collect::<Vec<_>>().join(", "),
            read_only_reason.to_owned(),
            property.options.iter().map(|o| o.name.clone()).collect(),
        ),
        "relation" => (
            PropertyEditor::Line,
            values.join(", "),
            if property.is_computed {
                COMPUTED_STAYS
            } else {
                RELATION_STAYS
            }
            .to_owned(),
            Vec::new(),
        ),
        _ => (PropertyEditor::Line, first, read_only_reason.to_owned(), Vec::new()),
    };
    let mut row = PropertyRow::new(property.id.clone(), property.name.clone(), editor, value);
    row.read_only_reason = reason;
    row.candidates = candidates;
    row
}

/// The gesture a value set in the grid comes to: a `setCell` of the row, with an option named by its name in the
/// grid given by its id, as the file holds it. A value that is no value of its property, a relation and a table that
/// cannot be changed are refused with their sentence.
pub fn gesture(
    table: &KnowledgeTable,
    read_only_reason: &str,
    row_id: &str,
    property_id: &str,
    value: &str,
) -> Result<TableGesture, String> {
    if !read_only_reason.is_empty() {
        return Err(read_only_reason.to_owned());
    }
    let Some(property) = table.column(property_id) else {
        return Err("That property is no longer in this table.".into());
    };
    let mut values = Vec::new();
    match property.value_type.as_str() {
        "selection" | "multipleSelection" => {
            let names: Vec<&str> = if property.value_type == "selection" {
                vec![value]
            } else {
                value.split(',').collect()
            };
            for name in names.into_iter().map(str::trim).filter(|n| !n.is_empty()) {
                let Some(option) = property
                    .options
                    .iter()
                    .find(|o| o.name.to_lowercase() == name.to_lowercase())
                else {
                    return Err(format!("'{name}' is not one of the options of {}.", property.name));
                };
                values.push(option.id.clone());
            }
        }
        "relation" => {
            return Err(if property.is_computed {
                COMPUTED_STAYS
            } else {
                RELATION_STAYS
            }
            .into());
        }
        _ => values.push(value.to_owned()),
    }
    Ok(TableGesture::new("setCell")
        .row(row_id)
        .column(property_id)
        .values(values))
}

/// Sets one value of one row of the file at `body_path` as the grid does, as one step: what puts it back, when it
/// changed a byte, or why it was not done (the standalone host's `KnowledgeRowPropertyProvider.SetAsync`).
pub fn set(body_path: &Path, row_id: &str, property_id: &str, value: &str) -> Result<Option<KnowledgeRestore>, String> {
    let Some(body) = store::read(body_path).body.filter(|b| b.unreadable().is_empty()) else {
        return Err("That table is no longer there.".into());
    };
    let gesture = gesture(body.table(), &body.read_only_reason(), row_id, property_id, value)?;
    let extension = body_path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let mut new_id = definition::new_id;
    let edit = edits::plan(
        body.table(),
        "",
        &gesture,
        &mut new_id,
        definition::adds_beside_its_rule(&extension),
        None,
    );
    if edit.is_refused() {
        return Err(edit.refusal);
    }
    if edit.changes.is_empty() {
        return Ok(None);
    }
    KnowledgeEditCommand {
        body_path: body_path.to_path_buf(),
        changes: edit.changes,
        others: edit.others,
    }
    .execute()
}
