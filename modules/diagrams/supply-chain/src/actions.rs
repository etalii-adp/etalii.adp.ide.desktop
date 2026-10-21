//! What can be done to a selected group, node or flow, to empty canvas and to a finished connect gesture, and the
//! command each answer becomes (etalii.adp spec 030, task T076).
//!
//! Ported from the standalone host's `src/diagrams/supply-chain/backend/EtAlii.Adp.Diagram.SupplyChain/
//! SupplyChainContextActionProvider.cs` at `8b6ccfc`, over its `DiagramActionProvider`, and its toolbox's
//! `SupplyChainStageIcons`. The ids are the client's (`supplyChainIds.ts`). An action that needs no input is a command
//! at once; a rename and a new group ask for text first, and removing a node with flows asks to confirm.

use std::path::Path;

use adp_context::{Action, ActionGroup, Prompt, Shortcut};

use crate::commands::{GONE, SupplyChainCommand, SupplyChainEdit, keys};
use crate::model::{SupplyChainModel, stages};

pub const RENAME: &str = "supply-chain.rename";
pub const INCREASE: &str = "supply-chain.increase";
pub const DECREASE: &str = "supply-chain.decrease";
pub const GROUP: &str = "supply-chain.group";
pub const REMOVE: &str = "supply-chain.remove";
pub const ARRANGE: &str = "supply-chain.arrange";
pub const CONNECT: &str = "supply-chain.connect";
pub const ADD_GROUP: &str = "supply-chain.add-group";
const ADD_PREFIX: &str = "supply-chain.add.";

/// What an arrange on an empty diagram answers.
pub const NOTHING_TO_ARRANGE: &str = "There is nothing to arrange until this diagram has a node.";

/// The add action for one stage.
pub fn add_action_id(stage: &str) -> String {
    format!("{ADD_PREFIX}{stage}")
}

/// A group's icon, wherever a group is offered.
pub const GROUP_ICON: &str = "mdi-group";

/// The icon `stage` is shown with wherever it is offered, or a plain plus for a stage this module does not know.
pub fn stage_icon(stage: &str) -> &'static str {
    match stages::normalize(stage) {
        stages::SOURCE => "mdi-tray-arrow-up",
        stages::PROCESSOR => "mdi-cog-outline",
        stages::PRODUCER => "mdi-factory",
        stages::INTEGRATOR => "mdi-puzzle-outline",
        stages::HUB => "mdi-hub-outline",
        stages::OUTLET => "mdi-storefront-outline",
        stages::CONSUMER => "mdi-account-outline",
        _ => "mdi-plus",
    }
}

/// What an action is asked of.
#[derive(Debug, Clone, PartialEq)]
pub enum Subject {
    /// An entry, by its id.
    Entry(String),
    /// A drop or a menu on empty canvas, at a point (`new:x,y`).
    Placement(f64, f64),
    /// A finished connect gesture (`rel:a->b`).
    Relation(String, String),
}

impl Subject {
    fn id(&self) -> &str {
        match self {
            Subject::Entry(id) => id,
            _ => "",
        }
    }
}

fn action(id: &str, label: &str, icon: &str) -> Action {
    Action::new(id, label).with_icon(icon)
}

fn group_of(actions: Vec<Action>) -> ActionGroup {
    ActionGroup { actions }
}

