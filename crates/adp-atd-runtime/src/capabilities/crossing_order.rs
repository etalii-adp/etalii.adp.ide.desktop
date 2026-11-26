//! `layout.crossingOrder` (ATD §13.3; its contract is in `specifications/atd/capabilities/`): the order within each
//! column of a layered diagram whose weighted edges cross least, by the barycentre method. Ported from the Sankey
//! module's arrangement (`modules/diagrams/sankey/src/arrangement.rs`), itself ported from the standalone host's
//! `SankeyArrangement.cs` at `8b6ccfc`, with the columns and the weights given rather than computed.

use std::collections::HashMap;

use serde_json::Value as Json;

use super::{LayoutCapability, number_argument, number_query};

/// The least weight a node or an edge counts with, so that one that carries nothing still takes a little room.
const LEAST: f64 = 1e-6;

/// An edge between two columns, from its end in the lower column to its end in the higher.
struct Span {
    left: String,
    right: String,
    weight: f64,
}

/// What the capability reads of its input: each column's node ids in the input's order, each node's column, each
/// node's weight and the edges between columns.
struct Layered {
    columns: Vec<Vec<String>>,
    column_of: HashMap<String, usize>,
    weights: HashMap<String, f64>,
    spans: Vec<Span>,
}

fn read(input: &Json, args: &Json) -> Result<Layered, String> {
    let mut column_of = HashMap::new();
    let mut order = Vec::new();
    for node in input["nodes"].as_array().ok_or("The input has no `nodes`.")? {
        let Some(column) = number_query(node, args, "column")? else {
            continue;
        };
        let id = node["id"].as_str().unwrap_or_default().to_owned();
        column_of.insert(id.clone(), column.max(0.0) as usize);
        order.push(id);
    }
    let count = column_of.values().max().map_or(0, |c| c + 1);
    let mut columns = vec![Vec::new(); count];
    for id in &order {
        columns[column_of[id]].push(id.clone());
    }
    let mut spans = Vec::new();
    let (mut ins, mut outs): (HashMap<String, f64>, HashMap<String, f64>) = (HashMap::new(), HashMap::new());
    for edge in input["edges"].as_array().into_iter().flatten() {
        let (Some(from), Some(to)) = (edge["source"].as_str(), edge["target"].as_str()) else {
            continue;
        };
        let (Some(&from_column), Some(&to_column)) = (column_of.get(from), column_of.get(to)) else {
            continue;
        };
        if from_column == to_column {
            continue;
        }
        let Some(weight) = number_query(edge, args, "weight")? else {
            continue;
        };
        *outs.entry(from.to_owned()).or_default() += weight;
        *ins.entry(to.to_owned()).or_default() += weight;
        let (left, right) = if from_column < to_column {
            (from, to)
        } else {
            (to, from)
        };
        spans.push(Span {
            left: left.to_owned(),
            right: right.to_owned(),
            weight,
        });
    }
    let weights = order
        .iter()
        .map(|id| {
            let into = ins.get(id).copied().unwrap_or(0.0);
            let out_of = outs.get(id).copied().unwrap_or(0.0);
            (id.clone(), into.max(out_of).max(LEAST))
        })
        .collect();
    Ok(Layered {
        columns,
        column_of,
        weights,
        spans,
    })
}

/// Where each node's middle sits in its column's stack of weights, as a fraction of the column's total.
fn positions_of(columns: &[Vec<String>], weights: &HashMap<String, f64>) -> HashMap<String, f64> {
    let mut positions = HashMap::new();
    for column in columns {
        let total: f64 = column.iter().map(|id| weights[id]).sum();
        let mut above = 0.0;
        for id in column {
            positions.insert(id.clone(), (above + weights[id] / 2.0) / total);
            above += weights[id];
        }
    }
    positions
}

/// How much the edges cross: for every two edges that share no end, the product of their weights when their order at
/// one end of the columns both span differs from their order at the other.
fn cost(layered: &Layered, columns: &[Vec<String>]) -> f64 {
    let positions = positions_of(columns, &layered.weights);
    let column_of = &layered.column_of;
    let at = |span: &Span, column: f64| {
        let from = column_of[&span.left] as f64;
        let to = column_of[&span.right] as f64;
        let t = (column - from) / (to - from);
        positions[&span.left] + (positions[&span.right] - positions[&span.left]) * t
    };
    let spans = &layered.spans;
    let mut cost = 0.0;
    for i in 0..spans.len() {
        for j in i + 1..spans.len() {
            let (a, b) = (&spans[i], &spans[j]);
            if a.left == b.left || a.right == b.right {
                continue;
            }
            let start = column_of[&a.left].max(column_of[&b.left]) as f64;
            let end = column_of[&a.right].min(column_of[&b.right]) as f64;
            if end <= start {
                continue;
            }
            if (at(a, start) - at(b, start)) * (at(a, end) - at(b, end)) < 0.0 {
                cost += a.weight.max(LEAST) * b.weight.max(LEAST);
            }
        }
    }
    cost
}

