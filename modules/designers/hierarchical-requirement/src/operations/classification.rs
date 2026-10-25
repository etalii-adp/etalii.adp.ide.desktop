//! The operations on categories and states of `hierarchical-requirement.des` (the standalone host's
//! `CategoryStateOperations` at a7fdf03; FR-026, FR-030, FR-031): picking one on a requirement or clearing it, and
//! adding, renaming, recolouring, moving and deleting a member of either list. A requirement's category and state are
//! written in the Markdown file, the lists' colours and order in the registration's `categories` and `states` blocks;
//! an operation that changes both writes both in one step (spec 029 R17), byte for byte as the conformance fixture
//! `hierarchical-requirement-category-state` states.

use std::collections::BTreeMap;

use adp_cel::Value;
use adp_fbl::Setting;

use super::body_edit::BodyEdit;
use super::lists::{ListEdit, ListFiles, normalised, same};
use super::{registration, tags};
use crate::body::{RequirementBody, lower_ascii};
use crate::gestures::DRIFTED;
use crate::mapper;
use crate::model::{Document, Entry};
use crate::vocabulary::COLOURS;

/// One of the two lists: the operations' names end in its type, and its members' ids begin with its prefix.
struct List {
    /// The metamodel's type, which ends each operation's name: `setCategory`, `moveState`.
    type_name: &'static str,
    /// The requirement's attribute, which is also the param a pick names.
    attribute: &'static str,
    /// The registration's block that keeps the list.
    block: &'static str,
    /// What a member's id begins with, before its name in lower case.
    id_prefix: &'static str,
}

const LISTS: [List; 2] = [
    List {
        type_name: "Category",
        attribute: "category",
        block: "categories",
        id_prefix: "category/",
    },
    List {
        type_name: "State",
        attribute: "state",
        block: "states",
        id_prefix: "state/",
    },
];

const VERBS: [&str; 7] = ["set", "clear", "add", "rename", "recolour", "move", "delete"];

/// The list `operation` is about, with its verb.
fn list_of(operation: &str) -> Option<(&'static List, &str)> {
    LISTS.iter().find_map(|list| {
        let verb = operation.strip_suffix(list.type_name)?;
        VERBS.contains(&verb).then_some((list, verb))
    })
}

/// Whether `operation` is one of these.
pub fn handles(operation: &str) -> bool {
    list_of(operation).is_some()
}

/// Whether `operation` targets the document rather than a requirement or a member.
pub fn targets_the_document(operation: &str) -> bool {
    list_of(operation).is_some_and(|(_, verb)| verb == "add")
}

/// Whether `operation` targets a member of a list rather than a requirement.
pub fn targets_a_member(operation: &str) -> bool {
    list_of(operation).is_some_and(|(_, verb)| ["rename", "recolour", "move", "delete"].contains(&verb))
}

/// Whether `params` can be written by `operation`: the sentence saying why not, or empty. A new name is trimmed and a
/// space in it written `-`, as the operations do; what is left is one word.
pub fn refusal(operation: &str, params: &BTreeMap<String, String>) -> String {
    let Some((list, verb)) = list_of(operation) else {
        return String::new();
    };
    let noun = list.attribute;
    if verb == "add" || verb == "rename" {
        let name = params.get("name").map_or("", String::as_str);
        let written = normalised(name);
        if written.is_empty() {
            return format!("A {noun} needs a name.");
        }
        if written.chars().any(char::is_whitespace) {
            return format!("'{}' cannot be a {noun}: a {noun}'s name is one word.", name.trim());
        }
    }
    if verb == "set" && params.get(list.attribute).is_none_or(|p| p.trim().is_empty()) {
        return format!("Pick a {noun}.");
    }
    if verb == "add" || verb == "recolour" {
        let colour = params
            .get("colour")
            .map_or(if verb == "add" { "default" } else { "" }, String::as_str);
        if !COLOURS.contains(&colour) {
            return format!("'{colour}' is not a colour.");
        }
    }
    String::new()
}

