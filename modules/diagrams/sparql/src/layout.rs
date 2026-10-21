//! Where a query's nodes and regions go (etalii.adp spec 030, task T084).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/sparql/backend/EtAlii.Adp.Diagram.Sparql/
//! SparqlLayout.cs`. Pure and deterministic, recursive bottom-up over the scope structure with no physics: each scope
//! grid-places its own nodes and its child regions in projection order, three to a row; a parent treats a placed region
//! as one block; a region's bounds always contain its contents. An authored node position overrides its computed
//! place; an authored region position moves the region as an anchored frame, its computed contents following so
//! containment survives the move; an anonymous node always takes its computed place.

use std::collections::{BTreeMap, HashMap};

use adp_layout::grid_packer::{self, GridPacking};
use adp_layout::rect::Rect;

use crate::projection::{SparqlNode, SparqlNodeKind, SparqlProjectionResult, SparqlRegion};

/// A node's drawn size.
pub const NODE_WIDTH: f64 = 170.0;
pub const NODE_HEIGHT: f64 = 64.0;
const GAP_X: f64 = 60.0;
const GAP_Y: f64 = 48.0;
const REGION_PADDING: f64 = 36.0;
const REGION_LABEL_BAND: f64 = 30.0;
const ROOT_LEFT: f64 = 40.0;
/// Below the header band.
const ROOT_TOP: f64 = 90.0;
const COLUMNS: usize = 3;

/// Every node's top-left, authored positions applied, and every region's frame, anchors applied.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SparqlLayoutResult {
    pub node_positions: HashMap<String, (f64, f64)>,
    pub region_bounds: HashMap<String, Rect>,
}

struct Scopes<'a> {
    nodes_by_scope: HashMap<&'a str, Vec<&'a SparqlNode>>,
    regions_by_parent: HashMap<String, Vec<&'a SparqlRegion>>,
}

struct Item<'a> {
    id: String,
    size: Rect,
    region: Option<&'a SparqlRegion>,
}

fn parent_path_of(region: &SparqlRegion) -> String {
    if !region.parent_region_id.is_empty() {
        return region.parent_region_id["region:".len()..].to_owned();
    }
    if region.scope_path == "template" {
        String::new()
    } else {
        "where".to_owned()
    }
}

/// Positions for `projection`, with the `stored` authored positions applied.
pub fn compute(projection: &SparqlProjectionResult, stored: &BTreeMap<String, (f64, f64)>) -> SparqlLayoutResult {
    let mut nodes_by_scope: HashMap<&str, Vec<&SparqlNode>> = HashMap::new();
    for node in &projection.nodes {
        nodes_by_scope.entry(node.scope_path.as_str()).or_default().push(node);
    }
    let mut regions_by_parent: HashMap<String, Vec<&SparqlRegion>> = HashMap::new();
    for region in &projection.regions {
        regions_by_parent.entry(parent_path_of(region)).or_default().push(region);
    }
    let scopes = Scopes {
        nodes_by_scope,
        regions_by_parent,
    };
    let mut result = SparqlLayoutResult::default();

    // The open canvas holds the where scope's own nodes, its top-level regions, and the template region beside them.
    let mut root_items = scopes.items_of("where");
    if let Some(top_level) = scopes.regions_by_parent.get("") {
        for region in top_level.iter().filter(|r| r.scope_path == "template") {
            root_items.push(Item {
                id: region.id.clone(),
                size: scopes.measure_region(&region.scope_path),
                region: Some(region),
            });
        }
    }
    scopes.place_items(&root_items, ROOT_LEFT, ROOT_TOP, &mut result);

    // Authored region anchors: the frame moves, its computed contents follow.
    for region in &projection.regions {
        let (Some(&(x, y)), Some(frame)) = (stored.get(&region.id), result.region_bounds.get(&region.id)) else {
            continue;
        };
        let (dx, dy) = (x - frame.x, y - frame.y);
        shift_scope(&region.scope_path, dx, dy, projection, &mut result);
    }

    // Authored node positions: absolute overrides, except anonymous nodes, whose ordinals are not stable identities.
    for node in &projection.nodes {
        if node.kind != SparqlNodeKind::Anonymous
            && let Some(&authored) = stored.get(&node.id)
        {
            result.node_positions.insert(node.id.clone(), authored);
        }
    }
    result
}

