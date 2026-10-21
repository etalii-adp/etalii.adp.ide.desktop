//! What can be done to a selected node or flow, to the empty canvas and to a finished connect gesture, offered as data
//! and each answered by a command on the project's history; the property rows of a node or a flow; and the palette
//! (etalii.adp spec 030, task T077).
//!
//! Ported from the standalone host at `8b6ccfc`: `SankeyContextActionProvider.cs`, `SankeyContextPropertyProvider.cs`
//! and `SankeyToolboxProvider.cs`, over the core's `DiagramActionProvider` and `DiagramPropertyProvider`, with the
//! context service's rule that an action not offered, or offered disabled, is not run. The ids are the client's
//! (`client/sankeyIds.ts`). Every row edits through the one property command, so a value typed in the grid is checked
//! by the same rules as one stepped on the canvas. A colour is offered twice, as a palette word to pick and as a custom
//! `#rrggbb` to type, both writing the one `color` key; a node's value is shown and not edited, since it is what flows
//! through it.

use std::path::Path;

use adp_context::{Action, ActionGroup, Prompt, PropertyEditor, PropertyRow, Shortcut};
use adp_diagram::{ActionTarget, ToolboxItem};

use crate::commands::{self, NODE_VALUE, SankeyCommand, SankeyEdit};
use crate::format::{self, PALETTE};
use crate::layout::SankeyLayout;
use crate::lines::dsl_number;
use crate::model::{SankeyModel, geometry, keys};

pub const ADD: &str = "sankey.add";
pub const CONNECT: &str = "sankey.connect";
pub const INCREASE: &str = "sankey.increase";
pub const DECREASE: &str = "sankey.decrease";
pub const RENAME: &str = "sankey.rename";
pub const REMOVE: &str = "sankey.remove";
pub const THICKER: &str = "sankey.thicker";
pub const THINNER: &str = "sankey.thinner";
pub const ARRANGE: &str = "sankey.arrange";

/// What the context service answers for an action that is not offered, or offered disabled.
pub const NOT_AVAILABLE: &str = "That action is not available for this item.";

/// What a rename answered blank answers.
pub const NAME_REQUIRED: &str = "A node needs a name.";

fn action(id: &str, label: &str, icon: &str, shortcut: Option<&str>) -> Action {
    Action {
        icon: icon.to_owned(),
        shortcut: shortcut.map(Shortcut::key),
        ..Action::new(id, label)
    }
}

fn enabled_when(mut action: Action, enabled: bool, reason: &str) -> Action {
    if !enabled {
        action.unavailable_reason = reason.to_owned();
    }
    action
}

/// What applies to the whole diagram, offered wherever the reader is so it stays in the ribbon: arranging it, and
/// thicker and thinner, each disabled at its end of the range.
fn whole_diagram(model: &SankeyModel) -> ActionGroup {
    ActionGroup {
        actions: vec![
            enabled_when(
                action(ARRANGE, "Arrange diagram", "mdi-sitemap-outline", None),
                !model.nodes.is_empty(),
                "There is nothing to arrange until this diagram has a node.",
            ),
            enabled_when(
                action(THICKER, "Thicker bands", "mdi-arrow-expand-vertical", None),
                model.settings.thickness < geometry::MAXIMUM_SCALE,
                "The bands are already as thick as they go.",
            ),
            enabled_when(
                action(THINNER, "Thinner bands", "mdi-arrow-collapse-vertical", None),
                model.settings.thickness > geometry::MINIMUM_SCALE,
                "The bands are already as thin as they go.",
            ),
        ],
    }
}

