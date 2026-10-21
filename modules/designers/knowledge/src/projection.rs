//! What a knowledge file's model will be once changes still on their way to the file have been written: the same
//! changes, made to the elements in memory (the standalone host's `KnowledgeProjection` at 8b6ccfc).
//!
//! - **This is what lets an edit be shown at once and written behind.** Planning a change through the FBL runtime
//!   reads the whole body again, which on a file of ten thousand rows takes longer than an author will wait for a
//!   cell to change. Here a change is a few list operations.
//! - **It is never the truth.** The file is; a projection lives only until the write it anticipates has landed,
//!   when the session reads the body's own model again. It derives an id exactly as the reading does, so a later
//!   change in the queue can name what an earlier one made.

use std::collections::{HashMap, HashSet};

use adp_cel::{Map, Value};
use adp_fbl::model::IdRequest;
use adp_fbl::text::Span;
use adp_fbl::{FblElement, FblModel, ModelChange};

use crate::definition;
use crate::values;

/// `model` with `changes` made, in order; `beside_its_rule` is whether the file's format adds an entry after the
/// last one its rule reads ([`definition::adds_beside_its_rule`]).
pub fn apply(model: &FblModel, changes: &[ModelChange], beside_its_rule: bool) -> FblModel {
    if changes.is_empty() {
        return model.clone();
    }
    let mut elements = model.elements.clone();
    for change in changes {
        match change {
            ModelChange::Add {
                type_name,
                id,
                attributes,
                parent_id,
                slot,
                ..
            } => add(
                &mut elements,
                type_name,
                id.as_deref(),
                attributes,
                parent_id.as_deref(),
                slot.as_deref(),
                beside_its_rule,
            ),
            ModelChange::Set { id, attributes } => set(&mut elements, id, attributes),
            ModelChange::Remove { id } => remove(&mut elements, id),
            ModelChange::Move {
                id,
                new_parent_id,
                index,
            } => move_to(&mut elements, id, new_parent_id.as_deref(), *index),
            _ => {}
        }
        readdress(&mut elements);
    }
    FblModel {
        elements,
        ..model.clone()
    }
}

/// The rule an added element is read by and the slot of its parent it is in, as the bindings have them.
fn placement(
    type_name: &str,
    slot: Option<&str>,
    attributes: &Map,
    parent: Option<&FblElement>,
) -> (String, Option<String>) {
    let (rule, slot) = match type_name {
        "Property" => ("property", None),
        "View" => ("view", None),
        "Row" => ("row", None),
        "Option" => ("option", Some("options")),
        "Column" => ("column", Some("columns")),
        "Sort" => ("sort", Some("sorts")),
        "Condition" if parent.is_some_and(|p| p.type_name == "View") => ("condition1", Some("filter")),
        "Condition" if parent.is_some_and(|p| p.rule == "filterGroup1") => ("condition2", Some("conditions")),
        "Condition" => ("condition3", Some("conditions")),
        "FilterGroup" if parent.is_some_and(|p| p.type_name == "View") => ("filterGroup1", Some("filter")),
        "FilterGroup" => ("filterGroup2", Some("conditions")),
        "GroupSetting" if slot == Some("hiddenGroups") => ("hiddenGroup", Some("hiddenGroups")),
        "GroupSetting" if slot == Some("collapsed") => ("collapsedGroup", Some("collapsed")),
        "GroupSetting" => ("groupOrder", Some("groupOrder")),
        "Cell" => ("cell", Some("cells")),
        "CellItem" if attributes.contains_key("option") => ("cellOption", Some("items")),
        "CellItem" => ("cellRow", Some("items")),
        other => return (other.to_owned(), slot.map(str::to_owned)),
    };
    (rule.to_owned(), slot.map(str::to_owned))
}

/// The elements ordered among each other with one of this type, parent and slot, by their places in `elements`.
fn siblings(elements: &[FblElement], type_name: &str, parent_id: Option<&str>, slot: Option<&str>) -> Vec<usize> {
    elements
        .iter()
        .enumerate()
        .filter(|(_, e)| match parent_id {
            None => e.parent_id.is_none() && e.type_name == type_name,
            Some(parent) => e.parent_id.as_deref() == Some(parent) && e.parent_slot.as_deref() == slot,
        })
        .map(|(i, _)| i)
        .collect()
}

/// What empties an attribute, as a set of it does in the body: nothing, an empty text, an empty list.
fn is_empty(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(text) => text.is_empty(),
        Value::List(items) => items.is_empty(),
        Value::Map(map) => map.is_empty(),
        _ => false,
    }
}

fn element(
    id: String,
    id_is_stored: bool,
    type_name: &str,
    rule: String,
    attributes: Map,
    parent_id: Option<String>,
    slot: Option<String>,
) -> FblElement {
    FblElement {
        id,
        id_is_stored,
        type_name: type_name.to_owned(),
        rule,
        is_relation: false,
        attributes,
        parent_id,
        parent_slot: slot,
        source: None,
        target: None,
        own_span: Span::default(),
        line: 0,
        file: None,
    }
}

