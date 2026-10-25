//! The Hierarchical Requirement Designer's fixtures (etalii.adp spec 036, T019 and T020; the standalone host's
//! `Conformance/HierarchicalRequirementFixtures.Tests.cs` and `Support/HierarchicalRequirementDocument.cs` at
//! `a7fdf03`): each is run as FBL 0.7 section 15.3 says a host passes one, through [`Document`], which stands in for
//! what DESL adds to the library. What a fixture's `read` lists is read, every step plans exactly its splices and
//! leaves exactly its document, or is refused with nothing written, an unchanged save writes nothing, and every byte
//! of a readable input belongs to the reading.
//!
//! Of a fixture's findings the run compares those the library reports: the FBL codes and `std.unparseable`,
//! `std.unreadableEntry` and `std.duplicateId`. The others are DESL's (a missing id, containment, a mention, a
//! required value), which the module's own tests hold the designer to.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::sync::{LazyLock, OnceLock};

use adp_cel::{Map, Value};
use adp_fbl::documents::FblBinding;
use adp_fbl::{
    BodyReading, FblElement, FblModel, FblOptions, Finding, ModelChange, OpenBody, OpenRegistration, PlanResult,
    PlannedEdit, Splice, UndoResult,
};
use regex::Regex;
use serde_json::Value as Json;

use super::{
    attributes, describe, fixtures_root, in_body_order, load_binding, read_json, read_setting, read_splices, vendor,
};

/// The number of fixtures vendored: fewer means the enumeration broke.
const VENDORED_FIXTURES: usize = 23;

const ITEM: &str = "Item";
const REQUIREMENT: &str = "Requirement";

/// The registration's blocks that are the lists of tags, categories and states (DESL 0.2 section 7.7).
const LISTS: [(&str, &str, &str); 3] = [
    ("tags", "Tag", "tags"),
    ("categories", "Category", "category"),
    ("states", "State", "state"),
];

/// Every fixture whose folder starts with `hierarchical-requirement-`, at any depth, by its path under `fixtures/`.
pub(super) fn fixtures() -> Vec<String> {
    let mut found = Vec::new();
    collect(&fixtures_root(), "", &mut found);
    found.sort();
    found
}

fn collect(folder: &Path, prefix: &str, found: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(folder) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = format!("{prefix}{}", entry.file_name().to_string_lossy());
        if path.join("fixture.json").exists() && name.starts_with("hierarchical-requirement-") {
            found.push(name.clone());
        }
        collect(&path, &format!("{name}/"), found);
    }
}

// ---- what the reading takes from the vendored specification (Support/HierarchicalRequirementSpecification.cs) ----

/// The vendored `definitions/designers/hierarchical-requirement.des`, which stands in for DESL as the timeline's
/// specification does for DISL: the attributes' types and which hold many values, the binding types it maps as
/// unreadable, its enumerations, and the initial members of its lists.
fn specification() -> &'static Json {
    static SPECIFICATION: OnceLock<Json> = OnceLock::new();
    SPECIFICATION.get_or_init(|| read_json(&vendor().join("definitions/designers/hierarchical-requirement.des")))
}

fn attribute(type_name: &str, attribute: &str) -> Option<&'static Json> {
    specification()["metamodel"]["types"][type_name]["attributes"].get(attribute)
}

fn type_of(type_name: &str, name: &str) -> Option<String> {
    attribute(type_name, name)?["type"].as_str().map(str::to_owned)
}

fn many(type_name: &str, name: &str) -> bool {
    attribute(type_name, name).is_some_and(|a| a["many"].as_bool() == Some(true))
}

/// The attributes the specification gives `type_name`, in its order.
fn attributes_of(type_name: &str) -> Vec<String> {
    specification()["metamodel"]["types"][type_name]["attributes"]
        .as_object()
        .map(|a| a.keys().cloned().collect())
        .unwrap_or_default()
}

/// The binding types `persistence.typeMap` maps as unreadable.
fn unreadable_types() -> HashSet<String> {
    specification()["persistence"]["typeMap"]
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(_, entry)| entry["as"].as_str() == Some("unreadable"))
        .map(|(name, _)| name.clone())
        .collect()
}

fn is_enumeration(type_name: &str) -> bool {
    specification()["metamodel"]["enums"].get(type_name).is_some()
}

fn values(enumeration: &str) -> Vec<(String, String)> {
    specification()["metamodel"]["enums"][enumeration]["values"]
        .as_object()
        .into_iter()
        .flatten()
        .map(|(key, value)| (key.clone(), value["value"].as_str().unwrap_or(key).to_owned()))
        .collect()
}

/// The key of the value of `enumeration` stored as `stored`, as the model holds it; the stored form itself when none
/// is.
fn key(enumeration: &str, stored: &str) -> String {
    values(enumeration)
        .into_iter()
        .find(|(_, s)| s == stored)
        .map_or_else(|| stored.to_owned(), |(key, _)| key)
}

