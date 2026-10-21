//! Every edit of a `.supply` document, as a command on the project's history (etalii.adp spec 030, task T076).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/supply-chain/backend/EtAlii.Adp.Diagram.SupplyChain/
//! Commands/` (one handler per command, each checking its edit and splicing it through [`crate::writer`]) and the
//! core's `EtAlii.Adp.History/DocumentEditRun.cs` and `RestoreDocumentCommand.cs`. A command names what it changes by
//! the body's path and the entry's id, never by a live object, so it is plain data a history can hold. It is run on a
//! copy of the body's lines and that copy is written whole; its inverse puts back the text the body had before, and
//! redoes the edit itself, with the id it was given, when that is undone in turn. An id a command creates is minted
//! before the command is made, so a redo writes the same one.

use std::path::{Path, PathBuf};

use adp_diagram::DiagramDocuments;
use adp_history::{Applied, Command, Refusal};
use indexmap::IndexMap;

use crate::layout::SupplyChainLayout;
use crate::lines::{LineDocument, parse_number, round4};
use crate::model::{SupplyChainFlow, SupplyChainGroup, SupplyChainModel, SupplyChainNode, geometry, stages};
use crate::parser;
use crate::store;
use crate::writer;

/// The refusal of an entry that is not there, in one wording.
pub const GONE: &str = "That is no longer in this diagram.";

/// What an id already used answers.
pub const TAKEN: &str = "That id is already used in this diagram.";

/// The keys an edit may set, as the document spells them (`SupplyChainKeys`).
pub mod keys {
    pub const NAME: &str = "name";
    pub const DESCRIPTION: &str = "description";
    pub const TYPE: &str = "type";
    pub const GROUP: &str = "group";
    pub const QUANTITY: &str = "quantity";
    pub const UNIT: &str = "unit";
    pub const STEP: &str = "step";
    pub const PRODUCT: &str = "product";
    pub const VOLUME: &str = "volume";

    /// The keys that hold a number.
    pub const NUMBERS: [&str; 3] = [QUANTITY, STEP, VOLUME];
}

/// One edit, as its standalone command said it.
#[derive(Debug, Clone, PartialEq)]
pub enum SupplyChainEdit {
    /// A node of `stage` centred on (`x`, `y`) (`AddSupplyChainNodeCommand`).
    AddNode { stage: String, x: f64, y: f64, id: String },
    /// An empty group whose frame is centred on (`x`, `y`) (`AddSupplyChainGroupCommand`).
    AddGroup { x: f64, y: f64, id: String },
    /// A flow from `from` to `to` (`ConnectSupplyChainNodesCommand`).
    Connect { from: String, to: String, id: String },
    /// A group named `name` around one node (`GroupSupplyChainNodeCommand`).
    Group { node_id: String, name: String, id: String },
    /// A node or a group's frame moved so its top-left is (`x`, `y`) (`MoveSupplyChainEntryCommand`).
    Move { entry_id: String, x: f64, y: f64 },
    /// Nodes placed so each one's top-left is its point (`PlaceSupplyChainNodesCommand`).
    Place { places: Vec<(String, (f64, f64))> },
    /// A node with its flows, a flow, or a group with its members left ungrouped (`RemoveSupplyChainEntryCommand`).
    Remove { entry_id: String },
    /// The layered layout written into every node (`ArrangeSupplyChainCommand`).
    Arrange,
    /// One key of one entry (`SetSupplyChainPropertyCommand`).
    SetProperty {
        entry_id: String,
        key: String,
        value: String,
    },
    /// `steps` steps added to a node's quantity or a flow's volume, negative to take them away
    /// (`StepSupplyChainValueCommand`).
    Step { entry_id: String, steps: i32 },
}

/// One edit of one body.
#[derive(Debug, Clone, PartialEq)]
pub struct SupplyChainCommand {
    pub body_path: PathBuf,
    pub edit: SupplyChainEdit,
}

