//! The operations that shape the hierarchy (the standalone host's `StructureOperations` at a7fdf03; FR-026): the adds,
//! the moves and the delete of `hierarchical-requirement.des`, and giving a duplicate a number of its own (DESL 0.2
//! sections 5.3.4, 5.3.5). What they write is what the conformance fixtures state, byte for byte.
//!
//! **An operation is written in two steps of one edit.** The numbering every edit writes, each placeholder's number
//! and the marks, the new entry's number included (spec 029 R17), is made first; the structure is planned against the
//! bytes that leaves. Planned together against the bytes as read, the splices of a move or a delete would overlap
//! those of a placeholder inside the entries it carries. The numbering moves no entry, so an entry is found again by
//! its place in reading order, and the undo still takes back both, since the command keeps the bytes from before the
//! first.

use std::collections::BTreeMap;

use adp_cel::{Map, Value};
use adp_fbl::{FblElement, ModelChange};

use super::body_edit::{self, BodyEdit};
use crate::body::RequirementBody;
use crate::gestures::DRIFTED;
use crate::mapper;

pub const ADD_TOP_ITEM: &str = "addTopItem";
pub const ADD_ITEM: &str = "addItem";
pub const ADD_ITEM_BELOW: &str = "addItemBelow";
pub const ADD_REQUIREMENT: &str = "addRequirement";
pub const ADD_REQUIREMENT_BELOW: &str = "addRequirementBelow";
pub const MOVE_ITEM: &str = "moveItem";
pub const MOVE_REQUIREMENT: &str = "moveRequirement";
pub const DELETE_ENTRY: &str = "deleteEntry";

/// Not an operation of the specification but the user's action DESL 0.2 section 5.3.4 asks for: the duplicate it
/// targets is given the next number.
pub const RENUMBER_DUPLICATE: &str = "renumberDuplicate";

/// What `addRequirementBelow` is unavailable with on a requirement at the top level, as the specification says it.
pub const NO_ITEM: &str = "This requirement stands in no item. Indent it under one first.";

const ITEM: &str = "Item";
const REQUIREMENT: &str = "Requirement";

/// Whether `operation` is one of these.
pub fn handles(operation: &str) -> bool {
    [
        ADD_TOP_ITEM,
        ADD_ITEM,
        ADD_ITEM_BELOW,
        ADD_REQUIREMENT,
        ADD_REQUIREMENT_BELOW,
        MOVE_ITEM,
        MOVE_REQUIREMENT,
        DELETE_ENTRY,
        RENUMBER_DUPLICATE,
    ]
    .contains(&operation)
}

/// The entries an operation names, by their place in reading order.
struct Named {
    target: Option<usize>,
    item: Option<usize>,
    after: Option<usize>,
}

/// The bytes of the body `body`, named `file_name`, after `operation` on the entry the surface names `target_id`, with
/// `params` naming entries by the surface's ids too; or the sentence saying why it cannot be, in which case nothing
/// changed. An entry the file no longer has is refused as a change made against an older file.
pub fn apply(
    body: &[u8],
    file_name: &str,
    operation: &str,
    target_id: &str,
    params: &BTreeMap<String, String>,
) -> Result<Vec<u8>, String> {
    let mut read = RequirementBody::read(body.to_vec(), file_name, None);
    if !read.document.read_only_reason.is_empty() {
        return Err(read.document.read_only_reason.clone());
    }
    let ids = mapper::node_ids(&read.document.entries);
    let find = |id: &str| ids.iter().position(|candidate| candidate == id);
    let named_param = |name: &str| -> Result<Option<usize>, String> {
        match params.get(name).filter(|id| !id.is_empty()) {
            None => Ok(None),
            Some(id) => find(id).map(Some).ok_or_else(|| DRIFTED.to_owned()),
        }
    };
    let target = if operation == ADD_TOP_ITEM {
        None
    } else {
        Some(find(target_id).ok_or_else(|| DRIFTED.to_owned())?)
    };
    let named = Named {
        target,
        item: named_param("item")?,
        after: named_param("after")?,
    };
    let refused = refusal(&read, &ids, operation, &named);
    if !refused.is_empty() {
        return Err(refused);
    }

    let target_is_item = named.target.map(|t| read.document.entries[t].is_item());
    let given = match operation {
        ADD_TOP_ITEM | ADD_ITEM | ADD_ITEM_BELOW => Some(body_edit::next(&read.document, true)),
        ADD_REQUIREMENT | ADD_REQUIREMENT_BELOW => Some(body_edit::next(&read.document, false)),
        RENUMBER_DUPLICATE => Some(body_edit::next(&read.document, target_is_item == Some(true))),
        _ => None,
    };
    let renumbered = (operation == RENUMBER_DUPLICATE).then(|| read.document.entries[named.target.unwrap()].clone());
    let mut numbering = BodyEdit::new(&mut read);
    if let Some(given) = &given {
        numbering.give(given);
    }
    if let (Some(entry), Some(given)) = (renumbered, &given) {
        let element = numbering.element_of(&entry);
        numbering.set(&element, "id", Value::String(given.clone()));
        return numbering.write();
    }
    let numbered = numbering.write()?;

    // The structure, against the numbered bytes, with each entry found again at its place in reading order.
    let mut again = RequirementBody::read(numbered, file_name, None);
    let change = change(&again, operation, given.as_deref(), &named);
    let mut edit = BodyEdit::new(&mut again);
    if let Some(change) = change {
        edit.change(change);
    }
    edit.write()
}