/// The form the file stores the value `key` of `enumeration` in.
fn stored(enumeration: &str, key: &str) -> String {
    values(enumeration)
        .into_iter()
        .find(|(k, _)| k == key)
        .map_or_else(|| key.to_owned(), |(_, stored)| stored)
}

/// The initial members of the list `settings` keeps, by their name and value, in order.
fn initial(settings: &str) -> Vec<(String, String)> {
    let setting = &specification()["persistence"]["settings"][settings];
    let key = setting["key"].as_str().unwrap_or_default();
    let value = setting["value"].as_str().unwrap_or_default();
    setting["initial"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|member| {
            (
                member[key].as_str().unwrap_or_default().to_owned(),
                member[value].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

/// The reading's options for a body named `file_name`: the types, the many attributes and the unreadable types.
fn options(file_name: &str) -> Rc<FblOptions> {
    Rc::new(FblOptions {
        file_name: file_name.to_owned(),
        attribute_type: Some(Rc::new(type_of)),
        many: Some(Rc::new(many)),
        unreadable_types: unreadable_types(),
        ..FblOptions::default()
    })
}

// ---- the derived ids (Support/DerivedIds.cs) ----

/// CEL's `lowerAscii`: the ASCII capitals lowered, every other character kept.
fn lower_ascii(text: &str) -> String {
    text.to_ascii_lowercase()
}

fn member_id(type_name: &str, name: &str) -> String {
    match type_name {
        "Tag" => format!("tag/{}", lower_ascii(name)),
        "Category" => format!("category/{}", lower_ascii(name)),
        _ => format!("state/{}", lower_ascii(name)),
    }
}

// ---- the document as the designer sees it (Support/HierarchicalRequirementDocument.cs) ----

static WRITTEN_ID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[ \t]*[-*+][ \t]+(?<id>\S+)").unwrap());

static COUNTER_ID: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:ITEM|REQ)-(?<number>\d+)$").unwrap());

fn prefix(type_name: &str) -> &'static str {
    if type_name == ITEM { "ITEM-" } else { "REQ-" }
}

fn counter(type_name: &str, number: u32) -> String {
    format!("{}{number:04}", prefix(type_name))
}

fn number(id: Option<&str>) -> u32 {
    id.and_then(|id| COUNTER_ID.captures(id))
        .and_then(|c| c["number"].parse().ok())
        .unwrap_or(0)
}

fn mark(type_name: &str) -> &'static str {
    if type_name == ITEM {
        "lastItem"
    } else {
        "lastRequirement"
    }
}

fn text_of(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_str)
}

/// An element of a requirement document as the designer holds it.
#[derive(Debug)]
pub(super) struct DesignerElement {
    pub id: String,
    pub type_name: String,
    pub parent_id: Option<String>,
    pub attributes: HashMap<String, Value>,
}

/// An entry of the body with the id the designer gives it, and whether it was a placeholder.
struct Entry {
    element: FblElement,
    id: String,
    placeholder: bool,
}

/// A requirement document as the Hierarchical Requirement Designer sees it, standing in for DESL in the fixture run
/// as the derived ids stand in for DISL: the body read through the library, its entries known by the ids the
/// designer gives them, and its changes written as the designer writes them. What it adds to the library is what
/// `hierarchical-requirement.des` says and FBL does not: the counter ids with their placeholders and marks (DESL 0.2
/// section 5.3.5), the assessments' ids by place, the enumeration's keys, and the members of the lists (section 7.7).
pub(super) struct Document {
    body: Option<OpenBody>,
    registration: Option<OpenRegistration>,
}

impl Document {
    /// The input of a fixture: a body, or a registration with the body of its base name beside it (FBL 0.7 section
    /// 8.1).
    pub(super) fn open(folder: &Path, input_name: &str, binding: Rc<FblBinding>) -> Document {
        let input = fs::read(folder.join(input_name)).unwrap();
        if !input_name.to_lowercase().ends_with(".adp") {
            return Document {
                body: Some(OpenBody::open(input, binding, options(input_name))),
                registration: None,
            };
        }
        let mut registration = OpenRegistration::open_with(input, &binding);
        let body_name = registration.document().body().map(str::to_owned).unwrap_or_else(|| {
            let stem = &input_name[..input_name.rfind('.').unwrap_or(input_name.len())];
            format!("{stem}.md")
        });
        let body = fs::read(folder.join(&body_name))
            .ok()
            .map(|bytes| OpenBody::open(bytes, binding, options(&body_name)));
        let model = body.as_ref().map(|b| Rc::new(b.model()));
        registration.absent_members = Some(Rc::new(move |block: &str| {
            if !LISTS.iter().any(|(name, _, _)| *name == block) {
                return None;
            }
            let members = implied_members(block, model.as_deref());
            (!members.is_empty()).then_some(members)
        }));
        let mut document = Document {
            body,
            registration: None,
        };
        if document.body.is_some() {
            registration.known_ids = Some(document.entries().into_iter().map(|e| e.id).collect());
        }
        document.registration = Some(registration);
        document
    }

