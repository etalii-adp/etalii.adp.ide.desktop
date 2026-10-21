//! Where everything in one document is drawn: each node's column, value and bar, and each flow's band, all computed
//! and none authored (etalii.adp spec 030, task T077).
//!
//! Ported from the standalone host's `SankeyLayout.cs` at `8b6ccfc`, number for number. A Sankey diagram's geometry
//! is its data: a bar is as tall as the larger of what flows in and what flows out, a band as thick as its value,
//! both on one scale. Columns come from the flows: a node sits one column right of the furthest node that flows into
//! it, every cycle broken at the flow that closes it, unless it states its own `column`. The order within a column is
//! the document's, and it is the one thing a reader arranges; everything else about the vertical placement is
//! relaxation, which draws each node towards the bands it shares with its neighbours and never reorders.

use std::collections::{HashMap, HashSet};

use adp_layout::{CyclePolicy, layering};

use crate::model::{SankeyFlow, SankeyModel, SankeyNode, geometry};

/// How many relaxation passes run, each weaker than the last.
const ITERATIONS: i32 = 24;

/// A node's bar: its top left corner and its size, in canvas units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bar {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Bar {
    pub fn centre_x(&self) -> f64 {
        self.x + self.width / 2.0
    }

    pub fn centre_y(&self) -> f64 {
        self.y + self.height / 2.0
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }
}

/// Where one flow is drawn: how thick, where along each end's edge its centre sits, as a fraction of that end's
/// height from its top, and whether it runs back to a column at or left of its source's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Band {
    pub thickness: f64,
    pub source_at: f64,
    pub target_at: f64,
    pub backward: bool,
}

/// The layout of one model.
#[derive(Debug, Clone, PartialEq)]
pub struct SankeyLayout {
    /// The nodes that can be drawn: an id, unique across the document.
    pub nodes: Vec<SankeyNode>,
    /// The flows that can be drawn: a unique id, both ends drawable, and not to itself.
    pub flows: Vec<SankeyFlow>,
    pub boxes: HashMap<String, Bar>,
    /// Each drawable node's zero-based column.
    pub columns: HashMap<String, usize>,
    /// Each drawable node's value: the larger of its inflow and its outflow.
    pub values: HashMap<String, f64>,
    pub bands: HashMap<String, Band>,
    /// The node ids of each column, top to bottom: the document's order.
    pub column_order: Vec<Vec<String>>,
    /// Canvas units per unit of value.
    pub scale: f64,
}

/// .NET's `Math.Round(value, digits)`: half to even, on the value scaled by a power of ten.
pub fn round(value: f64, digits: i32) -> f64 {
    let power = 10f64.powi(digits);
    (value * power).round_ties_even() / power
}

/// The left edge of a column.
pub fn left_of(column: usize) -> f64 {
    column as f64 * geometry::COLUMN_PITCH
}

/// The column nearest to a canvas x, never left of the first.
pub fn column_at(x: f64) -> usize {
    let column = ((x - geometry::NODE_WIDTH / 2.0) / geometry::COLUMN_PITCH).round_ties_even();
    if column > 0.0 { column as usize } else { 0 }
}

impl SankeyLayout {
    /// The layout of `model`.
    pub fn of(model: &SankeyModel) -> SankeyLayout {
        compute(model)
    }
}

