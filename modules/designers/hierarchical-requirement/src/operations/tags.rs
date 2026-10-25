//! The operations on tags of `hierarchical-requirement.des` (the standalone host's `TagOperations` at a7fdf03; FR-026):
//! `addTag` and `removeTag` on an entry, and `renameTag`, `recolourTag`, `moveTag` and `deleteTag` on a member of the
//! tag list. The tags an entry carries are in the Markdown file; the list's colours and order are in the
//! registration's `tags` block. An operation that changes both writes both in one step (spec 029 R17), byte for byte
//! as the conformance fixtures `hierarchical-requirement-tags` and `-tag-list` state.

use std::collections::BTreeMap;
use std::rc::Rc;

use adp_cel::Value;
use adp_fbl::{Setting, SettingPosition};

use super::body_edit::BodyEdit;
use super::lists::{ListEdit, ListFiles, normalised, same};
use super::registration;
use crate::body::{RequirementBody, lower_ascii};
use crate::gestures::DRIFTED;
use crate::mapper;
use crate::model::{Document, Entry};
use crate::vocabulary::COLOURS;

pub const ADD_TAG: &str = "addTag";
pub const REMOVE_TAG: &str = "removeTag";
pub const RENAME_TAG: &str = "renameTag";
pub const RECOLOUR_TAG: &str = "recolourTag";
pub const MOVE_TAG: &str = "moveTag";
pub const DELETE_TAG: &str = "deleteTag";

const TAGS_BLOCK: &str = "tags";
const TAGS_ATTRIBUTE: &str = "tags";
/// The prefix of a tag's id, `'tag/' + name.lowerAscii()` (hierarchical-requirement.des, ids).
const TAG_ID_PREFIX: &str = "tag/";

/// Whether `operation` is one of these.
pub fn handles(operation: &str) -> bool {
    operation == ADD_TAG || operation == REMOVE_TAG || targets_a_tag(operation)
}

/// Whether `operation` targets a member of the tag list rather than an entry.
pub fn targets_a_tag(operation: &str) -> bool {
    [RENAME_TAG, RECOLOUR_TAG, MOVE_TAG, DELETE_TAG].contains(&operation)
}

/// Whether `params` can be written by `operation`: the sentence saying why not, or empty. A name is trimmed and a
/// space in it written `-`; what is left is a word without `#`.
pub fn refusal(operation: &str, params: &BTreeMap<String, String>) -> String {
    match operation {
        ADD_TAG | REMOVE_TAG | RENAME_TAG => {
            let name = params.get("name").map_or("", String::as_str);
            let written = normalised(name);
            if written.is_empty() {
                return "A tag needs a name.".into();
            }
            if operation != REMOVE_TAG && (written.contains('#') || written.chars().any(char::is_whitespace)) {
                return format!("'{}' cannot be a tag: a tag's name is one word without #.", name.trim());
            }
            String::new()
        }
        RECOLOUR_TAG => {
            let colour = params.get("colour").map_or("", String::as_str);
            if COLOURS.contains(&colour) {
                String::new()
            } else {
                format!("'{colour}' is not a colour.")
            }
        }
        _ => String::new(),
    }
}