    /// The bytes a step changes: the registration's when the input is one, else the body's.
    pub(super) fn bytes(&self) -> Vec<u8> {
        match (&self.registration, &self.body) {
            (Some(registration), _) => registration.bytes().to_vec(),
            (None, Some(body)) => body.bytes().to_vec(),
            (None, None) => Vec::new(),
        }
    }

    fn unreadable(&self) -> bool {
        self.body.as_ref().is_some_and(|b| b.model().unreadable)
    }

    fn body(&self) -> &OpenBody {
        self.body.as_ref().expect("the fixture changes a body it does not have")
    }

    /// The findings of the reading that the library itself reports. A placeholder is no duplicate to DESL, which
    /// numbers it before ids are compared, so a duplicate the library reports at a placeholder is left out.
    fn findings(&self, input_name: &str) -> Vec<Finding> {
        let mut findings = Vec::new();
        if let Some(body) = &self.body {
            let placeholders: HashSet<usize> = self
                .entries()
                .iter()
                .filter(|e| e.placeholder)
                .map(|e| e.element.line)
                .collect();
            findings.extend(body.model().findings.into_iter().filter(|f| {
                !(f.code == "std.duplicateId" && placeholders.contains(&f.location.as_ref().map_or(0, |l| l.line)))
            }));
        }
        if let Some(registration) = &self.registration {
            findings.extend(registration.findings(input_name));
        }
        findings
    }

    /// The document's elements as the designer has them: its entries by their ids, then the members of its lists.
    fn elements(&self) -> Vec<DesignerElement> {
        let mut elements = Vec::new();
        if self.body.is_none() {
            return elements;
        }
        let entries = self.entries();
        let ids: HashMap<&str, &str> = entries.iter().map(|e| (e.element.id.as_str(), e.id.as_str())).collect();
        let collapsed: HashSet<String> = self
            .block("collapsed")
            .unwrap_or_default()
            .into_iter()
            .filter(|(_, value)| value == "yes")
            .map(|(key, _)| key)
            .collect();
        for entry in &entries {
            let element = &entry.element;
            let mut attributes: HashMap<String, Value> = element
                .attributes
                .iter()
                .map(|(name, value)| (name.clone(), view(&element.type_name, name, value)))
                .collect();
            for name in attributes_of(&element.type_name) {
                if attributes.get(&name).is_some_and(|v| !v.is_null()) {
                    continue;
                }
                let empty = if many(&element.type_name, &name) {
                    Value::list(Vec::new())
                } else {
                    match type_of(&element.type_name, &name).as_deref() {
                        Some("bool") => Value::Bool(false),
                        Some("string" | "text") => Value::from(""),
                        _ => Value::Null,
                    }
                };
                attributes.insert(name, empty);
            }
            if element.type_name == ITEM || element.type_name == REQUIREMENT {
                attributes.insert("collapsed".into(), Value::Bool(collapsed.contains(&entry.id)));
            }
            let parent = element
                .parent_id
                .as_deref()
                .map(|p| ids.get(p).copied().unwrap_or(p).to_owned());
            elements.push(DesignerElement {
                id: entry.id.clone(),
                type_name: element.type_name.clone(),
                parent_id: parent,
                attributes,
            });
        }
        for (block, type_name, _) in LISTS {
            for (name, colour) in self.members(block) {
                elements.push(DesignerElement {
                    id: member_id(type_name, &name),
                    type_name: type_name.to_owned(),
                    parent_id: None,
                    attributes: HashMap::from([
                        ("name".into(), Value::from(name.as_str())),
                        ("colour".into(), Value::from(colour.as_str())),
                    ]),
                });
            }
        }
        for (name, query) in self.block("filters").unwrap_or_default() {
            elements.push(DesignerElement {
                id: format!("filter/{name}"),
                type_name: "Filter".into(),
                parent_id: None,
                attributes: HashMap::from([
                    ("name".into(), Value::from(name.as_str())),
                    ("query".into(), Value::from(query.as_str())),
                ]),
            });
        }
        elements
    }

