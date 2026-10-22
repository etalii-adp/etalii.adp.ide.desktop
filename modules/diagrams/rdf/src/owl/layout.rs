//! The ontology reading's computed layout (etalii.adp spec 030, task T083), ported from the standalone host's
//! `Owl/OwlLayout.cs` at `8b6ccfc`: the asserted class tree as geometry. Classes fall into columns by asserted subclass
//! depth, a cycle collapsed to one column; expression nodes, datatype targets and Thing anchors stack in a lane beside
//! the node whose structure attaches them; individuals band below the classes; the header sits top-left. A class owns
//! a slot as tall as its satellites need, and a column is ordered by the barycentre of its parents' rows.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::owl::hierarchy::{self, Supers};
use crate::owl::projection::{OwlEdgeKind, OwlGraph, OwlNode, OwlNodeKind};
use crate::reading::{self, Positions};

const SHAPE_WIDTH: f64 = 190.0;
const SHAPE_HEIGHT: f64 = 70.0;
const THING_SCALE: f64 = 0.55;

const CARD_WIDTH: f64 = 220.0;
const CARD_HEADER_HEIGHT: f64 = 30.0;
const CARD_BADGES_HEIGHT: f64 = 16.0;
const CARD_ROW_HEIGHT: f64 = 16.0;
const CARD_FOOTER_PADDING: f64 = 10.0;

const LANE_OFFSET_X: f64 = 250.0;
const LANE_WIDTH: f64 = SHAPE_WIDTH;
const COLUMN_GAP: f64 = 90.0;
const COLUMN_WIDTH: f64 = LANE_OFFSET_X + LANE_WIDTH + COLUMN_GAP;

const SLOT_HEIGHT: f64 = 130.0;
const SATELLITE_HEIGHT: f64 = 110.0;
const SATELLITE_FIRST_OFFSET_Y: f64 = 10.0;

const HEADER_GAP: f64 = 80.0;
const BAND_GAP: f64 = 140.0;
const BAND_COLUMN_GAP: f64 = 60.0;
const BAND_ROW_GAP: f64 = 50.0;
const BAND_COLUMNS: usize = 5;

/// What a node occupies: the size the canvas draws it at (`OwlLayout.SizeOf`).
pub fn size_of(node: &OwlNode) -> (f64, f64) {
    match node.kind {
        OwlNodeKind::Individual | OwlNodeKind::OntologyHeader => (
            CARD_WIDTH,
            CARD_HEADER_HEIGHT
                + if node.badges.is_empty() {
                    0.0
                } else {
                    CARD_BADGES_HEIGHT
                }
                + node.rows.len() as f64 * CARD_ROW_HEIGHT
                + CARD_FOOTER_PADDING,
        ),
        OwlNodeKind::Thing => (SHAPE_WIDTH * THING_SCALE, SHAPE_HEIGHT * THING_SCALE),
        _ => (SHAPE_WIDTH, SHAPE_HEIGHT),
    }
}

