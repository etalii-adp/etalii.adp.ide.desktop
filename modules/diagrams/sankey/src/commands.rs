//! Every edit of a `.skv` document, as a command on the project's history (etalii.adp spec 030, task T077).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/sankey/backend/EtAlii.Adp.Diagram.Sankey/Commands/`
//! (one record and one handler per edit, and `SankeyEditHandler.cs`) and the core's `DocumentEditRun.cs` and
//! `RestoreDocumentCommand.cs`. A command names what it changes by the body's path and the entry's id, never by a live
//! object; it is run on a copy of the body's lines and the model read from them, the copy is written whole, and its
//! undo is the document's whole text as it was, with the command itself as the redo, so an undo gives back the
//! original bytes however much moved.

use std::path::{Path, PathBuf};

use adp_diagram::DiagramDocuments;
use adp_history::{Applied, Command, Refusal};

use crate::arrangement;
use crate::format::{self, PALETTE};
use crate::layout::{self, SankeyLayout, round};
use crate::lines::{LineDocument, LineRange, dsl_number, parse_int, parse_number};
use crate::model::{SankeyFlow, SankeyModel, SankeyNode, geometry, keys};
use crate::parser::{self, THICKNESS_KEY};
use crate::store;
use crate::writer;

/// The refusal for an entry that is not there, in one wording.
pub const GONE: &str = "That is no longer in this diagram.";

/// What a node's value row answers: it is what flows through it.
pub const NODE_VALUE: &str = "A node's value is what flows through it; change a flow's value instead.";

/// One edit, as plain data a history can hold.
#[derive(Debug, Clone, PartialEq)]
pub enum SankeyEdit {
    /// Adds a node where it was dropped: in the column nearest `x`, above the first node there whose middle is below
    /// `y`, with the id `node_id`.
    AddNode { x: f64, y: f64, node_id: String },
    /// Adds a flow from `from` to `to`.
    Connect { from: String, to: String },
    /// Removes a node with its flows, or a flow.
    Remove { entry_id: String },
    /// Moves a node's entry directly before `anchor_id`, or directly after it when `after`.
    MoveNode {
        node_id: String,
        anchor_id: String,
        after: bool,
    },
    /// Sets one key (one of [`keys`]) of one entry.
    SetProperty {
        entry_id: String,
        key: String,
        value: String,
    },
    /// Adds `steps` steps to a flow's value, or takes them away.
    StepValue { flow_id: String, steps: i32 },
    /// Draws every bar and band thicker (`steps` above zero) or thinner, by one factor per step.
    ScaleThickness { steps: i32 },
    /// Reorders the nodes of every column so their bands cross least, as one undo.
    Arrange,
}

/// An edit of the body at `body_path`.
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyCommand {
    pub body_path: PathBuf,
    pub edit: SankeyEdit,
}

impl SankeyCommand {
    pub fn new(body_path: &Path, edit: SankeyEdit) -> Self {
        SankeyCommand {
            body_path: body_path.to_path_buf(),
            edit,
        }
    }
}

/// What an edit of the copy came to.
enum Outcome {
    Applied,
    Refused(String),
}

fn refused(reason: impl Into<String>) -> Outcome {
    Outcome::Refused(reason.into())
}