    /// Plans and applies a fixture's edit, one change or a list of them, as one edit. A setting goes to the
    /// registration. The first change that is no save also numbers the placeholders and sets the marks (DESL 0.2
    /// section 5.3.5).
    pub(super) fn change(&mut self, edit: &Json) -> Result<PlanResult, String> {
        if let Some(setting) = edit.get("setting") {
            let registration = self
                .registration
                .as_mut()
                .ok_or("a setting of a body with no registration")?;
            return Ok(registration.change(&ModelChange::Setting(read_setting(setting))));
        }
        if self.body.is_none() {
            return Err("The fixture changes a body it does not have.".into());
        }
        if edit.get("save").is_some() {
            return Ok(self.body.as_mut().unwrap().change(&ModelChange::Save));
        }

        let entries = self.entries();
        let mut changes: Vec<ModelChange> = Vec::new();
        let mut sets: Vec<(String, Map)> = Vec::new();
        let mut added: Vec<String> = Vec::new();
        let parts: Vec<&Json> = match edit.as_array() {
            Some(list) => list.iter().collect(),
            None => vec![edit],
        };
        for part in parts {
            if let Some(set) = part.get("set") {
                let occurrence = set["occurrence"].as_u64().unwrap_or(1) as usize;
                let Some(entry) = find(&entries, set["element"].as_str().unwrap_or_default(), occurrence) else {
                    continue;
                };
                let attributes_of_entry = set_of(&mut sets, &entry.element.id);
                for (name, value) in attributes(&set["attributes"]).iter() {
                    attributes_of_entry.insert(name.clone(), stored_value(&entry.element.type_name, name, value));
                }
                if let Some(id) = text_of(attributes_of_entry.get("id")) {
                    added.push(id.to_owned());
                }
            } else if let Some(add) = part.get("add") {
                let type_name = add["type"].as_str().unwrap_or_default();
                if ![ITEM, REQUIREMENT, "Assessment"].contains(&type_name) {
                    continue;
                }
                let mut given = Map::default();
                for (name, value) in attributes(&add["attributes"]).iter() {
                    given.insert(name.clone(), stored_value(type_name, name, value));
                }
                if type_name == "Assessment" && given.get("resolved").is_none() {
                    given.insert("resolved", Value::from("false"));
                }
                let id = (type_name != "Assessment").then(|| add["id"].as_str().unwrap_or_default().to_owned());
                if let Some(id) = &id {
                    added.push(id.clone());
                }
                let parent = add.get("parent").and_then(|p| library(&entries, p.as_str()));
                let index = match add.get("position") {
                    Some(position) => Some(index(&entries, position, parent.as_deref(), type_name)?),
                    None => None,
                };
                changes.push(ModelChange::Add {
                    type_name: type_name.to_owned(),
                    id,
                    attributes: given,
                    parent_id: parent,
                    index,
                    slot: None,
                });
            } else if let Some(remove) = part.get("remove") {
                if let Some(entry) = find(&entries, remove["element"].as_str().unwrap_or_default(), 1) {
                    changes.push(ModelChange::Remove {
                        id: entry.element.id.clone(),
                    });
                }
            } else if let Some(moved) = part.get("move") {
                let id = moved["element"].as_str().unwrap_or_default();
                let entry = find(&entries, id, 1).ok_or_else(|| format!("no entry '{id}' to move"))?;
                let parent = library(&entries, moved["parent"].as_str());
                let index = index(
                    &entries,
                    &moved["position"],
                    parent.as_deref(),
                    &entry.element.type_name,
                )?;
                changes.push(ModelChange::Move {
                    id: entry.element.id.clone(),
                    new_parent_id: parent,
                    index,
                });
            } else {
                return Err(format!("Unknown fixture edit: {part}."));
            }
        }
        self.numbering(&entries, &mut sets, &mut changes, &added);
        let mut all: Vec<ModelChange> = sets
            .into_iter()
            .map(|(id, attributes)| ModelChange::Set { id, attributes })
            .collect();
        all.extend(changes);

        let body = self.body.as_mut().unwrap();
        let mut splices: Vec<Splice> = Vec::new();
        for change in &all {
            match body.plan(change) {
                PlanResult::Planned(part) => splices.extend(part.splices),
                other => return Ok(other),
            }
        }
        let mut indexed: Vec<(usize, Splice)> = splices.into_iter().enumerate().collect();
        indexed.sort_by_key(|(at, splice)| (splice.start, *at));
        let combined = PlannedEdit {
            splices: indexed.into_iter().map(|(_, s)| s).collect(),
            snapshot: false,
        };
        if !combined.splices.is_empty()
            && let Err(problem) = body.apply(&combined)
        {
            return Ok(PlanResult::Invalid(problem));
        }
        Ok(PlanResult::Planned(combined))
    }

    pub(super) fn undo(&mut self) -> UndoResult {
        match (&mut self.registration, &mut self.body) {
            (Some(registration), _) => registration.undo(),
            (None, Some(body)) => body.undo(),
            (None, None) => UndoResult::Refused("nothing is open".into()),
        }
    }

    pub(super) fn redo(&mut self) -> UndoResult {
        match (&mut self.registration, &mut self.body) {
            (Some(registration), _) => registration.redo(),
            (None, Some(body)) => body.redo(),
            (None, None) => UndoResult::Refused("nothing is open".into()),
        }
    }

