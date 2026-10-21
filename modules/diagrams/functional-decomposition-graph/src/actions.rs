//! What can be done to a selected element or connection, to empty canvas and to a finished connect gesture, and
//! the command each becomes (etalii.adp spec 030, task T072).
//!
//! Ported from the standalone host at `8b6ccfc`: the module's `FdgContextActionProvider.cs` over its
//! `DiagramActionProvider`, with the ids the client states once in `client/fdgIds.ts`. Add and connect are one id
//! per type (`fdg.add.{type}`, `fdg.connect.{relation}`) run on a canvas point or a gesture; a drop says everything
//! an add needs, and a connect is never trusted, since its command runs the type, cardinality and cycle checks
//! against the document as it is now. A rename asks for the text first, starting from what is there; a removal asks
//! first only when connections go with the element, naming how many.

use std::path::Path;

use adp_context::{Action, ActionGroup, Prompt, Shortcut};

use crate::commands::{FdgCommand, FdgEdit};
use crate::model::{FdgModel, element_types};
use crate::relations;

/// Rename an element in place: its name, or a Comment's text.
pub const RENAME: &str = "fdg.rename";
/// Rename a connection in place.
pub const RENAME_CONNECTION: &str = "fdg.rename-connection";
/// Remove an element and every connection to or from it.
pub const REMOVE: &str = "fdg.remove";
/// Remove a connection.
pub const DISCONNECT: &str = "fdg.disconnect";

const ADD_PREFIX: &str = "fdg.add.";
const CONNECT_PREFIX: &str = "fdg.connect.";

/// What an add on anything but a canvas point answers.
pub const ADD_BY_DROP: &str = "An element is added by dropping it where it belongs.";
/// What a connect on anything but a finished gesture answers.
pub const CONNECT_BY_GESTURE: &str = "A connection is drawn from one element to another.";
/// What an action on something no longer in the graph answers.
pub const GONE: &str = "That is no longer in this graph.";

/// The add action of one element type.
pub fn add_action_id(element_type: &str) -> String {
    format!("{ADD_PREFIX}{element_type}")
}

/// The connect action of one relation.
pub fn connect_action_id(relation: &str) -> String {
    format!("{CONNECT_PREFIX}{relation}")
}

/// What an action runs on: an element or a connection by its id, a point on the canvas a drop landed on, or a
/// finished connect gesture.
#[derive(Debug, Clone, PartialEq)]
pub enum FdgTarget<'a> {
    Item(&'a str),
    Placement { x: f64, y: f64 },
    Relation { from: &'a str, to: &'a str },
}

/// A document word as a menu reads it: `owns-data` becomes "owns data".
fn titled(word: &str) -> String {
    word.replace('-', " ")
}

fn action(id: &str, label: &str, icon: &str) -> Action {
    Action::new(id, label).with_icon(icon)
}

/// The menu of `target`.
pub fn discover(model: &FdgModel, target: &FdgTarget<'_>) -> Vec<ActionGroup> {
    match target {
        FdgTarget::Item(id) => {
            if let Some(element) = model.element_of(id) {
                return vec![ActionGroup {
                    actions: vec![
                        action(
                            RENAME,
                            if element.is_comment() {
                                "Edit text…"
                            } else {
                                "Rename…"
                            },
                            "mdi-pencil-outline",
                        )
                        .with_shortcut(Shortcut::key("F2")),
                        action(REMOVE, "Remove", "mdi-delete-outline").with_shortcut(Shortcut::key("Delete")),
                    ],
                }];
            }
            if model.connection_of(id).is_some() {
                return vec![ActionGroup {
                    actions: vec![
                        action(RENAME_CONNECTION, "Rename…", "mdi-pencil-outline").with_shortcut(Shortcut::key("F2")),
                        action(DISCONNECT, "Remove connection", "mdi-vector-polyline-remove")
                            .with_shortcut(Shortcut::key("Delete")),
                    ],
                }];
            }
            Vec::new()
        }
        FdgTarget::Placement { .. } => vec![ActionGroup {
            actions: element_types::ALL
                .iter()
                .map(|t| action(&add_action_id(t), &format!("Add {} here", titled(t)), "mdi-plus"))
                .collect(),
        }],
        FdgTarget::Relation { .. } => vec![ActionGroup {
            actions: relations::ALL
                .iter()
                .map(|r| action(&connect_action_id(r.id), &titled(r.id), "mdi-ray-start-arrow"))
                .collect(),
        }],
    }
}

/// Whether `target`'s menu offers `action_id`: executing an action by id finds only what its target discovers.
pub fn offers(model: &FdgModel, target: &FdgTarget<'_>, action_id: &str) -> bool {
    discover(model, target)
        .iter()
        .flat_map(|g| &g.actions)
        .any(|a| a.id == action_id)
}

