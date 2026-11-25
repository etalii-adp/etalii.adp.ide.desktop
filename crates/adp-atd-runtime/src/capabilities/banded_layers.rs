//! `layout.bandedLayers` (ATD §13.3; its contract and fixtures are in `specifications/atd/capabilities/`): a layered
//! layout, left to right, whose layers are at least a node's `minLayer` and at least one past every node that flows
//! into it, with every cycle broken at the edge that closes it; a long edge takes a slot in every layer it crosses;
//! nodes kept in bands, ordered by barycentre and dropped onto a per-layer skyline. A node whose id is a band, its own
//! included, is that band's frame: drawn around the band's members, `bandTitle` above them and `bandBottom` beside and below, or where it
//! is pinned while the band has no member. Moved from the supply chain module's layout
//! (`modules/diagrams/supply-chain/src/layout.rs`), which calls it here, itself ported from the standalone host's
//! `SupplyChainLayout.cs` at `8b6ccfc`.

use std::collections::{HashMap, HashSet};

use adp_layout::CyclePolicy;
use adp_layout::layering::{acyclic_edges, longest_path_with_floor};
use indexmap::IndexMap;
use serde_json::Value as Json;

use super::{LayoutCapability, number_argument, number_query, text_query};

/// The capability's arguments: the gaps of the layout, in the canvas's domain units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandArguments {
    pub layer_gap: f64,
    pub row_gap: f64,
    pub band_title: f64,
    pub band_bottom: f64,
    pub band_gap: f64,
    pub passes: usize,
    pub lanes: bool,
}

impl Default for BandArguments {
    fn default() -> Self {
        BandArguments {
            layer_gap: 128.0,
            row_gap: 28.0,
            band_title: 40.0,
            band_bottom: 20.0,
            band_gap: 32.0,
            passes: 8,
            lanes: true,
        }
    }
}

impl BandArguments {
    fn of(args: &Json) -> Self {
        let default = BandArguments::default();
        BandArguments {
            layer_gap: number_argument(args, "layerGap", default.layer_gap),
            row_gap: number_argument(args, "rowGap", default.row_gap),
            band_title: number_argument(args, "bandTitle", default.band_title),
            band_bottom: number_argument(args, "bandBottom", default.band_bottom),
            band_gap: number_argument(args, "bandGap", default.band_gap),
            passes: number_argument(args, "passes", default.passes as f64) as usize,
            lanes: args["lanes"].as_bool().unwrap_or(default.lanes),
        }
    }
}

/// A node to place: its id, its least layer and its band, if any.
#[derive(Debug, Clone, PartialEq)]
pub struct BandNode {
    pub id: String,
    pub min_layer: i32,
    pub band: Option<String>,
}

/// An edge between two nodes to place, from the one that flows into the other.
#[derive(Debug, Clone, PartialEq)]
pub struct BandEdge {
    pub id: String,
    pub from: String,
    pub to: String,
}

/// Every node's top-left and every long edge's route through the middle of its slots, for nodes all `size` large.
pub type Arranged = (IndexMap<String, (f64, f64)>, IndexMap<String, Vec<(f64, f64)>>);

/// The id no node can have: the band of the nodes outside every band.
const OUTSIDE: &str = "\u{0}ungrouped";

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