    /// What the first edit writes besides itself (DESL 0.2 section 5.3.5): a number for every placeholder, the next
    /// of its kind in reading order, and the marks, set to the last number given of each kind, by setting the mark
    /// line or, when there is none, adding it.
    fn numbering(
        &self,
        entries: &[Entry],
        sets: &mut Vec<(String, Map)>,
        changes: &mut Vec<ModelChange>,
        added: &[String],
    ) {
        let model = self.body().model();
        let marks = model.elements.iter().find(|e| e.type_name == "Marks");
        let mut given: Vec<(&str, u32)> = Vec::new();
        let give = |type_name: &'static str, id: &str, given: &mut Vec<(&str, u32)>| {
            let n = number(Some(id));
            match given.iter_mut().find(|(t, _)| *t == type_name) {
                Some((_, last)) => *last = (*last).max(n),
                None => given.push((type_name, n)),
            }
        };
        for entry in entries.iter().filter(|e| e.placeholder) {
            set_of(sets, &entry.element.id).insert("id", Value::from(entry.id.as_str()));
            let type_name = if entry.element.type_name == ITEM {
                ITEM
            } else {
                REQUIREMENT
            };
            give(type_name, &entry.id, &mut given);
        }
        for id in added {
            let type_name = if id.starts_with(prefix(ITEM)) {
                ITEM
            } else {
                REQUIREMENT
            };
            give(type_name, id, &mut given);
        }
        if given.is_empty() {
            return;
        }

        // Only the mark of a kind this edit gives a number to is set; the other is left as it is.
        let mut update = Map::default();
        for (type_name, n) in given {
            let name = mark(type_name);
            let written = marks.and_then(|m| text_of(m.attributes.get(name)));
            let highest = entries
                .iter()
                .filter(|e| e.element.type_name == type_name)
                .map(|e| number(Some(&e.id)))
                .max()
                .unwrap_or(0);
            let last = n.max(number(written).max(highest));
            if written != Some(counter(type_name, last).as_str()) {
                update.insert(name.to_owned(), Value::from(counter(type_name, last).as_str()));
            }
        }
        if update.is_empty() {
            return;
        }
        match marks {
            None => changes.insert(
                0,
                ModelChange::Add {
                    type_name: "Marks".into(),
                    id: None,
                    attributes: update,
                    parent_id: None,
                    index: None,
                    slot: None,
                },
            ),
            Some(marks) => {
                let id = marks.id.clone();
                match sets.iter_mut().find(|(set, _)| *set == id) {
                    Some((_, attributes)) => *attributes = update,
                    None => sets.push((id, update)),
                }
            }
        }
    }

    /// The body's entries by the designer's ids: an item or a requirement by the id written on its line, a
    /// placeholder by the number it is given, the next of its kind after the mark and every number the document
    /// holds, in reading order; an assessment by its place. A duplicate keeps its written number, which the library
    /// replaces by a place id.
    fn entries(&self) -> Vec<Entry> {
        let model = self.body().model();
        let text = String::from_utf8_lossy(self.body().bytes()).into_owned();
        let lines: Vec<&str> = text.split('\n').collect();
        let marks = model.elements.iter().find(|e| e.type_name == "Marks");
        let written = |element: &FblElement| -> String {
            let line = if element.line > 0 && element.line <= lines.len() {
                lines[element.line - 1]
            } else {
                ""
            };
            WRITTEN_ID
                .captures(line)
                .map_or_else(|| element.id.clone(), |c| c["id"].to_owned())
        };
        let highest = |type_name: &str| {
            model
                .elements
                .iter()
                .filter(|e| e.type_name == type_name)
                .map(|e| number(Some(&written(e))))
                .max()
                .unwrap_or(0)
                .max(number(marks.and_then(|m| text_of(m.attributes.get(mark(type_name))))))
        };
        let mut next: HashMap<&str, u32> = HashMap::from([(ITEM, highest(ITEM)), (REQUIREMENT, highest(REQUIREMENT))]);
        let mut entries = Vec::new();
        let mut assessments = 0;
        for element in &model.elements {
            if element.type_name == ITEM || element.type_name == REQUIREMENT {
                let id = written(element);
                let kind = if element.type_name == ITEM { ITEM } else { REQUIREMENT };
                if id == format!("{}NEW", prefix(kind)) {
                    let n = next.get_mut(kind).unwrap();
                    *n += 1;
                    entries.push(Entry {
                        element: element.clone(),
                        id: counter(kind, *n),
                        placeholder: true,
                    });
                } else {
                    entries.push(Entry {
                        element: element.clone(),
                        id,
                        placeholder: false,
                    });
                }
            } else if element.type_name == "Assessment" {
                entries.push(Entry {
                    element: element.clone(),
                    id: format!("assessment/{assessments}"),
                    placeholder: false,
                });
                assessments += 1;
            }
        }
        entries
    }

    /// The members of a list (DESL 0.2 sections 5.5, 7.7.2): its block when there is one, else its initial members
    /// and then those the body implies, in their order.
    fn members(&self, block: &str) -> Vec<(String, String)> {
        if let Some(stored) = self.block(block) {
            return stored;
        }
        implied_members(block, self.body.as_ref().map(|b| b.model()).as_ref())
    }

    fn block(&self, name: &str) -> Option<Vec<(String, String)>> {
        self.registration.as_ref()?.block_entries(name)
    }
}

