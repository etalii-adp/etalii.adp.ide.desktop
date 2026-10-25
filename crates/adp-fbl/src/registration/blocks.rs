//! Writing a setting into a block of the registration that the binding declares (FBL 0.7 sections 8.1 and 8.4; the
//! standalone host's `Registration/DeclaredBlockWriter`): an entry `<key>: <value>`, indented by two spaces, with the
//! registration's line ending; the block created when it is missing, after the nearest block before it in the order
//! `layout:`, `identities:`, then the declared blocks; and the entries of a block keyed by ids that name no element
//! pruned in the same edit (FBL section 8.5).

use std::collections::HashSet;

use super::{RegistrationBlock, RegistrationDocument, RegistrationEntry, ordered, splice};
use crate::documents::types::RegistrationBlockDeclaration;
use crate::planning::plan::{Planning, refuse};
use crate::planning::{Setting, SettingPosition};
use crate::splice::{Operation, Splice};
use crate::text::BodyText;

pub(super) fn plan(
    document: &RegistrationDocument,
    declarations: &[RegistrationBlockDeclaration],
    declaration: &RegistrationBlockDeclaration,
    setting: &Setting,
    known_ids: Option<&HashSet<String>>,
    members: Option<Vec<(String, String)>>,
) -> Planning<Vec<Splice>> {
    let name = &declaration.name;
    let block = document.block(name);
    if let (None, Some(members)) = (block, members) {
        return whole(document, declarations, declaration, setting, members);
    }
    let stale: Vec<&RegistrationEntry> = match (block, known_ids) {
        (Some(block), Some(known)) if declaration.keyed_by_ids => block
            .entries
            .iter()
            .filter(|e| e.key != setting.key && !known.contains(&e.key))
            .collect(),
        _ => Vec::new(),
    };
    let mut splices: Vec<Splice> = stale
        .iter()
        .map(|e| splice(Operation::RemoveEntry, e.line.start, e.line.end, String::new()))
        .collect();
    let kept: Vec<&RegistrationEntry> = block
        .map(|b| b.entries.iter().filter(|e| !stale.contains(e)).collect())
        .unwrap_or_default();
    let existing = kept.iter().copied().find(|e| e.key == setting.key);
    if setting.position.is_some() && !declaration.kept {
        return refuse(format!(
            "The entries of '{name}' stand in the order of their keys, so none is moved."
        ));
    }

    if setting.remove_entry {
        let Some(existing) = existing else {
            return refuse(format!("The block '{name}' has no entry '{}'.", setting.key));
        };
        if kept.len() == 1 {
            let block = block.expect("a block with an entry");
            splices.push(splice(
                Operation::RemoveContainer,
                block.name_line.start,
                block.name_line.end,
                String::new(),
            ));
        }
        splices.push(splice(
            Operation::RemoveEntry,
            existing.line.start,
            existing.line.end,
            String::new(),
        ));
        return Ok(ordered(splices));
    }
    if let Some(value) = &setting.value {
        check_value(value)?;
    }
    let text = document.text();

    if let Some(existing) = existing {
        if let Some(rename) = &setting.rename {
            check_key(rename)?;
            if *rename != existing.key && kept.iter().any(|e| e.key == *rename) {
                return refuse(format!("The block '{name}' has an entry '{rename}' already."));
            }
            if *rename != existing.key {
                splices.push(splice(
                    Operation::ReplaceValue,
                    existing.key_span.start,
                    existing.key_span.end,
                    rename.clone(),
                ));
            }
        }
        if let Some(position) = &setting.position {
            let others: Vec<&RegistrationEntry> = kept.iter().copied().filter(|e| *e != existing).collect();
            let at = place(&others, Some(position), block.expect("a block with an entry"))?;
            let mut line = text.text_of(existing.line).to_owned();
            if let Some(moved) = &setting.value
                && *moved != existing.value
            {
                let start = existing.value_span.start - existing.line.start;
                let end = existing.value_span.end - existing.line.start;
                line = format!("{}{moved}{}", &line[..start], &line[end..]);
            }
            splices.push(splice(
                Operation::RemoveEntry,
                existing.line.start,
                existing.line.end,
                String::new(),
            ));
            splices.push(splice(Operation::InsertEntry, at, at, line));
        } else if let Some(changed) = &setting.value
            && *changed != existing.value
        {
            splices.push(splice(
                Operation::ReplaceValue,
                existing.value_span.start,
                existing.value_span.end,
                changed.clone(),
            ));
        }
        return Ok(ordered(splices));
    }

    check_key(&setting.key)?;
    if setting.rename.is_some() {
        return refuse(format!("The block '{name}' has no entry '{}' to rename.", setting.key));
    }
    let Some(added) = &setting.value else {
        return refuse(format!("A new entry of '{name}' needs a value."));
    };
    let indent = " ".repeat(kept.first().map(|e| e.indent).unwrap_or(2));
    let key = &setting.key;
    let Some(block) = block else {
        let offset = create_at(document, declarations, name);
        let newline = text.newline_at(offset.saturating_sub(1), "\n");
        if endless(text, offset) {
            splices.push(splice(
                Operation::EnsureContainer,
                offset,
                offset,
                format!("{newline}{name}:"),
            ));
            splices.push(splice(
                Operation::InsertEntry,
                offset,
                offset,
                format!("{newline}{indent}{key}: {added}"),
            ));
        } else {
            splices.push(splice(
                Operation::EnsureContainer,
                offset,
                offset,
                format!("{name}:{newline}"),
            ));
            splices.push(splice(
                Operation::InsertEntry,
                offset,
                offset,
                format!("{indent}{key}: {added}{newline}"),
            ));
        }
        return Ok(splices);
    };
    let at = if declaration.kept {
        place(&kept, setting.position.as_ref(), block)?
    } else {
        kept.iter()
            .find(|e| e.key.as_bytes() > key.as_bytes())
            .map(|e| e.line.start)
            .or_else(|| kept.last().map(|e| e.line.end))
            .unwrap_or(block.name_line.end)
    };
    let ending = text.newline_at(at.saturating_sub(1), "\n");
    let entry = if endless(text, at) {
        format!("{ending}{indent}{key}: {added}")
    } else {
        format!("{indent}{key}: {added}{ending}")
    };
    splices.push(splice(Operation::InsertEntry, at, at, entry));
    Ok(ordered(splices))
}