impl Command<DiagramDocuments> for SankeyCommand {
    fn label(&self) -> String {
        match &self.edit {
            SankeyEdit::AddNode { .. } => "Add node",
            SankeyEdit::Connect { .. } => "Add flow",
            SankeyEdit::Remove { .. } => "Remove",
            SankeyEdit::MoveNode { .. } => "Move node",
            SankeyEdit::SetProperty { .. } => "Change property",
            SankeyEdit::StepValue { .. } => "Change value",
            SankeyEdit::ScaleThickness { .. } => "Change thickness",
            SankeyEdit::Arrange => "Arrange diagram",
        }
        .to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        // What is refused before the document is read.
        match &self.edit {
            SankeyEdit::SetProperty { key, value, .. } => {
                if let Err(refusal) = read_value(key, value) {
                    return Err(Refusal::new(refusal));
                }
            }
            SankeyEdit::StepValue { steps: 0, .. } | SankeyEdit::ScaleThickness { steps: 0 } => {
                return Err(Refusal::new("A step of nothing changes nothing."));
            }
            _ => {}
        }
        let document = store::get_or_load(documents, &self.body_path);
        if let Some(reason) = &document.unreadable {
            return Err(Refusal::new(format!(
                "{} could not be read, so nothing can be edited until it can: {reason}",
                self.body_path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            )));
        }
        let before = document.text.clone();
        let mut copy = LineDocument::parse(&before);
        let model = parser::parse(&copy);
        match apply_edit(&mut copy, &model, &self.edit) {
            Outcome::Refused(reason) => Err(Refusal::new(reason)),
            Outcome::Applied => {
                store::save(documents, &self.body_path, &copy.text()).map_err(Refusal::new)?;
                Ok(Applied {
                    inverse: Some(Box::new(RestoreSankeyDocument {
                        body_path: self.body_path.clone(),
                        text: before,
                        redo: self.clone(),
                    })),
                    warning: String::new(),
                })
            }
        }
    }
}

/// The undo of every edit: the document's whole text as it was, and the edit to run again as its redo.
#[derive(Debug, Clone)]
pub struct RestoreSankeyDocument {
    pub body_path: PathBuf,
    pub text: String,
    pub redo: SankeyCommand,
}

impl Command<DiagramDocuments> for RestoreSankeyDocument {
    fn label(&self) -> String {
        self.redo.label()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        store::restore(documents, &self.body_path, &self.text).map_err(Refusal::new)?;
        Ok(Applied {
            inverse: Some(Box::new(self.redo.clone())),
            warning: String::new(),
        })
    }
}

fn apply_edit(document: &mut LineDocument, model: &SankeyModel, edit: &SankeyEdit) -> Outcome {
    match edit {
        SankeyEdit::AddNode { x, y, node_id } => add_node(document, model, *x, *y, node_id),
        SankeyEdit::Connect { from, to } => connect(document, model, from, to),
        SankeyEdit::Remove { entry_id } => {
            if let Some(node) = model.node_of(entry_id) {
                writer::remove_node(document, model, node);
                Outcome::Applied
            } else if let Some(flow) = model.flow_of(entry_id) {
                writer::remove(document, flow.range);
                Outcome::Applied
            } else {
                refused(GONE)
            }
        }
        SankeyEdit::MoveNode {
            node_id,
            anchor_id,
            after,
        } => match (model.node_of(node_id), model.node_of(anchor_id)) {
            (Some(node), Some(anchor)) => match writer::move_node(document, node, anchor, *after) {
                Ok(()) => Outcome::Applied,
                Err(reason) => refused(reason),
            },
            _ => refused(GONE),
        },
        SankeyEdit::SetProperty { entry_id, key, value } => set_property(document, model, entry_id, key, value),
        SankeyEdit::StepValue { flow_id, steps } => {
            let Some(flow) = model.flow_of(flow_id) else {
                return refused(if model.node_of(flow_id).is_some() {
                    NODE_VALUE
                } else {
                    GONE
                });
            };
            match stepped(flow.value, flow.step, *steps) {
                Some(value) => {
                    writer::set_number(document, flow.range, keys::VALUE, Some(value));
                    Outcome::Applied
                }
                None => refused("The value is already zero."),
            }
        }
        SankeyEdit::ScaleThickness { steps } => {
            let current = model.settings.thickness;
            let scaled = round(
                (current * geometry::THICKNESS_FACTOR.powf(f64::from(*steps)))
                    .clamp(geometry::MINIMUM_SCALE, geometry::MAXIMUM_SCALE),
                2,
            );
            if (scaled - current).abs() < TOLERANCE {
                return refused(if *steps > 0 {
                    "The bands are already as thick as this diagram draws them."
                } else {
                    "The bands are already as thin as this diagram draws them."
                });
            }
            writer::set_root_key(
                document,
                model.settings.thickness_line,
                THICKNESS_KEY,
                &dsl_number(scaled),
            );
            Outcome::Applied
        }
        SankeyEdit::Arrange => arrange(document, model),
    }
}

/// The standalone host's `double.Tolerance`.
const TOLERANCE: f64 = 0.000_001_f32 as f64;