fn add(
    elements: &mut Vec<FblElement>,
    type_name: &str,
    id: Option<&str>,
    attributes: &Map,
    parent_id: Option<&str>,
    slot: Option<&str>,
    beside_its_rule: bool,
) {
    let parent = parent_id.and_then(|p| elements.iter().find(|e| e.id == p));
    let (rule, slot) = placement(type_name, slot, attributes, parent);
    let siblings = siblings(elements, type_name, parent_id, slot.as_deref());
    let attributes: Map = attributes
        .iter()
        .filter(|(_, v)| !is_empty(v))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let stored = id.is_some();
    let id = id.map(str::to_owned).unwrap_or_else(|| {
        definition::derive_id(&IdRequest {
            rule: &rule,
            type_name,
            attributes: &attributes,
            source: None,
            target: None,
            line: 0,
            parent_id,
            parent_slot: slot.as_deref(),
            position_in_slot: siblings.len(),
        })
        .unwrap_or_else(definition::new_id)
    });
    let added = element(
        id,
        stored,
        type_name,
        rule.clone(),
        attributes,
        parent_id.map(str::to_owned),
        slot,
    );
    // Last of its siblings - or, in a format that adds beside the entry's own rule, after the last sibling that rule
    // reads: a condition then goes before a group that follows the conditions. Only the order among siblings is
    // read, so last of all the elements is last of them.
    let last_of_rule = siblings.iter().rev().find(|i| elements[**i].rule == rule).copied();
    match (beside_its_rule, last_of_rule, siblings.last()) {
        (true, Some(last), Some(&final_sibling)) if last != final_sibling => elements.insert(last + 1, added),
        _ => elements.push(added),
    }
}

fn set(elements: &mut [FblElement], id: &str, changed: &Map) {
    let Some(element) = elements.iter_mut().find(|e| e.id == id) else {
        return;
    };
    let mut attributes = element.attributes.clone();
    for (name, value) in changed.iter() {
        if is_empty(value) {
            attributes.remove(name);
        } else {
            attributes.insert(name.clone(), value.clone());
        }
    }
    element.attributes = attributes;
}

/// Removes an element with everything it contains, and with what the bindings cascade to: the cells, columns, sorts
/// and conditions of a removed property, and the cells, cell items and conditions that name a removed option.
fn remove(elements: &mut Vec<FblElement>, id: &str) {
    let Some(removed) = elements.iter().find(|e| e.id == id) else {
        return;
    };
    let mut gone: HashSet<String> = HashSet::from([id.to_owned()]);
    let naming = match removed.type_name.as_str() {
        "Property" => Some("property"),
        "Option" => Some("option"),
        _ => None,
    };
    if let Some(naming) = naming {
        for element in elements.iter() {
            if element
                .attributes
                .get(naming)
                .is_some_and(|v| values::text(Some(v)) == id)
            {
                gone.insert(element.id.clone());
            }
        }
    }
    // Document order puts a parent before what it contains, so one pass reaches every depth.
    for element in elements.iter() {
        if element.parent_id.as_ref().is_some_and(|p| gone.contains(p)) {
            gone.insert(element.id.clone());
        }
    }
    elements.retain(|e| !gone.contains(&e.id));
}

fn move_to(elements: &mut Vec<FblElement>, id: &str, new_parent_id: Option<&str>, index: i64) {
    let Some(at) = elements.iter().position(|e| e.id == id) else {
        return;
    };
    // The index counts the siblings as they are before the move, the element itself included.
    let (type_name, slot) = (elements[at].type_name.clone(), elements[at].parent_slot.clone());
    let siblings = siblings(elements, &type_name, new_parent_id, slot.as_deref());
    let before = usize::try_from(index).ok().and_then(|i| siblings.get(i).copied());
    if before == Some(at) {
        return;
    }
    let before_id = before.map(|b| elements[b].id.clone());
    let mut moved = elements.remove(at);
    moved.parent_id = new_parent_id.map(str::to_owned);
    match before_id.and_then(|b| elements.iter().position(|e| e.id == b)) {
        None => elements.push(moved),
        Some(place) => elements.insert(place, moved),
    }
}

/// Gives a filter's items the ids the reading would give them. A condition and a group are addressed by their place,
/// so adding, moving or removing one changes the address of those after it - and a later change in the queue names
/// them by the address they will have.
fn readdress(elements: &mut [FblElement]) {
    let items: Vec<usize> = elements
        .iter()
        .enumerate()
        .filter(|(_, e)| matches!(e.type_name.as_str(), "Condition" | "FilterGroup") && e.parent_id.is_some())
        .map(|(i, _)| i)
        .collect();
    if items.is_empty() {
        return;
    }
    let mut under: HashMap<String, Vec<usize>> = HashMap::new();
    for &i in &items {
        under
            .entry(elements[i].parent_id.clone().expect("an item of a filter has a parent"))
            .or_default()
            .push(i);
    }
    let views: Vec<String> = elements
        .iter()
        .filter(|e| e.type_name == "View")
        .map(|e| e.id.clone())
        .collect();
    let mut addressed: Vec<(usize, String, String)> = Vec::new();

    fn walk(
        elements: &[FblElement],
        under: &HashMap<String, Vec<usize>>,
        parent_was: &str,
        parent_is: &str,
        prefix: &str,
        addressed: &mut Vec<(usize, String, String)>,
    ) {
        let Some(children) = under.get(parent_was) else {
            return;
        };
        for (index, &child) in children.iter().enumerate() {
            let id = format!("{prefix}/{index}");
            if elements[child].id != id || elements[child].parent_id.as_deref() != Some(parent_is) {
                addressed.push((child, id.clone(), parent_is.to_owned()));
            }
            walk(elements, under, &elements[child].id, &id, &id, addressed);
        }
    }

    for view in views {
        walk(
            elements,
            &under,
            &view,
            &view,
            &format!("{view}/filter"),
            &mut addressed,
        );
    }
    for (index, id, parent) in addressed {
        elements[index].id = id;
        elements[index].parent_id = Some(parent);
    }
}
