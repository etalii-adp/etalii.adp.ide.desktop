//! The ontology reading's computed layout (etalii.adp spec 030, task T083), ported from the standalone host's
//! `Owl/OwlLayout.cs` at `8b6ccfc`: the asserted class tree as geometry. Classes fall into columns by asserted subclass
//! depth, a cycle collapsed to one column; expression nodes, datatype targets and Thing anchors stack in a lane beside
//! the node whose structure attaches them; individuals band below the classes; the header sits top-left. A class owns
//! a slot as tall as its satellites need, and a column is ordered by the barycentre of its parents' rows. The placing
//! itself moved to the capability `layout.depthColumns` (etalii.adp spec 067, T040).

use adp_atd_runtime::capabilities::depth_columns::{DepthArguments, DepthNode, DepthRole, place_depth_columns};

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

/// A position for every drawn node of `graph` (`OwlLayout.Positions`): the capability `layout.depthColumns`.
pub fn positions(graph: &OwlGraph) -> Positions {
    let nodes: Vec<DepthNode> = graph
        .nodes
        .iter()
        .map(|node| {
            let (width, height) = size_of(node);
            DepthNode {
                id: node.id.clone(),
                width,
                height,
                role: match node.kind {
                    OwlNodeKind::OntologyHeader => DepthRole::Header,
                    OwlNodeKind::Individual => DepthRole::Individual,
                    OwlNodeKind::Class | OwlNodeKind::Thing if node.id.starts_with("res:") => DepthRole::Class,
                    _ => DepthRole::Satellite,
                },
                supers: graph
                    .edges
                    .iter()
                    .filter(|e| e.kind == OwlEdgeKind::Subclass && e.from_id == node.id)
                    .map(|e| e.to_id.clone())
                    .collect(),
                owner: node.owner_id.clone(),
            }
        })
        .collect();
    let edges: Vec<(String, String)> = graph
        .edges
        .iter()
        .map(|e| (e.from_id.clone(), e.to_id.clone()))
        .collect();
    place_depth_columns(&nodes, &edges, &DepthArguments::default())
        .into_iter()
        .map(|(id, x, y)| (id, (x, y)))
        .collect()
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
