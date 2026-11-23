//! `layout.rowPacking` (ATD §13.3; its contract and fixtures are in `specifications/atd/capabilities/`): rows for items
//! whose extent across is the data, in as few rows as the items allow, linked items on rows close together. The shared
//! row packing of `adp-layout` (`row_packing.rs`), which the timeline's and the Gartner hype cycle graph's Arrange
//! diagram call, read here from `$defs/LayoutInput`; itself ported from the standalone host's `RowPacking.cs` at
//! `8b6ccfc`.

use adp_layout::row_packing::{RowItem, pack};
use serde_json::{Map, Value as Json, json};

use super::{LayoutCapability, bool_query, number_argument, number_query, two_sided_tree::round2};

/// The capability over `$defs/LayoutInput`.
pub struct RowPacking;

impl RowPacking {
    /// The items, in input order, each with its top row: the row `adp-layout`'s packing gives it.
    fn rows(&self, input: &Json, args: &Json) -> Result<Vec<(RowItem, usize)>, String> {
        let nodes = input["nodes"].as_array().ok_or("The input has no `nodes`.")?;
        let mut items = Vec::with_capacity(nodes.len());
        for node in nodes {
            let id = node["id"].as_str().unwrap_or_default().to_owned();
            let left = number_query(node, args, "left")?.ok_or_else(|| format!("The node `{id}` has no `left`."))?;
            let right = number_query(node, args, "right")?.unwrap_or(left + node["width"].as_f64().unwrap_or(0.0));
            let rows = node["height"].as_f64().unwrap_or(1.0).max(1.0) as usize;
            items.push(RowItem::tall(id, left, right, rows));
        }
        let edges = input["edges"].as_array().cloned().unwrap_or_default();
        let mut links = Vec::with_capacity(edges.len());
        let mut apart = Vec::new();
        for edge in &edges {
            let (Some(source), Some(target)) = (edge["source"].as_str(), edge["target"].as_str()) else {
                continue;
            };
            links.push((source, target));
            let kept_apart = bool_query(edge, args, "apart")?.unwrap_or(false);
            if kept_apart {
                apart.push((source, target));
            }
        }
        let rows = pack(&items, &links, number_argument(args, "gap", 12.0), &apart);
        Ok(items
            .into_iter()
            .filter_map(|item| {
                let row = *rows.get(&item.id)?;
                Some((item, row))
            })
            .collect())
    }
}

impl LayoutCapability for RowPacking {
    /// The layout keeps no pin: across is the data, and only the row is the layout's.
    fn consults_pins(&self) -> bool {
        false
    }

    fn place(&self, input: &Json, args: &Json) -> Result<Vec<(String, f64, f64)>, String> {
        Ok(self
            .rows(input, args)?
            .into_iter()
            .map(|(item, row)| (item.id, item.left, row as f64))
            .collect())
    }

    /// A position's `y` is a row, counted from 0, and is written as the whole number it is.
    fn layout(&self, input: &Json, args: &Json) -> Result<Json, String> {
        let positions: Map<String, Json> = self
            .rows(input, args)?
            .into_iter()
            .map(|(item, row)| (item.id, json!({"x": round2(item.left), "y": row})))
            .collect();
        Ok(json!({ "positions": positions }))
    }
}