/// A position for every drawn node of `graph` (`OwlLayout.Positions`).
pub fn positions(graph: &OwlGraph) -> Positions {
    let mut positions: Positions = BTreeMap::new();

    let mut header_height = 0.0;
    if let Some(header) = graph.nodes.iter().find(|n| n.kind == OwlNodeKind::OntologyHeader) {
        positions.insert(header.id.clone(), (0.0, 0.0));
        header_height = size_of(header).1 + HEADER_GAP;
    }

    let order: HashMap<&str, usize> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();

    let class_ids: Vec<&str> = graph
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, OwlNodeKind::Class | OwlNodeKind::Thing) && n.id.starts_with("res:"))
        .map(|n| n.id.as_str())
        .collect();
    let class_set: HashSet<&str> = class_ids.iter().copied().collect();
    let mut supers: Supers = class_ids.iter().map(|id| ((*id).to_owned(), Vec::new())).collect();
    for edge in &graph.edges {
        if edge.kind == OwlEdgeKind::Subclass
            && class_set.contains(edge.from_id.as_str())
            && class_set.contains(edge.to_id.as_str())
        {
            supers[edge.from_id.as_str()].push(edge.to_id.clone());
        }
    }
    let depths = hierarchy::depths(&supers);

    let mut satellites: HashMap<&str, Vec<&str>> = class_ids.iter().map(|id| (*id, Vec::new())).collect();
    for node in &graph.nodes {
        if positions.contains_key(&node.id)
            || class_set.contains(node.id.as_str())
            || node.kind == OwlNodeKind::Individual
        {
            continue;
        }
        let host = if !node.owner_id.is_empty() && satellites.contains_key(node.owner_id.as_str()) {
            Some(node.owner_id.as_str())
        } else {
            graph
                .edges
                .iter()
                .filter(|e| e.from_id == node.id || e.to_id == node.id)
                .map(|e| {
                    if e.from_id == node.id {
                        e.to_id.as_str()
                    } else {
                        e.from_id.as_str()
                    }
                })
                .find(|other| satellites.contains_key(other))
        };
        if let Some(host) = host
            && let Some(list) = satellites.get_mut(host)
        {
            list.push(node.id.as_str());
        }
    }

    let mut columns: BTreeMap<i32, Vec<&str>> = BTreeMap::new();
    for id in &class_ids {
        let depth = depths.get(*id).copied().unwrap_or(0);
        columns.entry(depth).or_default().push(id);
    }

    let mut row_of: HashMap<&str, f64> = HashMap::new();
    let class_top = header_height;
    let mut max_y = class_top;
    for (depth, members) in &columns {
        let mut arranged = members.clone();
        if *depth == 0 {
            arranged.sort_by_key(|id| order[id]);
        } else {
            let key = |id: &&str| -> f64 {
                let parents: Vec<f64> = supers[*id]
                    .iter()
                    .filter_map(|parent| row_of.get(parent.as_str()).copied())
                    .collect();
                if parents.is_empty() {
                    f64::MAX
                } else {
                    parents.iter().sum::<f64>() / parents.len() as f64
                }
            };
            let keyed: Vec<(f64, usize, &str)> = arranged.iter().map(|id| (key(id), order[id], *id)).collect();
            let mut keyed = keyed;
            keyed.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            arranged = keyed.into_iter().map(|(_, _, id)| id).collect();
        }

        let mut y = class_top;
        for id in arranged {
            let attached = &satellites[id];
            let x = f64::from(*depth) * COLUMN_WIDTH;
            positions.insert(id.to_owned(), (x, y));
            row_of.insert(id, y);
            for (i, satellite) in attached.iter().enumerate() {
                positions.insert(
                    (*satellite).to_owned(),
                    (
                        x + LANE_OFFSET_X,
                        y + SATELLITE_FIRST_OFFSET_Y + i as f64 * SATELLITE_HEIGHT,
                    ),
                );
            }
            let slot = SLOT_HEIGHT.max(attached.len() as f64 * SATELLITE_HEIGHT + SLOT_HEIGHT / 2.0);
            y += slot;
            max_y = max_y.max(y);
        }
    }

    let individuals: Vec<&OwlNode> = graph
        .nodes
        .iter()
        .filter(|n| n.kind == OwlNodeKind::Individual && !positions.contains_key(&n.id))
        .collect();
    max_y = band(&mut positions, &individuals, max_y);
    let rest: Vec<&OwlNode> = graph.nodes.iter().filter(|n| !positions.contains_key(&n.id)).collect();
    band(&mut positions, &rest, max_y);
    positions
}

fn band(positions: &mut Positions, banded: &[&OwlNode], top: f64) -> f64 {
    if banded.is_empty() {
        return top;
    }
    let column_pitch = banded.iter().map(|n| size_of(n).0).fold(f64::MIN, f64::max) + BAND_COLUMN_GAP;
    let row_pitch = banded.iter().map(|n| size_of(n).1).fold(f64::MIN, f64::max) + BAND_ROW_GAP;
    let y = top + BAND_GAP;
    let mut bottom = top;
    for (placed, node) in banded.iter().enumerate() {
        let (x, down) = adp_layout::grid_packer::cell(placed, BAND_COLUMNS, column_pitch, row_pitch);
        positions.insert(node.id.clone(), (x, y + down));
        bottom = bottom.max(y + down + row_pitch);
    }
    bottom
}

/// Whether an element of this reading may take a stored position at all: an IRI-derived id (`res:`, `ind:`).
pub fn is_positionable(element_id: &str) -> bool {
    element_id.starts_with("res:") || element_id.starts_with("ind:")
}

/// The overlay under this reading's boundary: a stored position wins for an IRI-derived id and never applies to an
/// expression node or a materialized anchor (`OwlLayout.Apply`).
pub fn apply(computed: Positions, stored: &Positions) -> Positions {
    let positionable: Positions = stored
        .iter()
        .filter(|(id, _)| is_positionable(id))
        .map(|(id, p)| (id.clone(), *p))
        .collect();
    reading::apply(computed, &positionable)
}