/// The files after `operation` on `target_id`, the surface's id of an entry or a tag's id; or the sentence saying why
/// it cannot be, in which case nothing changed. A target the files no longer have is refused as a change made against
/// an older file.
pub fn apply(
    files: ListFiles<'_>,
    operation: &str,
    target_id: &str,
    params: &BTreeMap<String, String>,
) -> Result<ListEdit, String> {
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
    let members = members(&document);
    let held = read.registration.as_ref().and_then(|r| r.block_entries(TAGS_BLOCK));
    let mut edit = BodyEdit::new(&mut read);
    let name = || normalised(params.get("name").map_or("", String::as_str));
    let mut setting: Option<Setting>;

    if operation == ADD_TAG || operation == REMOVE_TAG {
        let ids = mapper::node_ids(&document.entries);
        let Some(at) = ids.iter().position(|id| id == target_id) else {
            return Err(DRIFTED.into());
        };
        let entry = &document.entries[at];
        let name = name();
        if operation == REMOVE_TAG {
            setting = None;
            let kept = entry.tags.iter().filter(|t| !same(t, &name)).cloned().collect();
            set_tags(&mut edit, entry, kept);
        } else {
            // A name equal to a member's in any case picks it, with the list's spelling; one equal to none creates it,
            // last (FR-027).
            let found = members.iter().find(|m| same(m, &name));
            setting = found.is_none().then(|| setting_of(&name, Some("default")));
            if !entry.tags.iter().any(|t| same(t, &name)) {
                let mut tags = entry.tags.clone();
                tags.push(found.cloned().unwrap_or(name));
                set_tags(&mut edit, entry, tags);
            }
        }
    } else {
        let Some(member) = members.iter().find(|m| id_of(m) == target_id).cloned() else {
            return Err(DRIFTED.into());
        };
        match operation {
            RENAME_TAG => {
                let name = name();
                if members.iter().any(|other| *other != member && same(other, &name)) {
                    return Err(format!("There is already a tag named {name}."));
                }
                for carrier in carriers(&document, &member) {
                    let renamed = carrier
                        .tags
                        .iter()
                        .map(|t| if same(t, &member) { name.clone() } else { t.clone() })
                        .collect();
                    set_tags(&mut edit, carrier, renamed);
                }
                setting = (name != member).then(|| Setting {
                    rename: Some(name),
                    ..setting_of(&member, None)
                });
            }
            RECOLOUR_TAG => setting = Some(setting_of(&member, params.get("colour").map(String::as_str))),
            MOVE_TAG => setting = moved(&members, &member, params.get("after").map_or("", String::as_str))?,
            _ => {
                for carrier in carriers(&document, &member) {
                    let kept = carrier.tags.iter().filter(|t| !same(t, &member)).cloned().collect();
                    set_tags(&mut edit, carrier, kept);
                }
                setting = Some(Setting {
                    remove_entry: true,
                    ..setting_of(&member, None)
                });
            }
        }
        // A member the block does not hold, an agent's tag the list has not taken in, has nothing there to change.
        if let Some(held) = &held
            && held.iter().all(|(key, _)| *key != member)
        {
            setting = setting.filter(|s| !s.remove_entry && s.rename.is_none());
        }
    }

    // Only an operation that changes an entry's tags writes the body, and with it the numbering every edit of it writes.
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

/// Where `moveTag` puts `member`: first without `after`, otherwise directly after the member whose id it is; none when
/// it stands there already.
pub(crate) fn moved(members: &[String], member: &str, after: &str) -> Result<Option<Setting>, String> {
    moved_in(TAGS_BLOCK, members, member, after, id_of)
}

/// Where a move puts `member` of the list kept in `block`, its members' ids made by `id_of`.
pub(crate) fn moved_in(
    block: &str,
    members: &[String],
    member: &str,
    after: &str,
    id_of: impl Fn(&str) -> String,
) -> Result<Option<Setting>, String> {
    let at = |position: SettingPosition| Setting {
        block: block.into(),
        key: member.into(),
        position: Some(position),
        ..Setting::default()
    };
    if after.is_empty() {
        let first = &members[0];
        return Ok((!same(first, member)).then(|| {
            at(SettingPosition {
                before: Some(first.clone()),
                ..SettingPosition::default()
            })
        }));
    }
    let Some(previous) = members.iter().find(|m| id_of(m) == after) else {
        return Err(DRIFTED.into());
    };
    Ok((!same(previous, member)).then(|| {
        at(SettingPosition {
            after: Some(previous.clone()),
            ..SettingPosition::default()
        })
    }))
}

fn setting_of(key: &str, value: Option<&str>) -> Setting {
    Setting {
        block: TAGS_BLOCK.into(),
        key: key.into(),
        value: value.map(str::to_owned),
        ..Setting::default()
    }
}

fn id_of(member: &str) -> String {
    format!("{TAG_ID_PREFIX}{}", lower_ascii(member))
}

/// The tag list as the page shows it: its members, then the tags the entries carry that it does not hold.
fn members(document: &Document) -> Vec<String> {
    let mut members: Vec<String> = document.tags.iter().map(|t| t.name.clone()).collect();
    for tag in document.entries.iter().flat_map(|e| &e.tags) {
        if !members.iter().any(|m| same(m, tag)) {
            members.push(tag.clone());
        }
    }
    members
}

/// The entries that carry `tag` in any case. The standalone host takes the items first, as the operations' forEach
/// does; the edits are made together in the file's order, so the order they are taken in changes no byte.
fn carriers<'d>(document: &'d Document, tag: &str) -> Vec<&'d Entry> {
    document
        .entries
        .iter()
        .filter(|e| e.tags.iter().any(|t| same(t, tag)))
        .collect()
}

fn set_tags(edit: &mut BodyEdit<'_>, entry: &Entry, tags: Vec<String>) {
    if tags == entry.tags {
        return;
    }
    let element = edit.element_of(entry);
    let values = tags.into_iter().map(Value::String).collect();
    edit.set(&element, TAGS_ATTRIBUTE, Value::List(Rc::new(values)));
}