/// The menu of `target`.
pub fn discover(model: &SankeyModel, target: &ActionTarget) -> Vec<ActionGroup> {
    match target {
        ActionTarget::Element(id) => {
            if model.node_of(id).is_some() {
                return vec![
                    ActionGroup {
                        actions: vec![
                            action(RENAME, "Rename…", "mdi-pencil-outline", Some("F2")),
                            action(REMOVE, "Remove", "mdi-delete-outline", Some("Delete")),
                        ],
                    },
                    whole_diagram(model),
                ];
            }
            if let Some(flow) = model.flow_of(id) {
                let at_zero = flow.value.unwrap_or(0.0) <= 0.0;
                return vec![
                    ActionGroup {
                        actions: vec![
                            action(INCREASE, "Increase value", "mdi-plus-circle-outline", Some("+")),
                            enabled_when(
                                action(DECREASE, "Decrease value", "mdi-minus-circle-outline", Some("-")),
                                !at_zero,
                                "The value is already zero.",
                            ),
                        ],
                    },
                    ActionGroup {
                        actions: vec![action(
                            REMOVE,
                            "Remove flow",
                            "mdi-vector-polyline-remove",
                            Some("Delete"),
                        )],
                    },
                    whole_diagram(model),
                ];
            }
            Vec::new()
        }
        ActionTarget::Canvas { .. } => vec![
            ActionGroup {
                actions: vec![action(ADD, "Add node here", "mdi-plus", None)],
            },
            whole_diagram(model),
        ],
        ActionTarget::Connection {
            source: Some(source),
            target: Some(end),
        } if !source.is_empty() && !end.is_empty() => vec![ActionGroup {
            actions: vec![action(CONNECT, "Add flow", "mdi-arrow-right-thin", None)],
        }],
        ActionTarget::Connection { .. } => Vec::new(),
    }
}

/// The offered action `action_id` of `target`, when it is offered and available.
fn offered(model: &SankeyModel, target: &ActionTarget, action_id: &str) -> bool {
    discover(model, target)
        .iter()
        .flat_map(|group| &group.actions)
        .any(|action| action.id == action_id && action.unavailable_reason.is_empty())
}

/// What running `action_id` on `target` comes to: a command to run, a question to ask first, or why it cannot.
pub enum Run {
    Command(SankeyCommand),
    Prompt(Prompt),
    Refused(String),
    /// Offered, and nothing to do.
    Nothing,
}

/// Runs action `action_id` on `target` of the body at `body_path`; `new_id` mints the id of a node an add makes.
pub fn run(
    model: &SankeyModel,
    body_path: &Path,
    target: &ActionTarget,
    action_id: &str,
    new_id: impl FnOnce() -> Result<String, String>,
) -> Run {
    if !offered(model, target, action_id) {
        return Run::Refused(NOT_AVAILABLE.to_owned());
    }
    let command = |edit| Run::Command(SankeyCommand::new(body_path, edit));
    let id = match target {
        ActionTarget::Element(id) => id.as_str(),
        _ => "",
    };
    let node = model.node_of(id);
    let flow = model.flow_of(id);
    match action_id {
        ADD => match target {
            ActionTarget::Canvas { x, y } => match new_id() {
                Ok(node_id) => command(SankeyEdit::AddNode { x: *x, y: *y, node_id }),
                Err(reason) => Run::Refused(reason),
            },
            _ => Run::Refused("A node is added by dropping it where it belongs.".to_owned()),
        },
        CONNECT => match target {
            ActionTarget::Connection {
                source: Some(from),
                target: Some(to),
            } => command(SankeyEdit::Connect {
                from: from.clone(),
                to: to.clone(),
            }),
            _ => Run::Refused("A flow is drawn from one node to another.".to_owned()),
        },
        ARRANGE => command(SankeyEdit::Arrange),
        THICKER | THINNER => command(SankeyEdit::ScaleThickness {
            steps: if action_id == THICKER { 1 } else { -1 },
        }),
        INCREASE | DECREASE if flow.is_some() => command(SankeyEdit::StepValue {
            flow_id: id.to_owned(),
            steps: if action_id == INCREASE { 1 } else { -1 },
        }),
        RENAME if node.is_some() => Run::Prompt(Prompt::Input {
            title: "Rename".to_owned(),
            icon: "mdi-pencil-outline".to_owned(),
            label: "Name".to_owned(),
            initial_value: node.map(|n| n.label().to_owned()).unwrap_or_default(),
            confirm_label: "Rename".to_owned(),
        }),
        REMOVE if node.is_some() => {
            let node = node.expect("checked");
            let going = model
                .flows
                .iter()
                .filter(|candidate| candidate.from == node.id || candidate.to == node.id)
                .count();
            if going == 0 {
                return command(SankeyEdit::Remove {
                    entry_id: id.to_owned(),
                });
            }
            Run::Prompt(Prompt::Confirm {
                title: "Remove".to_owned(),
                icon: "mdi-delete-outline".to_owned(),
                message: if going == 1 {
                    "Removing this node also removes the 1 flow to or from it.".to_owned()
                } else {
                    format!("Removing this node also removes the {going} flows to or from it.")
                },
                confirm_label: "Remove".to_owned(),
                danger: true,
            })
        }
        REMOVE if flow.is_some() => command(SankeyEdit::Remove {
            entry_id: id.to_owned(),
        }),
        RENAME | INCREASE | DECREASE | REMOVE => Run::Refused(commands::GONE.to_owned()),
        _ => Run::Nothing,
    }
}