/// What running `action_id` on `target` comes to.
#[derive(Debug)]
pub enum Run {
    /// The command to run now.
    Command(FdgCommand),
    /// The question to ask first.
    Ask(Prompt),
    /// Why it cannot run.
    Refused(String),
}

fn input(title: &str, label: &str, initial: &str, confirm: &str) -> Prompt {
    Prompt::Input {
        title: title.to_owned(),
        icon: "mdi-pencil-outline".to_owned(),
        label: label.to_owned(),
        initial_value: initial.to_owned(),
        confirm_label: confirm.to_owned(),
    }
}

/// Runs `action_id` on `target`, with `mint` giving the id of what an add or a connect makes, for the body at
/// `body_path`.
pub fn run(
    model: &FdgModel,
    body_path: &Path,
    target: &FdgTarget<'_>,
    action_id: &str,
    mint: &mut dyn FnMut(&str) -> Result<String, String>,
) -> Run {
    if let Some(element_type) = action_id.strip_prefix(ADD_PREFIX) {
        let FdgTarget::Placement { x, y } = target else {
            return Run::Refused(ADD_BY_DROP.to_owned());
        };
        if !element_types::is_known(element_type) {
            return Run::Refused(ADD_BY_DROP.to_owned());
        }
        return match mint(element_type) {
            Ok(element_id) => Run::Command(FdgCommand::new(
                body_path,
                FdgEdit::Add {
                    element_type: element_type.to_owned(),
                    x: *x,
                    y: *y,
                    element_id,
                },
            )),
            Err(refusal) => Run::Refused(refusal),
        };
    }
    if let Some(relation) = action_id.strip_prefix(CONNECT_PREFIX) {
        let FdgTarget::Relation { from, to } = target else {
            return Run::Refused(CONNECT_BY_GESTURE.to_owned());
        };
        return match mint(relation) {
            Ok(connection_id) => Run::Command(FdgCommand::new(
                body_path,
                FdgEdit::Connect {
                    relation: relation.to_owned(),
                    from: (*from).to_owned(),
                    to: (*to).to_owned(),
                    connection_id,
                },
            )),
            Err(refusal) => Run::Refused(refusal),
        };
    }
    let FdgTarget::Item(id) = target else {
        return Run::Refused(format!("'{action_id}' does not apply to this selection."));
    };
    let element = model.element_of(id);
    match action_id {
        RENAME if element.is_some() => {
            let element = element.expect("matched");
            Run::Ask(if element.is_comment() {
                input("Edit text", "Text", &element.text, "Save")
            } else {
                input("Rename", "Name", &element.name, "Rename")
            })
        }
        RENAME_CONNECTION if model.connection_of(id).is_some() => {
            let connection = model.connection_of(id).expect("matched");
            Run::Ask(input("Rename", "Name", &connection.name, "Rename"))
        }
        REMOVE if element.is_some() => {
            let element = element.expect("matched");
            let going = model
                .connections
                .iter()
                .filter(|c| c.from == element.id || c.to == element.id)
                .count();
            let command = FdgCommand::new(
                body_path,
                FdgEdit::Remove {
                    element_id: element.id.clone(),
                },
            );
            if going == 0 {
                return Run::Command(command);
            }
            Run::Ask(Prompt::Confirm {
                title: "Remove".to_owned(),
                icon: "mdi-delete-outline".to_owned(),
                message: if going == 1 {
                    "Removing this element also removes the 1 connection to or from it.".to_owned()
                } else {
                    format!("Removing this element also removes the {going} connections to or from it.")
                },
                confirm_label: "Remove".to_owned(),
                danger: true,
            })
        }
        DISCONNECT if model.connection_of(id).is_some() => Run::Command(FdgCommand::new(
            body_path,
            FdgEdit::Disconnect {
                connection_id: (*id).to_owned(),
            },
        )),
        RENAME | REMOVE | RENAME_CONNECTION | DISCONNECT => Run::Refused(GONE.to_owned()),
        _ => Run::Refused(format!("'{action_id}' does not apply to this selection.")),
    }
}

/// The command the answer `value` to `action_id` on the element or connection `id` becomes, or none when the action
/// does not apply.
pub fn commit(model: &FdgModel, body_path: &Path, id: &str, action_id: &str, value: &str) -> Option<FdgCommand> {
    let is_element = model.element_of(id).is_some();
    let is_connection = model.connection_of(id).is_some();
    let edit = match action_id {
        RENAME if is_element => FdgEdit::Rename {
            element_id: id.to_owned(),
            value: value.to_owned(),
        },
        REMOVE if is_element => FdgEdit::Remove {
            element_id: id.to_owned(),
        },
        RENAME_CONNECTION if is_connection => FdgEdit::RenameConnection {
            connection_id: id.to_owned(),
            name: value.to_owned(),
        },
        _ => return None,
    };
    Some(FdgCommand::new(body_path, edit))
}