/// The layered left-to-right layout of `nodes`, each `size` large, over `edges` whose ends are among them.
///
/// A node's layer is never left of its `min_layer`, nor left of one past every node that flows into it over the edges
/// with every cycle broken at the edge that closes it. An edge that skips layers is given a slot in every layer it
/// crosses, in the band of the node it delivers to. Bands, and the nodes within a band, are sorted `passes` times by
/// the mean position of what they are connected to, then dropped onto a per-layer skyline in that order.
pub fn arrange(nodes: &[BandNode], edges: &[BandEdge], size: (f64, f64), arguments: &BandArguments) -> Arranged {
    let mut result = IndexMap::new();
    let mut lanes = IndexMap::new();
    if nodes.is_empty() {
        return (result, lanes);
    }
    let (width, height) = size;
    let layer_pitch = width + arguments.layer_gap;
    let row_pitch = height + arguments.row_gap;

    let mut outgoing: HashMap<String, Vec<String>> = nodes.iter().map(|n| (n.id.clone(), Vec::new())).collect();
    for edge in edges {
        outgoing
            .get_mut(&edge.from)
            .expect("a placed end")
            .push(edge.to.clone());
    }
    let ids: Vec<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
    let forward = acyclic_edges(&ids, &outgoing);
    let floor: HashMap<&str, i32> = nodes.iter().map(|n| (n.id.as_str(), n.min_layer)).collect();
    let forward_edges: Vec<(String, String)> = nodes
        .iter()
        .flat_map(|n| forward[&n.id].iter().map(|t| (n.id.clone(), t.clone())))
        .collect();
    let mut layer: HashMap<String, i32> =
        longest_path_with_floor(ids.iter().copied(), forward_edges, CyclePolicy::Kahn, |id| floor[id]);

    // Every slot a layer holds: the nodes, then one lane per layer a forward edge skips. A lane's id cannot be a
    // node's: those never hold a control character.
    let mut index: IndexMap<String, usize> = nodes.iter().enumerate().map(|(p, n)| (n.id.clone(), p)).collect();
    let mut neighbours: HashMap<String, Vec<String>> = nodes.iter().map(|n| (n.id.clone(), Vec::new())).collect();
    let mut lane_slots: IndexMap<String, Vec<String>> = IndexMap::new();
    let mut lane_target: HashMap<String, String> = HashMap::new();
    let join = |neighbours: &mut HashMap<String, Vec<String>>, a: &str, b: &str| {
        neighbours.get_mut(a).expect("a slot").push(b.to_owned());
        neighbours.get_mut(b).expect("a slot").push(a.to_owned());
    };
    for edge in edges {
        let (from, to) = (layer[&edge.from], layer[&edge.to]);
        if !arguments.lanes || to - from <= 1 || !forward[&edge.from].contains(&edge.to) {
            join(&mut neighbours, &edge.from, &edge.to);
            continue;
        }
        let mut previous = edge.from.clone();
        let mut slots = Vec::new();
        for crossed in from + 1..to {
            let slot = format!("\u{1}{}\u{1}{crossed}", edge.id);
            let next = index.len();
            index.insert(slot.clone(), next);
            layer.insert(slot.clone(), crossed);
            neighbours.insert(slot.clone(), Vec::new());
            lane_target.insert(slot.clone(), edge.to.clone());
            join(&mut neighbours, &previous, &slot);
            slots.push(slot.clone());
            previous = slot;
        }
        join(&mut neighbours, &previous, &edge.to);
        lane_slots.insert(edge.id.clone(), slots);
    }

    // The bands, in the order of their first slot; a lane joins the band of the node its edge delivers to.
    let by_id: HashMap<&str, &BandNode> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let band_key = |id: &str| -> String {
        let owner = lane_target.get(id).map_or(id, String::as_str);
        by_id[owner].band.clone().unwrap_or_else(|| OUTSIDE.to_owned())
    };
    let mut grouped_bands: IndexMap<String, Vec<String>> = IndexMap::new();
    for id in index.keys() {
        grouped_bands.entry(band_key(id)).or_default().push(id.clone());
    }
    let bands: Vec<Vec<String>> = grouped_bands.into_values().collect();

    // A position to sort by, refined each pass, from the order of the input.
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
    for _ in 0..arguments.passes {
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
            let mut tallest = 0;
            for ((row_band, _), members) in &rows {
                if row_band != band {
                    continue;
                }
                for (row, member) in members.iter().enumerate() {
                    rank.insert(member.clone(), offset + row as f64);
                }
                tallest = tallest.max(members.len());
            }
            offset += tallest as f64 + 1.0;
        }
    }

    // Drop the bands onto a per-layer skyline, in order.
    let mut placed: HashMap<String, (f64, f64)> = HashMap::new();
    let layers = layer.values().copied().max().unwrap_or(0) + 1;
    let mut skyline = vec![0.0; layers.max(0) as usize];
    for band in &band_order {
        let spanned: Vec<i32> = bands[*band].iter().map(|id| layer[id]).collect();
        let (first, last) = (
            *spanned.iter().min().expect("a band has a member"),
            *spanned.iter().max().expect("a band has a member"),
        );
        let named = band_key(&bands[*band][0]) != OUTSIDE;
        let header = if named { arguments.band_title } else { 0.0 };
        let footer = if named { arguments.band_bottom } else { 0.0 };

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
                    (f64::from(l) * layer_pitch, top + header + row as f64 * row_pitch),
                );
            }
        }
        let bottom = top + header + tallest as f64 * row_pitch - (row_pitch - height) + footer + arguments.band_gap;
        for l in first..=last {
            skyline[l as usize] = bottom;
        }
    }

    for node in nodes {
        result.insert(node.id.clone(), placed[&node.id]);
    }
    // A lane is drawn through its slot's middle, where a node in that slot would have its centre.
    for (edge_id, slots) in lane_slots {
        lanes.insert(
            edge_id,
            slots
                .iter()
                .map(|slot| {
                    let (x, y) = placed[slot];
                    (x + width / 2.0, y + height / 2.0)
                })
                .collect(),
        );
    }
    (result, lanes)
}

/// The capability's result: each node's top-left, each frame's size, and each long edge's route.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Banded {
    pub positions: Vec<(String, f64, f64)>,
    pub sizes: Vec<(String, f64, f64)>,
    pub routes: super::Routes,
}

