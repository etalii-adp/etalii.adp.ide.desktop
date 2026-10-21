//! One knowledge file's body, read through FBL with the binding its extension selects (the standalone host's
//! `KnowledgeBody` at 8b6ccfc). The module has no parser and no writer of its own: reading is the library's reading
//! of the vendored binding's rules, so the three formats give one model, and a file that was not changed is the
//! bytes it was read from. A change is planned through FBL's splices, so every byte it does not touch is kept.

use std::collections::HashMap;
use std::rc::Rc;

use adp_cel::{Map, Value};
use adp_fbl::model::FindingSeverity;
use adp_fbl::{FblBinding, FblModel, FblOptions, ModelChange, OpenBody, PlanResult};

use crate::definition;
use crate::model::{self, KnowledgeTable};
use crate::validator;

/// What is said of a file that does not open with the mark of the format version the vendored bindings read. The
/// bindings require that mark, so such a file is not read at all: a file of a later version may hold what this
/// version would misread, and a misread file that is then written is a damaged file.
pub const ANOTHER_VERSION: &str = "This file does not say it is a knowledge file of version 0.1, the version this application knows. It is left exactly as it is and cannot be changed here.";

/// What a change planned against an earlier state of the file is refused with.
pub const CHANGED_SINCE: &str = "The file has changed since this edit was made, so it was not written.";

/// One knowledge file's body.
pub struct KnowledgeBody {
    bytes: Vec<u8>,
    opened: Option<(Rc<FblBinding>, Rc<FblOptions>, OpenBody)>,
    model: FblModel,
    unreadable: String,
    table: KnowledgeTable,
}

impl KnowledgeBody {
    /// Reads a body. Never fails on content: a file that is not well-formed, or whose extension no binding reads,
    /// comes back with [`KnowledgeBody::unreadable`] saying why.
    pub fn read(bytes: Vec<u8>, file_name: &str) -> KnowledgeBody {
        let name = file_name.rsplit(['/', '\\']).next().unwrap_or(file_name);
        let extension = name.rfind('.').map(|at| &name[at..]).unwrap_or_default();
        let Some(binding) = definition::binding_for(extension) else {
            return KnowledgeBody {
                bytes,
                opened: None,
                model: FblModel {
                    unreadable: true,
                    ..FblModel::default()
                },
                unreadable: format!("A knowledge file is YAML, JSON or XML; '{extension}' is none of them."),
                table: KnowledgeTable::default(),
            };
        };
        let options = Rc::new(FblOptions {
            file_name: name.to_owned(),
            derive_id: Some(Rc::new(definition::derive_id)),
            attribute_type: Some(Rc::new(definition::attribute_type)),
            report_unbound_keys: true,
            ..FblOptions::default()
        });
        let body = OpenBody::open(bytes.clone(), binding.clone(), options.clone());
        let model = body.model();
        let mut unreadable = String::new();
        if model.unreadable {
            let finding = model
                .findings
                .iter()
                .find(|f| f.severity == FindingSeverity::Error)
                .or_else(|| model.findings.first());
            unreadable = match finding {
                None => "This file cannot be read.".into(),
                Some(f) if f.message.starts_with("The body does not start with the mark") => ANOTHER_VERSION.into(),
                Some(f) => match &f.location {
                    Some(at) => format!(
                        "This file cannot be read: {} (line {}, column {})",
                        f.message, at.line, at.column
                    ),
                    None => format!("This file cannot be read: {}", f.message),
                },
            };
        }
        let table = if unreadable.is_empty() {
            model::read(&model)
        } else {
            KnowledgeTable::default()
        };
        KnowledgeBody {
            bytes,
            opened: Some((binding, options, body)),
            model,
            unreadable,
            table,
        }
    }

    /// The bytes the body was read from.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// What the reading gave: the elements, and what it has to report.
    pub fn model(&self) -> &FblModel {
        &self.model
    }

    /// The table the elements make. Empty for a body that cannot be read.
    pub fn table(&self) -> &KnowledgeTable {
        &self.table
    }

    /// Why the body cannot be read at all, with where - the format's own error and its position - or empty when it
    /// can. Such a body is shown as its finding and is never written.
    pub fn unreadable(&self) -> &str {
        &self.unreadable
    }

