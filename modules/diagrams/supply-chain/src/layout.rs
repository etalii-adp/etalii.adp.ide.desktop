//! Where everything in one document is drawn: each node's box, each group's frame, the lanes a long flow runs
//! through, and which entries can be drawn at all (etalii.adp spec 030, task T076; `supply-chain.dis`, the layout
//! `plugin:net.etalii.adp.etalii.supplyChainLayers`).
//!
//! Ported from the standalone host's `src/diagrams/supply-chain/backend/EtAlii.Adp.Diagram.SupplyChain/
//! SupplyChainLayout.cs` at `8b6ccfc`. A node that states `x` and `y` is drawn there; every other node is placed by the
//! layered left-to-right layout computed over the whole document, so a node that is dragged, and so gains
//! coordinates, moves nobody else. A group's frame is the box around its members, so it follows them; a group with no
//! members is drawn where its own `x` and `y` place it, one node's frame in size, and without them draws nothing.

use std::collections::HashSet;

use adp_layout::Rect;
use indexmap::IndexMap;

use crate::model::{SupplyChainFlow, SupplyChainGroup, SupplyChainModel, SupplyChainNode, geometry, stages};

/// The horizontal distance between the left edges of two neighbouring layers.
pub const LAYER_PITCH: f64 = geometry::NODE_WIDTH + 128.0;

/// The vertical distance between the tops of two nodes stacked in one band.
const ROW_PITCH: f64 = geometry::NODE_HEIGHT + 28.0;

/// The clear space between two bands that share a layer.
const BAND_GAP: f64 = 32.0;

/// Within this, a drawn position is the arranged one (the standalone host's `double.Tolerance`).
const TOLERANCE: f64 = 0.000_001_f32 as f64;

/// The layout of one model.
#[derive(Debug, Clone, Default)]
pub struct SupplyChainLayout {
    /// The groups that can be drawn: an id unique across the document, and a member or a place of its own.
    pub groups: Vec<SupplyChainGroup>,
    /// The nodes that can be drawn: an id unique across the document, and a known stage.
    pub nodes: Vec<SupplyChainNode>,
    /// The flows that can be drawn: a unique id, and both ends drawable nodes.
    pub flows: Vec<SupplyChainFlow>,
    /// Each drawable node's box, authored or arranged.
    pub node_boxes: IndexMap<String, Rect>,
    /// Each drawn group's frame.
    pub group_boxes: IndexMap<String, Rect>,
    /// Every drawable node's top-left as the layered layout places it, whatever the document states.
    pub arranged: IndexMap<String, (f64, f64)>,
    /// The centre of the slot a long flow runs through in each layer it crosses, left to right; only for a flow whose
    /// two ends are drawn where the layout put them.
    pub lanes: IndexMap<String, Vec<(f64, f64)>>,
}

