//! Where the boxes go when nobody has said otherwise (etalii.adp spec 030, task T082), ported from the standalone
//! host's `DotNetDependencyGraphLayout.cs` at `8b6ccfc`: projects layered by dependency depth, a project one layer
//! beyond the deepest project it references, a layer of more than twelve wrapped into columns beside itself, and the
//! packages banded beyond the last project column, ordered by id. A cycle cannot hang it: an edge closing one counts
//! its target's depth as zero. Positions are a box's top-left, in canvas units.

use std::collections::HashMap;

use adp_layout::CyclePolicy;
use adp_layout::layering;

use crate::graph::{DependencyGraphModel, DependsOnKind};
use crate::paths;

/// Horizontal distance between two layers.
const LAYER_WIDTH: f64 = 320.0;

/// Vertical distance between two boxes in one layer.
const ROW_HEIGHT: f64 = 90.0;

/// How far the package band sits beyond the last project layer.
const PACKAGE_BAND_GAP: f64 = 200.0;

/// How many boxes one layer stacks before it wraps into a second column beside itself.
pub const LAYER_WRAP_AT: usize = 12;

/// How far a wrapped column sits from the one before it, inside one layer.
const WRAP_COLUMN_WIDTH: f64 = 260.0;

/// A position for every node of `graph`, by id.
pub fn compute(graph: &DependencyGraphModel) -> HashMap<String, (f64, f64)> {
    let references: Vec<(&str, &str)> = graph
        .edges
        .iter()
        .filter(|e| e.kind == DependsOnKind::Project)
        .map(|e| (e.to.as_str(), e.from.as_str()))
        .collect();
    let nodes = graph
        .projects
        .iter()
        .map(|p| p.id.as_str())
        .chain(references.iter().flat_map(|(to, from)| [*to, *from]));
    let depths = layering::longest_path(nodes, references.iter().copied(), CyclePolicy::BackEdgeIsZero);

    // The projects grouped by depth, each group in the order its members were first met, the groups by depth.
    let mut layers: Vec<(i32, Vec<usize>)> = Vec::new();
    for (index, project) in graph.projects.iter().enumerate() {
        let depth = depths.get(&project.id).copied().unwrap_or(0);
        match layers.iter_mut().find(|(d, _)| *d == depth) {
            Some((_, members)) => members.push(index),
            None => layers.push((depth, vec![index])),
        }
    }
    layers.sort_by_key(|(depth, _)| *depth);

    let mut positions = HashMap::new();
    let mut layer_x = 0.0;
    let mut rightmost: f64 = 0.0;
    for (_, mut members) in layers {
        members.sort_by(|a, b| paths::compare_ignoring_case(&graph.projects[*a].name, &graph.projects[*b].name));
        for (index, member) in members.iter().enumerate() {
            let column = index / LAYER_WRAP_AT;
            let row = index % LAYER_WRAP_AT;
            let x = layer_x + column as f64 * WRAP_COLUMN_WIDTH;
            positions.insert(graph.projects[*member].id.clone(), (x, row as f64 * ROW_HEIGHT));
            rightmost = rightmost.max(x);
        }
        let columns = members.len().div_ceil(LAYER_WRAP_AT).max(1);
        layer_x += LAYER_WIDTH + (columns - 1) as f64 * WRAP_COLUMN_WIDTH;
    }

    let package_x = rightmost + LAYER_WIDTH + PACKAGE_BAND_GAP;
    let mut packages: Vec<_> = graph.packages.iter().collect();
    packages.sort_by(|a, b| paths::compare_ignoring_case(&a.package_id, &b.package_id));
    for (row, package) in packages.iter().enumerate() {
        positions.insert(package.id.clone(), (package_x, row as f64 * ROW_HEIGHT));
    }
    positions
}
