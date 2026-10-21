//! Where everything in one document is drawn: each node's box, each group's frame, the lanes a long flow runs
//! through, and which entries can be drawn at all (etalii.adp spec 030, task T076; `supply-chain.dis`, the layout
//! `plugin:net.etalii.adp.etalii.supplyChainLayers`).
//!
//! Ported from the standalone host's `src/diagrams/supply-chain/backend/EtAlii.Adp.Diagram.SupplyChain/
//! SupplyChainLayout.cs` at `8b6ccfc`. A node that states `x` and `y` is drawn there; every other node is placed by the
//! layered left-to-right layout computed over the whole document, so a node that is dragged, and so gains
//! coordinates, moves nobody else. A group's frame is the box around its members, so it follows them; a group with no
//! members is drawn where its own `x` and `y` place it, one node's frame in size, and without them draws nothing.

use std::collections::{HashMap, HashSet};

use adp_layout::layering::{acyclic_edges, longest_path_with_floor};
use adp_layout::{CyclePolicy, Rect};
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

/// The id no document id can be: the band of the ungrouped nodes.
const UNGROUPED: &str = "\u{0}ungrouped";

fn order(a: f64, b: f64) -> std::cmp::Ordering {
    a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
}

/// The mean of `values`, summed in order as `Enumerable.Average` sums them.
fn average(values: impl Iterator<Item = f64>) -> f64 {
    let (mut sum, mut count) = (0.0, 0usize);
    for value in values {
        sum += value;
        count += 1;
    }
    sum / count as f64
}

