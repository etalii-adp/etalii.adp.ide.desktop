//! `layout.flowRelaxation` (ATD §13.3; its contract and fixtures are in `specifications/atd/capabilities/`): nodes in
//! columns, each column's vertical positions relaxed so that the weighted edges between columns run as level as they
//! can, the layered flow layout of a Sankey diagram. Ported from the Sankey module's layout
//! (`modules/diagrams/sankey/src/layout.rs`), itself ported from the standalone host's `SankeyLayout.cs` at
//! `8b6ccfc`, number for number, with the columns, heights, weights and thicknesses given rather than computed.

use std::collections::HashMap;

use serde_json::Value as Json;

use super::{LayoutCapability, number_argument, number_query};

/// A node to place: its id, its column and its height.
#[derive(Debug, Clone, PartialEq)]
pub struct FlowNode {
    pub id: String,
    pub column: usize,
    pub height: f64,
}

/// An edge between two nodes to place: its id, its ends, its weight and its thickness where it is stacked.
#[derive(Debug, Clone, PartialEq)]
pub struct FlowEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub weight: f64,
    pub thickness: f64,
}

/// The gaps and the relaxation of the layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlowArguments {
    pub origin: (f64, f64),
    pub column_gap: f64,
    pub node_gap: f64,
    pub passes: usize,
    pub decay: f64,
}

impl Default for FlowArguments {
    fn default() -> Self {
        FlowArguments {
            origin: (0.0, 0.0),
            column_gap: 320.0,
            node_gap: 36.0,
            passes: 24,
            decay: 0.99,
        }
    }
}

/// Where the layout puts everything: each node's top (unrounded) and where each edge's centre leaves and reaches its
/// ends, as a fraction of their heights (unrounded).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Relaxed {
    pub top: HashMap<String, f64>,
    pub source_at: HashMap<String, f64>,
    pub target_at: HashMap<String, f64>,
    pub values: HashMap<String, f64>,
}

/// The tops of `nodes`, columns listing them in their order, and the ends of `edges`, as the contract states them.
pub fn relax(nodes: &[FlowNode], edges: &[FlowEdge], arguments: &FlowArguments) -> Relaxed {
    let heights: HashMap<&str, f64> = nodes.iter().map(|n| (n.id.as_str(), n.height)).collect();
    let column_of: HashMap<&str, usize> = nodes.iter().map(|n| (n.id.as_str(), n.column)).collect();
    let mut inflow: HashMap<&str, f64> = nodes.iter().map(|n| (n.id.as_str(), 0.0)).collect();
    let mut outflow = inflow.clone();
    for edge in edges {
        *outflow.get_mut(edge.from.as_str()).expect("a placed end") += edge.weight;
        *inflow.get_mut(edge.to.as_str()).expect("a placed end") += edge.weight;
    }
    let values: HashMap<String, f64> = nodes
        .iter()
        .map(|n| (n.id.clone(), inflow[n.id.as_str()].max(outflow[n.id.as_str()])))
        .collect();
    let column_count = nodes.iter().map(|n| n.column + 1).max().unwrap_or(0);
    let order: Vec<Vec<String>> = (0..column_count)
        .map(|column| {
            nodes
                .iter()
                .filter(|n| n.column == column)
                .map(|n| n.id.clone())
                .collect()
        })
        .collect();
    let gap = arguments.node_gap;
    let stacked = |column: &Vec<String>| {
        column.iter().fold(0.0, |sum, id| sum + heights[id.as_str()]) + column.len().saturating_sub(1) as f64 * gap
    };
    let extent = order
        .iter()
        .map(stacked)
        .fold(None, |max: Option<f64>, height| {
            Some(max.map_or(height, |m| m.max(height)))
        })
        .unwrap_or(0.0);
    let forward: Vec<&FlowEdge> = edges
        .iter()
        .filter(|e| column_of[e.to.as_str()] > column_of[e.from.as_str()])
        .collect();
    let input_order: HashMap<&str, usize> = edges.iter().enumerate().map(|(i, e)| (e.id.as_str(), i)).collect();

    let mut top: HashMap<String, f64> = HashMap::new();
    for column in &order {
        let mut y = (extent - stacked(column)) / 2.0;
        for id in column {
            top.insert(id.clone(), y);
            y += heights[id.as_str()] + gap;
        }
    }
    let geometry = Geometry {
        edges,
        heights: &heights,
        input_order: &input_order,
    };
    let forward_in = |id: &str| -> Vec<&FlowEdge> { forward.iter().copied().filter(|e| e.to == id).collect() };
    let forward_out = |id: &str| -> Vec<&FlowEdge> { forward.iter().copied().filter(|e| e.from == id).collect() };
    for pass in 0..arguments.passes {
        let alpha = arguments.decay.powf(pass as f64);
        // Right to left: each node towards where its edges out would run level.
        for c in (0..column_count.saturating_sub(1)).rev() {
            for id in &order[c] {
                let along = forward_out(id);
                move_towards(&mut top, id, &along, alpha, |top, edge| {
                    top[&edge.to] + geometry.offset_in(top, edge) - geometry.offset_out(top, edge)
                });
            }
            resolve(&mut top, &order[c], &heights, extent, gap);
        }
        // Left to right: each node towards where its edges in would run level.
        for column in order.iter().skip(1) {
            for id in column {
                let along = forward_in(id);
                move_towards(&mut top, id, &along, alpha, |top, edge| {
                    top[&edge.from] + geometry.offset_out(top, edge) - geometry.offset_in(top, edge)
                });
            }
            resolve(&mut top, column, &heights, extent, gap);
        }
    }
    for column in &order {
        resolve(&mut top, column, &heights, extent, gap);
    }
    let fraction = |offset: f64, id: &str| {
        let (height, value) = (heights[id], values[id]);
        if value <= 0.0 || height <= 0.0 {
            0.5
        } else {
            (offset / height).clamp(0.0, 1.0)
        }
    };
    let mut relaxed = Relaxed::default();
    for edge in edges {
        relaxed.source_at.insert(
            edge.id.clone(),
            fraction(geometry.offset_out(&top, edge) + edge.thickness / 2.0, &edge.from),
        );
        relaxed.target_at.insert(
            edge.id.clone(),
            fraction(geometry.offset_in(&top, edge) + edge.thickness / 2.0, &edge.to),
        );
    }
    relaxed.top = top;
    relaxed.values = values;
    relaxed
}

