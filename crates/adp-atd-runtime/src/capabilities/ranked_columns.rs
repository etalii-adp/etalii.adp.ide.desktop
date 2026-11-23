//! `layout.rankedColumns` (ATD §13.3; its contract and fixtures are in `specifications/atd/capabilities/`): nodes in
//! columns by a rank, left to right, each column as wide as its widest node, its nodes stacked from the top in their
//! order; a rank no node has takes no room, a long column wraps, and the nodes without a rank stand in a band below or
//! after the columns. The column placement the Helm chart module's anatomy (`modules/diagrams/helm-chart/src/
//! layout.rs`) does for its five bands, the module keeping which column a node is in (etalii.adp spec 035, T132).

use std::collections::BTreeMap;

use serde_json::Value as Json;

use super::{LayoutCapability, answer, number_argument, pinned_over};

/// A node to place: its id, its size, its rank (none for the band) and the key it is ordered by.
struct Ranked {
    id: String,
    width: f64,
    height: f64,
    rank: Option<i64>,
    order: Option<Json>,
}

/// The ascending order of two order keys: numbers by value, texts by their bytes, a number before a text, an absent key
/// last; ties keep the order of the input, as the sort is stable.
fn order_of(a: &Option<Json>, b: &Option<Json>) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (Some(Json::Number(a)), Some(Json::Number(b))) => {
            a.as_f64().partial_cmp(&b.as_f64()).unwrap_or(Ordering::Equal)
        }
        (Some(Json::String(a)), Some(Json::String(b))) => a.cmp(b),
        (Some(Json::Number(_)), Some(_)) => Ordering::Less,
        (Some(_), Some(Json::Number(_))) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        _ => Ordering::Equal,
    }
}

/// The top-left corner of every node: the columns from `origin`, then the band. `uniformWidth` changes the width a node
/// is drawn at, never where it stands, so it takes no part here.
#[allow(clippy::too_many_arguments)]
fn place(
    nodes: &[Ranked],
    origin: (f64, f64),
    column_gap: f64,
    row_gap: f64,
    wrap_after: Option<usize>,
    wrap_gap: f64,
    band_gap: f64,
    band_after: bool,
) -> Vec<(String, f64, f64)> {
    let mut columns: BTreeMap<i64, Vec<&Ranked>> = BTreeMap::new();
    let mut band: Vec<&Ranked> = Vec::new();
    for node in nodes {
        match node.rank {
            Some(rank) => columns.entry(rank).or_default().push(node),
            None => band.push(node),
        }
    }
    let mut placed = Vec::with_capacity(nodes.len());
    let mut x = origin.0;
    let mut bottom = origin.1;
    let mut any_column = false;
    for (_, mut column) in columns {
        column.sort_by(|a, b| order_of(&a.order, &b.order));
        let chunk = wrap_after.filter(|w| *w > 0).unwrap_or(usize::MAX);
        let parts: Vec<&[&Ranked]> = column.chunks(chunk.min(column.len().max(1))).collect();
        for (index, part) in parts.iter().enumerate() {
            let widest = part.iter().map(|n| n.width).fold(0.0_f64, f64::max);
            let mut y = origin.1;
            for node in part.iter() {
                placed.push((node.id.clone(), x, y));
                y += node.height + row_gap;
                bottom = bottom.max(y - row_gap);
            }
            any_column = true;
            x += widest;
            x += if index + 1 < parts.len() { wrap_gap } else { column_gap };
        }
    }
    if band.is_empty() {
        return placed;
    }
    band.sort_by(|a, b| order_of(&a.order, &b.order));
    if band_after {
        // One column after the last, `bandGap` beyond it.
        let mut y = origin.1;
        let left = if any_column {
            x - column_gap + band_gap
        } else {
            origin.0
        };
        for node in band {
            placed.push((node.id.clone(), left, y));
            y += node.height + row_gap;
        }
    } else {
        // One row below the tallest column, `bandGap` beneath it, the nodes `columnGap` apart.
        let top = if any_column { bottom + band_gap } else { origin.1 };
        let mut left = origin.0;
        for node in band {
            placed.push((node.id.clone(), left, top));
            left += node.width + column_gap;
        }
    }
    placed
}

/// The capability over `$defs/LayoutInput`.
pub struct RankedColumns;

impl LayoutCapability for RankedColumns {
    fn draws_its_sizes(&self) -> bool {
        true
    }

    fn place(&self, input: &Json, args: &Json) -> Result<Vec<(String, f64, f64)>, String> {
        let nodes = input["nodes"].as_array().ok_or("The input has no `nodes`.")?;
        let mut ranked = Vec::with_capacity(nodes.len());
        for node in nodes {
            let in_band = matches!(answer(node, args, "band")?, Some(Json::Bool(true)))
                || matches!(answer(node, args, "band")?, Some(Json::String(ref t)) if t == "true");
            let rank = if in_band {
                None
            } else {
                answer(node, args, "rank")?.and_then(|value| match value {
                    Json::Number(number) => number.as_f64().map(|n| n as i64),
                    Json::String(text) => text.parse::<f64>().ok().map(|n| n as i64),
                    _ => None,
                })
            };
            let order = answer(node, args, "order")?.map(|value| match value {
                Json::String(text) => text.parse::<f64>().map_or(Json::String(text), |n| serde_json::json!(n)),
                other => other,
            });
            ranked.push(Ranked {
                id: node["id"].as_str().unwrap_or_default().to_owned(),
                width: node["width"].as_f64().unwrap_or(0.0),
                height: node["height"].as_f64().unwrap_or(0.0),
                rank,
                order,
            });
        }
        let origin = args["origin"]
            .as_array()
            .map(|o| {
                (
                    o.first().and_then(Json::as_f64).unwrap_or(0.0),
                    o.get(1).and_then(Json::as_f64).unwrap_or(0.0),
                )
            })
            .unwrap_or((0.0, 0.0));
        let placed = place(
            &ranked,
            origin,
            number_argument(args, "columnGap", 60.0),
            number_argument(args, "rowGap", 24.0),
            args["wrapAfter"].as_u64().map(|w| w as usize),
            number_argument(args, "wrapGap", 40.0),
            number_argument(args, "bandGap", 96.0),
            args["bandPlacement"].as_str() == Some("after"),
        );
        Ok(pinned_over(placed, nodes))
    }
}
