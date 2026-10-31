//! `layout.layeredColumns` (ATD §13.3; its contract and 14 fixtures are in `specifications/atd/capabilities/`): columns
//! left to right, one row per node within a column, and a band of nodes beneath. Moved (not rewritten) from the Databricks
//! module's layout of the job and pipeline readings, which calls it here (etalii.adp spec 067, T027), itself ported from
//! the standalone host's `DatabricksJobLayout.cs` and `DatabricksPipelineLayout.cs` at `8b6ccfc`.

use std::collections::BTreeMap;

use adp_layout::CyclePolicy;
use adp_layout::layering::longest_path;
use serde_json::Value as Json;

use super::{LayoutCapability, number_argument, query_of};

/// A node to place, in input order. One id may stand twice, as a repeated key does in a module's model: each takes its
/// row, and the later one's place is the one that counts.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnNode {
    pub id: String,
    /// The column the tool type fixes it in; none lets the longest-path layer decide.
    pub column: Option<i32>,
    /// Whether it stands in the band beneath the columns.
    pub band: bool,
    /// Whether it is placed one column before its first successor.
    pub placeholder: bool,
}

impl ColumnNode {
    /// A node the longest-path layer places.
    pub fn layered(id: String) -> Self {
        ColumnNode {
            id,
            column: None,
            band: false,
            placeholder: false,
        }
    }
}

/// The capability's arguments.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColumnArguments {
    pub column_width: f64,
    pub row_height: f64,
    pub band_gap: f64,
    /// Whether a column's rows are centred on the tallest column, rather than starting at the top.
    pub middle: bool,
}

impl Default for ColumnArguments {
    fn default() -> Self {
        ColumnArguments {
            column_width: 260.0,
            row_height: 120.0,
            band_gap: 80.0,
            middle: false,
        }
    }
}

/// The top-left corner of every node along `edges`, each edge from the node that comes first to the node that waits for
/// it: the column nodes, in input order with the placeholders last, then the band's.
pub fn place_nodes(
    nodes: &[ColumnNode],
    edges: &[(String, String)],
    arguments: &ColumnArguments,
) -> Vec<(String, f64, f64)> {
    let in_columns = |node: &&ColumnNode| !node.band && !node.placeholder;
    let layered = nodes
        .iter()
        .filter(in_columns)
        .filter(|node| node.column.is_none())
        .map(|node| node.id.as_str());
    let walked = edges.iter().map(|(from, to)| (from.as_str(), to.as_str()));
    let layers = longest_path(layered, walked, CyclePolicy::BackEdgeIgnored);
    let column_of = |node: &ColumnNode| {
        node.column
            .unwrap_or_else(|| layers.get(&node.id).copied().unwrap_or_default())
    };

    // The column of each node, the columns' nodes first and the placeholders after them.
    let mut placed: Vec<(&str, i32)> = nodes
        .iter()
        .filter(in_columns)
        .map(|n| (n.id.as_str(), column_of(n)))
        .collect();
    for placeholder in nodes.iter().filter(|node| node.placeholder && !node.band) {
        let successor = nodes
            .iter()
            .filter(in_columns)
            .find(|node| edges.iter().any(|(from, to)| *from == placeholder.id && *to == node.id));
        let column = successor.map_or(0, |node| (column_of(node) - 1).max(0));
        placed.push((placeholder.id.as_str(), column));
    }

    let mut rows_of: BTreeMap<i32, i32> = BTreeMap::new();
    for (_, column) in &placed {
        *rows_of.entry(*column).or_default() += 1;
    }
    let tallest = rows_of.values().copied().max().unwrap_or(0);
    let mut next: BTreeMap<i32, i32> = BTreeMap::new();
    let mut positions: Vec<(String, f64, f64)> = placed
        .into_iter()
        .map(|(id, column)| {
            let row = next.entry(column).or_default();
            let mut y = f64::from(*row) * arguments.row_height;
            *row += 1;
            if arguments.middle {
                y += f64::from(tallest - rows_of[&column]) * arguments.row_height / 2.0;
            }
            (id.to_owned(), f64::from(column) * arguments.column_width, y)
        })
        .collect();

    let band_y = f64::from(tallest.max(1)) * arguments.row_height + arguments.band_gap;
    positions.extend(
        nodes
            .iter()
            .filter(|node| node.band)
            .enumerate()
            .map(|(index, node)| (node.id.clone(), index as f64 * arguments.column_width, band_y)),
    );
    positions
}

/// The capability over `$defs/LayoutInput`.
pub struct LayeredColumns;

impl LayoutCapability for LayeredColumns {
    fn place(&self, input: &Json, args: &Json) -> Result<Vec<(String, f64, f64)>, String> {
        let defaults = ColumnArguments::default();
        let arguments = ColumnArguments {
            column_width: number_argument(args, "columnWidth", defaults.column_width),
            row_height: number_argument(args, "rowHeight", defaults.row_height),
            band_gap: number_argument(args, "bandGap", defaults.band_gap),
            middle: args["align"].as_str() == Some("middle"),
        };
        let given = input["nodes"].as_array().ok_or("The input has no `nodes`.")?;
        let mut nodes = Vec::with_capacity(given.len());
        for node in given {
            nodes.push(ColumnNode {
                id: node["id"].as_str().unwrap_or_default().to_owned(),
                column: query_of(node, args, "column")?
                    .and_then(|value| value.as_number())
                    .map(|column| column as i32),
                band: query_of(node, args, "band")?.is_some_and(|value| value == adp_cel::Value::Bool(true)),
                placeholder: query_of(node, args, "placeholder")?
                    .is_some_and(|value| value == adp_cel::Value::Bool(true)),
            });
        }
        let edges: Vec<(String, String)> = input["edges"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|edge| {
                let end = |name: &str| edge[name].as_str().unwrap_or_default().to_owned();
                (end("source"), end("target"))
            })
            .collect();
        Ok(super::pinned_over(place_nodes(&nodes, &edges, &arguments), given))
    }
}
