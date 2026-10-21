//! The rows "Arrange diagram" gives a timeline, the specification's plugin layout
//! `net.etalii.adp.generic.timelineRowArrange` (the standalone host's `TimelineArrangement.cs` and
//! `Commands/ArrangeTimelineCommandHandler.cs` at `8b6ccfc`): as few rows as its elements allow, with related
//! elements on rows close together; their times never change.
//!
//! Across is time, so only rows move: the arrangement is the shared row packing (`adp-layout`), fed with what each
//! element covers across at the canvas's first fit (the timeline's span over 1000 of its 1200 units, at least a day):
//! a period from its begin to its end, at least 2 units wide, a moment as its diamond with its label beside it. A
//! moment linked to something later would draw that link level through its own label if the two shared a row, so
//! such pairs are kept apart. Elements without an id, without a readable begin, or with an id an earlier element has,
//! keep their row. Only `row` is written, in one step: each element whose row changes, the last holder of an id
//! first, as the standalone host batched them.

use std::collections::{HashMap, HashSet};

use adp_cel::{Map, Value};
use adp_disl::{DislChange, DislDiagram, DislElement};
use adp_layout::row_packing::{self, RowItem};
use adp_layout::text_metric;

use crate::reading::TimelineElement;
use crate::scale;
use crate::times::written_end;

/// The plugin this implements.
pub const PLUGIN: &str = "net.etalii.adp.generic.timelineRowArrange";

/// What arranging a timeline without an element answers.
pub const NOTHING_TO_ARRANGE: &str = "There is nothing to arrange until this timeline has an element.";

/// What arranging an arranged timeline answers.
pub const ALREADY_ARRANGED: &str = "This timeline is already arranged.";

/// The width the client's first fit spreads the timeline's span over, without its margin.
const FIT_WIDTH: f64 = 1200.0 / 1.2;

/// The canvas's label size.
const LABEL_FONT_SIZE: f64 = 12.0;

/// A moment's diamond radius, and the space between it and its label.
const MOMENT_RADIUS: f64 = 9.0;
const LABEL_GAP: f64 = 6.0;

/// The least clear space between two elements on one row.
const GAP: f64 = 12.0;

const DAY: f64 = 86_400.0;

/// The nodes of `diagram` in the order they are written, each with how it reads.
fn written(diagram: &DislDiagram) -> Vec<(DislElement, TimelineElement)> {
    let mut nodes: Vec<(usize, DislElement)> = diagram.nodes().into_iter().enumerate().collect();
    nodes.sort_by_key(|(index, node)| (node.line().unwrap_or(i32::MAX), *index));
    nodes
        .into_iter()
        .map(|(_, node)| {
            let read = TimelineElement::of(&node);
            (node, read)
        })
        .collect()
}

/// The row of every element that can be drawn, by the id it is written with.
pub fn rows_of(elements: &[TimelineElement], links: &[(String, String)]) -> HashMap<String, i64> {
    let mut taken = HashSet::new();
    let drawn: Vec<&TimelineElement> = elements
        .iter()
        .filter(|e| !e.id.is_empty() && e.begin_instant().is_some() && taken.insert(e.id.clone()))
        .collect();
    if drawn.is_empty() {
        return HashMap::new();
    }
    let begin = |e: &TimelineElement| scale::to_seconds(e.begin_instant().unwrap_or_default());
    let end = |e: &TimelineElement| match e.end_instant() {
        Some(end) => begin(e).max(scale::to_seconds(end)),
        None => begin(e),
    };
    let first = drawn.iter().map(|e| begin(e)).fold(f64::INFINITY, f64::min);
    let last = drawn.iter().map(|e| end(e)).fold(f64::NEG_INFINITY, f64::max);
    let units_per_second = FIT_WIDTH / (last - first).max(DAY);
    let by_id: HashMap<&str, &TimelineElement> = drawn.iter().map(|e| (e.id.as_str(), *e)).collect();

    let is_moment = |id: &str| by_id.get(id).is_some_and(|e| e.end_instant().is_none());
    let is_later_than = |id: &str, moment: &str| {
        by_id
            .get(id)
            .is_some_and(|e| begin(e) > by_id.get(moment).map(|m| begin(m)).unwrap_or(f64::INFINITY))
    };
    let links: Vec<(&str, &str)> = links.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let apart: Vec<(&str, &str)> = links
        .iter()
        .copied()
        .filter(|(from, to)| (is_moment(from) && is_later_than(to, from)) || (is_moment(to) && is_later_than(from, to)))
        .collect();
    let items: Vec<RowItem> = drawn
        .iter()
        .map(|e| {
            let left = (begin(e) - first) * units_per_second;
            if e.end_instant().is_some() {
                RowItem::new(
                    e.id.clone(),
                    left,
                    (left + 2.0).max((end(e) - first) * units_per_second),
                )
            } else {
                RowItem::new(
                    e.id.clone(),
                    left - MOMENT_RADIUS,
                    left + MOMENT_RADIUS + LABEL_GAP + text_metric::width_of(e.shown_label(), LABEL_FONT_SIZE),
                )
            }
        })
        .collect();
    row_packing::pack(&items, &links, GAP, &apart)
        .into_iter()
        .map(|(id, row)| (id, row as i64))
        .collect()
}

/// The elements of `diagram` in the order they are written, and its relations' ends as written.
pub fn read(diagram: &DislDiagram) -> (Vec<TimelineElement>, Vec<(String, String)>) {
    let elements = written(diagram).into_iter().map(|(_, read)| read).collect();
    (elements, links(diagram))
}

fn links(diagram: &DislDiagram) -> Vec<(String, String)> {
    diagram
        .relations()
        .iter()
        .map(|r| (written_end(r, "source"), written_end(r, "target")))
        .collect()
}

/// The changes that arrange `diagram`, or why there are none.
pub fn arrange(diagram: &DislDiagram) -> Result<Vec<DislChange>, String> {
    let nodes = written(diagram);
    let elements: Vec<TimelineElement> = nodes.iter().map(|(_, read)| read.clone()).collect();
    let rows = rows_of(&elements, &links(diagram));
    if rows.is_empty() {
        return Err(NOTHING_TO_ARRANGE.to_owned());
    }
    let mut written_ids = HashSet::new();
    let mut changes = Vec::new();
    for (node, read) in nodes.iter().rev() {
        let Some(row) = rows.get(&read.id).copied() else {
            continue;
        };
        if row == read.row || !written_ids.insert(read.id.clone()) {
            continue;
        }
        let mut attributes = Map::default();
        attributes.insert("row", Value::Int(row));
        changes.push(DislChange::Set {
            element_id: node.id(),
            type_name: node.type_name(),
            attributes,
        });
    }
    if changes.is_empty() {
        return Err(ALREADY_ARRANGED.to_owned());
    }
    Ok(changes)
}