/// The menu of `subject`.
pub fn discover(model: &SupplyChainModel, subject: &Subject) -> Vec<ActionGroup> {
    let arrange = {
        let a = action(ARRANGE, "Arrange diagram", "mdi-sitemap-outline");
        if model.nodes.is_empty() {
            a.unavailable(NOTHING_TO_ARRANGE)
        } else {
            a
        }
    };
    match subject {
        Subject::Entry(id) => {
            if let Some(node) = model.node_of(id) {
                let at_zero = node.quantity.unwrap_or(0.0) <= 0.0;
                let mut decrease =
                    action(DECREASE, "Decrease quantity", "mdi-minus-circle-outline").with_shortcut(Shortcut::key("-"));
                if at_zero {
                    decrease = decrease.unavailable("The quantity is already zero.");
                }
                return vec![
                    group_of(vec![
                        action(INCREASE, "Increase quantity", "mdi-plus-circle-outline")
                            .with_shortcut(Shortcut::key("+")),
                        decrease,
                    ]),
                    group_of(vec![
                        action(RENAME, "Rename…", "mdi-pencil-outline").with_shortcut(Shortcut::key("F2")),
                        action(GROUP, "Put in a new group…", "mdi-group"),
                        action(REMOVE, "Remove", "mdi-delete-outline").with_shortcut(Shortcut::key("Delete")),
                    ]),
                    group_of(vec![arrange]),
                ];
            }
            if let Some(flow) = model.flow_of(id) {
                let at_zero = flow.volume.unwrap_or(0.0) <= 0.0;
                let mut decrease =
                    action(DECREASE, "Decrease volume", "mdi-minus-circle-outline").with_shortcut(Shortcut::key("-"));
                if at_zero {
                    decrease = decrease.unavailable("The volume is already zero.");
                }
                return vec![
                    group_of(vec![
                        action(INCREASE, "Increase volume", "mdi-plus-circle-outline")
                            .with_shortcut(Shortcut::key("+")),
                        decrease,
                    ]),
                    group_of(vec![
                        action(RENAME, "Rename product…", "mdi-pencil-outline").with_shortcut(Shortcut::key("F2")),
                        action(REMOVE, "Remove flow", "mdi-vector-polyline-remove")
                            .with_shortcut(Shortcut::key("Delete")),
                    ]),
                ];
            }
            if model.group_of(id).is_some() {
                return vec![
                    group_of(vec![
                        action(RENAME, "Rename…", "mdi-pencil-outline").with_shortcut(Shortcut::key("F2")),
                        action(REMOVE, "Remove group", "mdi-ungroup").with_shortcut(Shortcut::key("Delete")),
                    ]),
                    group_of(vec![arrange]),
                ];
            }
            Vec::new()
        }
        // A drop, a background menu and a finished gesture each discover what may be executed against them.
        Subject::Placement(..) => vec![
            group_of(
                stages::ALL
                    .iter()
                    .map(|stage| {
                        action(
                            &add_action_id(stage),
                            &format!("Add {} here", stages::display(stage).to_lowercase()),
                            stage_icon(stage),
                        )
                    })
                    .collect(),
            ),
            group_of(vec![action(ADD_GROUP, "Add group here", GROUP_ICON)]),
            group_of(vec![arrange]),
        ],
        Subject::Relation(..) => vec![group_of(vec![action(CONNECT, "Add flow", "mdi-arrow-right-thin")])],
    }
}

/// What running an action comes to.
#[derive(Debug, Clone, PartialEq)]
pub enum Run {
    /// The command to run on the project's history.
    Command(SupplyChainCommand),
    /// What to ask first.
    Ask(Prompt),
    /// Nothing to do.
    Completed,
}

fn input(title: &str, icon: &str, label: &str, initial: &str, confirm: &str) -> Prompt {
    Prompt::Input {
        title: title.to_owned(),
        icon: icon.to_owned(),
        label: label.to_owned(),
        initial_value: initial.to_owned(),
        confirm_label: confirm.to_owned(),
    }
}

