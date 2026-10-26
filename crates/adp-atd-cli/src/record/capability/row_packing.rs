//! `layout.rowPacking` from the shared row packing (`crates/adp-layout/src/row_packing.rs`), as the timeline's and the
//! Gartner hype cycle graph's Arrange diagram feed it (`modules/diagrams/timeline/src/arrangement.rs`,
//! `modules/diagrams/gartner-hype-cycle-graph/src/arrangement.rs`), on every document of their parity scripts: each
//! item with where it starts across and how wide and how many rows tall it is, the links with the pairs kept apart, and
//! the row each item is given.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use adp_layout::row_packing::RowItem;
use serde_json::{Map, Value as Json, json};

use super::documents::{opened, round2};
use super::{Recorded, write_fixture};

const CAPABILITY: &str = "layout.rowPacking";

/// Records the fixtures of `layout.rowPacking` into `cases`.
pub fn record(cases: &Path) -> Result<Vec<PathBuf>, String> {
    let mut written = Vec::new();
    for document in opened(adp_diagram_timeline::ORIGIN, false)? {
        let (elements, links) = adp_diagram_timeline::arrangement::read(&document.diagram);
        let packing = adp_diagram_timeline::arrangement::packing_of(&elements, &links);
        let rows = adp_diagram_timeline::arrangement::rows_of(&elements, &links);
        let ids: Vec<String> = document.diagram.relations().iter().map(|r| r.id()).collect();
        let edges: Vec<(String, String, String)> = ids
            .into_iter()
            .zip(links)
            .map(|(id, (source, target))| (id, source, target))
            .collect();
        let from = Recorded {
            capability: CAPABILITY,
            code: "crates/adp-layout/src/row_packing.rs, fed by modules/diagrams/timeline/src/arrangement.rs",
            source: &document.source,
            note: "",
        };
        written.push(write_fixture(
            cases,
            &document.name,
            &from,
            input(&packing.items, &edges, &packing.apart),
            json!({"gap": adp_diagram_timeline::arrangement::GAP}),
            output(&packing.items, &rows),
        )?);
    }
    for document in opened(adp_diagram_hype_cycle::ORIGIN, false)? {
        let packing = adp_diagram_hype_cycle::arrangement::packing_of(&document.diagram);
        let rows = adp_diagram_hype_cycle::arrangement::rows_of(&document.diagram);
        let from = Recorded {
            capability: CAPABILITY,
            code: "crates/adp-layout/src/row_packing.rs, fed by modules/diagrams/gartner-hype-cycle-graph/src/arrangement.rs",
            source: &document.source,
            note: "",
        };
        written.push(write_fixture(
            cases,
            &document.name,
            &from,
            input(&packing.items, &packing.links, &[]),
            json!({"gap": adp_diagram_hype_cycle::arrangement::GAP}),
            output(&packing.items, &rows),
        )?);
    }
    Ok(written)
}

/// The items as nodes, in the order they are packed in, and the links as edges, those kept apart marked.
fn input(items: &[RowItem], edges: &[(String, String, String)], apart: &[(String, String)]) -> Json {
    let apart: HashSet<(&str, &str)> = apart.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let nodes: Vec<Json> = items
        .iter()
        .enumerate()
        .map(|(order, item)| {
            json!({
                "id": item.id, "width": round2(item.right - item.left), "height": item.rows.max(1), "order": order,
                "attributes": {"left": round2(item.left)},
            })
        })
        .collect();
    let edges: Vec<Json> = edges
        .iter()
        .map(|(id, source, target)| {
            let mut edge = Map::new();
            edge.insert("id".into(), json!(id));
            edge.insert("source".into(), json!(source));
            edge.insert("target".into(), json!(target));
            if apart.contains(&(source.as_str(), target.as_str())) {
                edge.insert("attributes".into(), json!({"apart": true}));
            }
            Json::Object(edge)
        })
        .collect();
    json!({"nodes": nodes, "edges": edges})
}

/// Each item's row, with where it starts across, in the order the items were given.
fn output(items: &[RowItem], rows: &HashMap<String, i64>) -> Json {
    let positions: Map<String, Json> = items
        .iter()
        .filter_map(|item| {
            let row = rows.get(&item.id)?;
            Some((item.id.clone(), json!({"x": round2(item.left), "y": row})))
        })
        .collect();
    json!({"positions": positions})
}
