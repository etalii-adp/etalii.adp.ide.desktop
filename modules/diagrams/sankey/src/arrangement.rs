//! The order "Arrange diagram" gives the nodes of each column: the one whose bands cross least, weighed by how thick
//! the crossing bands are (etalii.adp spec 030, task T077).
//!
//! Ported from the standalone host's `SankeyArrangement.cs` at `8b6ccfc`. The order within a column is all a reader
//! arranges, so the clutter left to remove is bands crossing each other: the layered crossing-minimisation problem,
//! answered by the barycentre method, sweeping the columns left to right and back, each node taking the place its
//! neighbours' positions pull it to, weighted by how much flows between them. A position is where a bar's middle sits
//! in its column's stack of values; two bands crossing cost the product of their values. Only an improvement is kept:
//! every sweep is scored and the best arrangement seen is returned, the reader's own first, so arranging an arranged
//! diagram changes nothing.

use std::collections::{HashMap, VecDeque};

use crate::layout::SankeyLayout;
use crate::model::SankeyModel;

/// How many sweeps run, alternating direction.
const SWEEPS: usize = 24;

/// A band between two columns, from its left end to its right end.
struct Span {
    left: String,
    right: String,
    value: f64,
}

/// The node ids of each column, top to bottom, as the arrangement orders them.
fn columns_of(layout: &SankeyLayout) -> Vec<Vec<String>> {
    let columns = &layout.columns;
    let values = &layout.values;
    let flows = flows_of(layout);

    let mut current = layout.column_order.clone();
    let mut best = current.clone();
    let mut best_cost = cost(&best, &flows, columns, values);

    let mut sweep = 0;
    while sweep < SWEEPS && best_cost > 0.0 {
        let rightwards = sweep % 2 == 0;
        let mut positions = positions_of(&current, values);
        let range: Vec<usize> = if rightwards {
            (1..current.len().max(1)).collect()
        } else {
            (0..current.len().saturating_sub(1)).rev().collect()
        };
        for c in range {
            // Each node towards the value-weighted mean of its neighbours on the side the sweep comes from; a node
            // with none there keeps where it is.
            let pulls: HashMap<&str, f64> = current[c]
                .iter()
                .map(|id| {
                    let along: Vec<(&str, f64)> = flows
                        .iter()
                        .filter(|flow| {
                            if rightwards {
                                &flow.right == id
                            } else {
                                &flow.left == id
                            }
                        })
                        .map(|flow| {
                            (
                                if rightwards {
                                    flow.left.as_str()
                                } else {
                                    flow.right.as_str()
                                },
                                flow.value,
                            )
                        })
                        .collect();
                    let weight = along.iter().fold(0.0, |sum, (_, value)| sum + value);
                    let pull = if weight > 0.0 {
                        along
                            .iter()
                            .fold(0.0, |sum, (other, value)| sum + positions[*other] * value)
                            / weight
                    } else {
                        positions[id.as_str()]
                    };
                    (id.as_str(), pull)
                })
                .collect();
            // A stable sort: two nodes pulled to the same place keep their order.
            let mut sorted = current[c].clone();
            sorted.sort_by(|a, b| {
                pulls[a.as_str()]
                    .partial_cmp(&pulls[b.as_str()])
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            current[c] = sorted;
            positions = positions_of(&current, values);
        }

        let swept = cost(&current, &flows, columns, values);
        if swept < best_cost - 1e-9 {
            best_cost = swept;
            best = current.clone();
        }
        sweep += 1;
    }
    best
}

/// How much the bands of the diagram as it reads now cross: for every two crossing bands, the product of their values.
/// Zero means no band crosses another.
pub fn crossings_of(model: &SankeyModel) -> f64 {
    let layout = SankeyLayout::of(model);
    cost(
        &layout.column_order,
        &flows_of(&layout),
        &layout.columns,
        &layout.values,
    )
}

/// The drawable nodes in the order the document should list them: each column's nodes take the places that column's
/// nodes held, in their new order, so a document that lists the nodes column by column, or interleaved, keeps its
/// shape.
pub fn document_order_of(model: &SankeyModel) -> Vec<String> {
    let layout = SankeyLayout::of(model);
    let mut queues: Vec<VecDeque<String>> = columns_of(&layout).into_iter().map(VecDeque::from).collect();
    layout
        .nodes
        .iter()
        .map(|node| {
            queues[layout.columns[&node.id]]
                .pop_front()
                .expect("a column holds as many nodes as it held")
        })
        .collect()
}

/// The bands that run between two columns, each from its left end to its right end.
fn flows_of(layout: &SankeyLayout) -> Vec<Span> {
    let columns = &layout.columns;
    layout
        .flows
        .iter()
        .filter(|flow| columns[&flow.from] != columns[&flow.to])
        .map(|flow| {
            let value = flow.amount();
            if columns[&flow.from] < columns[&flow.to] {
                Span {
                    left: flow.from.clone(),
                    right: flow.to.clone(),
                    value,
                }
            } else {
                Span {
                    left: flow.to.clone(),
                    right: flow.from.clone(),
                    value,
                }
            }
        })
        .collect()
}

/// Where each node's middle sits in its column, as a fraction of the column's total value.
fn positions_of(columns: &[Vec<String>], values: &HashMap<String, f64>) -> HashMap<String, f64> {
    // A node that carries nothing still takes a little room, so empty nodes do not all collapse onto one point.
    let weight = |id: &str| values[id].max(1e-6);
    let mut positions = HashMap::new();
    for column in columns {
        let total = column.iter().fold(0.0, |sum, id| sum + weight(id));
        let mut above = 0.0;
        for id in column {
            positions.insert(id.clone(), (above + weight(id) / 2.0) / total);
            above += weight(id);
        }
    }
    positions
}

/// How much the bands cross: for every two bands whose columns overlap, the product of their values when their order
/// at one end of the overlap differs from their order at the other.
fn cost(
    columns: &[Vec<String>],
    flows: &[Span],
    column_of: &HashMap<String, usize>,
    values: &HashMap<String, f64>,
) -> f64 {
    let positions = positions_of(columns, values);
    let at = |flow: &Span, column: f64| {
        let from = column_of[&flow.left] as f64;
        let to = column_of[&flow.right] as f64;
        let t = (column - from) / (to - from);
        positions[&flow.left] + (positions[&flow.right] - positions[&flow.left]) * t
    };
    let mut cost = 0.0;
    for i in 0..flows.len() {
        for j in i + 1..flows.len() {
            let (a, b) = (&flows[i], &flows[j]);
            if a.left == b.left || a.right == b.right {
                // Bands sharing an end are stacked by the layout so they never cross there.
                continue;
            }
            let start = column_of[&a.left].max(column_of[&b.left]) as f64;
            let end = column_of[&a.right].min(column_of[&b.right]) as f64;
            if end <= start {
                continue;
            }
            let before = at(a, start) - at(b, start);
            let after = at(a, end) - at(b, end);
            if before * after < 0.0 {
                cost += a.value.max(1e-6) * b.value.max(1e-6);
            }
        }
    }
    cost
}
