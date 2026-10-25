//! One edit of a requirement document's body, as the designer writes it (the standalone host's `BodyEdit` at
//! a7fdf03): the changes an operation asks for, together with what DESL 0.2 section 5.3.5 has every edit write besides
//! itself - the number reading gave each placeholder, and the marks of the counters (spec 029 R17). The changes are
//! planned through the FBL library and made as one edit, so one undo takes all of them back.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use adp_cel::{Map, Value};
use adp_fbl::{ModelChange, PlanResult, PlannedEdit};
use regex::Regex;

use crate::body::RequirementBody;
use crate::model::{Document, Entry};

const ITEM: &str = "Item";
const REQUIREMENT: &str = "Requirement";
const MARKS: &str = "Marks";

/// A counter's number, `ITEM-0004` or `REQ-0031`.
static COUNTER_ID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:ITEM|REQ)-(?<number>\d+)$").expect("the pattern compiles"));

/// One edit of the body read as `read`.
pub struct BodyEdit<'a> {
    read: &'a mut RequirementBody,
    /// The attributes set of each element, in the order the elements were first set.
    sets: Vec<(String, Map)>,
    changes: Vec<ModelChange>,
    given: Vec<String>,
}

impl<'a> BodyEdit<'a> {
    pub fn new(read: &'a mut RequirementBody) -> BodyEdit<'a> {
        BodyEdit {
            read,
            sets: Vec::new(),
            changes: Vec::new(),
            given: Vec::new(),
        }
    }

    /// The library's id of the entry the designer names `entry`: its bullet's element.
    pub fn element_of(&self, entry: &Entry) -> String {
        element_of(self.read, entry)
    }

    /// Sets `attribute` of the library's element `element_id` to `value`.
    pub fn set(&mut self, element_id: &str, attribute: &str, value: Value) {
        self.sets_of(element_id).insert(attribute, value);
    }

    /// Whether the operation set any attribute, before what [`BodyEdit::write`] adds of its own.
    pub fn sets_any(&self) -> bool {
        !self.sets.is_empty()
    }

    /// A change of the body's structure, an add, a move or a remove, planned with the rest.
    pub fn change(&mut self, change: ModelChange) {
        self.changes.push(change);
    }

    /// Gives the number `id` in this edit besides the placeholders', so that the mark of its kind is set to it when it
    /// is the last (DESL 0.2 section 5.3.5).
    pub fn give(&mut self, id: &str) {
        self.given.push(id.to_owned());
    }

    /// The body's bytes after the edit, with every placeholder numbered and the marks set; or the sentence saying why
    /// the library refused it, in which case the body is unchanged.
    pub fn write(mut self) -> Result<Vec<u8>, String> {
        self.numbering();
        let mut changes: Vec<ModelChange> = std::mem::take(&mut self.sets)
            .into_iter()
            .map(|(id, attributes)| ModelChange::Set { id, attributes })
            .collect();
        changes.append(&mut self.changes);
        let mut splices = Vec::new();
        for change in &changes {
            match self.read.body.plan(change) {
                PlanResult::Planned(planned) => splices.extend(planned.splices),
                PlanResult::Refused(reason) | PlanResult::Invalid(reason) => return Err(reason),
            }
        }
        // Every change was planned against the bytes as read, so their splices are made together, in the file's order.
        splices.sort_by_key(|splice| splice.start);
        if !splices.is_empty() {
            self.read.body.apply(&PlannedEdit {
                splices,
                snapshot: false,
            })?;
        }
        Ok(self.read.body.bytes().to_vec())
    }

    fn sets_of(&mut self, element_id: &str) -> &mut Map {
        let at = match self.sets.iter().position(|(id, _)| id == element_id) {
            Some(at) => at,
            None => {
                self.sets.push((element_id.to_owned(), Map::default()));
                self.sets.len() - 1
            }
        };
        &mut self.sets[at].1
    }

    /// What every edit writes besides itself (DESL 0.2 section 5.3.5): each placeholder's number, the one reading gave
    /// it, and the mark of each kind given a number, set to the last number of that kind, on the mark line or, when
    /// the file has none, on a mark line added.
    fn numbering(&mut self) {
        let document = self.read.document.clone();
        let mut given: BTreeMap<&'static str, u32> = BTreeMap::new();
        for entry in document.entries.iter().filter(|e| e.placeholder) {
            let element = self.element_of(entry);
            self.set(&element, "id", Value::String(entry.id.clone()));
            let number = given.entry(entry.type_name()).or_default();
            *number = (*number).max(number_of(Some(&entry.id)));
        }
        for id in &self.given {
            let kind = if id.starts_with("ITEM-") { ITEM } else { REQUIREMENT };
            let number = given.entry(kind).or_default();
            *number = (*number).max(number_of(Some(id)));
        }
        if given.is_empty() {
            return;
        }

        // Only the mark of a kind this edit gives a number to is set; the other is left as it is. The standalone host
        // keeps them in the order it met them, the placeholders' first.
        let mut update = Map::default();
        for kind in order(&document, &self.given, &given) {
            let number = given[kind];
            let (mark, written) = if kind == ITEM {
                ("lastItem", document.last_item.as_deref())
            } else {
                ("lastRequirement", document.last_requirement.as_deref())
            };
            let highest = highest(&document, kind == ITEM);
            let last = counter(kind, number.max(number_of(written).max(highest)));
            if written != Some(last.as_str()) {
                update.insert(mark, Value::String(last));
            }
        }
        if update.is_empty() {
            return;
        }
        let marks = self
            .read
            .body
            .model()
            .elements
            .into_iter()
            .find(|e| e.type_name == MARKS);
        match marks {
            None => self.changes.insert(
                0,
                ModelChange::Add {
                    type_name: MARKS.into(),
                    id: None,
                    attributes: update,
                    parent_id: None,
                    index: None,
                    slot: None,
                },
            ),
            Some(marks) => {
                for (mark, value) in update.iter() {
                    self.set(&marks.id, mark, value.clone());
                }
            }
        }
    }
}

/// The kinds given a number, in the order the standalone host's dictionary meets them: the placeholders' in reading
/// order, then those given by the edit itself.
fn order(document: &Document, given_ids: &[String], given: &BTreeMap<&'static str, u32>) -> Vec<&'static str> {
    let mut kinds: Vec<&'static str> = Vec::new();
    let placeholders = document.entries.iter().filter(|e| e.placeholder).map(|e| e.type_name());
    let own = given_ids
        .iter()
        .map(|id| if id.starts_with("ITEM-") { ITEM } else { REQUIREMENT });
    for kind in placeholders.chain(own) {
        if given.contains_key(kind) && !kinds.contains(&kind) {
            kinds.push(kind);
        }
    }
    kinds
}

/// The library's id of the element of `entry`'s bullet in `read`.
pub fn element_of(read: &RequirementBody, entry: &Entry) -> String {
    read.body
        .model()
        .elements
        .into_iter()
        .find(|e| e.line == entry.line && (e.type_name == ITEM || e.type_name == REQUIREMENT))
        .map(|e| e.id)
        .expect("every entry is a bullet's element")
}

/// The number the next entry of its kind is given (DESL 0.2 section 5.3.5): one more than the mark and than every
/// number `document` holds, those reading gave its placeholders included.
pub fn next(document: &Document, item: bool) -> String {
    let mark = if item {
        document.last_item.as_deref()
    } else {
        document.last_requirement.as_deref()
    };
    let kind = if item { ITEM } else { REQUIREMENT };
    counter(kind, highest(document, item).max(number_of(mark)) + 1)
}

fn highest(document: &Document, item: bool) -> u32 {
    document
        .entries
        .iter()
        .filter(|e| e.is_item() == item)
        .map(|e| number_of(Some(&e.id)))
        .max()
        .unwrap_or(0)
}

fn counter(kind: &str, number: u32) -> String {
    format!("{}{number:04}", if kind == ITEM { "ITEM-" } else { "REQ-" })
}

fn number_of(id: Option<&str>) -> u32 {
    id.and_then(|id| COUNTER_ID.captures(id))
        .and_then(|c| c["number"].parse().ok())
        .unwrap_or(0)
}
