//! A knowledge file as the designer understands it: its properties, its views with every setting they have, and its
//! rows - read from the body's FBL model and from nowhere else (the standalone host's `KnowledgeModel.cs` at
//! 8b6ccfc, `KnowledgeModelReader`). Nothing here is kept outside the file (knowledge-designer Requirement 2.3).
//!
//! A knowledge table is the DESL runtime's [`TableContent`]: a property is a column, and its value type's kind is
//! what `knowledge.des` says of it.

use std::collections::HashMap;

use adp_desl::table::content::{
    Condition, ContentCell, ContentColumn, ContentOption, ContentRow, ContentView, FilterGroup, FilterItem,
    TableContent, ViewColumn, ViewSort,
};
use adp_fbl::{FblElement, FblModel};

use crate::definition;
use crate::values;
use crate::vocabulary;

/// A knowledge file's table.
pub type KnowledgeTable = TableContent;

/// One property: a column of the table, with its type and what its type has.
pub type KnowledgeProperty = ContentColumn;

/// A property as the file holds it, with the kind its value type has.
#[allow(clippy::too_many_arguments)]
pub fn property(
    id: &str,
    name: &str,
    value_type: &str,
    is_title: bool,
    options: Vec<ContentOption>,
    target_file: &str,
    limit: &str,
    counterpart: &str,
    is_computed: bool,
    is_parent: bool,
) -> KnowledgeProperty {
    KnowledgeProperty {
        is_title,
        options,
        target_file: target_file.to_owned(),
        limit: limit.to_owned(),
        counterpart: counterpart.to_owned(),
        is_computed,
        is_parent,
        ..ContentColumn::new(id, name, value_type, definition::kind_of(value_type))
    }
}

fn text(element: &FblElement, attribute: &str) -> String {
    values::text(element.attributes.get(attribute))
}

/// The one value a cell or a condition holds, whichever key its kind writes it under, and that key.
fn value_of(element: &FblElement) -> (String, String) {
    for candidate in vocabulary::VALUE_KEYS {
        if let Some(value) = element.attributes.get(candidate).filter(|v| !v.is_null()) {
            return (values::text(Some(value)), candidate.to_owned());
        }
    }
    (String::new(), String::new())
}

/// The elements under each parent and slot, in document order.
type Children<'m> = HashMap<(String, String), Vec<&'m FblElement>>;

fn children_in<'m>(children: &Children<'m>, parent: &FblElement, slot: &str) -> Vec<&'m FblElement> {
    children
        .get(&(parent.id.clone(), slot.to_owned()))
        .cloned()
        .unwrap_or_default()
}

fn filter_item(element: &FblElement, children: &Children<'_>) -> FilterItem {
    if element.type_name == "FilterGroup" {
        FilterItem::Group(FilterGroup {
            any: text(element, "match") == "any",
            items: children_in(children, element, "conditions")
                .into_iter()
                .map(|c| filter_item(c, children))
                .collect(),
        })
    } else {
        FilterItem::Condition(Condition::new(
            text(element, "property"),
            text(element, "operator"),
            value_of(element).0,
        ))
    }
}