/// The first change to a block that is absent while its list has members (DESL 0.2 section 5.5, absent or
/// complete): the whole list with the change applied, as one `ensure-container` and one `insert-entry` per member,
/// all at the place the block is created (FBL 0.7 section 8.4). A member the change names is changed where it
/// stands; a new one goes at its position, else last.
fn whole(
    document: &RegistrationDocument,
    declarations: &[RegistrationBlockDeclaration],
    declaration: &RegistrationBlockDeclaration,
    setting: &Setting,
    members: Vec<(String, String)>,
) -> Planning<Vec<Splice>> {
    let name = &declaration.name;
    let mut list = members;
    let mut at = list.iter().position(|(key, _)| *key == setting.key);
    if setting.position.is_some() && !declaration.kept {
        return refuse(format!(
            "The entries of '{name}' stand in the order of their keys, so none is moved."
        ));
    }
    if setting.remove_entry {
        let Some(at) = at else {
            return refuse(format!("The block '{name}' has no entry '{}'.", setting.key));
        };
        list.remove(at);
    } else {
        if let Some(value) = &setting.value {
            check_value(value)?;
        }
        let at = match at.take() {
            Some(at) => at,
            None => {
                check_key(&setting.key)?;
                if setting.rename.is_some() {
                    return refuse(format!("The block '{name}' has no entry '{}' to rename.", setting.key));
                }
                let Some(value) = &setting.value else {
                    return refuse(format!("A new entry of '{name}' needs a value."));
                };
                list.push((setting.key.clone(), value.clone()));
                list.len() - 1
            }
        };
        let mut key = list[at].0.clone();
        if let Some(rename) = &setting.rename
            && *rename != key
        {
            check_key(rename)?;
            if list.iter().any(|(k, _)| k == rename) {
                return refuse(format!("The block '{name}' has an entry '{rename}' already."));
            }
            key = rename.clone();
        }
        let entry = (key, setting.value.clone().unwrap_or_else(|| list[at].1.clone()));
        list[at] = entry.clone();
        if let Some(position) = &setting.position {
            list.remove(at);
            let index = index(&list, position)?;
            list.insert(index, entry);
        }
    }
    if !declaration.kept {
        list.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    }
    let mut splices = Vec::new();
    if list.is_empty() {
        return Ok(splices);
    }
    let text = document.text();
    let offset = create_at(document, declarations, name);
    let newline = text.newline_at(offset.saturating_sub(1), "\n");
    let endless = endless(text, offset);
    splices.push(splice(
        Operation::EnsureContainer,
        offset,
        offset,
        if endless {
            format!("{newline}{name}:")
        } else {
            format!("{name}:{newline}")
        },
    ));
    for (key, value) in list {
        splices.push(splice(
            Operation::InsertEntry,
            offset,
            offset,
            if endless {
                format!("{newline}  {key}: {value}")
            } else {
                format!("  {key}: {value}{newline}")
            },
        ));
    }
    Ok(splices)
}