/// The files after `operation` on `target_id`, the surface's id of a requirement or a member's id; or the sentence
/// saying why it cannot be, in which case nothing changed. A target the files no longer have is refused as a change
/// made against an older file.
pub fn apply(
    files: ListFiles<'_>,
    operation: &str,
    target_id: &str,
    params: &BTreeMap<String, String>,
) -> Result<ListEdit, String> {
    let Some((list, verb)) = list_of(operation) else {
        return Err(format!("'{operation}' cannot be done here."));
    };
    let refused = refusal(operation, params);
    if !refused.is_empty() {
        return Err(refused);
    }
    let mut read = RequirementBody::read(
        files.body.to_vec(),
        files.body_name,
        files.registration.map(|r| (r.to_vec(), files.registration_name)),
    );
    let document = read.document.clone();
    if !document.read_only_reason.is_empty() {
        return Err(document.read_only_reason);
    }
    let members = members(&document, list);
    let held = read.registration.as_ref().and_then(|r| r.block_entries(list.block));
    let mut edit = BodyEdit::new(&mut read);
    let id_of = |member: &str| format!("{}{}", list.id_prefix, lower_ascii(member));
    let setting_of = |key: &str| Setting {
        block: list.block.into(),
        key: key.into(),
        ..Setting::default()
    };
    let mut setting: Option<Setting> = None;

    if verb == "set" || verb == "clear" {
        let ids = mapper::node_ids(&document.entries);
        let Some(requirement) = document
            .entries
            .iter()
            .zip(&ids)
            .find(|(e, id)| !e.is_item() && *id == target_id)
            .map(|(e, _)| e)
        else {
            return Err(DRIFTED.into());
        };
        let element = edit.element_of(requirement);
        if verb == "clear" {
            if value_of(requirement, list).is_some() {
                edit.set(&element, list.attribute, Value::Null);
            }
        } else {
            // A name equal to a member's in any case picks it, written with the list's spelling (DESL 0.2 section
            // 7.7.2).
            let picked = params[list.attribute].trim();
            let Some(member) = members.iter().find(|m| same(m, picked) || id_of(m) == picked) else {
                return Err(format!("'{picked}' is not a {} of this document.", list.attribute));
            };
            if value_of(requirement, list) != Some(member.as_str()) {
                edit.set(&element, list.attribute, Value::String(member.clone()));
            }
        }
    } else if verb == "add" {
        let name = normalised(&params["name"]);
        if members.iter().any(|m| same(m, &name)) {
            return Err(format!("There is already a {} named {name}.", list.attribute));
        }
        setting = Some(Setting {
            value: Some(params.get("colour").cloned().unwrap_or_else(|| "default".into())),
            ..setting_of(&name)
        });
    } else {
        let Some(member) = members.iter().find(|m| id_of(m) == target_id).cloned() else {
            return Err(DRIFTED.into());
        };
        match verb {
            "rename" => {
                let name = normalised(&params["name"]);
                if members.iter().any(|other| *other != member && same(other, &name)) {
                    return Err(format!("There is already a {} named {name}.", list.attribute));
                }
                for carrier in carriers(&document, list, &member) {
                    let element = edit.element_of(carrier);
                    edit.set(&element, list.attribute, Value::String(name.clone()));
                }
                setting = (name != member).then(|| Setting {
                    rename: Some(name),
                    ..setting_of(&member)
                });
            }
            "recolour" => {
                setting = Some(Setting {
                    value: params.get("colour").cloned(),
                    ..setting_of(&member)
                });
            }
            "move" => {
                let after = params.get("after").map_or("", String::as_str);
                setting = tags::moved_in(list.block, &members, &member, after, id_of)?;
            }
            _ => {
                for carrier in carriers(&document, list, &member) {
                    let element = edit.element_of(carrier);
                    edit.set(&element, list.attribute, Value::Null);
                }
                setting = Some(Setting {
                    remove_entry: true,
                    ..setting_of(&member)
                });
            }
        }
        // A member the list holds only because a requirement names it has no entry to rename or delete while the block
        // is written; with no block, the first change writes the whole list, and it is among it.
        if let Some(held) = &held
            && held.iter().all(|(key, _)| *key != member)
        {
            setting = setting.filter(|s| !s.remove_entry && s.rename.is_none());
        }
    }

    // Only an operation that changes a requirement writes the body, and with it the numbering every edit of it writes.
    let body = if edit.sets_any() { Some(edit.write()?) } else { None };
    let Some(setting) = setting else {
        return Ok(ListEdit {
            body,
            ..ListEdit::default()
        });
    };
    let (before, after) = registration::apply(files, &document, setting)?;
    Ok(ListEdit {
        body,
        registration_before: Some(before),
        registration_after: Some(after),
    })
}

/// A list as the page shows it: its members, then the names the requirements carry that it does not hold.
fn members(document: &Document, list: &List) -> Vec<String> {
    let mut members: Vec<String> = registration::members(document, list.block)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    for name in document.entries.iter().filter_map(|e| value_of(e, list)) {
        if !members.iter().any(|m| same(m, name)) {
            members.push(name.to_owned());
        }
    }
    members
}

/// The requirements that carry `member` in any case, in the file's order, as the operations' forEach takes them.
fn carriers<'d>(document: &'d Document, list: &List, member: &str) -> Vec<&'d Entry> {
    document
        .entries
        .iter()
        .filter(|e| value_of(e, list).is_some_and(|v| same(v, member)))
        .collect()
}

fn value_of<'e>(entry: &'e Entry, list: &List) -> Option<&'e str> {
    if list.block == "categories" {
        entry.category()
    } else {
        entry.state()
    }
}
