//! `layout.rowPacked` from the row-packed layout (`crates/adp-layout/src/row_packed.rs`), as the Gartner hype cycle
//! graph's Compact viewpoint feeds it (`modules/diagrams/gartner-hype-cycle-graph/src/compact.rs`), on every document
//! of its parity scripts: each element with its box where it is now, the width it is packed at and the room its label
//! takes before it, the influences, and the top-left corner and width the packing gives each element.

use std::path::{Path, PathBuf};

use adp_layout::{LayoutAlgorithm, RowPackedLayout, RowPackedWidth};
use serde_json::{Map, Value as Json, json};

use super::documents::{opened, round2};
use super::{Recorded, write_fixture};

/// Records the fixtures of `layout.rowPacked` into `cases`.
pub fn record(cases: &Path) -> Result<Vec<PathBuf>, String> {
    use adp_diagram_hype_cycle::compact;

    let definition = compact::definition();
    let declaration = definition
        .row_packed
        .clone()
        .ok_or("Compact declares the row-packed layout")?;
    let mut args = Map::new();
    args.insert("gap".into(), json!(declaration.gap));
    if let Some(step) = declaration.row_step {
        args.insert("rowStep".into(), json!(step));
    }
    args.insert("followConnections".into(), json!(declaration.follow_connections));
    if let Some(types) = &declaration.types {
        args.insert("types".into(), json!(types));
    }
    if let RowPackedWidth::Fixed(width) = declaration.width {
        args.insert("width".into(), json!(width));
    }

    let mut written = Vec::new();
    for document in opened(adp_diagram_hype_cycle::ORIGIN, false)? {
        let input = compact::input(&document.diagram);
        let placements = RowPackedLayout
            .place(&input, &definition, None)
            .ok_or("Compact's packing places the graph")?;
        let nodes: Vec<Json> = input
            .elements
            .iter()
            .map(|element| {
                let mut node = Map::new();
                node.insert("id".into(), json!(element.id));
                if let Some(kind) = &element.element_type {
                    node.insert("type".into(), json!(kind));
                }
                node.insert("width".into(), json!(round2(element.width)));
                node.insert("height".into(), json!(round2(element.height)));
                node.insert(
                    "position".into(),
                    json!({"x": round2(element.x - element.width / 2.0), "y": round2(element.y - element.height / 2.0)}),
                );
                let mut attributes = Map::new();
                if let Some(width) = element.packed_width {
                    attributes.insert("packedWidth".into(), json!(round2(width)));
                }
                if let Some(leading) = element.leading {
                    attributes.insert("leading".into(), json!(round2(leading)));
                }
                if !attributes.is_empty() {
                    node.insert("attributes".into(), Json::Object(attributes));
                }
                Json::Object(node)
            })
            .collect();
        // The influences the packing follows, with the ids they are written with.
        let edges: Vec<Json> = document
            .diagram
            .relations_of_type("Influence")
            .iter()
            .filter_map(|influence| {
                Some(json!({
                    "id": influence.id(), "source": influence.source_id()?, "target": influence.target_id()?,
                }))
            })
            .collect();
        let mut positions = Map::new();
        let mut sizes = Map::new();
        for element in &input.elements {
            let Some(placed) = placements.get(&element.id) else {
                continue;
            };
            let width = placed.width.unwrap_or(element.width);
            positions.insert(
                element.id.clone(),
                json!({"x": round2(placed.x - width / 2.0), "y": round2(placed.y - element.height / 2.0)}),
            );
            if let Some(width) = placed.width {
                sizes.insert(element.id.clone(), json!({"width": round2(width)}));
            }
        }
        let from = Recorded {
            capability: "layout.rowPacked",
            code: "crates/adp-layout/src/row_packed.rs, fed by modules/diagrams/gartner-hype-cycle-graph/src/compact.rs",
            source: &document.source,
            note: "",
        };
        written.push(write_fixture(
            cases,
            &document.name,
            &from,
            json!({"nodes": nodes, "edges": edges}),
            Json::Object(args.clone()),
            json!({"positions": positions, "sizes": sizes}),
        )?);
    }
    Ok(written)
}