impl SupplyChainLayout {
    /// The layout of `model`.
    pub fn of(model: &SupplyChainModel) -> SupplyChainLayout {
        // One id space across groups, nodes and flows: the first entry with an id is drawn, a later one reusing it
        // is not.
        let mut taken: HashSet<String> = HashSet::new();
        let declared_groups: Vec<SupplyChainGroup> = model
            .groups
            .iter()
            .filter(|g| !g.id.is_empty() && taken.insert(g.id.clone()))
            .cloned()
            .collect();
        let nodes: Vec<SupplyChainNode> = model
            .nodes
            .iter()
            .filter(|n| !n.id.is_empty() && stages::is_known(&n.node_type) && taken.insert(n.id.clone()))
            .cloned()
            .collect();
        let node_ids: HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
        let flows: Vec<SupplyChainFlow> = model
            .flows
            .iter()
            .filter(|f| {
                !f.id.is_empty()
                    && node_ids.contains(f.from.as_str())
                    && node_ids.contains(f.to.as_str())
                    && f.from != f.to
                    && taken.insert(f.id.clone())
            })
            .cloned()
            .collect();

        let group_ids: HashSet<&str> = declared_groups.iter().map(|g| g.id.as_str()).collect();
        let group_of = |node: &SupplyChainNode| -> Option<String> {
            group_ids.contains(node.group.as_str()).then(|| node.group.clone())
        };

        let (arranged, arranged_lanes) = arrange(&nodes, &flows, &group_of);

        let node_boxes: IndexMap<String, Rect> = nodes
            .iter()
            .map(|node| {
                let (x, y) = match (node.x, node.y) {
                    (Some(x), Some(y)) => (x, y),
                    _ => arranged[&node.id],
                };
                (
                    node.id.clone(),
                    Rect::new(x, y, geometry::NODE_WIDTH, geometry::NODE_HEIGHT),
                )
            })
            .collect();

        let mut group_boxes: IndexMap<String, Rect> = IndexMap::new();
        for group in &declared_groups {
            let members: Vec<Rect> = nodes
                .iter()
                .filter(|n| group_of(n).as_deref() == Some(group.id.as_str()))
                .map(|n| node_boxes[&n.id])
                .collect();
            if members.is_empty() {
                if let (Some(x), Some(y)) = (group.x, group.y) {
                    group_boxes.insert(
                        group.id.clone(),
                        Rect::new(x, y, geometry::EMPTY_GROUP_WIDTH, geometry::EMPTY_GROUP_HEIGHT),
                    );
                }
                continue;
            }
            let left = members.iter().map(|b| b.x).fold(f64::INFINITY, f64::min) - geometry::GROUP_PADDING;
            let top = members.iter().map(|b| b.y).fold(f64::INFINITY, f64::min) - geometry::GROUP_HEADER;
            let right = members.iter().map(Rect::right).fold(f64::NEG_INFINITY, f64::max) + geometry::GROUP_PADDING;
            let bottom = members.iter().map(Rect::bottom).fold(f64::NEG_INFINITY, f64::max) + geometry::GROUP_PADDING;
            group_boxes.insert(group.id.clone(), Rect::new(left, top, right - left, bottom - top));
        }

        let at_arranged = |id: &str| {
            let (box_, place) = (node_boxes[id], arranged[id]);
            (box_.x - place.0).abs() < TOLERANCE && (box_.y - place.1).abs() < TOLERANCE
        };
        let lanes: IndexMap<String, Vec<(f64, f64)>> = flows
            .iter()
            .filter(|f| arranged_lanes.contains_key(&f.id) && at_arranged(&f.from) && at_arranged(&f.to))
            .map(|f| (f.id.clone(), arranged_lanes[&f.id].clone()))
            .collect();

        SupplyChainLayout {
            groups: declared_groups
                .iter()
                .filter(|g| group_boxes.contains_key(&g.id))
                .cloned()
                .collect(),
            nodes,
            flows,
            node_boxes,
            group_boxes,
            arranged,
            lanes,
        }
    }
}

type Lanes = IndexMap<String, Vec<(f64, f64)>>;

/// The layered left-to-right layout: each node's top-left, with goods flowing to the right and every group kept
/// together in a band of its own (`SupplyChainLayout.Arrange`), as the capability `layout.bandedLayers` lays it out:
/// a node's least layer is its stage's column, and a flow that skips layers is given a slot in every layer it crosses.
fn arrange(
    nodes: &[SupplyChainNode],
    flows: &[SupplyChainFlow],
    group_of: &dyn Fn(&SupplyChainNode) -> Option<String>,
) -> (IndexMap<String, (f64, f64)>, Lanes) {
    use adp_atd_runtime::capabilities::banded_layers::{BandArguments, BandEdge, BandNode, arrange};
    let placed: Vec<BandNode> = nodes
        .iter()
        .map(|node| BandNode {
            id: node.id.clone(),
            min_layer: stages::rank_of(&node.node_type),
            band: group_of(node),
        })
        .collect();
    let edges: Vec<BandEdge> = flows
        .iter()
        .map(|flow| BandEdge {
            id: flow.id.clone(),
            from: flow.from.clone(),
            to: flow.to.clone(),
        })
        .collect();
    let arguments = BandArguments {
        layer_gap: LAYER_PITCH - geometry::NODE_WIDTH,
        row_gap: ROW_PITCH - geometry::NODE_HEIGHT,
        band_title: geometry::GROUP_HEADER,
        band_bottom: geometry::GROUP_PADDING,
        band_gap: BAND_GAP,
        passes: 8,
        lanes: true,
    };
    arrange(
        &placed,
        &edges,
        (geometry::NODE_WIDTH, geometry::NODE_HEIGHT),
        &arguments,
    )
}