/// The command an answer to the prompt of `action_id` on `target` becomes, or why it is refused.
pub fn commit(
    model: &SankeyModel,
    body_path: &Path,
    target: &ActionTarget,
    action_id: &str,
    value: &str,
) -> Result<SankeyCommand, String> {
    if action_id == RENAME && value.trim().is_empty() {
        return Err(NAME_REQUIRED.to_owned());
    }
    let id = match target {
        ActionTarget::Element(id) if model.node_of(id).is_some() => id,
        _ => return Err(format!("'{action_id}' does not apply to this selection.")),
    };
    match action_id {
        RENAME => Ok(SankeyCommand::new(
            body_path,
            SankeyEdit::SetProperty {
                entry_id: id.clone(),
                key: keys::NAME.to_owned(),
                value: value.to_owned(),
            },
        )),
        REMOVE => Ok(SankeyCommand::new(
            body_path,
            SankeyEdit::Remove { entry_id: id.clone() },
        )),
        _ => Err(format!("'{action_id}' does not apply to this selection.")),
    }
}

pub const NAME_PROPERTY: &str = "sankey.name";
pub const COLOR_PROPERTY: &str = "sankey.color";
pub const CUSTOM_COLOR_PROPERTY: &str = "sankey.custom-color";
pub const NOTE_PROPERTY: &str = "sankey.note";
pub const FORMAT_PROPERTY: &str = "sankey.format";
pub const COLUMN_PROPERTY: &str = "sankey.column";
pub const DESCRIPTION_PROPERTY: &str = "sankey.description";
pub const VALUE_PROPERTY: &str = "sankey.value";
pub const STEP_PROPERTY: &str = "sankey.step";

/// The colour choice that removes the entry's own colour.
pub const DEFAULT_COLOR: &str = "(default)";

/// The colour choice shown while a custom colour is in use.
pub const CUSTOM_COLOR: &str = "(custom)";

/// The column choice that lets the flows decide.
pub const AUTO_COLUMN: &str = "auto";

const IDENTITY_GROUP: &str = "Identity";
const LOOK_GROUP: &str = "Look";
const AMOUNT_GROUP: &str = "Amount";

fn row(id: &str, label: &str, value: &str, editor: PropertyEditor, group: &str) -> PropertyRow {
    PropertyRow {
        group: group.to_owned(),
        ..PropertyRow::new(id, label, editor, value)
    }
}

fn choice(id: &str, label: &str, value: &str, group: &str, candidates: Vec<String>) -> PropertyRow {
    PropertyRow {
        candidates,
        ..row(id, label, value, PropertyEditor::Choice, group)
    }
}

fn color_choice(color: &str) -> String {
    if color.is_empty() {
        DEFAULT_COLOR.to_owned()
    } else if format::is_hex(color) {
        CUSTOM_COLOR.to_owned()
    } else {
        color.to_owned()
    }
}

fn color_choices(color: &str) -> Vec<String> {
    std::iter::once(DEFAULT_COLOR)
        .chain(PALETTE)
        .chain(format::is_hex(color).then_some(CUSTOM_COLOR))
        .map(str::to_owned)
        .collect()
}

fn custom(color: &str) -> &str {
    if format::is_hex(color) { color } else { "" }
}