/// Runs `action_id` on `subject` (`RunAsync`); `mint` gives the id of what an add creates, of the type it names.
pub fn run(
    model: &SupplyChainModel,
    body_path: &Path,
    subject: &Subject,
    action_id: &str,
    mint: &dyn Fn(&str) -> Result<String, String>,
) -> Result<Run, String> {
    let command = |edit| Ok(Run::Command(SupplyChainCommand::new(body_path, edit)));
    let id = subject.id();
    let (node, flow, group) = (model.node_of(id), model.flow_of(id), model.group_of(id));

    if let Some(stage) = action_id.strip_prefix(ADD_PREFIX) {
        return match subject {
            Subject::Placement(x, y) if stages::is_known(stage) => command(SupplyChainEdit::AddNode {
                stage: stage.to_owned(),
                x: *x,
                y: *y,
                id: mint(stages::normalize(stage))?,
            }),
            _ => Err("A node is added by dropping it where it belongs.".to_owned()),
        };
    }
    match action_id {
        ADD_GROUP => match subject {
            Subject::Placement(x, y) => command(SupplyChainEdit::AddGroup {
                x: *x,
                y: *y,
                id: mint("group")?,
            }),
            _ => Err("A group is added by dropping it where it belongs.".to_owned()),
        },
        CONNECT => match subject {
            Subject::Relation(from, to) => command(SupplyChainEdit::Connect {
                from: from.clone(),
                to: to.clone(),
                id: mint("flow")?,
            }),
            _ => Err("A flow is drawn from one node to another.".to_owned()),
        },
        ARRANGE => command(SupplyChainEdit::Arrange),
        INCREASE | DECREASE if node.is_some() || flow.is_some() => command(SupplyChainEdit::Step {
            entry_id: id.to_owned(),
            steps: if action_id == INCREASE { 1 } else { -1 },
        }),
        RENAME if node.is_some() => Ok(Run::Ask(input(
            "Rename",
            "mdi-pencil-outline",
            "Name",
            &node.expect("a node").name,
            "Rename",
        ))),
        RENAME if group.is_some() => Ok(Run::Ask(input(
            "Rename",
            "mdi-pencil-outline",
            "Name",
            &group.expect("a group").name,
            "Rename",
        ))),
        RENAME if flow.is_some() => Ok(Run::Ask(input(
            "Rename product",
            "mdi-pencil-outline",
            "Product",
            &flow.expect("a flow").product,
            "Rename",
        ))),
        // Not in place: the value is a new group's name, which is no label on the canvas yet.
        GROUP if node.is_some() => Ok(Run::Ask(input(
            "Put in a new group",
            "mdi-group",
            "Group name",
            "",
            "Create group",
        ))),
        REMOVE if node.is_some() => {
            let going = model.flows.iter().filter(|f| f.from == id || f.to == id).count();
            if going == 0 {
                return command(SupplyChainEdit::Remove {
                    entry_id: id.to_owned(),
                });
            }
            Ok(Run::Ask(Prompt::Confirm {
                title: "Remove".to_owned(),
                icon: "mdi-delete-outline".to_owned(),
                message: if going == 1 {
                    "Removing this node also removes the 1 flow to or from it.".to_owned()
                } else {
                    format!("Removing this node also removes the {going} flows to or from it.")
                },
                confirm_label: "Remove".to_owned(),
                danger: true,
            }))
        }
        // A group's members stay where they are, ungrouped, so nothing is lost to confirm.
        REMOVE if flow.is_some() || group.is_some() => command(SupplyChainEdit::Remove {
            entry_id: id.to_owned(),
        }),
        RENAME | INCREASE | DECREASE | GROUP | REMOVE => Err(GONE.to_owned()),
        _ => Ok(Run::Completed),
    }
}

/// Judges an answer as it is typed (`Validate`): only a new group's name is refused blank.
pub fn validate(action_id: &str, value: &str) -> Result<(), String> {
    if action_id == GROUP && value.trim().is_empty() {
        return Err("A group needs a name.".to_owned());
    }
    Ok(())
}

/// The command the answer `value` to `action_id` on `subject` becomes (`CommitCommand`), or why there is none.
pub fn commit(
    model: &SupplyChainModel,
    body_path: &Path,
    subject: &Subject,
    action_id: &str,
    value: &str,
    mint: &dyn Fn(&str) -> Result<String, String>,
) -> Result<SupplyChainCommand, String> {
    let id = subject.id();
    let set = |key: &str| SupplyChainEdit::SetProperty {
        entry_id: id.to_owned(),
        key: key.to_owned(),
        value: value.to_owned(),
    };
    let edit = match action_id {
        RENAME if model.flow_of(id).is_some() => set(keys::PRODUCT),
        RENAME if model.node_of(id).is_some() || model.group_of(id).is_some() => set(keys::NAME),
        GROUP if model.node_of(id).is_some() => SupplyChainEdit::Group {
            node_id: id.to_owned(),
            name: value.to_owned(),
            id: mint("group")?,
        },
        REMOVE if model.node_of(id).is_some() => SupplyChainEdit::Remove {
            entry_id: id.to_owned(),
        },
        _ => return Err(format!("'{action_id}' does not apply to this selection.")),
    };
    Ok(SupplyChainCommand::new(body_path, edit))
}
