//! What can be done to an activity file from the canvas, and the command each answer becomes (etalii.adp spec 030,
//! task T075): the menu the definition derives for an element, a row or a relation, and the gestures the canvas makes
//! on its own - a drop, a drawn line, a fold, the archived switch.
//!
//! Ported from the standalone host's `src/diagrams/agent-activity-diagram/backend/EtAlii.Adp.Diagram.AgentActivityDiagram/
//! AadContextActionProvider.cs` at `8b6ccfc`. **Three entries of the derived menu are answered differently from how the
//! definition words them**, each because of where the host does the work: *Lock position* is offered only on an
//! element that is not locked and is answered by the canvas, which alone knows where the layout drew it (it reports
//! that position as a move, and a move locks); *Unlock position* is offered on a locked element only; and the three
//! *Open link* operations are left out, a link being opened by its symbol on the canvas.

use std::path::Path;

use adp_context::{Action, ActionGroup, Prompt};
use adp_diagram::DislDiagramDefinition;
use adp_disl::DerivedMenuEntry;
use adp_disl::behavior::deletion;

use crate::commands::{AadCommand, AadEdit};
use crate::definition::{self, AadDefinition, GONE};
use crate::store::AadDocument;

/// What precedes a toolbox tool's id in the action a drop of it executes, as the definition's `x-aad.tools` names it.
pub const ADD_PREFIX: &str = "add-";
pub const CONNECT: &str = "connect";
pub const RENAME: &str = "editLabel";
pub const ADD_ROW: &str = "create-child";
pub const REMOVE: &str = "delete";
pub const PIN: &str = "pin";
pub const UNPIN: &str = "unpin";
pub const UNPIN_ALL: &str = "unpinAll";
/// What precedes a task status in the action that sets it.
pub const SET_TASK_STATUS_PREFIX: &str = "setTaskStatus:";
const SET_TASK_STATUS: &str = "setTaskStatus";
pub const COLLAPSE_GROUP: &str = "collapse-group";
pub const EXPAND_GROUP: &str = "expand-group";
pub const SHOW_ARCHIVED: &str = "show-archived";
pub const HIDE_ARCHIVED: &str = "hide-archived";

/// The id of the element that carries the diagram's own settings. No file entry can have it: an id cannot hold a
/// space.
pub const VIEW_ID: &str = "diagram view";

/// What an action is asked of.
#[derive(Debug, Clone, PartialEq)]
pub enum Subject {
    /// An element, a row, a relation, the diagram's switch or a group, by its id.
    Entry(String),
    /// A drop or a menu on empty canvas, at a point (`new:x,y`).
    Placement(f64, f64),
    /// A finished connect gesture (`rel:a->b`).
    Relation(String, String),
}

/// The target of a fold: one group of one element, written `element#group`. An id in the file cannot hold a `#` the
/// binding would read, so the last one splits the two (`AadGroupTarget`).
pub fn group_target(element_id: &str, group: &str) -> String {
    format!("{element_id}#{group}")
}

/// The element and the group a fold's target names, or none for any other id.
pub fn parse_group_target(target: &str) -> Option<(&str, &str)> {
    let cut = target.rfind('#')?;
    (cut > 0 && cut < target.len() - 1).then(|| (&target[..cut], &target[cut + 1..]))
}

/// What running an action comes to.
pub enum Run {
    /// An edit to make.
    Command(AadCommand),
    /// A question to ask first.
    Ask(Prompt),
    /// Nothing to do here: the canvas answers it.
    Completed,
}

fn action(id: &str, label: &str, icon: &str) -> Action {
    Action::new(id, label).with_icon(icon)
}