/// Why `operation` cannot be done with these entries, or empty when it can.
fn refusal(read: &RequirementBody, ids: &[String], operation: &str, named: &Named) -> String {
    let entries = &read.document.entries;
    let target = named.target.map(|t| &entries[t]);
    let item = named.item.map(|t| &entries[t]);
    let after = named.after.map(|t| &entries[t]);
    let is_item = |entry: Option<&crate::model::Entry>| entry.is_some_and(|e| e.is_item());
    let is_requirement = |entry: Option<&crate::model::Entry>| entry.is_some_and(|e| !e.is_item());
    if [ADD_ITEM, ADD_ITEM_BELOW, ADD_REQUIREMENT, MOVE_ITEM].contains(&operation) && !is_item(target) {
        return format!("'{operation}' is for an item.");
    }
    if [ADD_REQUIREMENT_BELOW, MOVE_REQUIREMENT].contains(&operation) && !is_requirement(target) {
        return format!("'{operation}' is for a requirement.");
    }
    if operation == ADD_REQUIREMENT_BELOW && target.is_some_and(|t| t.parent_id.is_none()) {
        return NO_ITEM.into();
    }
    if let Some(item) = item.filter(|i| !i.is_item()) {
        return format!("{} is a requirement, which holds nothing.", item.id);
    }
    if operation == MOVE_REQUIREMENT && item.is_none() {
        return "A requirement is moved into an item.".into();
    }
    if let Some(after) = after {
        let same_kind = target.is_some_and(|t| t.is_item() == after.is_item());
        let into = item.map(|i| element(read, i));
        if !same_kind || parent(read, &element(read, after)) != into {
            return DRIFTED.into();
        }
    }
    if operation == MOVE_ITEM
        && let (Some(target), Some(item)) = (target, item)
    {
        let own = element(read, target);
        let mut enclosing = Some(element(read, item));
        while let Some(current) = enclosing {
            if current == own {
                return format!("{} cannot be moved into itself or into anything it holds.", target.id);
            }
            enclosing = parent(read, &current);
        }
    }
    if operation == RENUMBER_DUPLICATE
        && let Some(at) = named.target
    {
        let target = &entries[at];
        if target.placeholder || ids[at] == target.id {
            return format!("{} is the only entry with its number, which it keeps.", target.id);
        }
    }
    String::new()
}