/// Reads a table out of a body's FBL model.
pub fn read(model: &FblModel) -> KnowledgeTable {
    // One pass to group every element under its parent and slot, so reading stays linear in the size of the file:
    // ten thousand rows are ten thousand lookups, not ten thousand scans.
    let mut children: Children<'_> = HashMap::new();
    for element in &model.elements {
        if let Some(parent) = &element.parent_id {
            children
                .entry((parent.clone(), element.parent_slot.clone().unwrap_or_default()))
                .or_default()
                .push(element);
        }
    }
    let children_of = |parent: &FblElement, slot: &str| children_in(&children, parent, slot);

    let table = model.elements.iter().find(|e| e.type_name == "Table");
    let properties = model
        .elements
        .iter()
        .filter(|e| e.type_name == "Property")
        .map(|p| {
            // An option that names no colour has the default one, and a relation that names no limit has none: the
            // bindings' defaults, restated for a model that was not read through them.
            let options = children_of(p, "options")
                .into_iter()
                .map(|o| {
                    let colour = text(o, "colour");
                    ContentOption::new(
                        o.id.clone(),
                        text(o, "name"),
                        if colour.is_empty() { "default".into() } else { colour },
                    )
                })
                .collect();
            let limit = text(p, "limit");
            property(
                &p.id,
                &text(p, "name"),
                &text(p, "valueType"),
                values::is_true(p.attributes.get("title")),
                options,
                &text(p, "targetFile"),
                if limit.is_empty() { "none" } else { &limit },
                &text(p, "counterpart"),
                values::is_true(p.attributes.get("computed")),
                values::is_true(p.attributes.get("isParent")),
            )
        })
        .collect();

    let views = model
        .elements
        .iter()
        .filter(|e| e.type_name == "View")
        .map(|v| ContentView {
            id: v.id.clone(),
            name: text(v, "name"),
            columns: children_of(v, "columns")
                .into_iter()
                .map(|c| ViewColumn {
                    column_id: text(c, "property"),
                    // A column that does not say is shown: the binding's default, restated for a family that gives none.
                    visible: c
                        .attributes
                        .get("visible")
                        .is_none_or(|v| v.is_null() || values::is_true(Some(v))),
                    width: values::whole(c.attributes.get("width")).unwrap_or(0),
                    wrap: values::is_true(c.attributes.get("wrap")),
                    calculate: text(c, "calculate"),
                })
                .collect(),
            sorts: children_of(v, "sorts")
                .into_iter()
                .map(|s| ViewSort {
                    column_id: text(s, "property"),
                    descending: text(s, "direction") == "descending",
                })
                .collect(),
            filter: FilterGroup {
                any: text(v, "filterMatch") == "any",
                items: children_of(v, "filter")
                    .into_iter()
                    .map(|f| filter_item(f, &children))
                    .collect(),
            },
            group_by: text(v, "groupBy"),
            hide_empty_groups: values::is_true(v.attributes.get("hideEmptyGroups")),
            group_order: children_of(v, "groupOrder")
                .into_iter()
                .map(|s| text(s, "key"))
                .collect(),
            hidden_groups: children_of(v, "hiddenGroups")
                .into_iter()
                .map(|s| text(s, "key"))
                .collect(),
            collapsed: children_of(v, "collapsed")
                .into_iter()
                .map(|s| text(s, "key"))
                .collect(),
            freeze: text(v, "freeze"),
        })
        .collect();

    let rows = model
        .elements
        .iter()
        .filter(|e| e.type_name == "Row")
        .map(|row| {
            let cells = children_of(row, "cells")
                .into_iter()
                .map(|cell| {
                    let items = children_of(cell, "items");
                    let property = text(cell, "property");
                    if let Some(first) = items.first() {
                        // Several values are several items: an option each, or a related row each.
                        let several: Vec<String> = items
                            .iter()
                            .map(|i| {
                                if i.attributes.contains_key("option") {
                                    text(i, "option")
                                } else {
                                    text(i, "row")
                                }
                            })
                            .filter(|v| !v.is_empty())
                            .collect();
                        let key = if first.attributes.contains_key("option") {
                            "options"
                        } else {
                            "rows"
                        };
                        return ContentCell::new(property, several, key);
                    }
                    let (one, key) = value_of(cell);
                    if one.is_empty() {
                        ContentCell::new(property, Vec::<String>::new(), "")
                    } else {
                        ContentCell::new(property, [one], key)
                    }
                })
                .collect();
            ContentRow::new(row.id.clone(), cells)
        })
        .collect();

    KnowledgeTable {
        name: table.map(|t| text(t, "name")).unwrap_or_default(),
        active_view_id: table.map(|t| text(t, "activeView")).unwrap_or_default(),
        columns: properties,
        views,
        rows,
    }
}
