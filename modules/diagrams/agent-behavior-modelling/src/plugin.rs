//! The persistence plugin `net.etalii.adp.etalii.abmMarkdown` of the binding `agent-behavior-modelling.fbl#abm`
//! (etalii.adp spec 030, FR-031 and FR-032; FBL section 11), ported from the standalone host's `AbmMarkdownPlugin.cs`
//! at `8b6ccfc`.
//!
//! It reads the tree with the [`parser`](crate::parser) and plans every change with the [`writer`](crate::writer), so
//! the Markdown is written exactly as the module has always written it, and every byte outside the lines a change
//! touches survives:
//!
//! - A node is an element of the DISL type `x-abm.types` maps to its kind, with `label`, `notes`, a Retry's
//!   `attempts` and a Do's `implicit` (an item without a keyword). Its id is its place, `1.2.1`, which is not stored:
//!   the definition derives it.
//! - It keeps no state (FBL section 11.4): `plan` parses the bytes it is handed again, makes the change on a copy of
//!   their lines, and hands back the bytes that differ as one splice, or two for a move, the subtree taken out and put
//!   in again. An edit the writer refuses comes back refused, in its words.
//! - Its one finding is `abm.no-behavior`, information that the file has no Behavior heading (line 1) or that the
//!   section holds no list yet (the heading's line).

use adp_cel::{Map, Value};
use adp_fbl::ModelChange;
use adp_fbl::model::{FblElement, Finding, FindingSeverity, SourceLocation};
use adp_fbl::plugins::{
    PersistencePlugin, PluginPlanRequest, PluginPlanResult, PluginReadRequest, PluginReadResult, PluginSplice,
    PluginTemplateRequest,
};
use adp_fbl::splice::{Operation, Splice};
use adp_fbl::text::Span;

use crate::kinds;
use crate::lines::LineDocument;
use crate::model::{Model, Node, rule_ids};
use crate::{parser, writer};

/// The plugin's id, as the binding's `reader.plugin` names it.
pub const PLUGIN_ID: &str = "net.etalii.adp.etalii.abmMarkdown";

/// The refusal for a node that is not there, in one wording.
pub const GONE: &str = "That node is no longer in this behavior model.";

/// The Markdown reader and planner.
#[derive(Debug, Clone, Copy, Default)]
pub struct AbmMarkdownPlugin;

/// The model of `bytes`, as the plugin and the module read it.
pub fn read_model(bytes: &[u8]) -> Model {
    parser::parse(&LineDocument::parse(&String::from_utf8_lossy(bytes)))
}

impl PersistencePlugin for AbmMarkdownPlugin {
    fn id(&self) -> &str {
        PLUGIN_ID
    }

    fn read(&self, request: &PluginReadRequest) -> PluginReadResult {
        let Some(body) = request.files.iter().find(|f| f.relative_path.is_empty()) else {
            return PluginReadResult::new(Vec::new(), Vec::new(), true);
        };
        let bytes = &body.bytes;
        let model = read_model(bytes);
        let starts = line_starts(bytes);
        let elements = model
            .nodes
            .iter()
            .map(|node| element_of(node, &starts, bytes.len()))
            .collect();
        PluginReadResult::new(elements, findings(&model), false)
    }

    fn plan(&self, request: &PluginPlanRequest<'_>) -> PluginPlanResult {
        let Some(body) = request.files.iter().find(|f| f.relative_path.is_empty()) else {
            return PluginPlanResult::Refused("A behavior model is a file body.".into());
        };
        let before = &body.bytes;
        let mut document = LineDocument::parse(&String::from_utf8_lossy(before));
        let model = parser::parse(&document);
        if let Err(refusal) = apply(&mut document, &model, request.change) {
            return PluginPlanResult::Refused(refusal);
        }
        let after = document.text().into_bytes();
        if let ModelChange::Move {
            id,
            new_parent_id,
            index,
        } = request.change
            && let Some(moved) = model.node(id)
            && let Some(splices) = move_splices(before, &after, &model, moved, new_parent_id.as_deref(), *index)
        {
            return PluginPlanResult::Planned(splices);
        }
        PluginPlanResult::Planned(
            difference(before, &after, operation_of(request.change))
                .into_iter()
                .map(|splice| PluginSplice {
                    file: String::new(),
                    splice,
                })
                .collect(),
        )
    }

