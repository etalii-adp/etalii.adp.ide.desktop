//! Every edit of an activity file, as a command on the project's history (etalii.adp spec 030, task T075).
//!
//! Ported from the standalone host's `src/diagrams/agent-activity-diagram/backend/EtAlii.Adp.Diagram.AgentActivityDiagram/
//! Commands/AadCommands.cs` and `Commands/AadEditHandler.cs` at `8b6ccfc`, over the core's `RestoreDocumentCommand`. A
//! command names what it changes by the body's path and ids, never by a live object, so it is plain data a history can
//! hold. Each is made on the body as it is now, through the binding's splices; then whatever the reader set for an
//! element an agent has since removed goes with the write. **A newer file is shown and not changed.** Its inverse puts
//! back the text the body had before, and redoes the edit itself, with the id it was given, when that is undone in
//! turn: what was added, joined and undone comes back as the element the later edits name.

use std::path::{Path, PathBuf};

use adp_cel::{Map, Value};
use adp_diagram::DiagramDocuments;
use adp_disl::behavior::{deletion, ids, operations, write};
use adp_disl::{DislChange, DislElement, DislInvocation, DislTransaction};
use adp_fbl::ModelChange;
use adp_history::{Applied, Command, Refusal};

use crate::body::{AadBody, KNOWN_VERSION};
use crate::definition::{self, AadDefinition, GONE};
use crate::store;

/// One edit, as its standalone command said it.
#[derive(Debug, Clone, PartialEq)]
pub enum AadEdit {
    /// A toolbox tool dropped: a new element with the tool's initial values, locked where it was dropped
    /// (`AddAadElementCommand`).
    AddElement { tool: String, x: f64, y: f64, id: String },
    /// A task added to a specification or a pull request to a location, updated now (`AddAadRowCommand`).
    AddRow { parent_id: String, id: String },
    /// An element or a row removed, with what the deletion policy takes or clears with it (`RemoveAadElementCommand`).
    Remove { element_id: String },
    /// One attribute of an element or a row; a row's change is stamped (`SetAadAttributeCommand`).
    SetAttribute {
        element_id: String,
        attribute: String,
        value: String,
    },
    /// A task's status, through the definition's own operation, which stamps the task (`SetAadTaskStatusCommand`).
    SetTaskStatus { task_id: String, status: String },
    /// The one relation the two elements' types allow, drawn either way (`ConnectAadCommand`).
    Connect { from: String, to: String },
    /// A relation removed: the key that states it is cleared (`DisconnectAadCommand`).
    Disconnect { relation_id: String },
    /// An element locked at a position, or its lock moved (`PinAadElementCommand`).
    Pin { element_id: String, x: f64, y: f64 },
    /// One element unlocked, or with none every one (`UnpinAadElementCommand`).
    Unpin { element_id: Option<String> },
    /// One group of one element folded or unfolded (`SetAadGroupCommand`).
    SetGroup {
        element_id: String,
        group: String,
        collapsed: bool,
    },
    /// Archived specifications shown or hidden (`SetAadShowArchivedCommand`).
    SetShowArchived { show: bool },
}

/// One edit of one body.
#[derive(Debug, Clone, PartialEq)]
pub struct AadCommand {
    pub body_path: PathBuf,
    pub edit: AadEdit,
}