fn grid(items: &[Item<'_>]) -> GridPacking {
    let sizes: Vec<(f64, f64)> = items.iter().map(|i| (i.size.width, i.size.height)).collect();
    grid_packer::pack(&sizes, COLUMNS, GAP_X, GAP_Y)
}

impl<'a> Scopes<'a> {
    fn items_of(&self, scope_path: &str) -> Vec<Item<'a>> {
        let mut items = Vec::new();
        if let Some(nodes) = self.nodes_by_scope.get(scope_path) {
            items.extend(nodes.iter().map(|node| Item {
                id: node.id.clone(),
                size: Rect::new(0.0, 0.0, NODE_WIDTH, NODE_HEIGHT),
                region: None,
            }));
        }
        if let Some(regions) = self.regions_by_parent.get(scope_path) {
            for region in regions.iter().filter(|r| r.scope_path != "template") {
                items.push(Item {
                    id: region.id.clone(),
                    size: self.measure_region(&region.scope_path),
                    region: Some(region),
                });
            }
        }
        items
    }

    /// Bottom-up: a region's extent is its own grid's extent plus padding and its label band.
    fn measure_region(&self, scope_path: &str) -> Rect {
        let items = self.items_of(scope_path);
        let (width, height) = if items.is_empty() {
            (NODE_WIDTH, NODE_HEIGHT / 2.0)
        } else {
            let packed = grid(&items);
            (packed.width, packed.height)
        };
        Rect::new(
            0.0,
            0.0,
            (width + 2.0 * REGION_PADDING).max(NODE_WIDTH),
            (height + 2.0 * REGION_PADDING + REGION_LABEL_BAND).max(NODE_HEIGHT + REGION_LABEL_BAND),
        )
    }

    /// Top-down: rows of up to three items, each row as tall as its tallest member.
    fn place_items(&self, items: &[Item<'a>], origin_x: f64, origin_y: f64, result: &mut SparqlLayoutResult) {
        let packed = grid(items);
        for (index, item) in items.iter().enumerate() {
            let x = origin_x + packed.cells[index].x;
            let y = origin_y + packed.cells[index].y;
            match item.region {
                None => {
                    result.node_positions.insert(item.id.clone(), (x, y));
                }
                Some(region) => {
                    result
                        .region_bounds
                        .insert(item.id.clone(), Rect::new(x, y, item.size.width, item.size.height));
                    let inner = self.items_of(&region.scope_path);
                    self.place_items(
                        &inner,
                        x + REGION_PADDING,
                        y + REGION_LABEL_BAND + REGION_PADDING,
                        result,
                    );
                }
            }
        }
    }
}

/// Shifts a scope's frame and every computed position inside it.
fn shift_scope(scope_path: &str, dx: f64, dy: f64, projection: &SparqlProjectionResult, result: &mut SparqlLayoutResult) {
    let prefix = format!("{scope_path}/");
    for node in &projection.nodes {
        if (node.scope_path == scope_path || node.scope_path.starts_with(&prefix))
            && let Some(position) = result.node_positions.get_mut(&node.id)
        {
            *position = (position.0 + dx, position.1 + dy);
        }
    }
    for region in &projection.regions {
        if (region.scope_path == scope_path || region.scope_path.starts_with(&prefix))
            && let Some(frame) = result.region_bounds.get_mut(&region.id)
        {
            *frame = Rect::new(frame.x + dx, frame.y + dy, frame.width, frame.height);
        }
    }
}