    fn template(&self, request: &PluginTemplateRequest) -> Vec<u8> {
        writer::empty_document(&request.name, "\r\n").into_bytes()
    }
}

/// The change made on `document`, as the module's commands make it.
fn apply(document: &mut LineDocument, model: &Model, change: &ModelChange) -> Result<(), String> {
    match change {
        ModelChange::Set { id, attributes } => {
            if model.node(id).is_none() {
                return Err(GONE.to_owned());
            }
            for (name, value) in attributes.iter() {
                // Each attribute is written on the lines as they are after the one before.
                let reread = parser::parse(document);
                let node = reread.node(id).ok_or_else(|| GONE.to_owned())?.clone();
                match name.as_str() {
                    "label" => writer::set_label(document, &node, &text(value))?,
                    "notes" => writer::set_notes(document, &node, &text(value))?,
                    "attempts" if node.kind != kinds::RETRY => return Err("Only a Retry has attempts.".to_owned()),
                    "attempts" => match number(value) {
                        Some(attempts) => writer::set_kind(document, &node, kinds::RETRY, attempts)?,
                        None => return Err("A Retry's attempts are a whole number.".to_owned()),
                    },
                    other => return Err(format!("A behavior model node has no `{other}` to write.")),
                }
            }
            Ok(())
        }
        ModelChange::Add {
            type_name,
            attributes,
            parent_id,
            index,
            ..
        } => {
            let Some(kind) = kinds::of_type(type_name) else {
                return Err(format!("A behavior model has no `{type_name}` node."));
            };
            let parent = match parent_id {
                Some(parent_id) => Some(model.node(parent_id).ok_or_else(|| GONE.to_owned())?),
                None => None,
            };
            let attempts = attributes.get("attempts").map(|v| number(v).unwrap_or(0)).unwrap_or(0);
            let label = text(attributes.get("label").unwrap_or(&Value::Null));
            let id = writer::add(document, model, parent, index.unwrap_or(-1), kind.id, &label, attempts)?;
            let notes = text(attributes.get("notes").unwrap_or(&Value::Null));
            if notes.is_empty() {
                return Ok(());
            }
            let reread = parser::parse(document);
            let added = reread.node(&id).ok_or_else(|| GONE.to_owned())?.clone();
            writer::set_notes(document, &added, &notes)
        }
        ModelChange::Remove { id } => match model.node(id) {
            Some(node) => writer::remove(document, node),
            None => Err(GONE.to_owned()),
        },
        ModelChange::Move {
            id,
            new_parent_id,
            index,
        } => {
            let Some(node) = model.node(id) else {
                return Err(GONE.to_owned());
            };
            let parent = match new_parent_id {
                Some(parent_id) => Some(
                    model
                        .node(parent_id)
                        .ok_or("The node it was moved under is no longer in this behavior model.")?,
                ),
                None => None,
            };
            writer::move_node(document, model, node, parent, *index)
        }
        ModelChange::Retype {
            id,
            type_name,
            attributes,
        } => {
            let Some(node) = model.node(id) else {
                return Err(GONE.to_owned());
            };
            let Some(kind) = kinds::of_type(type_name) else {
                return Err(format!("A behavior model has no `{type_name}` node."));
            };
            let attempts = attributes.get("attempts").map(|v| number(v).unwrap_or(0)).unwrap_or(0);
            writer::set_kind(document, node, kind.id, attempts)
        }
        ModelChange::Save => Ok(()),
        _ => Err("A behavior model cannot make that change.".to_owned()),
    }
}

