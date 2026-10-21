//! A `table` surface's content read out of a body's FBL model by the roles its DESL specification gives (DESL sections
//! 7.2 to 7.5): the columns' type with their name, value type, title, parent and options; the rows' type; the cells'
//! type with the column it names, holding a value under its value type's key or, for a type that holds several, as
//! items; and the views by name, with the document's active view.
//!
//! A host shows a designer type that has no module of its own through this. What DESL does not name is not read: the
//! attributes of a view's settings (`surface.views.settings` names their types, not what they hold) are the designer
//! type's companion document's, so such a table opens with its views unsorted, unfiltered and ungrouped. The Knowledge
//! designer reads its views itself (`modules/designers/knowledge`).

use std::collections::HashMap;

use adp_cel::Value;
use adp_fbl::planning::dotnet_double;
use adp_fbl::{FblElement, FblModel, IdRequest};

use super::content::{ContentCell, ContentColumn, ContentOption, ContentRow, ContentView, TableContent, ValueKind};
use crate::loading::specification::{DeslSpecification, DeslTable};

/// An id for an element whose rule stores none, made from where it is: its parent's id, its slot and its place there
/// (or its rule and line at the top level). It stands in for a DESL `derived` id rule, whose CEL this runtime does not
/// evaluate (see [`crate::loading::loader`]), so that the elements a body holds without ids - a table's cells and
/// their items - are told apart and found under their parents. Pass it as FBL's `derive_id`.
pub fn structural_id(request: &IdRequest<'_>) -> Option<String> {
    Some(match request.parent_id {
        Some(parent) => format!(
            "{parent}/{}/{}",
            request.parent_slot.unwrap_or(request.rule),
            request.position_in_slot
        ),
        None => format!("{}@{}", request.rule, request.line),
    })
}

/// The key a cell that holds several values of a type is read under: the items' key, marked as several.
pub fn many_key(key: &str) -> String {
    format!("{key}[]")
}

/// What a value type of the table says of its values, with the key this reading gives its cells.
pub fn kind_of(table: &DeslTable, value_type: &str) -> ValueKind {
    match table.value_types.get(value_type) {
        Some(declared) => {
            let key = if declared.many {
                many_key(&declared.key)
            } else {
                declared.key.clone()
            };
            ValueKind::of(declared, key)
        }
        None => ValueKind::unknown(value_type),
    }
}

/// A value in its written form.
fn text(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(text)) => text.clone(),
        Some(Value::Bool(flag)) => if *flag { "true" } else { "false" }.into(),
        Some(Value::Int(whole)) => whole.to_string(),
        Some(Value::Double(real)) => dotnet_double(*real),
        Some(Value::List(items)) => text(items.first()),
        Some(other) => other.to_string(),
    }
}

fn is_true(value: Option<&Value>) -> bool {
    matches!(value, Some(Value::Bool(true))) || matches!(value, Some(Value::String(s)) if s == "true")
}

/// The table `specification`'s surface lays out, read from `model`; none for a surface of another kind.
pub fn read(specification: &DeslSpecification, model: &FblModel) -> Option<TableContent> {
    let table = specification.table()?;
    let mut children: HashMap<&str, Vec<&FblElement>> = HashMap::new();
    for element in &model.elements {
        if let Some(parent) = &element.parent_id {
            children.entry(parent.as_str()).or_default().push(element);
        }
    }
    let under = |parent: &FblElement, type_name: &str| -> Vec<&FblElement> {
        children
            .get(parent.id.as_str())
            .map(|c| c.iter().copied().filter(|e| e.type_name == type_name).collect())
            .unwrap_or_default()
    };
    let attribute = |element: &FblElement, name: &Option<String>| -> String {
        name.as_deref()
            .map(|n| text(element.attributes.get(n)))
            .unwrap_or_default()
    };

    let columns: Vec<ContentColumn> = model
        .elements
        .iter()
        .filter(|e| e.type_name == table.columns_type)
        .map(|c| {
            let value_type = text(c.attributes.get(&table.column_value_type));
            let options = table
                .options
                .as_ref()
                .map(|o| {
                    under(c, &o.type_name)
                        .into_iter()
                        .map(|option| {
                            let colour = attribute(option, &o.colour);
                            ContentOption::new(
                                option.id.clone(),
                                text(option.attributes.get(&o.name)),
                                if colour.is_empty() { "default".into() } else { colour },
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            ContentColumn {
                is_title: table
                    .column_title
                    .as_deref()
                    .is_some_and(|t| is_true(c.attributes.get(t))),
                is_parent: table
                    .column_parent
                    .as_deref()
                    .is_some_and(|p| is_true(c.attributes.get(p))),
                options,
                ..ContentColumn::new(
                    c.id.clone(),
                    text(c.attributes.get(&table.column_name)),
                    value_type.clone(),
                    kind_of(table, &value_type),
                )
            }
        })
        .collect();
    let value_type_of: HashMap<&str, &str> = columns.iter().map(|c| (c.id.as_str(), c.value_type.as_str())).collect();

    let rows = model
        .elements
        .iter()
        .filter(|e| e.type_name == table.rows_type)
        .map(|row| {
            let cells = under(row, &table.cells_type)
                .into_iter()
                .map(|cell| {
                    let column = text(cell.attributes.get(&table.cell_column));
                    let declared = value_type_of
                        .get(column.as_str())
                        .and_then(|t| table.value_types.get(*t));
                    let items = table.cell_items.as_ref().map(|i| under(cell, i)).unwrap_or_default();
                    if !items.is_empty() {
                        // Several values are several items, each under the key of the type that holds several.
                        let key = declared
                            .filter(|d| d.many)
                            .map(|d| d.key.clone())
                            .or_else(|| table.value_types.values().find(|d| d.many).map(|d| d.key.clone()));
                        let key = key.unwrap_or_default();
                        let values: Vec<String> = items
                            .iter()
                            .map(|i| text(i.attributes.get(&key)))
                            .filter(|v| !v.is_empty())
                            .collect();
                        return ContentCell::new(column, values, many_key(&key));
                    }
                    // One value, under the key of its column's type, or the first key of any type the cell holds.
                    let keys = declared
                        .filter(|d| !d.many)
                        .map(|d| d.key.as_str())
                        .into_iter()
                        .chain(table.value_types.values().filter(|d| !d.many).map(|d| d.key.as_str()));
                    for key in keys {
                        if let Some(value) = cell.attributes.get(key).filter(|v| !v.is_null()) {
                            return ContentCell::new(column, [text(Some(value))], key);
                        }
                    }
                    ContentCell::new(column, Vec::<String>::new(), "")
                })
                .collect();
            ContentRow::new(row.id.clone(), cells)
        })
        .collect();

    let views = model
        .elements
        .iter()
        .filter(|e| e.type_name == table.views_type)
        .map(|v| ContentView::new(v.id.clone(), text(v.attributes.get(&table.view_name))))
        .collect();

    // The document's own attributes are those of the element its type map makes the document (DESL section 5.2).
    let document = specification
        .persistence
        .type_map
        .iter()
        .find(|(_, target)| *target == "document")
        .and_then(|(type_name, _)| model.elements.iter().find(|e| e.type_name == *type_name));
    let active_view_id = match (&table.view_active, document) {
        (Some(active), Some(document)) => text(document.attributes.get(active)),
        _ => String::new(),
    };
    let name = document.map(|d| text(d.attributes.get("name"))).unwrap_or_default();
    Some(TableContent {
        name,
        active_view_id,
        columns,
        views,
        rows,
    })
}