fn compute(model: &SankeyModel) -> SankeyLayout {
    // One id space across nodes and flows: the first entry with an id is drawn and every later one reusing it is not.
    let mut taken: HashSet<String> = HashSet::new();
    let nodes: Vec<SankeyNode> = model
        .nodes
        .iter()
        .filter(|node| !node.id.is_empty() && taken.insert(node.id.clone()))
        .cloned()
        .collect();
    let node_ids: HashSet<&str> = nodes.iter().map(|node| node.id.as_str()).collect();
    let flows: Vec<SankeyFlow> = model
        .flows
        .iter()
        .filter(|flow| {
            node_ids.contains(flow.from.as_str())
                && node_ids.contains(flow.to.as_str())
                && flow.from != flow.to
                && taken.insert(flow.id())
        })
        .cloned()
        .collect();

    let mut inflow: HashMap<&str, f64> = nodes.iter().map(|node| (node.id.as_str(), 0.0)).collect();
    let mut outflow = inflow.clone();
    for flow in &flows {
        *outflow.get_mut(flow.from.as_str()).expect("a drawn end") += flow.amount();
        *inflow.get_mut(flow.to.as_str()).expect("a drawn end") += flow.amount();
    }
    let values: HashMap<String, f64> = nodes
        .iter()
        .map(|node| (node.id.clone(), inflow[node.id.as_str()].max(outflow[node.id.as_str()])))
        .collect();
    let columns = columns_of(&nodes, &flows);
    let column_count = columns.values().max().map_or(0, |max| max + 1);

    // The document's order, column by column.
    let order: Vec<Vec<String>> = (0..column_count)
        .map(|column| {
            nodes
                .iter()
                .filter(|node| columns[&node.id] == column)
                .map(|node| node.id.clone())
                .collect()
        })
        .collect();

    // One scale for every bar and band: the fullest column fills the base height.
    let fullest = order
        .iter()
        .map(|column| column.iter().fold(0.0, |sum, id| sum + values[id]))
        .fold(None, |max: Option<f64>, total| {
            Some(max.map_or(total, |m| m.max(total)))
        })
        .unwrap_or(0.0);
    let scale = if fullest > 0.0 {
        geometry::BASE_HEIGHT * model.settings.thickness / fullest
    } else {
        0.0
    };
    let heights: HashMap<String, f64> = nodes
        .iter()
        .map(|node| {
            (
                node.id.clone(),
                geometry::MINIMUM_NODE_HEIGHT.max(values[&node.id] * scale),
            )
        })
        .collect();
    let stacked = |column: &Vec<String>| {
        column.iter().fold(0.0, |sum, id| sum + heights[id])
            + column.len().saturating_sub(1) as f64 * geometry::NODE_GAP
    };
    let extent = order
        .iter()
        .map(stacked)
        .fold(None, |max: Option<f64>, height| {
            Some(max.map_or(height, |m| m.max(height)))
        })
        .unwrap_or(0.0);

    let forward: Vec<&SankeyFlow> = flows
        .iter()
        .filter(|flow| columns[&flow.to] > columns[&flow.from])
        .collect();
    let document_order: HashMap<String, usize> = flows
        .iter()
        .enumerate()
        .map(|(index, flow)| (flow.id(), index))
        .collect();

    // Every column starts stacked from the top and centred on the tallest.
    let mut top: HashMap<String, f64> = HashMap::new();
    for column in &order {
        let mut y = (extent - stacked(column)) / 2.0;
        for id in column {
            top.insert(id.clone(), y);
            y += heights[id] + geometry::NODE_GAP;
        }
    }

    let geometry = Geometry {
        flows: &flows,
        heights: &heights,
        scale,
        document_order: &document_order,
    };
    let forward_in = |id: &str| -> Vec<&SankeyFlow> { forward.iter().copied().filter(|flow| flow.to == id).collect() };
    let forward_out =
        |id: &str| -> Vec<&SankeyFlow> { forward.iter().copied().filter(|flow| flow.from == id).collect() };

    for iteration in 0..ITERATIONS {
        let alpha = 0.99f64.powf(f64::from(iteration));

        // Right to left: each node towards where its outgoing bands would run level.
        for c in (0..column_count.saturating_sub(1)).rev() {
            for id in &order[c] {
                let along = forward_out(id);
                relax(&mut top, id, &along, alpha, |top, flow| {
                    top[&flow.to] + geometry.offset_in(top, flow) - geometry.offset_out(top, flow)
                });
            }
            resolve(&mut top, &order[c], &heights, extent);
        }

        // Left to right: each node towards where its incoming bands would run level.
        for column in order.iter().skip(1) {
            for id in column {
                let along = forward_in(id);
                relax(&mut top, id, &along, alpha, |top, flow| {
                    top[&flow.from] + geometry.offset_out(top, flow) - geometry.offset_in(top, flow)
                });
            }
            resolve(&mut top, column, &heights, extent);
        }
    }
    for column in &order {
        resolve(&mut top, column, &heights, extent);
    }

    // Whole hundredths: a position that changes in the fifth decimal on every recompute is a delta nobody can see.
    let boxes: HashMap<String, Bar> = nodes
        .iter()
        .map(|node| {
            (
                node.id.clone(),
                Bar {
                    x: left_of(columns[&node.id]),
                    y: round(top[&node.id], 2),
                    width: geometry::NODE_WIDTH,
                    height: round(heights[&node.id], 2),
                },
            )
        })
        .collect();
    let bands: HashMap<String, Band> = flows
        .iter()
        .map(|flow| {
            let thickness = geometry.thickness(flow);
            (
                flow.id(),
                Band {
                    thickness: round(geometry::MINIMUM_THICKNESS.max(thickness), 2),
                    source_at: fraction(
                        geometry.offset_out(&top, flow) + thickness / 2.0,
                        heights[&flow.from],
                        values[&flow.from],
                    ),
                    target_at: fraction(
                        geometry.offset_in(&top, flow) + thickness / 2.0,
                        heights[&flow.to],
                        values[&flow.to],
                    ),
                    backward: columns[&flow.to] <= columns[&flow.from],
                },
            )
        })
        .collect();

    SankeyLayout {
        nodes,
        flows: flows.clone(),
        boxes,
        columns,
        values,
        bands,
        column_order: order,
        scale,
    }
}

/// What the band offsets read: the drawn flows, the heights, the scale and the flows' order in the document.
struct Geometry<'a> {
    flows: &'a [SankeyFlow],
    heights: &'a HashMap<String, f64>,
    scale: f64,
    document_order: &'a HashMap<String, usize>,
}

