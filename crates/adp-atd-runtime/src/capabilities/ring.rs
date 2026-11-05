//! `layout.ring` (ATD §13.3; its contract and fixtures are in `specifications/atd/capabilities/`): nodes evenly around a
//! ring, seated by a depth-first walk of the edges read as undirected, from the top and clockwise, so a cycle is the rim
//! rather than chords across it. Moved (not rewritten) from the causal loop diagram's `ring.rs`, which calls it here
//! (etalii.adp spec 076, T029), itself ported from the standalone host's `CausalLoopLayout.cs` at `8b6ccfc`.

use std::collections::HashMap;

use serde_json::Value as Json;

use super::{LayoutCapability, number_argument};

/// A node to seat: its id and its size.
#[derive(Debug, Clone, PartialEq)]
pub struct RingNode {
    pub id: String,
    pub width: f64,
    pub height: f64,
}

/// Every node's top-left corner on the ring, by id: `nodes` in input order, `edges` by the ids of their ends, and the
/// clear space `separation` kept between neighbours.
pub fn compute(nodes: &[RingNode], edges: &[(String, String)], separation: f64) -> HashMap<String, (f64, f64)> {
    let mut corners = HashMap::with_capacity(nodes.len());
    if nodes.is_empty() {
        return corners;
    }
    // One node sits at the origin: a real position, not a missing one.
    if let [only] = nodes {
        corners.insert(only.id.clone(), (0.0, 0.0));
        return corners;
    }
    let slot = nodes.iter().map(|n| n.width).fold(f64::MIN, f64::max) + separation;
    let radius = slot.max(nodes.len() as f64 * slot / (2.0 * std::f64::consts::PI));
    let order = ring_order(nodes, edges);
    let count = order.len() as f64;
    for (index, &node) in order.iter().enumerate() {
        // From the top, clockwise, as a reader traces a loop.
        let angle = (2.0 * std::f64::consts::PI * index as f64 / count) - (std::f64::consts::PI / 2.0);
        let node = &nodes[node];
        corners.insert(
            node.id.clone(),
            (
                settled(radius * angle.cos() - node.width / 2.0),
                settled(radius * angle.sin() - node.height / 2.0),
            ),
        );
    }
    corners
}

/// A coordinate with the platform taken out of it: rounded to a millionth, as .NET's `Math.Round(value, 6)` rounds
/// (half to even after scaling), and never a negative zero.
pub fn settled(value: f64) -> f64 {
    const POWER: f64 = 1e6;
    let rounded = if value.abs() < 1e16 {
        (value * POWER).round_ties_even() / POWER
    } else {
        value
    };
    rounded + 0.0
}

/// The order nodes are seated around the ring, as indices into `nodes`: a depth-first walk of the edges read as
/// undirected, from the first node, earliest neighbour first, continuing from the earliest node left when a component
/// is exhausted.
pub fn ring_order(nodes: &[RingNode], edges: &[(String, String)]) -> Vec<usize> {
    let position: HashMap<&str, usize> = nodes.iter().enumerate().map(|(i, n)| (n.id.as_str(), i)).collect();
    let mut neighbours: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    for (from, to) in edges {
        let (Some(&from), Some(&to)) = (position.get(from.as_str()), position.get(to.as_str())) else {
            continue;
        };
        if from == to {
            continue;
        }
        neighbours[from].push(to);
        neighbours[to].push(from);
    }
    for list in &mut neighbours {
        list.sort_unstable();
    }
    let mut placed = vec![false; nodes.len()];
    let mut order = Vec::with_capacity(nodes.len());
    for seed in 0..nodes.len() {
        if placed[seed] {
            continue;
        }
        let mut stack = vec![seed];
        while let Some(current) = stack.pop() {
            if placed[current] {
                continue;
            }
            placed[current] = true;
            order.push(current);
            // Pushed in reverse so the earliest neighbour is popped first.
            for &next in neighbours[current].iter().rev() {
                if !placed[next] {
                    stack.push(next);
                }
            }
        }
    }
    order
}

/// `layout.ring` over `$defs/LayoutInput`: its nodes in input order and its edges, `separation` among the arguments.
pub struct Ring;

impl LayoutCapability for Ring {
    /// A stored position wins for its node, which the host draws there: the ring does not consult pins.
    fn consults_pins(&self) -> bool {
        false
    }

    fn place(&self, input: &Json, args: &Json) -> Result<Vec<(String, f64, f64)>, String> {
        let nodes: Vec<RingNode> = input["nodes"]
            .as_array()
            .ok_or("The input has no `nodes`.")?
            .iter()
            .map(|node| RingNode {
                id: node["id"].as_str().unwrap_or_default().to_owned(),
                width: node["width"].as_f64().unwrap_or(0.0),
                height: node["height"].as_f64().unwrap_or(0.0),
            })
            .collect();
        let edges: Vec<(String, String)> = input["edges"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|edge| Some((edge["source"].as_str()?.to_owned(), edge["target"].as_str()?.to_owned())))
            .collect();
        let corners = compute(&nodes, &edges, number_argument(args, "separation", 40.0));
        Ok(nodes
            .iter()
            .filter_map(|node| {
                let (x, y) = corners.get(&node.id)?;
                Some((node.id.clone(), *x, *y))
            })
            .collect())
    }
}