/// What the edge offsets read: the edges, the heights and the edges' order in the input.
struct Geometry<'a> {
    edges: &'a [FlowEdge],
    heights: &'a HashMap<&'a str, f64>,
    input_order: &'a HashMap<&'a str, usize>,
}

impl Geometry<'_> {
    fn centre(&self, top: &HashMap<String, f64>, id: &str) -> f64 {
        top[id] + self.heights[id] / 2.0
    }

    /// A node's edges, stacked in the order of the centres of the nodes at their other ends, then of the input.
    fn stacked<'f>(
        &'f self,
        top: &HashMap<String, f64>,
        along: impl Iterator<Item = &'f FlowEdge>,
        other: impl Fn(&FlowEdge) -> &str,
    ) -> Vec<&'f FlowEdge> {
        let mut edges: Vec<&FlowEdge> = along.collect();
        edges.sort_by(|a, b| {
            self.centre(top, other(a))
                .partial_cmp(&self.centre(top, other(b)))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(self.input_order[a.id.as_str()].cmp(&self.input_order[b.id.as_str()]))
        });
        edges
    }

    fn offset_out(&self, top: &HashMap<String, f64>, edge: &FlowEdge) -> f64 {
        self.stacked(top, self.edges.iter().filter(|e| e.from == edge.from), |e| &e.to)
            .into_iter()
            .take_while(|other| other.id != edge.id)
            .fold(0.0, |sum, other| sum + other.thickness)
    }

    fn offset_in(&self, top: &HashMap<String, f64>, edge: &FlowEdge) -> f64 {
        self.stacked(top, self.edges.iter().filter(|e| e.to == edge.to), |e| &e.from)
            .into_iter()
            .take_while(|other| other.id != edge.id)
            .fold(0.0, |sum, other| sum + other.thickness)
    }
}

/// Moves `id` towards the weight-weighted mean of where `along` would run level, by `alpha` of the way.
fn move_towards(
    top: &mut HashMap<String, f64>,
    id: &str,
    along: &[&FlowEdge],
    alpha: f64,
    level_top: impl Fn(&HashMap<String, f64>, &FlowEdge) -> f64,
) {
    let weight = along.iter().fold(0.0, |sum, edge| sum + edge.weight);
    if weight <= 0.0 {
        return;
    }
    let wanted = along
        .iter()
        .fold(0.0, |sum, edge| sum + level_top(top, edge) * edge.weight)
        / weight;
    let current = top[id];
    top.insert(id.to_owned(), current + (wanted - current) * alpha);
}