fn operation_of(change: &ModelChange) -> Operation {
    match change {
        ModelChange::Set { attributes, .. } if attributes.len() == 1 && attributes.get("notes").is_some() => {
            Operation::ReplaceValue
        }
        ModelChange::Add { .. } => Operation::InsertEntry,
        ModelChange::Remove { .. } => Operation::RemoveEntry,
        _ => Operation::ReEmitLine,
    }
}

/// A move as what it is: the subtree's bytes taken out, and its re-indented lines put in where it lands; none when
/// the writer's bytes are not that, which the one-splice difference then covers.
fn move_splices(
    before: &[u8],
    after: &[u8],
    model: &Model,
    node: &Node,
    new_parent_id: Option<&str>,
    index: i64,
) -> Option<Vec<PluginSplice>> {
    let starts = line_starts(before);
    let removed_start = start(&starts, node.line, before.len());
    let removed_end = start(&starts, node.subtree_end + 1, before.len());
    let parent = new_parent_id.and_then(|id| model.node(id));
    let siblings = match parent {
        Some(parent) => model.children_of(parent),
        None => model.roots(),
    };
    let at = if index < 0 {
        siblings.len()
    } else {
        (index as usize).min(siblings.len())
    };
    let target_line = if at < siblings.len() {
        siblings[at].line
    } else {
        parent
            .map(|p| p.subtree_end)
            .or_else(|| model.roots().last().map(|r| r.subtree_end))?
            + 1
    };
    let target = start(&starts, target_line, before.len());
    if target > removed_start && target < removed_end {
        return None;
    }
    let removed = removed_end - removed_start;
    let inserted = after.len().checked_sub(before.len() - removed)?;

    // The bytes the move must give, laid out from the old ones; anything else is not this move.
    let insert_at;
    if target >= removed_end {
        insert_at = removed_start + (target - removed_end);
        if !same(before, 0, after, 0, removed_start)
            || !same(before, removed_end, after, removed_start, target - removed_end)
            || !same(before, target, after, insert_at + inserted, before.len() - target)
        {
            return None;
        }
    } else {
        insert_at = target;
        if !same(before, 0, after, 0, target)
            || !same(before, target, after, target + inserted, removed_start - target)
            || !same(
                before,
                removed_end,
                after,
                removed_start + inserted,
                before.len() - removed_end,
            )
        {
            return None;
        }
    }

    let text = std::str::from_utf8(after.get(insert_at..insert_at + inserted)?)
        .ok()?
        .to_owned();
    let removal = PluginSplice {
        file: String::new(),
        splice: Splice {
            operation: Operation::RemoveEntry,
            start: removed_start,
            end: removed_end,
            text: String::new(),
        },
    };
    let insertion = PluginSplice {
        file: String::new(),
        splice: Splice {
            operation: Operation::InsertEntry,
            start: target,
            end: target,
            text,
        },
    };
    // In body order: an insertion where the subtree starts goes before its removal.
    Some(if target <= removed_start {
        vec![insertion, removal]
    } else {
        vec![removal, insertion]
    })
}

/// The one splice that turns `before` into `after`: the bytes between their common start and their common end,
/// widened to whole characters; none when nothing differs.
pub fn difference(before: &[u8], after: &[u8], operation: Operation) -> Option<Splice> {
    let limit = before.len().min(after.len());
    let mut prefix = 0;
    while prefix < limit && before[prefix] == after[prefix] {
        prefix += 1;
    }
    if prefix == before.len() && prefix == after.len() {
        return None;
    }
    let mut suffix = 0;
    while suffix < limit - prefix && before[before.len() - 1 - suffix] == after[after.len() - 1 - suffix] {
        suffix += 1;
    }
    // A UTF-8 continuation byte is never where a character starts.
    while prefix > 0 && is_continuation(after, prefix) {
        prefix -= 1;
    }
    while suffix > 0 && is_continuation(after, after.len() - suffix) {
        suffix -= 1;
    }
    Some(Splice {
        operation,
        start: prefix,
        end: before.len() - suffix,
        text: String::from_utf8_lossy(&after[prefix..after.len() - suffix]).into_owned(),
    })
}