/// The menu of `subject`.
pub fn discover(document: &AadDocument, subject: &Subject) -> Vec<ActionGroup> {
    if !document.is_usable() {
        return Vec::new();
    }
    let definition = definition::get();
    let id = match subject {
        Subject::Placement(..) => {
            let mut unpin_all = action(UNPIN_ALL, "Unlock all positions", "mdi-lock-open-variant-outline");
            if document.model.placements.is_empty() {
                unpin_all = unpin_all.unavailable("No position is locked.");
            }
            return vec![
                ActionGroup {
                    actions: definition
                        .definition
                        .toolbox()
                        .iter()
                        .map(|tool| {
                            action(
                                &tool.drop_action_id,
                                &format!("Add {}", tool.label.to_lowercase()),
                                &tool.icon,
                            )
                        })
                        .collect(),
                },
                ActionGroup {
                    actions: vec![unpin_all],
                },
            ];
        }
        Subject::Relation(..) => {
            return vec![ActionGroup {
                actions: vec![action(CONNECT, "Connect", "mdi-vector-line")],
            }];
        }
        Subject::Entry(id) => id,
    };
    if id == VIEW_ID {
        return vec![ActionGroup {
            actions: vec![
                action(SHOW_ARCHIVED, "Show archived", "mdi-archive-eye-outline"),
                action(HIDE_ARCHIVED, "Hide archived", "mdi-archive-off-outline"),
            ],
        }];
    }
    if parse_group_target(id).is_some() {
        return vec![ActionGroup {
            actions: vec![
                action(COLLAPSE_GROUP, "Fold", "mdi-chevron-right"),
                action(EXPAND_GROUP, "Unfold", "mdi-chevron-down"),
            ],
        }];
    }
    let Some(element) = AadDefinition::element_of(&document.body.typed().diagram, id) else {
        return Vec::new();
    };
    let pinned = document.model.placements.contains_key(id);
    definition
        .menus(&element)
        .iter()
        .map(|group| ActionGroup {
            actions: group.iter().filter_map(|entry| offered(entry, pinned)).collect(),
        })
        .filter(|group| !group.actions.is_empty())
        .collect()
}

/// A derived menu entry as this host offers it: none, itself, or a submenu of the values it can set.
fn offered(entry: &DerivedMenuEntry, pinned: bool) -> Option<Action> {
    match (entry.kind.as_str(), entry.operation.as_deref()) {
        ("operation", Some("openLink" | "openBranchLink" | "openFolderLink")) => None,
        ("pin", _) if pinned => None,
        ("unpin", _) if !pinned => None,
        (_, Some(SET_TASK_STATUS)) => {
            let mut submenu = DislDiagramDefinition::action(entry);
            // The operation's one parameter, asked as a submenu: one entry per status.
            submenu.items = vec![ActionGroup {
                actions: definition::get()
                    .task_status()
                    .members
                    .iter()
                    .map(|status| {
                        action(
                            &format!("{SET_TASK_STATUS_PREFIX}{}", status.name),
                            &status.label,
                            "mdi-circle-small",
                        )
                    })
                    .collect(),
            }];
            Some(submenu)
        }
        _ => Some(DislDiagramDefinition::action(entry)),
    }
}

/// Whether `action_id` is on `menu`, at any depth, and available.
pub fn is_offered(menu: &[ActionGroup], action_id: &str) -> bool {
    menu.iter().flat_map(|g| &g.actions).any(|a| {
        (a.id == action_id && a.is_available() && a.items.is_empty())
            || (a.is_available() && is_offered(&a.items, action_id))
    })
}

