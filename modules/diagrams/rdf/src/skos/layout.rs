//! The scheme reading's tree-shaped layout (etalii.adp spec 030, task T083), ported from the standalone host's
//! `Skos/SkosLayout.cs` and `Skos/_Model/SkosLayoutResult.cs` at `8b6ccfc`.
//!
//! Schemes as stacked regions in IRI order, top concepts on a region's first row, every concept one layer below its
//! deepest placed broader concept, polyhierarchy placed once, unfiled concepts in a final band, collections in a
//! trailing band of their own. A layer wider than its band's column budget wraps onto centred rows, siblings kept
//! together. Cycles are detected here and nowhere else: layering runs on an acyclic subset made by excluding, per knot,
//! the edge whose (broader, narrower) pair sorts lowest - still drawn - and the validator reads [`SkosLayoutResult`]'s
//! cycles, so the picture and the report cannot disagree.

use std::collections::{HashMap, HashSet};

use adp_layout::cycle_policy::CyclePolicy;
use adp_layout::{grid_packer, layering};

use crate::model::RdfTriple;
use crate::reading::Positions;
use crate::skos::projection::{SkosEdge, SkosEdgeKind, SkosProjectionResult};

const COLUMN_WIDTH: f64 = 240.0;
const ROW_HEIGHT: f64 = 110.0;
const REGION_GAP: f64 = 90.0;
const REGION_HEADER: f64 = 60.0;
const LAYER_GAP: f64 = 50.0;
const MINIMUM_COLUMNS: usize = 8;

/// One cycle in the asserted hierarchy (`SkosCycle`): its concepts sorted, the excluded edge, and every triple stating
/// an edge on it.
#[derive(Debug, Clone, PartialEq)]
pub struct SkosCycle {
    pub concept_ids: Vec<String>,
    pub excluded_edge_id: String,
    pub triples: Vec<RdfTriple>,
}

/// The computed layout, and the cycles found on the way (`SkosLayoutResult`).
#[derive(Debug, Clone, PartialEq)]
pub struct SkosLayoutResult {
    pub positions: Positions,
    pub cycles: Vec<SkosCycle>,
}

/// The widest a row may be, in columns: about the square root of twice the elements to place.
fn columns_for(count: usize) -> usize {
    MINIMUM_COLUMNS.max((2.0 * count as f64).sqrt().ceil() as usize)
}

/// Positions for every drawn element, and the cycles found (`SkosLayout.Layout`).
pub fn layout(projection: &SkosProjectionResult) -> SkosLayoutResult {
    let hierarchy: Vec<&SkosEdge> = projection
        .edges
        .iter()
        .filter(|e| e.kind == SkosEdgeKind::Hierarchy)
        .collect();
    let (acyclic, cycles) = break_cycles(hierarchy);

    let mut broaders_of: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in &acyclic {
        broaders_of
            .entry(edge.to_id.as_str())
            .or_default()
            .push(edge.from_id.as_str());
    }

    let mut band = Band {
        acyclic: &acyclic,
        broaders_of: &broaders_of,
        positions: Positions::new(),
        placed: HashSet::new(),
    };
    let mut y = 0.0;

    let mut schemes: Vec<_> = projection.schemes.iter().collect();
    schemes.sort_by(|a, b| a.iri.cmp(&b.iri));
    for scheme in schemes {
        band.positions.insert(scheme.id.clone(), (0.0, y));
        let members: HashSet<String> = projection
            .concepts
            .iter()
            .filter(|c| c.scheme_iris.contains(&scheme.iri) && !band.placed.contains(&c.id))
            .map(|c| c.id.clone())
            .collect();
        y = band.place(&members, y + REGION_HEADER) + REGION_GAP;
    }

    let unfiled: HashSet<String> = projection
        .concepts
        .iter()
        .filter(|c| !band.placed.contains(&c.id))
        .map(|c| c.id.clone())
        .collect();
    if !unfiled.is_empty() {
        y = band.place(&unfiled, y) + REGION_GAP;
    }

    let mut collections: Vec<_> = projection.collections.iter().collect();
    collections.sort_by(|a, b| a.iri.cmp(&b.iri));
    let collection_columns = columns_for(collections.len());
    for (index, collection) in collections.iter().enumerate() {
        let (x, down) = grid_packer::cell(index, collection_columns, COLUMN_WIDTH, ROW_HEIGHT);
        band.positions.insert(collection.id.clone(), (x, y + down));
    }

    SkosLayoutResult {
        positions: band.positions,
        cycles,
    }
}

struct Band<'a> {
    acyclic: &'a [&'a SkosEdge],
    broaders_of: &'a HashMap<&'a str, Vec<&'a str>>,
    positions: Positions,
    placed: HashSet<String>,
}