/// The layout of `$defs/LayoutInput` with the capability's `args`.
pub fn layout(input: &Json, args: &Json) -> Result<Banded, String> {
    let arguments = BandArguments::of(args);
    let all = input["nodes"].as_array().ok_or("The input has no `nodes`.")?;
    let mut bands_of: Vec<(&Json, Option<String>)> = Vec::new();
    for node in all {
        bands_of.push((node, text_query(node, args, "band")?.filter(|band| !band.is_empty())));
    }
    let named_bands: HashSet<&str> = bands_of.iter().filter_map(|(_, band)| band.as_deref()).collect();
    let id_of = |node: &Json| node["id"].as_str().unwrap_or_default().to_owned();
    // A node whose id is a band is that band's frame, not one of the nodes laid out.
    let frames: Vec<&Json> = all.iter().filter(|n| named_bands.contains(id_of(n).as_str())).collect();
    let frame_ids: HashSet<String> = frames.iter().map(|n| id_of(n)).collect();
    let mut members = Vec::new();
    for (node, band) in &bands_of {
        if frame_ids.contains(&id_of(node)) {
            continue;
        }
        let min_layer = number_query(node, args, "minLayer")?.unwrap_or(0.0) as i32;
        members.push((
            *node,
            BandNode {
                id: id_of(node),
                min_layer,
                band: band.clone(),
            },
        ));
    }
    let member_ids: HashSet<&str> = members.iter().map(|(_, m)| m.id.as_str()).collect();
    let edges: Vec<BandEdge> = input["edges"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|edge| {
            let (from, to) = (edge["source"].as_str()?, edge["target"].as_str()?);
            (from != to && member_ids.contains(from) && member_ids.contains(to)).then(|| BandEdge {
                id: edge["id"].as_str().unwrap_or_default().to_owned(),
                from: from.to_owned(),
                to: to.to_owned(),
            })
        })
        .collect();
    let size = members
        .iter()
        .map(|(node, _)| {
            (
                node["width"].as_f64().unwrap_or(0.0),
                node["height"].as_f64().unwrap_or(0.0),
            )
        })
        .fold((0.0_f64, 0.0_f64), |(w, h), (nw, nh)| (w.max(nw), h.max(nh)));
    let nodes: Vec<BandNode> = members.iter().map(|(_, m)| m.clone()).collect();
    let (arranged, lanes) = arrange(&nodes, &edges, size, &arguments);

    let pin = |node: &Json| node["pinned"]["x"].as_f64().zip(node["pinned"]["y"].as_f64());
    let mut banded = Banded::default();
    let mut boxes: HashMap<String, (f64, f64, f64, f64)> = HashMap::new();
    let mut at_arranged: HashSet<String> = HashSet::new();
    for (node, member) in &members {
        let (x, y) = match pin(node) {
            Some(pinned) => pinned,
            None => {
                at_arranged.insert(member.id.clone());
                arranged[&member.id]
            }
        };
        let (w, h) = (
            node["width"].as_f64().unwrap_or(0.0),
            node["height"].as_f64().unwrap_or(0.0),
        );
        boxes.insert(member.id.clone(), (x, y, w, h));
        banded.positions.push((member.id.clone(), x, y));
    }
    for frame in frames {
        let id = id_of(frame);
        let inside: Vec<(f64, f64, f64, f64)> = members
            .iter()
            .filter(|(_, m)| m.band.as_deref() == Some(id.as_str()))
            .map(|(_, m)| boxes[&m.id])
            .collect();
        if inside.is_empty() {
            if let Some((x, y)) = pin(frame) {
                banded.positions.push((id, x, y));
            }
            continue;
        }
        let left = inside.iter().map(|b| b.0).fold(f64::INFINITY, f64::min) - arguments.band_bottom;
        let top = inside.iter().map(|b| b.1).fold(f64::INFINITY, f64::min) - arguments.band_title;
        let right = inside.iter().map(|b| b.0 + b.2).fold(f64::NEG_INFINITY, f64::max) + arguments.band_bottom;
        let bottom = inside.iter().map(|b| b.1 + b.3).fold(f64::NEG_INFINITY, f64::max) + arguments.band_bottom;
        banded.positions.push((id.clone(), left, top));
        banded.sizes.push((id, right - left, bottom - top));
    }
    for edge in &edges {
        if let Some(route) = lanes.get(&edge.id)
            && at_arranged.contains(&edge.from)
            && at_arranged.contains(&edge.to)
        {
            banded.routes.push((edge.id.clone(), route.clone()));
        }
    }
    Ok(banded)
}

/// The capability over `$defs/LayoutInput`.
pub struct BandedLayers;

impl LayoutCapability for BandedLayers {
    fn place(&self, input: &Json, args: &Json) -> Result<Vec<(String, f64, f64)>, String> {
        Ok(layout(input, args)?.positions)
    }

    fn sizes(&self, input: &Json, args: &Json) -> Result<Vec<(String, f64, f64)>, String> {
        Ok(layout(input, args)?.sizes)
    }

    fn routes(&self, input: &Json, args: &Json) -> Result<super::Routes, String> {
        Ok(layout(input, args)?.routes)
    }

    fn draws_its_sizes(&self) -> bool {
        true
    }
}