/// The structural change `operation` makes, as the specification's actions say it, against the reading `read`; or
/// none when it changes nothing, as a move to where an entry already is.
fn change(read: &RequirementBody, operation: &str, given: Option<&str>, named: &Named) -> Option<ModelChange> {
    let entries = &read.document.entries;
    let target = named.target.map(|t| &entries[t]);
    let item = named.item.map(|t| &entries[t]);
    let after = named.after.map(|t| &entries[t]);
    match operation {
        ADD_TOP_ITEM => Some(add(ITEM, given?, None, None)),
        ADD_ITEM => Some(add(ITEM, given?, Some(element(read, target?)), None)),
        ADD_ITEM_BELOW | ADD_REQUIREMENT_BELOW => {
            let target = target?;
            let own = element(read, target);
            let parent = parent(read, &own);
            let index = siblings(read, parent.as_deref(), target.is_item())
                .iter()
                .position(|s| *s == own)
                .map_or(0, |at| at as i64 + 1);
            Some(add(
                if target.is_item() { ITEM } else { REQUIREMENT },
                given?,
                parent,
                Some(index),
            ))
        }
        ADD_REQUIREMENT => {
            let target = target?;
            let parent = element(read, target);
            let children = siblings(read, Some(&parent), target.is_item());
            let last_requirement = children.iter().rposition(|c| type_of(read, c) == REQUIREMENT);
            let first_item = children.iter().position(|c| type_of(read, c) == ITEM);
            let index = match (last_requirement, first_item) {
                (Some(at), _) => Some(at as i64 + 1),
                (None, Some(at)) => Some(at as i64),
                (None, None) => None,
            };
            Some(add(REQUIREMENT, given?, Some(parent), index))
        }
        MOVE_ITEM | MOVE_REQUIREMENT => moved(read, target?, item, after),
        DELETE_ENTRY => Some(ModelChange::Remove {
            id: element(read, target?),
        }),
        _ => None,
    }
}

/// A move as `moveItem` and `moveRequirement` say it: after `after` when it is given; else, within its own parent, to
/// the front of its kind, and into another, where a new entry of its kind would go.
fn moved(
    read: &RequirementBody,
    target: &crate::model::Entry,
    item: Option<&crate::model::Entry>,
    after: Option<&crate::model::Entry>,
) -> Option<ModelChange> {
    let own = element(read, target);
    let into = item.map(|i| element(read, i));
    let siblings = siblings(read, into.as_deref(), target.is_item());
    let position = |id: &str| siblings.iter().position(|s| s == id).map_or(-1, |at| at as i64);
    let to = |index: i64| ModelChange::Move {
        id: own.clone(),
        new_parent_id: into.clone(),
        index,
    };
    if let Some(after) = after {
        return Some(to(position(&element(read, after)) + 1));
    }
    let kind = type_of(read, &own);
    let of_its_kind: Vec<&String> = siblings
        .iter()
        .filter(|s| **s != own && type_of(read, s) == kind)
        .collect();
    if parent(read, &own) == into {
        return of_its_kind.first().map(|first| to(position(first)));
    }
    if kind == ITEM || !of_its_kind.is_empty() {
        let index = match of_its_kind.last() {
            Some(last) if kind == REQUIREMENT => position(last) + 1,
            _ => -1,
        };
        return Some(to(index));
    }
    let first_item = siblings.iter().find(|s| type_of(read, s) == ITEM);
    Some(to(first_item.map_or(-1, |first| position(first))))
}

fn add(type_name: &str, id: &str, parent_id: Option<String>, index: Option<i64>) -> ModelChange {
    let title = if type_name == ITEM {
        "New item"
    } else {
        "New requirement"
    };
    let attributes: Map = [("title", Value::String(title.into()))].into_iter().collect();
    ModelChange::Add {
        type_name: type_name.into(),
        id: Some(id.into()),
        attributes,
        parent_id,
        index,
        slot: None,
    }
}

/// The library's elements an entry of an item's or a requirement's kind is placed among under `parent`, in their
/// order: an item's children of either type, and at the top level, where the library orders each type apart, those of
/// the entry's type.
fn siblings(read: &RequirementBody, parent: Option<&str>, item: bool) -> Vec<String> {
    let own_type = if item { ITEM } else { REQUIREMENT };
    read.body
        .model()
        .elements
        .into_iter()
        .filter(|e| {
            (e.type_name == ITEM || e.type_name == REQUIREMENT)
                && e.parent_id.as_deref() == parent
                && (parent.is_some() || e.type_name == own_type)
        })
        .map(|e| e.id)
        .collect()
}

fn element(read: &RequirementBody, entry: &crate::model::Entry) -> String {
    body_edit::element_of(read, entry)
}

fn of(read: &RequirementBody, element: &str) -> FblElement {
    read.body
        .model()
        .elements
        .into_iter()
        .find(|e| e.id == element)
        .expect("the element is the body's")
}

fn parent(read: &RequirementBody, element: &str) -> Option<String> {
    of(read, element).parent_id
}

fn type_of(read: &RequirementBody, element: &str) -> String {
    of(read, element).type_name
}