/// The members a list stands for while its block is absent: its initial members, then those the body implies.
fn implied_members(block: &str, model: Option<&FblModel>) -> Vec<(String, String)> {
    let mut members = initial(block);
    let Some(model) = model else { return members };
    let attribute = LISTS
        .iter()
        .find(|(b, _, _)| *b == block)
        .map_or("state", |(_, _, a)| *a);
    for element in model
        .elements
        .iter()
        .filter(|e| e.type_name == ITEM || e.type_name == REQUIREMENT)
    {
        let names: Vec<String> = match element.attributes.get(attribute) {
            Some(Value::List(many)) => many.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect(),
            Some(Value::String(one)) => vec![one.clone()],
            _ => Vec::new(),
        };
        for name in names {
            if members
                .iter()
                .all(|(member, _)| lower_ascii(member) != lower_ascii(&name))
            {
                members.push((name, "default".into()));
            }
        }
    }
    members
}

fn set_of<'a>(sets: &'a mut Vec<(String, Map)>, id: &str) -> &'a mut Map {
    if let Some(at) = sets.iter().position(|(set, _)| set == id) {
        return &mut sets[at].1;
    }
    sets.push((id.to_owned(), Map::default()));
    &mut sets.last_mut().unwrap().1
}

/// A value as the designer holds it: an enumeration by its key, a bool as a bool.
fn view(type_name: &str, attribute: &str, value: &Value) -> Value {
    let (Some(declared), Some(text)) = (type_of(type_name, attribute), value.as_str()) else {
        return value.clone();
    };
    if is_enumeration(&declared) {
        return Value::from(key(&declared, text).as_str());
    }
    if declared == "bool" {
        return Value::Bool(text == "true");
    }
    value.clone()
}

/// A value as the file stores it: an enumeration by its stored form, a bool as its text.
fn stored_value(type_name: &str, attribute: &str, value: &Value) -> Value {
    let declared = type_of(type_name, attribute);
    if let (Value::Bool(flag), Some("bool")) = (value, declared.as_deref()) {
        return Value::from(if *flag { "true" } else { "false" });
    }
    if let (Some(declared), Some(key)) = (&declared, value.as_str())
        && is_enumeration(declared)
    {
        return Value::from(stored(declared, key).as_str());
    }
    value.clone()
}

fn find<'a>(entries: &'a [Entry], id: &str, occurrence: usize) -> Option<&'a Entry> {
    entries.iter().filter(|e| e.id == id).nth(occurrence.saturating_sub(1))
}

fn library(entries: &[Entry], id: Option<&str>) -> Option<String> {
    let id = id?;
    Some(find(entries, id, 1).map_or_else(|| id.to_owned(), |e| e.element.id.clone()))
}

/// The model change's index for a fixture's position, among the siblings of `type_name` under `parent` as they are
/// before the change.
fn index(entries: &[Entry], position: &Json, parent: Option<&str>, type_name: &str) -> Result<i64, String> {
    if let Some(index) = position.get("index") {
        return Ok(index.as_i64().unwrap_or(-1));
    }
    let siblings: Vec<&str> = entries
        .iter()
        .filter(|e| e.element.parent_id.as_deref() == parent && (parent.is_some() || e.element.type_name == type_name))
        .map(|e| e.id.as_str())
        .collect();
    let place = |sibling: &Json| -> i64 {
        let sibling = sibling.as_str().unwrap_or_default();
        siblings.iter().position(|s| *s == sibling).map_or(-1, |at| at as i64)
    };
    if let Some(after) = position.get("after") {
        return Ok(place(after) + 1);
    }
    if let Some(before) = position.get("before") {
        return Ok(place(before));
    }
    Err(format!("Unknown fixture position: {position}."))
}

// ---- the run (Conformance/HierarchicalRequirementFixtures.Tests.cs) ----

const LIBRARY_CODES: [&str; 3] = ["std.unparseable", "std.unreadableEntry", "std.duplicateId"];

fn is_librarys(code: &str) -> bool {
    code.starts_with("fbl.") || LIBRARY_CODES.contains(&code)
}

/// Each attribute a fixture's element names has that value and that JSON type; a list is compared member by
/// member.
fn same(expected: &Json, actual: &Value) -> bool {
    match expected {
        Json::String(s) => actual.as_str() == Some(s.as_str()),
        Json::Number(n) => matches!(actual, Value::Int(_) | Value::Double(_)) && actual.as_number() == n.as_f64(),
        Json::Bool(b) => matches!(actual, Value::Bool(x) if x == b),
        Json::Null => actual.is_null(),
        Json::Array(items) => match actual {
            Value::List(list) => list.len() == items.len() && items.iter().zip(list.iter()).all(|(e, a)| same(e, a)),
            _ => false,
        },
        _ => false,
    }
}

fn written(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::String(text) => format!("the text {text:?}"),
        Value::Bool(flag) => format!("the bool {flag}"),
        Value::List(list) => format!("the list [{}]", list.iter().map(written).collect::<Vec<_>>().join(", ")),
        other => format!("the value {other}"),
    }
}