/// Runs `action_id` on `subject`, minting what it makes with `mint`.
pub fn run(
    document: &AadDocument,
    body_path: &Path,
    subject: &Subject,
    action_id: &str,
    mint: &dyn Fn(&str) -> Result<String, String>,
) -> Result<Run, String> {
    let definition = definition::get();
    let command = |edit| Ok(Run::Command(AadCommand::new(body_path, edit)));
    if let Some(tool) = action_id.strip_prefix(ADD_PREFIX) {
        // The drop said everything an add needs, so nothing is asked.
        return match subject {
            Subject::Placement(x, y) => command(AadEdit::AddElement {
                tool: tool.to_owned(),
                x: *x,
                y: *y,
                id: mint(tool)?,
            }),
            _ => Err("An element is added by dropping it where it should be.".to_owned()),
        };
    }
    match (action_id, subject) {
        (CONNECT, Subject::Relation(from, to)) => {
            return command(AadEdit::Connect {
                from: from.clone(),
                to: to.clone(),
            });
        }
        (CONNECT, _) => return Err("A relation is drawn from one element to another.".to_owned()),
        (UNPIN_ALL, _) => return command(AadEdit::Unpin { element_id: None }),
        (SHOW_ARCHIVED | HIDE_ARCHIVED, _) => {
            return command(AadEdit::SetShowArchived {
                show: action_id == SHOW_ARCHIVED,
            });
        }
        (COLLAPSE_GROUP | EXPAND_GROUP, Subject::Entry(id)) => {
            return match parse_group_target(id) {
                Some((owner, group)) => command(AadEdit::SetGroup {
                    element_id: owner.to_owned(),
                    group: group.to_owned(),
                    collapsed: action_id == COLLAPSE_GROUP,
                }),
                None => Err("A group is folded on the element that lists it.".to_owned()),
            };
        }
        _ => {}
    }
    let Subject::Entry(id) = subject else {
        return Ok(Run::Completed);
    };
    let element = AadDefinition::element_of(&document.body.typed().diagram, id);
    match (action_id, element) {
        // An element is renamed over its own label; a row has no label to edit in place, so its title is asked for.
        (RENAME, Some(element)) if !element.is_relation() => Ok(Run::Ask(Prompt::Input {
            title: "Rename".to_owned(),
            icon: "mdi-rename-outline".to_owned(),
            label: "Name".to_owned(),
            initial_value: definition.name_of(&element),
            confirm_label: "Rename".to_owned(),
        })),
        (ADD_ROW, Some(_)) => command(AadEdit::AddRow {
            parent_id: id.clone(),
            id: mint("row")?,
        }),
        (UNPIN, Some(_)) => command(AadEdit::Unpin {
            element_id: Some(id.clone()),
        }),
        (status, Some(_)) if status.starts_with(SET_TASK_STATUS_PREFIX) => command(AadEdit::SetTaskStatus {
            task_id: id.clone(),
            status: status[SET_TASK_STATUS_PREFIX.len()..].to_owned(),
        }),
        (REMOVE, Some(element)) if element.is_relation() => command(AadEdit::Disconnect {
            relation_id: id.clone(),
        }),
        // The definition's confirmation, asked before anything goes; with none, no ceremony.
        (REMOVE, Some(element)) => {
            match deletion::confirmation(definition.specification(), &element, None, Some(&definition.env())) {
                Some(confirmation) => Ok(Run::Ask(Prompt::Confirm {
                    title: confirmation.title,
                    icon: "mdi-delete-outline".to_owned(),
                    message: confirmation.message,
                    confirm_label: confirmation.confirm_label,
                    danger: confirmation.danger,
                })),
                None => command(AadEdit::Remove { element_id: id.clone() }),
            }
        }
        (RENAME | ADD_ROW | UNPIN | REMOVE, _) => Err(GONE.to_owned()),
        _ => Ok(Run::Completed),
    }
}

/// Checks an answer before it is committed: the one value asked for is a name, and the only wrong one is an empty
/// one.
pub fn validate(action_id: &str, value: &str) -> Result<(), String> {
    if action_id == RENAME && value.trim().is_empty() {
        return Err("It needs a name.".to_owned());
    }
    Ok(())
}

/// The command an answered prompt becomes.
pub fn commit(
    document: &AadDocument,
    body_path: &Path,
    subject: &Subject,
    action_id: &str,
    value: &str,
) -> Result<AadCommand, String> {
    let Subject::Entry(id) = subject else {
        return Err(format!("'{action_id}' does not apply to this selection."));
    };
    let definition = definition::get();
    let element = AadDefinition::element_of(&document.body.typed().diagram, id).filter(|e| !e.is_relation());
    match (action_id, element) {
        (RENAME, Some(element)) => {
            let attribute = definition.label_attribute(&element).ok_or_else(|| GONE.to_owned())?;
            Ok(AadCommand::new(
                body_path,
                AadEdit::SetAttribute {
                    element_id: id.clone(),
                    attribute,
                    value: value.to_owned(),
                },
            ))
        }
        (REMOVE, Some(_)) => Ok(AadCommand::new(body_path, AadEdit::Remove { element_id: id.clone() })),
        _ => Err(format!("'{action_id}' does not apply to this selection.")),
    }
}