impl SupplyChainCommand {
    pub fn new(body_path: &Path, edit: SupplyChainEdit) -> Self {
        SupplyChainCommand {
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

impl Command<DiagramDocuments> for SupplyChainCommand {
    fn label(&self) -> String {
        match &self.edit {
            SupplyChainEdit::AddNode { .. } | SupplyChainEdit::AddGroup { .. } => "Add",
            SupplyChainEdit::Connect { .. } => "Connect",
            SupplyChainEdit::Group { .. } => "Group",
            SupplyChainEdit::Move { .. } | SupplyChainEdit::Place { .. } => "Move",
            SupplyChainEdit::Remove { .. } => "Remove",
            SupplyChainEdit::Arrange => "Arrange diagram",
            SupplyChainEdit::SetProperty { .. } => "Change",
            SupplyChainEdit::Step { .. } => "Step",
        }
        .to_owned()
    }

    /// The edit run (`DocumentEditRun.Run`): what the command refuses before it looks, the body loaded, the edit made
    /// on a copy of its lines and their reading, the copy written and read again, and the inverse recorded.
    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        if let Some(refusal) = refusal_before(&self.edit) {
            return Err(Refusal::new(refusal));
        }
        let document = store::get_or_load(documents, &self.body_path);
        if let Some(reason) = &document.unreadable {
            return Err(Refusal::new(format!(
                "{} could not be read, so nothing can be edited until it can: {reason}",
                file_name(&self.body_path)
            )));
        }
        let before = document.text.clone();
        let mut copy = LineDocument::parse(&before);
        let model = parser::parse(&copy);
        edit(&mut copy, &model, &self.edit).map_err(Refusal::new)?;
        store::save(documents, &self.body_path, &copy.text()).map_err(Refusal::new)?;
        Ok(Applied {
            inverse: Some(Box::new(RestoreSupplyChain {
                body_path: self.body_path.clone(),
                text: before,
                redo: self.clone(),
            })),
            warning: String::new(),
        })
    }
}

/// The undo of an edit: the body's text as it was, and the edit to make again when this is undone in turn
/// (`RestoreDocumentCommand`).
#[derive(Debug, Clone, PartialEq)]
pub struct RestoreSupplyChain {
    pub body_path: PathBuf,
    pub text: String,
    pub redo: SupplyChainCommand,
}

impl Command<DiagramDocuments> for RestoreSupplyChain {
    fn label(&self) -> String {
        self.redo.label()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        store::save(documents, &self.body_path, &self.text).map_err(Refusal::new)?;
        Ok(Applied {
            inverse: Some(Box::new(self.redo.clone())),
            warning: String::new(),
        })
    }
}

/// What a command refuses before it reads the body: the value of a property that cannot be one, a step of nothing,
/// a stage the notation has not, a group without a name.
fn refusal_before(edit: &SupplyChainEdit) -> Option<String> {
    match edit {
        SupplyChainEdit::AddNode { stage, .. } if !stages::is_known(stage) => {
            Some(format!("This notation has no `{stage}` stage."))
        }
        SupplyChainEdit::Step { steps: 0, .. } => Some("A step of nothing changes nothing.".to_owned()),
        SupplyChainEdit::Group { name, .. } if name.trim().is_empty() => Some("A group needs a name.".to_owned()),
        SupplyChainEdit::SetProperty { key, value, .. } => read_value(key, value).err(),
        _ => None,
    }
}

/// The trimmed value and, for a number key, the number it reads as; or why it cannot be set.
fn read_value(key: &str, value: &str) -> Result<(String, Option<f64>), String> {
    let value = value.trim().to_owned();
    if !keys::NUMBERS.contains(&key) || value.is_empty() {
        return Ok((value, None));
    }
    match parse_number(&value) {
        Some(parsed) if parsed.is_finite() && parsed >= 0.0 => {
            if key == keys::STEP && parsed == 0.0 {
                return Err("A step of zero would make + and − do nothing.".to_owned());
            }
            Ok((value, Some(parsed)))
        }
        _ => Err(format!(
            "'{value}' is not an amount this diagram can hold: a number of zero or more."
        )),
    }
}

/// The name `name` takes among `taken`: itself, else itself and the lowest number from 2 that is free (`UniqueName`).
pub fn unique_name<'a>(taken: impl Iterator<Item = &'a str>, name: &str) -> String {
    let names: std::collections::HashSet<&str> = taken.collect();
    if !names.contains(name) {
        return name.to_owned();
    }
    (2..)
        .map(|n| format!("{name} {n}"))
        .find(|candidate| !names.contains(candidate.as_str()))
        .expect("some number is free")
}

/// Makes `edit` on `document`, read as `model`.
pub fn edit(document: &mut LineDocument, model: &SupplyChainModel, edit: &SupplyChainEdit) -> Result<(), String> {
    match edit {
        SupplyChainEdit::AddNode { stage, x, y, id } => {
            if model.is_taken(id) {
                return Err(TAKEN.to_owned());
            }
            // The drop is the centre; the document holds the top-left.
            let name = unique_name(
                model.nodes.iter().map(|n| n.name.as_str()),
                &format!("New {}", stages::display(stage).to_lowercase()),
            );
            let node = SupplyChainNode {
                id: id.clone(),
                node_type: stages::normalize(stage).to_owned(),
                name,
                description: String::new(),
                group: String::new(),
                quantity: Some(0.0),
                unit: String::new(),
                step: None,
                x: Some(x - geometry::NODE_WIDTH / 2.0),
                y: Some(y - geometry::NODE_HEIGHT / 2.0),
                range: Default::default(),
            };
            writer::add_node(document, model, &node);
            Ok(())
        }
        SupplyChainEdit::AddGroup { x, y, id } => {
            if model.is_taken(id) {
                return Err(TAKEN.to_owned());
            }
            let group = SupplyChainGroup {
                id: id.clone(),
                name: unique_name(model.groups.iter().map(|g| g.name.as_str()), "New group"),
                description: String::new(),
                range: Default::default(),
                x: Some(x - geometry::EMPTY_GROUP_WIDTH / 2.0),
                y: Some(y - geometry::EMPTY_GROUP_HEIGHT / 2.0),
            };
            writer::add_group(document, model, &group);
            Ok(())
        }
        SupplyChainEdit::Connect { from, to, id } => {
            if model.node_of(from).is_none() || model.node_of(to).is_none() {
                return Err("A flow runs from one node to another.".to_owned());
            }
            if from == to {
                return Err("A node does not supply itself.".to_owned());
            }
            if model.flows.iter().any(|f| f.from == *from && f.to == *to) {
                return Err("These two are already connected by a flow in this direction.".to_owned());
            }
            if model.is_taken(id) {
                return Err(TAKEN.to_owned());
            }
            let flow = SupplyChainFlow {
                id: id.clone(),
                from: from.clone(),
                to: to.clone(),
                product: String::new(),
                description: String::new(),
                volume: None,
                unit: String::new(),
                step: None,
                range: Default::default(),
            };
            writer::add_flow(document, model, &flow);
            Ok(())
        }
        SupplyChainEdit::Group { node_id, name, id } => {
            let Some(node) = model.node_of(node_id) else {
                return Err(GONE.to_owned());
            };
            if model.is_taken(id) {
                return Err(TAKEN.to_owned());
            }
            // The node's key first, then the group read afresh: adding the group moves every line below the groups
            // section, the node's among them.
            writer::set_text(document, node.range, keys::GROUP, id, false);
            let group = SupplyChainGroup {
                id: id.clone(),
                name: name.trim().to_owned(),
                description: String::new(),
                range: Default::default(),
                x: None,
                y: None,
            };
            let reread = parser::parse(document);
            writer::add_group(document, &reread, &group);
            Ok(())
        }
        SupplyChainEdit::Move { entry_id, x, y } => move_entry(document, model, entry_id, *x, *y),
        SupplyChainEdit::Place { places } => {
            let places: IndexMap<String, (f64, f64)> = places.iter().cloned().collect();
            writer::place(document, model, &places)
        }
        SupplyChainEdit::Remove { entry_id } => {
            if let Some(node) = model.node_of(entry_id) {
                writer::remove_node(document, model, node);
            } else if let Some(flow) = model.flow_of(entry_id) {
                writer::remove(document, flow.range);
            } else if let Some(group) = model.group_of(entry_id) {
                writer::remove_group(document, model, group);
            } else {
                return Err(GONE.to_owned());
            }
            Ok(())
        }
        SupplyChainEdit::Arrange => {
            let arranged = SupplyChainLayout::of(model).arranged;
            if arranged.is_empty() {
                return Err("There is nothing to arrange until this diagram has a node.".to_owned());
            }
            writer::place(document, model, &arranged)
        }
        SupplyChainEdit::SetProperty { entry_id, key, value } => set_property(document, model, entry_id, key, value),
        SupplyChainEdit::Step { entry_id, steps } => {
            if let Some(node) = model.node_of(entry_id) {
                return match stepped(node.quantity, node.step, *steps) {
                    Some(quantity) => {
                        writer::set_number(document, node.range, keys::QUANTITY, Some(quantity));
                        Ok(())
                    }
                    None => Err("The quantity is already zero.".to_owned()),
                };
            }
            if let Some(flow) = model.flow_of(entry_id) {
                return match stepped(flow.volume, flow.step, *steps) {
                    Some(volume) => {
                        writer::set_number(document, flow.range, keys::VOLUME, Some(volume));
                        Ok(())
                    }
                    None => Err("The volume is already zero.".to_owned()),
                };
            }
            if model.group_of(entry_id).is_some() {
                return Err("A group has no value of its own; its members do.".to_owned());
            }
            Err(GONE.to_owned())
        }
    }
}

/// The value `steps` steps from `current`, or none when it is already zero and asked to go lower; rounded to the four
/// decimals the writer keeps, so ten steps of 0.1 come back to a whole number, and never below zero.
pub fn stepped(current: Option<f64>, step: Option<f64>, steps: i32) -> Option<f64> {
    let from = current.unwrap_or(0.0);
    if from <= 0.0 && steps < 0 {
        return None;
    }
    let step = match step {
        Some(step) if step > 0.0 => step,
        _ => geometry::DEFAULT_STEP,
    };
    let value = round4(from + f64::from(steps) * step);
    // `Math.Max(0, value)`: never below zero, and a zero is a positive one.
    Some(if value <= 0.0 { 0.0 } else { value })
}

fn set_property(
    document: &mut LineDocument,
    model: &SupplyChainModel,
    entry_id: &str,
    key: &str,
    value: &str,
) -> Result<(), String> {
    let (value, number) = read_value(key, value)?;
    let unknown = || Err(format!("`{key}` cannot be set on this entry."));
    if let Some(node) = model.node_of(entry_id) {
        let range = node.range;
        match key {
            keys::NAME => writer::set_text(document, range, key, &value, false),
            keys::DESCRIPTION | keys::UNIT => writer::set_text(document, range, key, &value, true),
            keys::QUANTITY | keys::STEP => writer::set_number(document, range, key, number),
            keys::TYPE => {
                if !stages::is_known(&value) {
                    return Err(format!("`{value}` is not a stage: {}.", stages::ALL.join(", ")));
                }
                writer::set_text(document, range, key, stages::normalize(&value), false);
            }
            keys::GROUP => {
                if !value.is_empty() && model.group_of(&value).is_none() {
                    return Err(format!("There is no group `{value}` in this diagram."));
                }
                writer::set_text(document, range, key, &value, true);
            }
            _ => return unknown(),
        }
        return Ok(());
    }
    if let Some(flow) = model.flow_of(entry_id) {
        let range = flow.range;
        match key {
            keys::PRODUCT | keys::DESCRIPTION | keys::UNIT => writer::set_text(document, range, key, &value, true),
            keys::VOLUME | keys::STEP => writer::set_number(document, range, key, number),
            _ => return unknown(),
        }
        return Ok(());
    }
    if let Some(group) = model.group_of(entry_id) {
        let range = group.range;
        match key {
            keys::NAME => writer::set_text(document, range, key, &value, false),
            keys::DESCRIPTION => writer::set_text(document, range, key, &value, true),
            _ => return unknown(),
        }
        return Ok(());
    }
    Err(GONE.to_owned())
}

/// Moves a node, or a group with every member, in one edit; a node dropped inside another group's frame joins it
/// (`MoveSupplyChainEntryCommandHandler`).
///
/// A node's centre decides which frame it is dropped in, and the smallest frame wins where two overlap; dropped
/// anywhere else it stays in its own group, whose frame grows to follow it. A group a node leaves empty stays where it
/// was, its frame's top-left written as its own `x` and `y`. A group moves its members by the distance its frame
/// moved, and its own place with them.
fn move_entry(
    document: &mut LineDocument,
    model: &SupplyChainModel,
    entry_id: &str,
    x: f64,
    y: f64,
) -> Result<(), String> {
    let layout = SupplyChainLayout::of(model);
    if layout.node_boxes.contains_key(entry_id) {
        return move_node(document, model, &layout, entry_id, x, y);
    }
    match layout.group_boxes.get(entry_id) {
        Some(frame) => move_group(document, model, &layout, *frame, entry_id, x, y),
        None => Err("That is not something this diagram can move.".to_owned()),
    }
}

fn move_node(
    document: &mut LineDocument,
    model: &SupplyChainModel,
    layout: &SupplyChainLayout,
    entry_id: &str,
    x: f64,
    y: f64,
) -> Result<(), String> {
    let node = layout
        .nodes
        .iter()
        .find(|n| n.id == entry_id)
        .expect("a node with a box");
    let current = if layout.group_boxes.contains_key(&node.group) {
        node.group.as_str()
    } else {
        ""
    };
    let joins = group_at(
        layout,
        current,
        x + geometry::NODE_WIDTH / 2.0,
        y + geometry::NODE_HEIGHT / 2.0,
    );

    let places: IndexMap<String, (f64, f64)> = [(node.id.clone(), (x, y))].into_iter().collect();
    writer::place(document, model, &places)?;
    let Some(joins) = joins else {
        return Ok(());
    };

    // Every splice moves lines, so each next edit locates its entry in the document as it now is.
    let placed = parser::parse(document);
    let range = placed.node_of(&node.id).expect("the node just placed").range;
    writer::set_text(document, range, keys::GROUP, &joins, false);

    let left = !current.is_empty() && !layout.nodes.iter().any(|o| o.id != node.id && o.group == current);
    if left {
        let frame = layout.group_boxes[current];
        let regrouped = parser::parse(document);
        let group = regrouped.group_of(current).expect("the group it left").clone();
        writer::place_group(document, &group, frame.x, frame.y);
    }
    Ok(())
}

/// The group a node whose centre is (`x`, `y`) is dropped into, or none when it stays where it is: inside its own
/// frame, or inside no other.
fn group_at(layout: &SupplyChainLayout, current: &str, x: f64, y: f64) -> Option<String> {
    let holds = |b: &adp_layout::Rect| x >= b.x && x <= b.right() && y >= b.y && y <= b.bottom();
    if !current.is_empty() && holds(&layout.group_boxes[current]) {
        return None;
    }
    let mut candidates: Vec<&SupplyChainGroup> = layout
        .groups
        .iter()
        .filter(|g| g.id != current && holds(&layout.group_boxes[&g.id]))
        .collect();
    candidates.sort_by(|a, b| {
        let area = |g: &SupplyChainGroup| {
            let frame = layout.group_boxes[&g.id];
            frame.width * frame.height
        };
        area(a).partial_cmp(&area(b)).unwrap_or(std::cmp::Ordering::Equal)
    });
    candidates.first().map(|g| g.id.clone())
}

fn move_group(
    document: &mut LineDocument,
    model: &SupplyChainModel,
    layout: &SupplyChainLayout,
    frame: adp_layout::Rect,
    entry_id: &str,
    x: f64,
    y: f64,
) -> Result<(), String> {
    let group = layout
        .groups
        .iter()
        .find(|g| g.id == entry_id)
        .expect("a group with a frame")
        .clone();
    let (dx, dy) = (x - frame.x, y - frame.y);
    let places: IndexMap<String, (f64, f64)> = layout
        .nodes
        .iter()
        .filter(|n| n.group == group.id)
        .map(|n| {
            let box_ = layout.node_boxes[&n.id];
            (n.id.clone(), (box_.x + dx, box_.y + dy))
        })
        .collect();
    if !places.is_empty() {
        writer::place(document, model, &places)?;
    }
    // An empty group is placed where it was dropped; a placed group with members keeps its own place the same
    // distance from its frame, for when they go.
    if places.is_empty() || group.is_placed() {
        let (gx, gy) = if places.is_empty() {
            (x, y)
        } else {
            (group.x.unwrap_or_default() + dx, group.y.unwrap_or_default() + dy)
        };
        let reread = parser::parse(document);
        let current = reread.group_of(&group.id).expect("the group moved").clone();
        writer::place_group(document, &current, gx, gy);
    }
    Ok(())
}
