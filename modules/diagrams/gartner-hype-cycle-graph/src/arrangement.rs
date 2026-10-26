//! The rows Arrange diagram gives a hype cycle graph (the standalone host's `GhgArrangement.cs` and
//! `Commands/ArrangeGhgCommandHandler.cs` at `8b6ccfc`; the layout `rowArrange` of `gartner-hype-cycle-graph.dis`):
//! as few rows as its trends, triggers and notes allow, with influenced trends on rows close together; no date ever
//! changes.
//!
//! Across is time, so only rows move: the arrangement is the shared row packing (`adp-layout`), fed with what each
//! element covers across as the canvas draws it: a trend's banner with its name before it, a trigger's circle with
//! its name and date before it, a note's box over as many rows as it is tall. The influences are the links, so a trend
//! sits near what it influences; a note is linked to nothing and fills whatever room is left. An element without an
//! id, without a readable place, or with an id an earlier element has keeps its row. Only `row` is written, in one
//! step: each element whose row changes, the last written first, as the standalone host batched them bottom-up.

use std::collections::{HashMap, HashSet};

use adp_cel::{Map, Value};
use adp_disl::{DislChange, DislDiagram, DislElement};
use adp_layout::row_packing::{self, RowItem};
use adp_layout::text_metric;

use crate::reading::{self, Note, Trend, Trigger};
use crate::scale;

/// The layout algorithm this implements, as the specification's `rowArrange` declares it.
pub const ALGORITHM: &str = "rows";

/// What arranging a graph without anything to arrange answers.
pub const NOTHING_TO_ARRANGE: &str = "There is nothing to arrange until this graph has a trend.";

/// What arranging an arranged graph answers.
pub const ALREADY_ARRANGED: &str = "This graph is already arranged.";

/// The canvas's label size, and the space between a label and what it names.
const LABEL_FONT_SIZE: f64 = 12.0;
const LABEL_GAP: f64 = 8.0;

/// The least clear space between two elements on one row.
pub const GAP: f64 = 16.0;

/// The nodes of `type_name` in the order they are written.
fn nodes_of(diagram: &DislDiagram, type_name: &str) -> Vec<DislElement> {
    let mut nodes: Vec<(usize, DislElement)> = diagram.nodes_of_type(type_name).into_iter().enumerate().collect();
    nodes.sort_by_key(|(index, node)| (node.line().unwrap_or(i32::MAX), *index));
    nodes.into_iter().map(|(_, node)| node).collect()
}

/// What the row packing is fed: every element that can be drawn with what it covers across and how many rows it is
/// tall, and the influences as the links, by relation id, source and target.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Packing {
    pub items: Vec<RowItem>,
    pub links: Vec<(String, String, String)>,
}

/// The row of every element that can be drawn, by the id it is written with (`GhgArrangement.RowsOf`).
pub fn rows_of(diagram: &DislDiagram) -> HashMap<String, i64> {
    let packing = packing_of(diagram);
    let links: Vec<(&str, &str)> = packing.links.iter().map(|(_, a, b)| (a.as_str(), b.as_str())).collect();
    row_packing::pack(&packing.items, &links, GAP, &[])
        .into_iter()
        .map(|(id, row)| (id, row as i64))
        .collect()
}

/// The items and links that [`rows_of`] packs, with [`GAP`] between neighbours on a row.
pub fn packing_of(diagram: &DislDiagram) -> Packing {
    let unit = reading::unit_of(diagram);
    let mut taken = HashSet::new();
    let mut items = Vec::new();
    for node in nodes_of(diagram, "Trend") {
        let trend = Trend::of(&node);
        let Some((start, _)) = trend.span() else {
            continue;
        };
        if trend.id.is_empty() || !taken.insert(trend.id.clone()) {
            continue;
        }
        let left = scale::x_of(start, unit);
        items.push(RowItem::new(
            trend.id.clone(),
            left - LABEL_GAP - text_metric::width_of(&trend.name, LABEL_FONT_SIZE),
            left + scale::width_of(trend.months(), unit),
        ));
    }
    for node in nodes_of(diagram, "Trigger") {
        let trigger = Trigger::of(&node);
        let Some(date) = trigger.date else {
            continue;
        };
        if trigger.id.is_empty() || !taken.insert(trigger.id.clone()) {
            continue;
        }
        let centre = scale::x_of(date, unit);
        let half = scale::TRIGGER_SIZE / 2.0;
        let label = format!("{} \u{b7} {}", trigger.name, scale::format_when(date, unit));
        items.push(RowItem::new(
            trigger.id.clone(),
            centre - half - LABEL_GAP - text_metric::width_of(&label, LABEL_FONT_SIZE),
            centre + half,
        ));
    }
    for node in nodes_of(diagram, "Note") {
        let note = Note::of(&node);
        let Some((at, width, height)) = note.placed() else {
            continue;
        };
        if note.id.is_empty() || !taken.insert(note.id.clone()) {
            continue;
        }
        let left = scale::x_of(at, unit);
        items.push(RowItem::tall(
            note.id.clone(),
            left,
            left + width,
            (height / scale::ROW_STEP).ceil() as usize,
        ));
    }
    let links = diagram
        .relations_of_type("Influence")
        .iter()
        .map(|influence| {
            (
                influence.id(),
                influence.source_id().unwrap_or_default(),
                influence.target_id().unwrap_or_default(),
            )
        })
        .collect();
    Packing { items, links }
}

/// The id, the row and the line of `element`, as read.
fn read(element: &DislElement) -> (String, i64) {
    if element.is_a("Trend") {
        let trend = Trend::of(element);
        (trend.id, trend.row)
    } else if element.is_a("Trigger") {
        let trigger = Trigger::of(element);
        (trigger.id, trigger.row)
    } else {
        let note = Note::of(element);
        (note.id, note.row)
    }
}

/// The changes that arrange `diagram`, or why there are none (`ArrangeGhgCommandHandler`).
pub fn arrange(diagram: &DislDiagram) -> Result<Vec<DislChange>, String> {
    let rows = rows_of(diagram);
    if rows.is_empty() {
        return Err(NOTHING_TO_ARRANGE.to_owned());
    }
    let mut written_ids = HashSet::new();
    let mut changes: Vec<(i32, DislChange)> = Vec::new();
    for element in ["Trend", "Trigger", "Note"]
        .into_iter()
        .flat_map(|type_name| nodes_of(diagram, type_name))
    {
        let (id, was) = read(&element);
        let Some(row) = rows.get(&id).copied() else {
            continue;
        };
        if row == was || !written_ids.insert(id) {
            continue;
        }
        let mut attributes = Map::default();
        attributes.insert("row", Value::Int(row));
        changes.push((
            element.line().unwrap_or(i32::MAX),
            DislChange::Set {
                element_id: element.id(),
                type_name: element.type_name(),
                attributes,
            },
        ));
    }
    if changes.is_empty() {
        return Err(ALREADY_ARRANGED.to_owned());
    }
    changes.sort_by_key(|(line, _)| std::cmp::Reverse(*line));
    Ok(changes.into_iter().map(|(_, change)| change).collect())
}