/// The layered left-to-right layout: each node's top-left, with goods flowing to the right and every group kept
/// together in a band of its own (`SupplyChainLayout.Arrange`).
///
/// A node's layer is never left of its stage's column, nor left of one past every node that supplies it over the
/// flows with every cycle broken at the edge that closes it. A flow that skips layers is given a slot in every layer
/// it crosses, in the band of the node it delivers to. Bands, the groups and one for the ungrouped nodes, and the
/// nodes within a band, are sorted eight times by the mean position of what they are connected to, then dropped onto
/// a per-layer skyline in that order.
fn arrange(
    nodes: &[SupplyChainNode],
    flows: &[SupplyChainFlow],
    group_of: &dyn Fn(&SupplyChainNode) -> Option<String>,
) -> (IndexMap<String, (f64, f64)>, Lanes) {
    let mut result = IndexMap::new();
    let mut lanes = IndexMap::new();
    if nodes.is_empty() {
        return (result, lanes);
    }

    let mut outgoing: HashMap<String, Vec<String>> = nodes.iter().map(|n| (n.id.clone(), Vec::new())).collect();
    for flow in flows {
        outgoing
            .get_mut(&flow.from)
            .expect("a drawable end")
            .push(flow.to.clone());
    }
    let ids: Vec<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
    let forward = acyclic_edges(&ids, &outgoing);
    let stage: HashMap<&str, i32> = nodes
        .iter()
        .map(|n| (n.id.as_str(), stages::rank_of(&n.node_type)))
        .collect();
    let edges: Vec<(String, String)> = nodes
        .iter()
        .flat_map(|n| forward[&n.id].iter().map(|t| (n.id.clone(), t.clone())))
        .collect();
    let mut layer: HashMap<String, i32> =
        longest_path_with_floor(ids.iter().copied(), edges, CyclePolicy::Kahn, |id| stage[id]);

    // Every slot a layer holds: the nodes, then one lane per layer a forward flow skips. A lane's id cannot be a
    // document id: those never hold a control character.
    let mut index: IndexMap<String, usize> = nodes.iter().enumerate().map(|(p, n)| (n.id.clone(), p)).collect();
    let mut neighbours: HashMap<String, Vec<String>> = nodes.iter().map(|n| (n.id.clone(), Vec::new())).collect();
    let mut lane_slots: IndexMap<String, Vec<String>> = IndexMap::new();
    let mut lane_target: HashMap<String, String> = HashMap::new();
    let join = |neighbours: &mut HashMap<String, Vec<String>>, a: &str, b: &str| {
        neighbours.get_mut(a).expect("a slot").push(b.to_owned());
        neighbours.get_mut(b).expect("a slot").push(a.to_owned());
    };
    for flow in flows {
        let (from, to) = (layer[&flow.from], layer[&flow.to]);
        if to - from <= 1 || !forward[&flow.from].contains(&flow.to) {
            join(&mut neighbours, &flow.from, &flow.to);
            continue;
        }
        let mut previous = flow.from.clone();
        let mut slots = Vec::new();
        for crossed in from + 1..to {
            let slot = format!("\u{1}{}\u{1}{crossed}", flow.id);
            let next = index.len();
            index.insert(slot.clone(), next);
            layer.insert(slot.clone(), crossed);
            neighbours.insert(slot.clone(), Vec::new());
            lane_target.insert(slot.clone(), flow.to.clone());
            join(&mut neighbours, &previous, &slot);
            slots.push(slot.clone());
            previous = slot;
        }
        join(&mut neighbours, &previous, &flow.to);
        lane_slots.insert(flow.id.clone(), slots);
    }

    // The bands, in the order of their first slot; a lane joins the band of the node its flow delivers to.
    let by_id: HashMap<&str, &SupplyChainNode> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let band_key = |id: &str| -> String {
        let owner = lane_target.get(id).map_or(id, String::as_str);
        group_of(by_id[owner]).unwrap_or_else(|| UNGROUPED.to_owned())
    };
    let mut grouped_bands: IndexMap<String, Vec<String>> = IndexMap::new();
    for id in index.keys() {
        grouped_bands.entry(band_key(id)).or_default().push(id.clone());
    }
    let bands: Vec<Vec<String>> = grouped_bands.into_values().collect();

    // A position to sort by, refined each pass, from document order.
    let mut rank: HashMap<String, f64> = index.iter().map(|(id, at)| (id.clone(), *at as f64)).collect();
    let barycentre = |rank: &HashMap<String, f64>, id: &str| -> f64 {
        let around = &neighbours[id];
        if around.is_empty() {
            rank[id]
        } else {
            average(around.iter().map(|other| rank[other]))
        }
    };

    let mut band_order: Vec<usize> = (0..bands.len()).collect();
    let mut rows: HashMap<(usize, i32), Vec<String>> = HashMap::new();
    for _ in 0..8 {
        let score: HashMap<usize, f64> = band_order
            .iter()
            .map(|band| (*band, average(bands[*band].iter().map(|id| barycentre(&rank, id)))))
            .collect();
        let first_index = |band: usize| bands[band].iter().map(|id| index[id]).min().unwrap_or(0);
        band_order.sort_by(|a, b| order(score[a], score[b]).then(first_index(*a).cmp(&first_index(*b))));

        rows.clear();
        for band in &band_order {
            let mut by_layer: IndexMap<i32, Vec<String>> = IndexMap::new();
            for id in &bands[*band] {
                by_layer.entry(layer[id]).or_default().push(id.clone());
            }
            for (l, mut members) in by_layer {
                members.sort_by(|a, b| order(barycentre(&rank, a), barycentre(&rank, b)).then(index[a].cmp(&index[b])));
                rows.insert((*band, l), members);
            }
        }

        // The global rank: bands in order, then the row within the band.
        let mut offset = 0.0;
        for band in &band_order {
            let mut height = 0;
            for ((row_band, _), members) in &rows {
                if row_band != band {
                    continue;
                }
                for (row, member) in members.iter().enumerate() {
                    rank.insert(member.clone(), offset + row as f64);
                }
                height = height.max(members.len());
            }
            offset += height as f64 + 1.0;
        }
    }

    // Drop the bands onto a per-layer skyline, in order.
    let mut placed: HashMap<String, (f64, f64)> = HashMap::new();
    let layers = layer.values().copied().max().unwrap_or(0) + 1;
    let mut skyline = vec![0.0; layers as usize];
    for band in &band_order {
        let spanned: Vec<i32> = bands[*band].iter().map(|id| layer[id]).collect();
        let (first, last) = (
            *spanned.iter().min().expect("a band has a member"),
            *spanned.iter().max().expect("a band has a member"),
        );
        let grouped = band_key(&bands[*band][0]) != UNGROUPED;
        let header = if grouped { geometry::GROUP_HEADER } else { 0.0 };
        let footer = if grouped { geometry::GROUP_PADDING } else { 0.0 };

        let mut top: f64 = 0.0;
        for l in first..=last {
            top = top.max(skyline[l as usize]);
        }
        let mut tallest = 0;
        for l in first..=last {
            let Some(members) = rows.get(&(*band, l)) else {
                continue;
            };
            tallest = tallest.max(members.len());
            for (row, member) in members.iter().enumerate() {
                placed.insert(
                    member.clone(),
                    (f64::from(l) * LAYER_PITCH, top + header + row as f64 * ROW_PITCH),
                );
            }
        }
        let bottom =
            top + header + tallest as f64 * ROW_PITCH - (ROW_PITCH - geometry::NODE_HEIGHT) + footer + BAND_GAP;
        for l in first..=last {
            skyline[l as usize] = bottom;
        }
    }

    for node in nodes {
        result.insert(node.id.clone(), placed[&node.id]);
    }
    // A lane is drawn through its slot's middle, where a card in that slot would have its centre.
    for (flow_id, slots) in lane_slots {
        lanes.insert(
            flow_id,
            slots
                .iter()
                .map(|slot| {
                    let (x, y) = placed[slot];
                    (x + geometry::NODE_WIDTH / 2.0, y + geometry::NODE_HEIGHT / 2.0)
                })
                .collect(),
        );
    }
    (result, lanes)
}
