//! `layout.rowPacked` (ATD §13.3; its contract and fixtures are in `specifications/atd/capabilities/`): a drawing read
//! for its order rather than its durations, every node packed along the rows it covers in the order of its left edge.
//! The `row-packed` layout mode of `adp-layout` (`row_packed.rs`), which the Gartner hype cycle graph's Compact
//! viewpoint calls, read here from `$defs/LayoutInput`; itself ported from the standalone host's `rowPackedLayout.ts`
//! at `8b6ccfc`.

use adp_layout::{
    LayoutAlgorithm, LayoutConnection, LayoutDefinition, LayoutElement, LayoutInput, RowPackedDeclaration,
    RowPackedLayout, RowPackedWidth,
};
use serde_json::{Map, Value as Json, json};

use super::{LayoutCapability, number_argument, number_query, two_sided_tree::round2};

/// The capability over `$defs/LayoutInput`.
pub struct RowPacked;

/// A node as it is packed: its id, its top-left corner, its own size and the width it is drawn at.
struct Placed {
    id: String,
    x: f64,
    y: f64,
    width: Option<f64>,
}

impl RowPacked {
    fn pack(&self, input: &Json, args: &Json) -> Result<Vec<Placed>, String> {
        let nodes = input["nodes"].as_array().ok_or("The input has no `nodes`.")?;
        let declaration = RowPackedDeclaration {
            width: args["width"]
                .as_f64()
                .map_or(RowPackedWidth::Bound, RowPackedWidth::Fixed),
            gap: number_argument(args, "gap", 4.0),
            types: args["types"].as_array().map(|types| {
                types
                    .iter()
                    .filter_map(|kind| kind.as_str().map(str::to_owned))
                    .collect()
            }),
            row_step: args["rowStep"].as_f64(),
            follow_connections: args["followConnections"].as_bool().unwrap_or(false),
        };
        let mut elements = Vec::with_capacity(nodes.len());
        for node in nodes {
            let width = node["width"].as_f64().unwrap_or(0.0);
            let height = node["height"].as_f64().unwrap_or(0.0);
            let mut element = LayoutElement::new(
                node["id"].as_str().unwrap_or_default(),
                node["position"]["x"].as_f64().unwrap_or(0.0) + width / 2.0,
                node["position"]["y"].as_f64().unwrap_or(0.0) + height / 2.0,
                width,
                height,
            );
            element.element_type = node["type"].as_str().map(str::to_owned);
            element.packed_width = number_query(node, args, "packedWidth")?;
            element.leading = number_query(node, args, "leading")?;
            elements.push(element);
        }
        let connections = input["edges"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|edge| {
                Some(LayoutConnection::new(
                    edge["source"].as_str()?,
                    edge["target"].as_str()?,
                ))
            })
            .collect();
        let input = LayoutInput { elements, connections };
        let definition = LayoutDefinition {
            row_packed: Some(declaration),
            ..LayoutDefinition::default()
        };
        let placements = RowPackedLayout
            .place(&input, &definition, None)
            .ok_or("The row-packed layout places nothing.")?;
        Ok(input
            .elements
            .iter()
            .filter_map(|element| {
                let placed = placements.get(&element.id)?;
                let width = placed.width.unwrap_or(element.width);
                Some(Placed {
                    id: element.id.clone(),
                    x: placed.x - width / 2.0,
                    y: placed.y - element.height / 2.0,
                    width: placed.width,
                })
            })
            .collect())
    }
}

impl LayoutCapability for RowPacked {
    fn draws_its_sizes(&self) -> bool {
        true
    }

    /// The packing keeps no pin: every node is placed in its order.
    fn consults_pins(&self) -> bool {
        false
    }

    fn place(&self, input: &Json, args: &Json) -> Result<Vec<(String, f64, f64)>, String> {
        Ok(self
            .pack(input, args)?
            .into_iter()
            .map(|placed| (placed.id, placed.x, placed.y))
            .collect())
    }

    /// The positions, and the width each node of `types` is drawn at.
    fn layout(&self, input: &Json, args: &Json) -> Result<Json, String> {
        let mut positions = Map::new();
        let mut sizes = Map::new();
        for placed in self.pack(input, args)? {
            positions.insert(placed.id.clone(), json!({"x": round2(placed.x), "y": round2(placed.y)}));
            if let Some(width) = placed.width {
                sizes.insert(placed.id, json!({"width": round2(width)}));
            }
        }
        Ok(json!({ "positions": positions, "sizes": sizes }))
    }
}