/// `AddSankeyNodeCommandHandler`: its column is the one nearest the drop, written down because a node with no flows
/// yet would otherwise fall to the first; its place in the column is above the first node there whose middle is below
/// the drop.
fn add_node(document: &mut LineDocument, model: &SankeyModel, x: f64, y: f64, node_id: &str) -> Outcome {
    if model.is_taken(node_id) {
        return refused("That id is already used in this diagram.");
    }
    let layout = SankeyLayout::of(model);
    let column = layout::column_at(x);
    let below = layout
        .column_order
        .get(column)
        .and_then(|ids| ids.iter().find(|id| layout.boxes[*id].centre_y() > y));
    let node = SankeyNode {
        id: node_id.to_owned(),
        name: unique_name(model.nodes.iter().map(SankeyNode::label), "New node"),
        color: String::new(),
        note: String::new(),
        format: String::new(),
        column: (column > 0).then(|| column as i32 + 1),
        description: String::new(),
        range: LineRange::default(),
    };
    writer::add_node(document, model, &node, below.and_then(|id| model.node_of(id)));
    Outcome::Applied
}

/// `UniqueName.Of`: `name`, or it with the lowest number from 2 that makes it unique among `taken`.
pub fn unique_name<'a>(taken: impl Iterator<Item = &'a str>, name: &str) -> String {
    let names: std::collections::HashSet<&str> = taken.collect();
    if !names.contains(name) {
        return name.to_owned();
    }
    (2..)
        .map(|number| format!("{name} {number}"))
        .find(|candidate| !names.contains(candidate.as_str()))
        .expect("a free number")
}

/// `ConnectSankeyNodesCommandHandler`: a flow between two nodes, refusing one to itself or one that already exists;
/// its value is what the source has not yet passed on, or 1 when nothing is left over.
fn connect(document: &mut LineDocument, model: &SankeyModel, from: &str, to: &str) -> Outcome {
    if model.node_of(from).is_none() || model.node_of(to).is_none() {
        return refused("A flow runs from one node to another.");
    }
    if from == to {
        return refused("A node does not flow into itself.");
    }
    if model.is_taken(&SankeyFlow::id_of(from, to)) || model.flows.iter().any(|flow| flow.from == from && flow.to == to)
    {
        return refused("These two are already joined by a flow in this direction; change its value instead.");
    }
    let flow = SankeyFlow {
        explicit_id: String::new(),
        from: from.to_owned(),
        to: to.to_owned(),
        value: Some(remainder(model, from)),
        step: None,
        color: String::new(),
        description: String::new(),
        range: LineRange::default(),
    };
    writer::add_flow(document, model, &flow);
    Outcome::Applied
}

/// What flows into a node and has not flowed out again, or 1 when nothing is left over.
fn remainder(model: &SankeyModel, id: &str) -> f64 {
    let inflow = model
        .flows
        .iter()
        .filter(|flow| flow.to == id)
        .fold(0.0, |sum, flow| sum + flow.amount());
    let outflow = model
        .flows
        .iter()
        .filter(|flow| flow.from == id)
        .fold(0.0, |sum, flow| sum + flow.amount());
    let left = round(inflow - outflow, 4);
    if left > 0.0 { left } else { 1.0 }
}

/// What a value of `key` reads as, checked before the document is read (`SetSankeyPropertyCommandHandler.Read`): the
/// trimmed text, the number for a value or a step, the column; or why it cannot be one.
fn read_value(key: &str, value: &str) -> Result<(String, Option<f64>, Option<i32>), String> {
    let value = value.trim().to_owned();
    let mut number = None;
    let mut column = None;
    if (key == keys::VALUE || key == keys::STEP) && !value.is_empty() {
        let parsed = parse_number(&value)
            .filter(|parsed| parsed.is_finite() && *parsed >= 0.0)
            .ok_or_else(|| format!("'{value}' is not an amount this diagram can hold: a number of zero or more."))?;
        if key == keys::STEP && parsed == 0.0 {
            return Err("A step of zero would make + and − do nothing.".to_owned());
        }
        number = Some(parsed);
    }
    if key == keys::COLOR && !format::is_known(&value) {
        return Err(format!(
            "`{value}` is not a colour: one of {}, or #rrggbb.",
            PALETTE.join(", ")
        ));
    }
    if key == keys::COLUMN && !value.is_empty() {
        let parsed = parse_int(&value).filter(|parsed| *parsed >= 1).ok_or_else(|| {
            format!("'{value}' is not a column: a whole number from 1, or nothing to let the flows decide.")
        })?;
        column = Some(parsed);
    }
    Ok((value, number, column))
}