fn check_read(document: &Document, read: &Json, name: &str, input_name: &str) -> Result<(), String> {
    if let Some(unreadable) = read.get("unreadable").and_then(Json::as_bool)
        && unreadable != document.unreadable()
    {
        return Err(format!(
            "{name}: unreadable is {}, where the fixture says {unreadable}.",
            document.unreadable()
        ));
    }
    let elements = document.elements();
    let mut seen: HashMap<String, usize> = HashMap::new();
    for element in read["elements"].as_array().into_iter().flatten() {
        let id = element["id"].as_str().unwrap_or_default();
        let occurrence = {
            let count = seen.entry(id.to_owned()).or_default();
            *count += 1;
            *count
        };
        let Some(found) = elements.iter().filter(|e| e.id == id).nth(occurrence - 1) else {
            let all: Vec<String> = elements.iter().map(|e| format!("{} ({})", e.id, e.type_name)).collect();
            return Err(format!(
                "{name}: no element '{id}' (occurrence {occurrence}) was read; read: {}.",
                all.join(", ")
            ));
        };
        if element["type"].as_str() != Some(found.type_name.as_str()) {
            return Err(format!(
                "{name}: '{id}' is a {}, where the fixture reads {}.",
                found.type_name, element["type"]
            ));
        }
        if let Some(parent) = element.get("parent")
            && parent.as_str() != found.parent_id.as_deref()
        {
            return Err(format!(
                "{name}: '{id}' is read in {:?}, where the fixture reads {parent}.",
                found.parent_id
            ));
        }
        for (attribute, expected) in element["attributes"].as_object().into_iter().flatten() {
            let label = format!("{name}: '{attribute}' of '{id}'");
            let Some(actual) = found.attributes.get(attribute) else {
                return Err(format!("{label} was not read."));
            };
            if !same(expected, actual) {
                return Err(format!(
                    "{label} is {}, where the fixture reads {expected}.",
                    written(actual)
                ));
            }
        }
    }
    if let Some(findings) = read.get("findings").and_then(Json::as_array) {
        let mut wanted: Vec<(String, usize)> = findings
            .iter()
            .map(|f| {
                (
                    f["rule"].as_str().unwrap_or_default().to_owned(),
                    f["line"].as_u64().unwrap_or(0) as usize,
                )
            })
            .filter(|(code, _)| is_librarys(code))
            .collect();
        wanted.sort();
        let mut actual: Vec<(String, usize)> = document
            .findings(input_name)
            .into_iter()
            .filter(|f| is_librarys(&f.code))
            .map(|f| (f.code, f.location.map_or(0, |l| l.line)))
            .collect();
        actual.sort();
        if wanted != actual {
            return Err(format!(
                "{name}: the fixture expects the findings {wanted:?}; read: {actual:?}."
            ));
        }
    }
    Ok(())
}

fn open(name: &str) -> (std::path::PathBuf, String, Rc<FblBinding>) {
    let folder = fixtures_root().join(name);
    let fixture = read_json(&folder.join("fixture.json"));
    let (binding, _) = load_binding(&fixture, &folder).unwrap_or_else(|e| panic!("{name}: {e}"));
    (
        folder,
        fixture["input"].as_str().unwrap_or_default().to_owned(),
        binding,
    )
}

/// Runs a fixture's reading and steps; its binding and input are found from `folder`.
fn run(name: &str, folder: &Path, fixture: &Json) -> Result<(), String> {
    let (binding, _) = load_binding(fixture, folder)?;
    let input_name = fixture["input"].as_str().unwrap_or_default();
    let mut document = Document::open(folder, input_name, binding);
    if let Some(read) = fixture.get("read") {
        check_read(&document, read, name, input_name)?;
    }
    for (index, step) in fixture["steps"].as_array().into_iter().flatten().enumerate() {
        let label = format!("{name} step {}", index + 1);
        let before = document.bytes();
        let expected = in_body_order(read_splices(step));
        let actual = if step["undo"].as_bool() == Some(true) {
            match document.undo() {
                UndoResult::Done(splices) => splices,
                UndoResult::Refused(reason) => return Err(format!("{label}: refused with '{reason}'.")),
            }
        } else if step["redo"].as_bool() == Some(true) {
            match document.redo() {
                UndoResult::Done(splices) => splices,
                UndoResult::Refused(reason) => return Err(format!("{label}: refused with '{reason}'.")),
            }
        } else {
            let result = document.change(&step["edit"])?;
            if step.get("refused").is_some() {
                let PlanResult::Refused(reason) = result else {
                    return Err(format!("{label}: planned, where the fixture refuses: {result:?}."));
                };
                if reason.trim().is_empty() {
                    return Err(format!("{label}: refused without a reason."));
                }
                if before != document.bytes() {
                    return Err(format!("{label}: a refused edit changed the document."));
                }
                Vec::new()
            } else {
                match result {
                    PlanResult::Planned(edit) => edit.splices,
                    PlanResult::Refused(reason) | PlanResult::Invalid(reason) => {
                        return Err(format!("{label}: refused with '{reason}'."));
                    }
                }
            }
        };
        if expected != actual {
            return Err(format!(
                "{label}: expected splices\n  {}\nbut planned\n  {}",
                describe(&expected),
                describe(&actual)
            ));
        }
        // A step that names no document leaves the one it started from.
        let expect = match (step["expect"].as_str(), step["expectFile"].as_str()) {
            (Some(text), _) => text.as_bytes().to_vec(),
            (None, Some(file)) => fs::read(folder.join(file)).map_err(|e| e.to_string())?,
            (None, None) => before,
        };
        if expect != document.bytes() {
            return Err(format!(
                "{label}: the document differs from 'expect'.\nExpected:\n{}\nActual:\n{}",
                String::from_utf8_lossy(&expect),
                String::from_utf8_lossy(&document.bytes())
            ));
        }
    }
    Ok(())
}