/// The place in `list`, the moved member taken out, that `position` names.
fn index(list: &[(String, String)], position: &SettingPosition) -> Planning<usize> {
    if let Some(index) = position.index {
        return Ok(if index >= 0 && (index as usize) < list.len() {
            index as usize
        } else {
            list.len()
        });
    }
    let Some(key) = position.after.as_ref().or(position.before.as_ref()) else {
        return Ok(list.len());
    };
    let Some(at) = list.iter().position(|(k, _)| k == key) else {
        return refuse(format!("The position names '{key}', which is no entry of the block."));
    };
    Ok(if position.after.is_some() { at + 1 } else { at })
}

/// A value is one line of text, not empty, and holds no `": "`, since an entry's key is everything before the last
/// one (FBL 0.7 section 8.1).
fn check_value(value: &str) -> Planning {
    if value.is_empty() {
        return refuse("A setting's value cannot be empty.");
    }
    if value.contains(": ") {
        return refuse("A setting's value cannot hold ': '.");
    }
    if value.contains(['\n', '\r']) {
        return refuse("A setting's value is one line.");
    }
    Ok(())
}

fn check_key(key: &str) -> Planning {
    if key.is_empty() {
        return refuse("A setting's key cannot be empty.");
    }
    if key.contains(['\n', '\r']) {
        return refuse("A setting's key is one line.");
    }
    Ok(())
}

/// Where an entry goes in a `kept` block among `others` (FBL 0.7 section 8.4): at its position, else after the last
/// entry.
fn place(
    others: &[&RegistrationEntry],
    position: Option<&SettingPosition>,
    block: &RegistrationBlock,
) -> Planning<usize> {
    let last = others.last().map(|e| e.line.end).unwrap_or(block.name_line.end);
    let Some(position) = position else {
        return Ok(last);
    };
    if let Some(index) = position.index {
        return Ok(if index >= 0 && (index as usize) < others.len() {
            others[index as usize].line.start
        } else {
            last
        });
    }
    if let Some(after) = &position.after {
        return Ok(entry(others, after)?.line.end);
    }
    match &position.before {
        Some(before) => Ok(entry(others, before)?.line.start),
        None => Ok(last),
    }
}

fn entry<'a>(entries: &[&'a RegistrationEntry], key: &str) -> Planning<&'a RegistrationEntry> {
    match entries.iter().find(|e| e.key == key) {
        Some(entry) => Ok(entry),
        None => refuse(format!("The position names '{key}', which is no entry of the block.")),
    }
}

/// Where a missing block is created (FBL 0.7 section 8.4): after the nearest block before it in the order of
/// blocks that exists, else after the headers.
fn create_at(document: &RegistrationDocument, declarations: &[RegistrationBlockDeclaration], name: &str) -> usize {
    let order: Vec<&str> = ["layout", "identities"]
        .into_iter()
        .chain(declarations.iter().map(|d| d.name.as_str()))
        .collect();
    let position = order.iter().position(|n| *n == name).unwrap_or(0);
    order[..position]
        .iter()
        .rev()
        .find_map(|before| document.block(before).map(|b| b.span.end))
        .unwrap_or(document.after_headers)
}

/// Whether `offset` is the end of a registration whose last line has no line ending.
fn endless(text: &BodyText, offset: usize) -> bool {
    offset == text.len() && offset > 0 && text.lines().last().is_some_and(|l| l.ending.is_empty())
}