fn set_property(document: &mut LineDocument, model: &SankeyModel, entry_id: &str, key: &str, value: &str) -> Outcome {
    let Ok((value, number, column)) = read_value(key, value) else {
        return refused(GONE);
    };
    let unknown = || refused(format!("`{key}` cannot be set on this entry."));
    if let Some(node) = model.node_of(entry_id) {
        return match key {
            keys::NAME if value.is_empty() => refused("A node needs a name."),
            keys::NAME => {
                writer::set_text(document, node.range, key, &value, false);
                Outcome::Applied
            }
            keys::COLOR | keys::NOTE | keys::FORMAT | keys::DESCRIPTION => {
                writer::set_text(document, node.range, key, &value, true);
                Outcome::Applied
            }
            keys::COLUMN => {
                writer::set_number(document, node.range, key, column.map(f64::from));
                Outcome::Applied
            }
            _ => unknown(),
        };
    }
    if let Some(flow) = model.flow_of(entry_id) {
        return match key {
            keys::COLOR | keys::DESCRIPTION => {
                writer::set_text(document, flow.range, key, &value, true);
                Outcome::Applied
            }
            keys::VALUE | keys::STEP => {
                writer::set_number(document, flow.range, key, number);
                Outcome::Applied
            }
            _ => unknown(),
        };
    }
    refused(GONE)
}

/// What one step adds to a flow that states none: a tenth of the value's own order of magnitude, so a flow of 3.3
/// steps by 0.1 and one of 1,302 by 100.
pub fn default_step(value: Option<f64>) -> f64 {
    let magnitude = value.unwrap_or(0.0).abs();
    if magnitude < 1.0 {
        return 0.1;
    }
    let step = 10f64.powf(magnitude.log10().floor() - 1.0);
    if step >= 0.1 { step } else { 0.1 }
}

/// The new value, or none when it is already zero and asked to go lower; rounded to the precision the writer keeps,
/// so ten steps of 0.1 come back to a whole number.
pub fn stepped(current: Option<f64>, step: Option<f64>, steps: i32) -> Option<f64> {
    let from = current.unwrap_or(0.0).max(0.0);
    if from <= 0.0 && steps < 0 {
        return None;
    }
    let step = match step {
        Some(step) if step > 0.0 => step,
        _ => default_step(current),
    };
    Some(round(from + f64::from(steps) * step, 4).max(0.0))
}

/// `ArrangeSankeyCommandHandler`: the nodes moved among the others one entry at a time, each move read again, so
/// every move splices lines the parser just found.
fn arrange(document: &mut LineDocument, model: &SankeyModel) -> Outcome {
    let target = arrangement::document_order_of(model);
    if target.is_empty() {
        return refused("There is nothing to arrange until this diagram has a node.");
    }
    let wanted: std::collections::HashSet<&str> = target.iter().map(String::as_str).collect();
    let mut moved = false;
    for (index, id) in target.iter().enumerate() {
        let parsed = parser::parse(document);
        let mut taken = std::collections::HashSet::new();
        let current: Vec<&SankeyNode> = parsed
            .nodes
            .iter()
            .filter(|node| wanted.contains(node.id.as_str()) && taken.insert(node.id.clone()))
            .collect();
        if current[index].id == *id {
            continue;
        }
        let node = parsed.node_of(id).expect("a node of the arrangement").clone();
        let anchor = current[index].clone();
        if writer::move_node(document, &node, &anchor, false).is_err() {
            return refused("A node cannot be moved next to itself.");
        }
        moved = true;
    }
    if moved {
        Outcome::Applied
    } else {
        refused("This diagram is already arranged.")
    }
}
