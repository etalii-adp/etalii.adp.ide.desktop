//! `layout.tidyTree` from agent behaviour modelling's tree (`compute` in
//! `modules/diagrams/agent-behavior-modelling/src/layout.rs`, the Reingold-Tilford outline packing its `.dis` declares
//! as `tidyTree`), on every behaviour tree of its parity scripts: the nodes with their sizes, parents and order, and
//! the top-left corner the tree alone gives each. The rows an author dragged are the tool's, not the capability's.

use std::path::{Path, PathBuf};

use adp_diagram_agent_behavior_modelling::layout::{self, HORIZONTAL_GAP, NODE_HEIGHT, NODE_WIDTH, VERTICAL_GAP};
use adp_diagram_agent_behavior_modelling::plugin::read_model;
use serde_json::{Map, Value as Json, json};

use super::documents::{opened, round2};
use super::{Recorded, write_fixture};

/// Records the fixtures of `layout.tidyTree` into `cases`.
pub fn record(cases: &Path) -> Result<Vec<PathBuf>, String> {
    let mut written = Vec::new();
    for document in opened(adp_diagram_agent_behavior_modelling::ORIGIN, false)? {
        let model = read_model(&document.body);
        let nodes: Vec<Json> = model
            .nodes
            .iter()
            .map(|node| {
                let siblings = match &node.parent_id {
                    Some(parent) => model.node(parent).map(|p| p.child_ids.clone()).unwrap_or_default(),
                    None => model.roots().iter().map(|root| root.id.clone()).collect(),
                };
                let order = siblings.iter().position(|id| *id == node.id).unwrap_or_default();
                json!({
                    "id": node.id, "width": NODE_WIDTH, "height": NODE_HEIGHT, "parent": node.parent_id, "order": order,
                })
            })
            .collect();
        let roots: Vec<String> = model.roots().iter().map(|root| root.id.clone()).collect();
        let computed = layout::compute(&model);
        let positions: Map<String, Json> = model
            .nodes
            .iter()
            .filter_map(|node| {
                let (x, y) = computed.get(&node.id)?;
                Some((node.id.clone(), json!({"x": round2(*x), "y": round2(*y)})))
            })
            .collect();
        let from = Recorded {
            capability: "layout.tidyTree",
            code: "modules/diagrams/agent-behavior-modelling/src/layout.rs",
            source: &document.source,
            note: "",
        };
        written.push(write_fixture(
            cases,
            &document.name,
            &from,
            json!({"nodes": nodes, "edges": [], "roots": roots}),
            json!({"nodeGap": HORIZONTAL_GAP, "layerGap": VERTICAL_GAP, "componentGap": HORIZONTAL_GAP * 2.0}),
            json!({"positions": positions}),
        )?);
    }
    Ok(written)
}