/// Each column's node ids, top to bottom, in the order whose edges cross least: the cheapest of the input's own order
/// and every sweep's, the input's first.
fn arranged(layered: &Layered, sweeps: usize) -> Vec<Vec<String>> {
    let mut current = layered.columns.clone();
    let mut best = current.clone();
    let mut best_cost = cost(layered, &best);
    let mut sweep = 0;
    while sweep < sweeps && best_cost > 0.0 {
        let rightwards = sweep % 2 == 0;
        let mut positions = positions_of(&current, &layered.weights);
        let range: Vec<usize> = if rightwards {
            (1..current.len().max(1)).collect()
        } else {
            (0..current.len().saturating_sub(1)).rev().collect()
        };
        for c in range {
            let pulls: HashMap<String, f64> = current[c]
                .iter()
                .map(|id| {
                    let along: Vec<(&str, f64)> = layered
                        .spans
                        .iter()
                        .filter(|span| {
                            if rightwards {
                                &span.right == id
                            } else {
                                &span.left == id
                            }
                        })
                        .map(|span| {
                            (
                                if rightwards {
                                    span.left.as_str()
                                } else {
                                    span.right.as_str()
                                },
                                span.weight,
                            )
                        })
                        .collect();
                    let weight: f64 = along.iter().map(|(_, w)| w).sum();
                    let pull = if weight > 0.0 {
                        along.iter().map(|(other, w)| positions[*other] * w).sum::<f64>() / weight
                    } else {
                        positions[id]
                    };
                    (id.clone(), pull)
                })
                .collect();
            current[c].sort_by(|a, b| pulls[a].partial_cmp(&pulls[b]).unwrap_or(std::cmp::Ordering::Equal));
            positions = positions_of(&current, &layered.weights);
        }
        let swept = cost(layered, &current);
        if swept < best_cost - 1e-9 {
            best_cost = swept;
            best = current.clone();
        }
        sweep += 1;
    }
    best
}

/// The capability over `$defs/LayoutInput`.
pub struct CrossingOrder;

impl CrossingOrder {
    fn columns(input: &Json, args: &Json) -> Result<Vec<Vec<String>>, String> {
        let layered = read(input, args)?;
        Ok(arranged(&layered, number_argument(args, "sweeps", 24.0) as usize))
    }
}

impl LayoutCapability for CrossingOrder {
    /// Each column's nodes in their order from `origin`, `columnGap` apart and `nodeGap` between nodes.
    fn place(&self, input: &Json, args: &Json) -> Result<Vec<(String, f64, f64)>, String> {
        let heights: HashMap<&str, f64> = input["nodes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|n| Some((n["id"].as_str()?, n["height"].as_f64().unwrap_or(0.0))))
            .collect();
        let origin = args["origin"].as_array();
        let at = |i: usize| origin.and_then(|o| o.get(i)).and_then(Json::as_f64).unwrap_or(0.0);
        let column_gap = number_argument(args, "columnGap", 320.0);
        let node_gap = number_argument(args, "nodeGap", 36.0);
        let mut placed = Vec::new();
        for (c, column) in Self::columns(input, args)?.into_iter().enumerate() {
            let mut y = at(1);
            for id in column {
                let height = heights.get(id.as_str()).copied().unwrap_or(0.0);
                placed.push((id, at(0) + c as f64 * column_gap, y));
                y += height + node_gap;
            }
        }
        Ok(placed)
    }

    /// Each node's `rank`, its place in its column from 0.
    fn values(&self, input: &Json, args: &Json) -> Result<super::Values, String> {
        Ok(Self::columns(input, args)?
            .into_iter()
            .flat_map(|column| {
                column
                    .into_iter()
                    .enumerate()
                    .map(|(rank, id)| (id, vec![("rank".to_owned(), rank as f64)]))
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn args() -> Json {
        json!({ "column": "self.column", "weight": "self.weight" })
    }

    #[test]
    fn two_crossing_bands_are_uncrossed() {
        let input = json!({
            "nodes": [
                { "id": "a", "attributes": { "column": "0" } },
                { "id": "b", "attributes": { "column": "0" } },
                { "id": "x", "attributes": { "column": "1" } },
                { "id": "y", "attributes": { "column": "1" } }
            ],
            "edges": [
                { "id": "a->y", "source": "a", "target": "y", "attributes": { "weight": "1" } },
                { "id": "b->x", "source": "b", "target": "x", "attributes": { "weight": "1" } }
            ]
        });
        let ranks: HashMap<String, f64> = CrossingOrder
            .values(&input, &args())
            .unwrap()
            .into_iter()
            .map(|(id, v)| (id, v[0].1))
            .collect();
        assert_eq!((ranks["a"], ranks["b"]), (0.0, 1.0));
        assert_eq!((ranks["y"], ranks["x"]), (0.0, 1.0));
    }

    #[test]
    fn an_arranged_input_comes_back_unchanged() {
        let input = json!({
            "nodes": [
                { "id": "a", "attributes": { "column": "0" } },
                { "id": "b", "attributes": { "column": "0" } },
                { "id": "x", "attributes": { "column": "1" } },
                { "id": "y", "attributes": { "column": "1" } }
            ],
            "edges": [
                { "id": "a->x", "source": "a", "target": "x", "attributes": { "weight": "1" } },
                { "id": "b->y", "source": "b", "target": "y", "attributes": { "weight": "1" } }
            ]
        });
        let order: Vec<String> = CrossingOrder
            .values(&input, &args())
            .unwrap()
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(order, ["a", "b", "x", "y"]);
    }
}