impl Band<'_> {
    /// Layers a band's members from `top`, and answers the y just past the band.
    fn place(&mut self, members: &HashSet<String>, top: f64) -> f64 {
        let mut nodes: Vec<&String> = members.iter().collect();
        nodes.sort();
        let depth = layering::longest_path(
            nodes,
            self.acyclic.iter().map(|e| (e.from_id.as_str(), e.to_id.as_str())),
            CyclePolicy::Kahn,
        );
        let mut by_depth: Vec<(i32, Vec<String>)> = Vec::new();
        for (id, level) in depth {
            match by_depth.iter_mut().find(|(d, _)| *d == level) {
                Some((_, ids)) => ids.push(id),
                None => by_depth.push((level, vec![id])),
            }
        }
        by_depth.sort_by_key(|(d, _)| *d);

        let mut sequence: HashMap<String, usize> = HashMap::new();
        let mut ordered: Vec<Vec<String>> = Vec::new();
        for (_, mut level) in by_depth {
            let keys: HashMap<String, f64> = level
                .iter()
                .map(|id| (id.clone(), self.broader_position(id, &sequence)))
                .collect();
            level.sort_by(|a, b| keys[a].total_cmp(&keys[b]).then_with(|| a.cmp(b)));
            for id in &level {
                let next = sequence.len();
                sequence.insert(id.clone(), next);
            }
            ordered.push(level);
        }

        let columns = columns_for(members.len());
        let band_columns = ordered
            .iter()
            .map(Vec::len)
            .max()
            .map_or(0, |widest| columns.min(widest));
        let mut row_top = top;
        for level in &ordered {
            for row in level.chunks(columns) {
                let indent = (band_columns as f64 - row.len() as f64) / 2.0;
                for (x, id) in row.iter().enumerate() {
                    self.positions
                        .insert(id.clone(), ((indent + x as f64) * COLUMN_WIDTH, row_top));
                    self.placed.insert(id.clone());
                }
                row_top += ROW_HEIGHT;
            }
            row_top += LAYER_GAP;
        }
        if ordered.is_empty() { top } else { row_top - LAYER_GAP }
    }

    /// The mean place of a concept's already-placed broader concepts, or the largest number when it has none.
    fn broader_position(&self, id: &str, sequence: &HashMap<String, usize>) -> f64 {
        let places: Vec<f64> = self
            .broaders_of
            .get(id)
            .map(|broaders| {
                broaders
                    .iter()
                    .filter_map(|b| sequence.get(*b).map(|s| *s as f64))
                    .collect()
            })
            .unwrap_or_default();
        if places.is_empty() {
            f64::MAX
        } else {
            places.iter().sum::<f64>() / places.len() as f64
        }
    }
}

/// The acyclic layering subset and the cycles found: per remaining knot, the edge whose (broader, narrower) pair sorts
/// lowest is excluded and detection reruns, until nothing knots.
fn break_cycles(hierarchy: Vec<&SkosEdge>) -> (Vec<&SkosEdge>, Vec<SkosCycle>) {
    let mut included = hierarchy;
    let mut cycles = Vec::new();
    loop {
        let Some(knot) = find_knot(&included) else {
            return (included, cycles);
        };
        let knot_edges: Vec<&SkosEdge> = included
            .iter()
            .copied()
            .filter(|e| knot.contains(&e.from_id) && knot.contains(&e.to_id))
            .collect();
        let excluded = knot_edges
            .iter()
            .copied()
            .min_by(|a, b| a.from_id.cmp(&b.from_id).then_with(|| a.to_id.cmp(&b.to_id)))
            .expect("a knot has edges");
        let at = included
            .iter()
            .position(|e| std::ptr::eq(*e, excluded))
            .expect("the excluded edge is included");
        included.remove(at);
        let mut concept_ids: Vec<String> = knot.into_iter().collect();
        concept_ids.sort();
        cycles.push(SkosCycle {
            concept_ids,
            excluded_edge_id: excluded.id.clone(),
            triples: knot_edges.iter().flat_map(|e| e.triples.iter().cloned()).collect(),
        });
    }
}

/// One strongly connected set of two or more nodes (or a self-loop), or none when none is left: Tarjan's algorithm,
/// iteratively, from the sources in ordinal order.
fn find_knot(edges: &[&SkosEdge]) -> Option<HashSet<String>> {
    if let Some(edge) = edges.iter().find(|e| e.from_id == e.to_id) {
        return Some(HashSet::from([edge.from_id.clone()]));
    }

    let mut next: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut starts: Vec<&str> = Vec::new();
    for edge in edges {
        if !next.contains_key(edge.from_id.as_str()) {
            starts.push(edge.from_id.as_str());
        }
        next.entry(edge.from_id.as_str()).or_default().push(edge.to_id.as_str());
    }
    starts.sort();

    let mut index: HashMap<&str, usize> = HashMap::new();
    let mut low: HashMap<&str, usize> = HashMap::new();
    let mut on_stack: HashSet<&str> = HashSet::new();
    let mut stack: Vec<&str> = Vec::new();
    let mut counter = 0;
    let empty: Vec<&str> = Vec::new();

    for start in starts {
        if index.contains_key(start) {
            continue;
        }
        let mut work: Vec<(&str, usize)> = vec![(start, 0)];
        while let Some((node, child)) = work.pop() {
            if child == 0 {
                index.insert(node, counter);
                low.insert(node, counter);
                counter += 1;
                stack.push(node);
                on_stack.insert(node);
            }
            let children = next.get(node).unwrap_or(&empty);
            if child < children.len() {
                work.push((node, child + 1));
                let target = children[child];
                match index.get(target) {
                    None => work.push((target, 0)),
                    Some(&target_index) if on_stack.contains(target) => {
                        let lowest = low[node].min(target_index);
                        low.insert(node, lowest);
                    }
                    Some(_) => {}
                }
            } else {
                if low[node] == index[node] {
                    let mut component = HashSet::new();
                    loop {
                        let popped = stack.pop().expect("the node is on the stack");
                        on_stack.remove(popped);
                        component.insert(popped.to_owned());
                        if popped == node {
                            break;
                        }
                    }
                    if component.len() > 1 {
                        return Some(component);
                    }
                }
                if let Some(&(parent, _)) = work.last() {
                    let lowest = low[parent].min(low[node]);
                    low.insert(parent, lowest);
                }
            }
        }
    }
    None
}
