//! The scheme reading's tree-shaped layout (etalii.adp spec 030, task T083), ported from the standalone host's
//! `Skos/SkosLayout.cs` and `Skos/_Model/SkosLayoutResult.cs` at `8b6ccfc`.
//!
//! Schemes as stacked regions in IRI order, top concepts on a region's first row, every concept one layer below its
//! deepest placed broader concept, polyhierarchy placed once, unfiled concepts in a final band, collections in a
//! trailing band of their own. A layer wider than its band's column budget wraps onto centred rows, siblings kept
//! together. Cycles are detected here and nowhere else: layering runs on an acyclic subset made by excluding, per knot,
//! the edge whose (broader, narrower) pair sorts lowest - still drawn - and the validator reads [`SkosLayoutResult`]'s
//! cycles, so the picture and the report cannot disagree.
//! The placing itself moved to the capability `layout.layeredRegions` (etalii.adp spec 067, T040).

use adp_atd_runtime::capabilities::layered_regions::{Member, Region, RegionArguments, RegionEdge, place_regions};

use crate::model::RdfTriple;
use crate::reading::Positions;
use crate::skos::projection::{SkosEdgeKind, SkosProjectionResult};

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

/// Positions for every drawn element, and the cycles found (`SkosLayout.Layout`).
pub fn layout(projection: &SkosProjectionResult) -> SkosLayoutResult {
    let regions: Vec<Region> = projection
        .schemes
        .iter()
        .map(|scheme| Region {
            id: scheme.id.clone(),
            key: scheme.iri.clone(),
        })
        .collect();
    let members: Vec<Member> = projection
        .concepts
        .iter()
        .map(|concept| Member {
            id: concept.id.clone(),
            regions: concept.scheme_iris.clone(),
        })
        .collect();
    let trailing: Vec<Region> = projection
        .collections
        .iter()
        .map(|collection| Region {
            id: collection.id.clone(),
            key: collection.iri.clone(),
        })
        .collect();
    let hierarchy: Vec<_> = projection
        .edges
        .iter()
        .filter(|e| e.kind == SkosEdgeKind::Hierarchy)
        .collect();
    let edges: Vec<RegionEdge> = hierarchy
        .iter()
        .map(|e| RegionEdge {
            id: e.id.clone(),
            from: e.from_id.clone(),
            to: e.to_id.clone(),
        })
        .collect();
    let (positions, knots) = place_regions(&regions, &members, &trailing, &edges, &RegionArguments::default());
    let cycles = knots
        .into_iter()
        .map(|knot| SkosCycle {
            concept_ids: knot.members,
            excluded_edge_id: knot.excluded,
            triples: knot
                .edges
                .iter()
                .flat_map(|id| hierarchy.iter().filter(move |e| e.id == *id))
                .flat_map(|e| e.triples.iter().cloned())
                .collect(),
        })
        .collect();
    SkosLayoutResult { positions, cycles }
}