/// Within the extent, in order, never closer than the gap: pushed down from the top, then up from the bottom, then
/// down once more in case the column is as tall as the extent.
fn resolve(top: &mut HashMap<String, f64>, column: &[String], heights: &HashMap<&str, f64>, extent: f64, gap: f64) {
    let floor_of = |top: &HashMap<String, f64>, i: usize| {
        if i == 0 {
            0.0
        } else {
            top[&column[i - 1]] + heights[column[i - 1].as_str()] + gap
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
            top[&column[i + 1]] - gap
        };
        let value = top[&column[i]].min(ceiling - heights[column[i].as_str()]);
        top.insert(column[i].clone(), value);
    }
    for i in 0..column.len() {
        let floor = floor_of(top, i);
        let value = top[&column[i]].max(floor);
        top.insert(column[i].clone(), value);
    }
}

/// .NET's `Math.Round(value, digits)`: half to even, on the value scaled by a power of ten.
pub fn round(value: f64, digits: i32) -> f64 {
    let power = 10f64.powi(digits);
    (value * power).round_ties_even() / power
}

/// The nodes and edges of `$defs/LayoutInput` that take part, with the arguments.
fn read(input: &Json, args: &Json) -> Result<(Vec<FlowNode>, Vec<FlowEdge>, FlowArguments), String> {
    let mut nodes = Vec::new();
    for node in input["nodes"].as_array().ok_or("The input has no `nodes`.")? {
        let Some(column) = number_query(node, args, "column")? else {
            continue;
        };
        nodes.push(FlowNode {
            id: node["id"].as_str().unwrap_or_default().to_owned(),
            column: column.max(0.0) as usize,
            height: node["height"].as_f64().unwrap_or(0.0),
        });
    }
    let placed: std::collections::HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
    let mut edges = Vec::new();
    for edge in input["edges"].as_array().into_iter().flatten() {
        let (Some(from), Some(to)) = (edge["source"].as_str(), edge["target"].as_str()) else {
            continue;
        };
        if from == to || !placed.contains(from) || !placed.contains(to) {
            continue;
        }
        let Some(weight) = number_query(edge, args, "weight")? else {
            continue;
        };
        edges.push(FlowEdge {
            id: edge["id"].as_str().unwrap_or_default().to_owned(),
            from: from.to_owned(),
            to: to.to_owned(),
            weight,
            thickness: number_query(edge, args, "thickness")?.unwrap_or(0.0),
        });
    }
    let origin = args["origin"].as_array();
    let at = |i: usize| origin.and_then(|o| o.get(i)).and_then(Json::as_f64).unwrap_or(0.0);
    let default = FlowArguments::default();
    let arguments = FlowArguments {
        origin: (at(0), at(1)),
        column_gap: number_argument(args, "columnGap", default.column_gap),
        node_gap: number_argument(args, "nodeGap", default.node_gap),
        passes: number_argument(args, "passes", default.passes as f64) as usize,
        decay: number_argument(args, "decay", default.decay),
    };
    Ok((nodes, edges, arguments))
}

/// The capability over `$defs/LayoutInput`.
pub struct FlowRelaxation;

impl LayoutCapability for FlowRelaxation {
    /// A bar is drawn at the height it was placed with, which its value gives it.
    fn draws_its_sizes(&self) -> bool {
        true
    }

    /// A bar's height is its value on the diagram's scale, which the edges stacked at it fill exactly.
    fn exact_sizes(&self) -> bool {
        true
    }

    fn place(&self, input: &Json, args: &Json) -> Result<Vec<(String, f64, f64)>, String> {
        let (nodes, edges, arguments) = read(input, args)?;
        let relaxed = relax(&nodes, &edges, &arguments);
        Ok(nodes
            .iter()
            .map(|n| {
                let x = arguments.origin.0 + n.column as f64 * arguments.column_gap;
                let y = arguments.origin.1 + round(relaxed.top[&n.id], 2);
                (n.id.clone(), x, y)
            })
            .collect())
    }

    fn values(&self, input: &Json, args: &Json) -> Result<super::Values, String> {
        let (nodes, edges, arguments) = read(input, args)?;
        let relaxed = relax(&nodes, &edges, &arguments);
        let mut values: super::Values = nodes
            .iter()
            .map(|n| {
                let top = arguments.origin.1 + round(relaxed.top[&n.id], 2);
                (n.id.clone(), vec![("top".to_owned(), top)])
            })
            .collect();
        for edge in &edges {
            values.push((
                edge.id.clone(),
                vec![
                    ("sourceAt".to_owned(), round(relaxed.source_at[&edge.id], 6)),
                    ("targetAt".to_owned(), round(relaxed.target_at[&edge.id], 6)),
                ],
            ));
        }
        Ok(values)
    }
}