#[test]
fn the_fixtures_are_found() {
    assert_eq!(fixtures().len(), VENDORED_FIXTURES, "{:?}", fixtures());
}

#[test]
fn the_specification_is_read() {
    // A stand-in told nothing would read every attribute as text and every list as absent.
    assert!(many("Requirement", "tags"));
    assert!(!many("Requirement", "category"));
    assert_eq!(type_of("Assessment", "resolved").as_deref(), Some("bool"));
    assert!(unreadable_types().contains("Unreadable"));
    assert_eq!(key("AssessmentKind", "Question"), "question");
    assert_eq!(initial("categories").len(), 3);
}

#[test]
fn every_fixture_passes() {
    let mut failures = Vec::new();
    for name in fixtures() {
        let folder = fixtures_root().join(&name);
        if let Err(failure) = run(&name, &folder, &read_json(&folder.join("fixture.json"))) {
            failures.push(failure);
        }
    }
    assert!(
        failures.is_empty(),
        "{} fixtures failed:\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

/// The run fails on a fixture one of whose splices is off by a byte: a copy of `hierarchical-requirement-structure`,
/// the first splice of its first step ending one byte later.
#[test]
fn the_run_fails_on_a_splice_that_is_off_by_one_byte() {
    let name = "hierarchical-requirement-structure";
    let folder = fixtures_root().join(name);
    let mut fixture = read_json(&folder.join("fixture.json"));
    let end = &mut fixture["steps"][0]["splices"][0]["end"];
    *end = Json::from(end.as_u64().unwrap() + 1);

    let failure = run(name, &folder, &fixture).expect_err("the run passes a splice that is off by a byte");

    assert!(failure.contains("step 1: expected splices"), "{failure}");
}

/// The run fails at the step whose expected document has one wrong byte (029: Story 3, scenario 2): a copy of
/// `hierarchical-requirement-edits` with one byte of its second step's `expect` changed.
#[test]
fn the_run_fails_on_one_wrong_byte_of_an_expected_document() {
    let name = "hierarchical-requirement-edits";
    let folder = fixtures_root().join(name);
    let mut fixture = read_json(&folder.join("fixture.json"));
    let expect = fixture["steps"][1]["expect"]
        .as_str()
        .unwrap()
        .replacen("removed", "remover", 1);
    assert_ne!(
        Some(expect.as_str()),
        fixture["steps"][1]["expect"].as_str(),
        "the planted byte changes nothing"
    );
    fixture["steps"][1]["expect"] = Json::from(expect);

    let failure = run(name, &folder, &fixture).expect_err("the run passes a document with a wrong byte");

    assert!(
        failure.contains("step 2: the document differs from 'expect'"),
        "{failure}"
    );
}

#[test]
fn saving_without_an_edit_writes_the_bytes_that_were_read() {
    let mut failures = Vec::new();
    for name in fixtures() {
        let (folder, input_name, binding) = open(&name);
        let mut document = Document::open(&folder, &input_name, binding);
        let before = document.bytes();
        match document.change(&serde_json::json!({ "save": true })) {
            Ok(PlanResult::Planned(edit)) if edit.splices.is_empty() && document.bytes() == before => {}
            other => failures.push(format!("{name}: {other:?}")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_byte_of_a_readable_input_belongs_to_the_reading() {
    let mut failures = Vec::new();
    for name in fixtures() {
        let (folder, input_name, binding) = open(&name);
        if input_name.to_lowercase().ends_with(".adp") {
            continue;
        }
        let input = fs::read(folder.join(&input_name)).unwrap();
        // The byte-coverage invariant of FBL section 4.1, for the inputs that are read at all.
        let reading = BodyReading::read(input, binding, options(&input_name));
        if reading.unreadable_message().is_none() && !reading.unaccounted().is_empty() {
            failures.push(format!(
                "{name}: bytes outside the reading: {:?}",
                reading.unaccounted()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