/// The rows of `element_id`, or none when it names nothing.
pub fn properties(model: &SankeyModel, element_id: &str) -> Vec<PropertyRow> {
    if let Some(node) = model.node_of(element_id) {
        let layout = SankeyLayout::of(model);
        let value = layout.values.get(&node.id).copied().unwrap_or(0.0);
        let columns = layout.column_order.len() as i32;
        let column_count = columns.max(node.column.unwrap_or(0)) + 1;
        return vec![
            row(NAME_PROPERTY, "Name", &node.name, PropertyEditor::Line, IDENTITY_GROUP),
            row(
                DESCRIPTION_PROPERTY,
                "Description",
                &node.description,
                PropertyEditor::Text,
                IDENTITY_GROUP,
            ),
            choice(
                COLOR_PROPERTY,
                "Colour",
                &color_choice(&node.color),
                LOOK_GROUP,
                color_choices(&node.color),
            ),
            row(
                CUSTOM_COLOR_PROPERTY,
                "Custom colour",
                custom(&node.color),
                PropertyEditor::Line,
                LOOK_GROUP,
            ),
            choice(
                COLUMN_PROPERTY,
                "Column",
                &node
                    .column
                    .map_or_else(|| AUTO_COLUMN.to_owned(), |column| dsl_number(f64::from(column))),
                LOOK_GROUP,
                std::iter::once(AUTO_COLUMN.to_owned())
                    .chain((1..=column_count).map(|column| dsl_number(f64::from(column))))
                    .collect(),
            ),
            PropertyRow {
                read_only_reason: NODE_VALUE.to_owned(),
                ..row(
                    VALUE_PROPERTY,
                    "Value",
                    &dsl_number(value),
                    PropertyEditor::Line,
                    AMOUNT_GROUP,
                )
            },
            row(NOTE_PROPERTY, "Note", &node.note, PropertyEditor::Line, AMOUNT_GROUP),
            row(
                FORMAT_PROPERTY,
                "Format",
                &node.format,
                PropertyEditor::Line,
                AMOUNT_GROUP,
            ),
        ];
    }
    if let Some(flow) = model.flow_of(element_id) {
        return vec![
            row(
                DESCRIPTION_PROPERTY,
                "Description",
                &flow.description,
                PropertyEditor::Text,
                IDENTITY_GROUP,
            ),
            choice(
                COLOR_PROPERTY,
                "Colour",
                &color_choice(&flow.color),
                LOOK_GROUP,
                color_choices(&flow.color),
            ),
            row(
                CUSTOM_COLOR_PROPERTY,
                "Custom colour",
                custom(&flow.color),
                PropertyEditor::Line,
                LOOK_GROUP,
            ),
            row(
                VALUE_PROPERTY,
                "Value",
                &flow.value.map(dsl_number).unwrap_or_default(),
                PropertyEditor::Line,
                AMOUNT_GROUP,
            ),
            row(
                STEP_PROPERTY,
                "Step",
                &dsl_number(flow.step.unwrap_or_else(|| commands::default_step(flow.value))),
                PropertyEditor::Line,
                AMOUNT_GROUP,
            ),
        ];
    }
    Vec::new()
}

/// The command a row edit becomes: none for an edit that changes nothing, or why it is refused.
pub fn change(
    body_path: &Path,
    element_id: &str,
    property_id: &str,
    value: &str,
) -> Result<Option<SankeyCommand>, String> {
    let key = match property_id {
        NAME_PROPERTY => keys::NAME,
        COLOR_PROPERTY | CUSTOM_COLOR_PROPERTY => keys::COLOR,
        NOTE_PROPERTY => keys::NOTE,
        FORMAT_PROPERTY => keys::FORMAT,
        COLUMN_PROPERTY => keys::COLUMN,
        DESCRIPTION_PROPERTY => keys::DESCRIPTION,
        VALUE_PROPERTY => keys::VALUE,
        STEP_PROPERTY => keys::STEP,
        _ => return Err(format!("'{property_id}' cannot be edited on this selection.")),
    };
    if property_id == COLOR_PROPERTY && value == CUSTOM_COLOR {
        // Picking the custom entry keeps the custom colour; a new one is typed in its own row.
        return Ok(None);
    }
    let value = match property_id {
        COLOR_PROPERTY if value == DEFAULT_COLOR => "",
        COLUMN_PROPERTY if value == AUTO_COLUMN => "",
        _ => value,
    };
    Ok(Some(SankeyCommand::new(
        body_path,
        SankeyEdit::SetProperty {
            entry_id: element_id.to_owned(),
            key: key.to_owned(),
            value: value.to_owned(),
        },
    )))
}

/// The palette: one node, whose drop runs the add. What a node stands for is the document's business.
pub fn toolbox() -> Vec<ToolboxItem> {
    vec![ToolboxItem {
        id: "sankey.toolbox.node".to_owned(),
        label: "Node".to_owned(),
        icon: "mdi-chart-sankey".to_owned(),
        description: "Something a quantity flows into, out of, or through. Drop it in the column it belongs to."
            .to_owned(),
        drop_action_id: ADD.to_owned(),
    }]
}