    /// Why the table cannot be edited, or empty when it can: it cannot be read, or its binding says it is read-only.
    pub fn read_only_reason(&self) -> String {
        if !self.unreadable.is_empty() {
            return self.unreadable.clone();
        }
        match &self.opened {
            Some((_, _, body)) if body.is_read_only() => "This file cannot be changed here.".into(),
            _ => String::new(),
        }
    }

    /// The body's bytes with `changes` made, or why they cannot be. The changes are made to a copy, one after the
    /// other, each planned against the result of the one before; the first that is refused ends it, and this body
    /// is as it was either way.
    pub fn change(&self, changes: &[ModelChange]) -> Result<Vec<u8>, String> {
        let reason = self.read_only_reason();
        let Some((binding, options, _)) = self.opened.as_ref().filter(|_| reason.is_empty()) else {
            return Err(if reason.is_empty() {
                "This file cannot be changed here.".into()
            } else {
                reason
            });
        };
        let mut copy = OpenBody::open(self.bytes.clone(), binding.clone(), options.clone());

        // The first edit made here gives every entry without an id one, in the same step, so that it can be named
        // from then on (Requirement 8.5). From the last line up: an id is a line more, and an entry without one is
        // found by the line it is on.
        let mut given: HashMap<String, String> = HashMap::new();
        let mut unnamed: Vec<_> = self
            .model
            .elements
            .iter()
            .filter(|e| !e.id_is_stored && validator::IDENTIFIED.contains(&e.type_name.as_str()))
            .collect();
        unnamed.sort_by_key(|e| std::cmp::Reverse(e.line));
        for element in unnamed {
            let id = definition::new_id();
            match copy.change(&ModelChange::StoreId {
                id: element.id.clone(),
                new_id: id.clone(),
            }) {
                PlanResult::Refused(reason) => return Err(reason),
                PlanResult::Invalid(_) => return Err(CHANGED_SINCE.into()),
                PlanResult::Planned(_) => {}
            }
            given.insert(element.id.clone(), id);
        }

        for change in changes {
            let change = if given.is_empty() {
                change.clone()
            } else {
                renamed(change, &given)
            };
            match copy.change(&change) {
                PlanResult::Refused(reason) => return Err(reason),
                // What the edit names is not in the file as it is now: it was planned against an earlier state of it.
                PlanResult::Invalid(_) => return Err(CHANGED_SINCE.into()),
                PlanResult::Planned(_) => {}
            }
        }
        Ok(copy.bytes().to_vec())
    }
}

/// A change with every made-up id it names replaced by the id that entry was just given: in what it is about, where
/// it goes, and what its values name.
fn renamed(change: &ModelChange, given: &HashMap<String, String>) -> ModelChange {
    // An id made from its parents' ids holds theirs between slashes.
    let of = |id: &str| -> String {
        id.split('/')
            .map(|part| given.get(part).map(String::as_str).unwrap_or(part))
            .collect::<Vec<_>>()
            .join("/")
    };
    let values = |attributes: &Map| -> Map {
        attributes
            .iter()
            .map(|(k, v)| {
                let value = match v {
                    Value::String(text) => Value::String(given.get(text).cloned().unwrap_or_else(|| text.clone())),
                    other => other.clone(),
                };
                (k.clone(), value)
            })
            .collect()
    };
    match change {
        ModelChange::Add {
            type_name,
            id,
            attributes,
            parent_id,
            index,
            slot,
        } => ModelChange::Add {
            type_name: type_name.clone(),
            id: id.clone(),
            attributes: values(attributes),
            parent_id: parent_id.as_deref().map(of),
            index: *index,
            slot: slot.clone(),
        },
        ModelChange::Set { id, attributes } => ModelChange::Set {
            id: of(id),
            attributes: values(attributes),
        },
        ModelChange::Remove { id } => ModelChange::Remove { id: of(id) },
        ModelChange::Move {
            id,
            new_parent_id,
            index,
        } => ModelChange::Move {
            id: of(id),
            new_parent_id: new_parent_id.as_deref().map(of),
            index: *index,
        },
        other => other.clone(),
    }
}