impl Geometry<'_> {
    fn thickness(&self, flow: &SankeyFlow) -> f64 {
        flow.amount() * self.scale
    }

    fn centre(&self, top: &HashMap<String, f64>, id: &str) -> f64 {
        top[id] + self.heights[id] / 2.0
    }

    /// A node's bands, stacked in the order of the nodes at their other ends, so they do not cross at the bar.
    fn stacked<'f>(
        &'f self,
        top: &HashMap<String, f64>,
        along: impl Iterator<Item = &'f SankeyFlow>,
        other: impl Fn(&SankeyFlow) -> &str,
    ) -> Vec<&'f SankeyFlow> {
        let mut flows: Vec<&SankeyFlow> = along.collect();
        // A stable sort: by the other end's centre, then by the document's order.
        flows.sort_by(|a, b| {
            self.centre(top, other(a))
                .partial_cmp(&self.centre(top, other(b)))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(self.document_order[&a.id()].cmp(&self.document_order[&b.id()]))
        });
        flows
    }

    fn offset_out(&self, top: &HashMap<String, f64>, flow: &SankeyFlow) -> f64 {
        let id = flow.id();
        self.stacked(top, self.flows.iter().filter(|f| f.from == flow.from), |f| &f.to)
            .into_iter()
            .take_while(|other| other.id() != id)
            .fold(0.0, |sum, other| sum + self.thickness(other))
    }

    fn offset_in(&self, top: &HashMap<String, f64>, flow: &SankeyFlow) -> f64 {
        let id = flow.id();
        self.stacked(top, self.flows.iter().filter(|f| f.to == flow.to), |f| &f.from)
            .into_iter()
            .take_while(|other| other.id() != id)
            .fold(0.0, |sum, other| sum + self.thickness(other))
    }
}

/// Moves `id` towards the value-weighted mean of where `along` would run level, by `alpha` of the way.
fn relax(
    top: &mut HashMap<String, f64>,
    id: &str,
    along: &[&SankeyFlow],
    alpha: f64,
    level_top: impl Fn(&HashMap<String, f64>, &SankeyFlow) -> f64,
) {
    let weight = along.iter().fold(0.0, |sum, flow| sum + flow.amount());
    if weight <= 0.0 {
        return;
    }
    let wanted = along
        .iter()
        .fold(0.0, |sum, flow| sum + level_top(top, flow) * flow.amount())
        / weight;
    let current = top[id];
    top.insert(id.to_owned(), current + (wanted - current) * alpha);
}

/// Within the extent, in order, never closer than the gap: pushed down from the top, then up from the bottom, then
/// down once more in case the column is as tall as the extent.
fn resolve(top: &mut HashMap<String, f64>, column: &[String], heights: &HashMap<String, f64>, extent: f64) {
    let floor_of = |top: &HashMap<String, f64>, i: usize| {
        if i == 0 {
            0.0
        } else {
            top[&column[i - 1]] + heights[&column[i - 1]] + geometry::NODE_GAP
        }
    };
    for i in 0..column.len() {
        let floor = floor_of(top, i);
        let value = top[&column[i]].max(floor);
        top.insert(column[i].clone(), value);
    }
    for i in (0..column.len()).rev() {
        let ceiling = if i == column.len() - 1 {
            extent
        } else {
            top[&column[i + 1]] - geometry::NODE_GAP
        };
        let value = top[&column[i]].min(ceiling - heights[&column[i]]);
        top.insert(column[i].clone(), value);
    }
    for i in 0..column.len() {
        let floor = floor_of(top, i);
        let value = top[&column[i]].max(floor);
        top.insert(column[i].clone(), value);
    }
}

/// Where along a bar a band's centre sits; the middle of a bar that carries nothing.
fn fraction(offset: f64, height: f64, value: f64) -> f64 {
    if value <= 0.0 || height <= 0.0 {
        0.5
    } else {
        round((offset / height).clamp(0.0, 1.0), 6)
    }
}

/// Each node's column: the one it states, or one past the furthest node that flows into it over the flows that close
/// no cycle, so a source starts the diagram and a chain reads left to right.
fn columns_of(nodes: &[SankeyNode], flows: &[SankeyFlow]) -> HashMap<String, usize> {
    let mut outgoing: HashMap<String, Vec<String>> = nodes.iter().map(|node| (node.id.clone(), Vec::new())).collect();
    for flow in flows {
        outgoing.get_mut(&flow.from).expect("a drawn end").push(flow.to.clone());
    }
    let ids: Vec<&str> = nodes.iter().map(|node| node.id.as_str()).collect();
    let forward = layering::acyclic_edges(&ids, &outgoing);
    let stated: HashMap<&str, i32> = nodes
        .iter()
        .filter_map(|node| node.column.map(|column| (node.id.as_str(), column - 1)))
        .collect();
    // A node that states its column keeps it, so the flows into it place nothing.
    let edges: Vec<(&str, &str)> = ids
        .iter()
        .flat_map(|id| {
            forward[*id]
                .iter()
                .filter(|target| !stated.contains_key(target.as_str()))
                .map(move |target| (*id, target.as_str()))
        })
        .collect();
    layering::longest_path_with_floor(ids.iter().copied(), edges, CyclePolicy::Kahn, |id| {
        stated.get(id).copied().unwrap_or(0)
    })
    .into_iter()
    .map(|(id, layer)| (id, layer.max(0) as usize))
    .collect()
}
