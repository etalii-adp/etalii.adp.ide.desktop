//! `layout.ring` from the causal loop diagram's ring (`modules/diagrams/causal-loop-diagram/src/ring.rs`, the layout
//! `ring` its description names), on every diagram of its parity scripts: the variables with the sizes its metric gives
//! them and the links between them in, and out the top-left corner it draws each at, the point on the ring (etalii.adp
//! spec 076, T026).

use std::path::{Path, PathBuf};

use adp_diagram_causal_loop::metrics::Metrics;
use adp_diagram_causal_loop::reading::Model;
use adp_diagram_causal_loop::ring;
use serde_json::{Map, Value as Json, json};

use super::documents::{opened, round2};
use super::{Recorded, write_fixture};

/// Records the fixtures of `layout.ring` into `cases`.
pub fn record(cases: &Path) -> Result<Vec<PathBuf>, String> {
    let metrics = Metrics::default();
    let mut written = Vec::new();
    for document in opened(adp_diagram_causal_loop::ORIGIN, false)? {
        let model = Model::of(&document.diagram);
        let variables = model.distinct_variables();
        let nodes: Vec<Json> = variables
            .iter()
            .map(|variable| {
                json!({
                    "id": variable.element_id(),
                    "width": round2(metrics.width_of(variable.display())),
                    "height": round2(metrics.height()),
                })
            })
            .collect();
        let edges: Vec<Json> = model
            .links
            .iter()
            .map(|link| {
                json!({
                    "id": format!("link:{}|{}", link.from, link.to),
                    "source": format!("variable:{}", link.from),
                    "target": format!("variable:{}", link.to),
                })
            })
            .collect();
        let boxes = ring::compute(&model, &metrics);
        let positions: Map<String, Json> = variables
            .iter()
            .filter_map(|variable| {
                let rect = boxes.get(&variable.name)?;
                // The module draws a box with its top-left corner where it computes the box's centre: the point on the
                // ring is that corner.
                let (x, y) = if variables.len() == 1 {
                    (rect.x, rect.y)
                } else {
                    (rect.center_x(), rect.center_y())
                };
                Some((variable.element_id(), json!({"x": round2(x), "y": round2(y)})))
            })
            .collect();
        let from = Recorded {
            capability: "layout.ring",
            code: "modules/diagrams/causal-loop-diagram/src/ring.rs",
            source: &document.source,
            note: "",
        };
        written.push(write_fixture(
            cases,
            &document.name,
            &from,
            json!({"nodes": nodes, "edges": edges}),
            json!({"separation": metrics.separation}),
            json!({"positions": positions}),
        )?);
    }
    Ok(written)
}
