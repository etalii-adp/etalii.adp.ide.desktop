//! Where a causal loop diagram's variables are drawn when nobody has placed them: evenly around a ring (the standalone
//! host's `CausalLoopLayout.cs` at `8b6ccfc`, the specification's layout `ring` with its `adp.ring.*` options, which
//! DISL's `circular` does not define: `causal-loop-diagram.md`, gap 5).
//!
//! A ring, because the notation's subject is cycles, which a layering cannot draw. Its radius grows with the widest
//! variable and the count, so no two neighbours overlap; the variables are seated by a depth-first walk of the links
//! read as undirected, earliest-declared neighbour first, from the top and clockwise, so a cycle is the rim rather than
//! chords across it. It never writes anything: a document opens on it, and a position the registration stores wins
//! for its variable.

use std::collections::HashMap;

use adp_atd_runtime::capabilities::ring::{self, RingNode};
use adp_layout::Rect;

use crate::metrics::Metrics;
use crate::reading::{Model, Variable};

/// Every variable's box on the ring, by name; a name written twice is placed once, for its first variable. The
/// capability `layout.ring`, which this code moved to (etalii.adp spec 076, T029), seats them.
pub fn compute(model: &Model, metrics: &Metrics) -> HashMap<String, Rect> {
    let variables = model.distinct_variables();
    let height = metrics.height();
    let nodes: Vec<RingNode> = variables
        .iter()
        .map(|v| RingNode {
            id: v.name.clone(),
            width: metrics.width_of(v.display()),
            height,
        })
        .collect();
    let edges: Vec<(String, String)> = model.links.iter().map(|l| (l.from.clone(), l.to.clone())).collect();
    ring::compute(&nodes, &edges, metrics.separation)
        .into_iter()
        .map(|(name, (x, y))| {
            // The capability's corner is the point on the ring, which this module's box is centred on.
            let width = nodes.iter().find(|n| n.id == name).map_or(0.0, |n| n.width);
            if nodes.len() == 1 {
                return (name, Rect::new(x, y, width, height));
            }
            (
                name,
                Rect::new(settled(x - width / 2.0), settled(y - height / 2.0), width, height),
            )
        })
        .collect()
}

/// A coordinate with the platform taken out of it, as `layout.ring` settles it.
pub fn settled(value: f64) -> f64 {
    ring::settled(value)
}

/// The order variables are seated around the ring, as `layout.ring` seats them.
pub fn ring_order(model: &Model) -> Vec<Variable> {
    let nodes: Vec<RingNode> = model
        .variables
        .iter()
        .map(|v| RingNode {
            id: v.name.clone(),
            width: 0.0,
            height: 0.0,
        })
        .collect();
    let edges: Vec<(String, String)> = model.links.iter().map(|l| (l.from.clone(), l.to.clone())).collect();
    ring::ring_order(&nodes, &edges)
        .into_iter()
        .map(|i| model.variables[i].clone())
        .collect()
}