fn is_continuation(bytes: &[u8], index: usize) -> bool {
    index < bytes.len() && (bytes[index] & 0xC0) == 0x80
}

fn same(left: &[u8], left_start: usize, right: &[u8], right_start: usize, count: usize) -> bool {
    left_start + count <= left.len()
        && right_start + count <= right.len()
        && left[left_start..left_start + count] == right[right_start..right_start + count]
}

fn element_of(node: &Node, starts: &[usize], length: usize) -> FblElement {
    let kind = kinds::of(&node.kind);
    let mut attributes = Map::default();
    attributes.insert("label", Value::String(node.label.clone()));
    attributes.insert("notes", Value::String(node.notes.clone()));
    if node.kind == kinds::RETRY {
        attributes.insert("attempts", Value::Int(i64::from(node.retry_count)));
    }
    if node.kind == kinds::ACTION {
        attributes.insert("implicit", Value::Bool(!node.has_keyword));
    }
    FblElement {
        id: node.id.clone(),
        id_is_stored: false,
        type_name: kind.type_name.to_owned(),
        rule: "item".to_owned(),
        is_relation: false,
        attributes,
        parent_slot: node.parent_id.as_ref().map(|_| "children".to_owned()),
        parent_id: node.parent_id.clone(),
        source: None,
        target: None,
        referenced: Vec::new(),
        own_span: Span::new(
            start(starts, node.line, length),
            start(starts, node.subtree_end + 1, length),
        ),
        line: node.line + 1,
        file: None,
    }
}

fn findings(model: &Model) -> Vec<Finding> {
    match model.section_line {
        None => vec![no_behavior(
            "This file has no Behavior heading, so there is no behavior tree to draw. Add a node to start one.",
            0,
        )],
        Some(line) if model.nodes.is_empty() => vec![no_behavior(
            "The Behavior section holds no list yet. Add a node to start the tree.",
            line,
        )],
        Some(_) => Vec::new(),
    }
}

fn no_behavior(message: &str, line: usize) -> Finding {
    Finding {
        code: rule_ids::NO_BEHAVIOR.to_owned(),
        severity: FindingSeverity::Info,
        message: message.to_owned(),
        location: Some(SourceLocation {
            file: String::new(),
            line: line + 1,
            column: 1,
            length: 0,
        }),
        detail: None,
    }
}

/// A value as the writer takes it: text as it is, a number in its invariant form, nothing as empty.
fn text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Int(i) => i.to_string(),
        Value::Double(d) => adp_fbl::planning::dotnet_double(*d),
        Value::Bool(b) => if *b { "True" } else { "False" }.to_owned(),
        other => other.to_string(),
    }
}

/// A value as a whole number of attempts, or none.
fn number(value: &Value) -> Option<i32> {
    match value {
        Value::Int(i) => i32::try_from(*i).ok(),
        Value::Double(d) if d.fract() == 0.0 && *d >= f64::from(i32::MIN) && *d <= f64::from(i32::MAX) => {
            Some(*d as i32)
        }
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// The byte offset each line starts at: 0, then after every line feed.
fn line_starts(bytes: &[u8]) -> Vec<usize> {
    std::iter::once(0)
        .chain(
            bytes
                .iter()
                .enumerate()
                .filter(|(_, b)| **b == b'\n')
                .map(|(i, _)| i + 1),
        )
        .collect()
}

fn start(starts: &[usize], line: usize, length: usize) -> usize {
    starts.get(line).copied().unwrap_or(length)
}