impl AadCommand {
    pub fn new(body_path: &Path, edit: AadEdit) -> Self {
        AadCommand {
            body_path: body_path.to_path_buf(),
            edit,
        }
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

impl Command<DiagramDocuments> for AadCommand {
    fn label(&self) -> String {
        match &self.edit {
            AadEdit::AddElement { .. } | AadEdit::AddRow { .. } => "Add",
            AadEdit::Remove { .. } => "Remove",
            AadEdit::SetAttribute { .. } | AadEdit::SetTaskStatus { .. } => "Change",
            AadEdit::Connect { .. } => "Connect",
            AadEdit::Disconnect { .. } => "Disconnect",
            AadEdit::Pin { .. } => "Move",
            AadEdit::Unpin { .. } => "Unlock position",
            AadEdit::SetGroup { .. } => "Fold",
            AadEdit::SetShowArchived { .. } => "Show archived",
        }
        .to_owned()
    }

    /// The edit run (`AadEditHandler.Apply`): the body as it is now, refused unread or of a newer version, the edit
    /// made, the reader's entries about nothing taken with it, the text written and read again, the inverse recorded.
    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        let document = store::current(documents, &self.body_path);
        if let Some(reason) = &document.unreadable {
            return Err(Refusal::new(format!(
                "{} could not be read, so nothing can be edited until it can: {reason}",
                file_name(&self.body_path)
            )));
        }
        let before = document.text.clone();
        let mut body = AadBody::parse(&before);
        if body.version() > KNOWN_VERSION {
            return Err(Refusal::new(format!(
                "{} was written for version {} of the activity file, which this version of ADP shows and does not change.",
                file_name(&self.body_path),
                body.version()
            )));
        }
        edit(&mut body, &self.edit).map_err(Refusal::new)?;
        for orphan in body.orphaned_view_entries() {
            body.change(&ModelChange::Remove { id: orphan }).map_err(Refusal::new)?;
        }
        let after = body.text();
        store::save(documents, &self.body_path, &after).map_err(Refusal::new)?;
        Ok(Applied {
            inverse: Some(Box::new(RestoreAad {
                body_path: self.body_path.clone(),
                text: before,
                after,
                redo: self.clone(),
            })),
            warning: String::new(),
        })
    }
}

/// The undo of an edit: the body's text as it was, and the edit to make again when this is undone in turn
/// (`RestoreDocumentCommand`). It refuses once an agent has written the file after the edit: what the agent wrote is
/// not undone over.
#[derive(Debug, Clone, PartialEq)]
pub struct RestoreAad {
    pub body_path: PathBuf,
    pub text: String,
    /// The text the edit left, which the file must still hold to be put back.
    pub after: String,
    pub redo: AadCommand,
}

impl Command<DiagramDocuments> for RestoreAad {
    fn label(&self) -> String {
        self.redo.label()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        if store::current(documents, &self.body_path).text != self.after {
            return Err(Refusal::new(format!(
                "{} was written since this edit, so the edit is not undone over what was written.",
                file_name(&self.body_path)
            )));
        }
        store::save(documents, &self.body_path, &self.text).map_err(Refusal::new)?;
        Ok(Applied {
            inverse: Some(Box::new(self.redo.clone())),
            warning: String::new(),
        })
    }
}

/// The id of a locked position.
pub fn placement_id(element_id: &str) -> String {
    format!("pinned:{element_id}")
}

/// A position to the canvas unit: a drag gives fractions nobody reads, and the file is read by people. Rounded half to
/// even, as `Math.Round` rounds.
fn whole(value: f64) -> Value {
    Value::Int(value.round_ties_even() as i64)
}

fn map(entries: impl IntoIterator<Item = (&'static str, Value)>) -> Map {
    let mut map = Map::default();
    for (key, value) in entries {
        map.insert(key, value);
    }
    map
}

/// The lock of a new element.
fn new_placement(element_id: &str, x: f64, y: f64) -> ModelChange {
    ModelChange::Add {
        type_name: "Placement".to_owned(),
        id: Some(placement_id(element_id)),
        attributes: map([
            ("element", Value::String(element_id.to_owned())),
            ("x", whole(x)),
            ("y", whole(y)),
        ]),
        parent_id: None,
        index: None,
        slot: None,
    }
}

fn element_of(body: &AadBody, id: &str) -> Option<DislElement> {
    AadDefinition::element_of(&body.typed().diagram, id)
}

fn node_of(body: &AadBody, id: &str) -> Result<DislElement, String> {
    element_of(body, id)
        .filter(|e| !e.is_relation())
        .ok_or_else(|| GONE.to_owned())
}

/// Makes `edit` on `body`, or says why not.
pub fn edit(body: &mut AadBody, edit: &AadEdit) -> Result<(), String> {
    let definition = definition::get();
    let specification = definition.specification();
    match edit {
        AadEdit::AddElement { tool, x, y, id } => {
            let position = map([("x", Value::Double(*x)), ("y", Value::Double(*y))]);
            let transaction = operations::drop(
                specification,
                tool,
                &body.typed().diagram,
                &position,
                &ids::fixed(&[id.as_str()]),
                Some(&definition.env()),
            );
            body.apply(&transaction)?;
            match transaction.changes.iter().find_map(|c| match c {
                DislChange::Create { id, .. } => Some(id.clone()),
                _ => None,
            }) {
                Some(created) => body.change(&new_placement(&created, *x, *y)),
                None => Ok(()),
            }
        }
        AadEdit::AddRow { parent_id, id } => {
            let parent = node_of(body, parent_id)?;
            let now = Value::String(crate::now());
            let (type_name, attributes) = if parent.is_a("Specification") {
                (
                    "Task",
                    map([
                        ("title", Value::String("New task".to_owned())),
                        ("status", Value::String("pending".to_owned())),
                        ("updated", now),
                    ]),
                )
            } else if parent.is_a("Location") {
                (
                    "PullRequest",
                    map([
                        ("title", Value::String("New pull request".to_owned())),
                        ("updated", now),
                    ]),
                )
            } else {
                return Err("Only a specification lists tasks, and only a location lists pull requests.".to_owned());
            };
            body.apply(&DislTransaction {
                changes: vec![DislChange::Create {
                    type_name: type_name.to_owned(),
                    id: id.clone(),
                    attributes,
                    parent_id: Some(parent.id()),
                    at: Value::Null,
                    index: None,
                }],
                host_actions: Vec::new(),
                refusal: None,
            })
        }
        AadEdit::Remove { element_id } => {
            // Tasks and pull requests are written inside their element, and a locked position or a group state goes
            // with the element it names by the binding's own cascade.
            let element = node_of(body, element_id)?;
            let transaction = deletion::changes(specification, &element, true);
            body.apply(&transaction)
        }
        AadEdit::SetAttribute {
            element_id,
            attribute,
            value,
        } => {
            let element = node_of(body, element_id)?;
            let declared = element.element_type();
            let Some(declared_attribute) = declared.attributes.get(attribute) else {
                return Err(format!("A {} has no '{attribute}'.", element.type_name()));
            };
            // An enumeration is given by its label, its name or its stored word.
            let given = match definition.enum_of(&declared_attribute.type_name) {
                Some(choices) => match choices
                    .members
                    .iter()
                    .find(|m| m.label == *value || m.name == *value || m.stored == *value)
                {
                    Some(member) => Value::String(member.name.clone()),
                    None => return Err(format!("'{value}' is not one of the values this can have.")),
                },
                None if value.is_empty() => Value::Null,
                None => Value::String(value.clone()),
            };
            let mut attributes = Map::default();
            attributes.insert(
                attribute.clone(),
                write::stored_form(specification, &element.type_name(), attribute, &given),
            );
            // A change to a task or a pull request is the moment it was last updated, unless the moment itself is
            // what was set.
            if attribute != "updated" && declared.attributes.contains_key("updated") {
                attributes.insert("updated", Value::String(crate::now()));
            }
            body.change(&ModelChange::Set {
                id: element.id(),
                attributes,
            })
        }
        AadEdit::SetTaskStatus { task_id, status } => {
            let task = element_of(body, task_id)
                .filter(|e| e.is_a("Task"))
                .ok_or_else(|| GONE.to_owned())?;
            let invocation = DislInvocation {
                parameters: Some(map([("status", Value::String(status.clone()))])),
                ..DislInvocation::default()
            };
            let transaction = operations::run(
                specification,
                "setTaskStatus",
                &body.typed().diagram,
                Some(&task),
                &adp_disl::behavior::ids::short_guids(),
                Some(&invocation),
                Some(&definition.env()),
            );
            body.apply(&transaction)
        }
        AadEdit::Connect { from, to } => {
            let (Some(from), Some(to)) = (element_of(body, from), element_of(body, to)) else {
                return Err(GONE.to_owned());
            };
            // A relation has one direction in the file; the gesture may be drawn either way.
            let Some(relation) = definition.relation_between(&from, &to) else {
                return Err(definition.no_relation(&from, &to));
            };
            if let Some(occupied) = definition.occupied(&relation) {
                return Err(occupied);
            }
            let transaction = operations::connect(
                specification,
                &relation.relation.type_name,
                &body.typed().diagram,
                &relation.source,
                &relation.target,
                &adp_disl::behavior::ids::short_guids(),
                Some(&definition.env()),
            );
            body.apply(&transaction)
        }
        AadEdit::Disconnect { relation_id } => {
            let relation = element_of(body, relation_id)
                .filter(DislElement::is_relation)
                .ok_or_else(|| GONE.to_owned())?;
            let (holder, attribute) = definition.holder(&relation).ok_or_else(|| GONE.to_owned())?;
            let mut attributes = Map::default();
            attributes.insert(attribute, Value::Null);
            body.change(&ModelChange::Set {
                id: holder.id(),
                attributes,
            })
        }
        AadEdit::Pin { element_id, x, y } => {
            let element = node_of(body, element_id)?;
            if element.parent().is_some() {
                return Err(GONE.to_owned());
            }
            let id = placement_id(element_id);
            let change = if body.model().elements.iter().any(|e| e.id == id) {
                ModelChange::Set {
                    id,
                    attributes: map([("x", whole(*x)), ("y", whole(*y))]),
                }
            } else {
                new_placement(element_id, *x, *y)
            };
            body.change(&change)
        }
        AadEdit::Unpin { element_id } => {
            let pinned: Vec<String> = body
                .model()
                .elements
                .iter()
                .filter(|e| {
                    e.type_name == "Placement" && element_id.as_deref().is_none_or(|id| e.id == placement_id(id))
                })
                .map(|e| e.id.clone())
                .collect();
            if pinned.is_empty() {
                return Err(if element_id.is_none() {
                    "No position is locked."
                } else {
                    "Its position is not locked."
                }
                .to_owned());
            }
            for id in pinned {
                body.change(&ModelChange::Remove { id })?;
            }
            Ok(())
        }
        AadEdit::SetGroup {
            element_id,
            group,
            collapsed,
        } => {
            let element = element_of(body, element_id).ok_or_else(|| GONE.to_owned())?;
            if !definition.groups_of(&element).contains(group) {
                return Err(GONE.to_owned());
            }
            let id = format!("groups:{element_id}#{group}");
            let stored = body.model().elements.iter().any(|e| e.id == id);
            // The file says only what differs from the definition's default, so a group put back as it comes loses
            // its entry.
            let is_default = definition.collapsed_by_default().contains(group) == *collapsed;
            if is_default {
                return if stored {
                    body.change(&ModelChange::Remove { id })
                } else {
                    Ok(())
                };
            }
            body.change(&if stored {
                ModelChange::Set {
                    id,
                    attributes: map([("collapsed", Value::Bool(*collapsed))]),
                }
            } else {
                ModelChange::Add {
                    type_name: "GroupState".to_owned(),
                    id: Some(id),
                    attributes: map([
                        ("element", Value::String(element_id.clone())),
                        ("group", Value::String(definition.stored_group(group))),
                        ("collapsed", Value::Bool(*collapsed)),
                    ]),
                    parent_id: None,
                    index: None,
                    slot: None,
                }
            })
        }
        AadEdit::SetShowArchived { show } => body.change(&ModelChange::Set {
            id: "diagram".to_owned(),
            attributes: map([("showArchived", Value::Bool(*show))]),
        }),
    }
}
