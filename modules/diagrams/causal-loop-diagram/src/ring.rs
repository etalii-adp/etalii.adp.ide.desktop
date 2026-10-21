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

use adp_layout::Rect;

use crate::metrics::Metrics;
use crate::reading::{Model, Variable};

/// Every variable's box on the ring, by name; a name written twice is placed once, for its first variable.
pub fn compute(model: &Model, metrics: &Metrics) -> HashMap<String, Rect> {
    let variables = model.distinct_variables();
    let mut boxes = HashMap::with_capacity(variables.len());
    if variables.is_empty() {
        return boxes;
    }
    let height = metrics.height();
    let widths: HashMap<&str, f64> = variables
        .iter()
        .map(|v| (v.name.as_str(), metrics.width_of(v.display())))
        .collect();

    // One variable sits at the origin: a real position, not a missing one.
    if let [only] = variables.as_slice() {
        boxes.insert(
            only.name.clone(),
            Rect::new(0.0, 0.0, widths[only.name.as_str()], height),
        );
        return boxes;
    }

    let slot = widths.values().copied().fold(f64::MIN, f64::max) + metrics.separation;
    let radius = slot.max(variables.len() as f64 * slot / (2.0 * std::f64::consts::PI));
    let order = ring_order(&Model {
        variables: variables.clone(),
        links: model.links.clone(),
    });
    let count = order.len() as f64;
    for (index, variable) in order.iter().enumerate() {
        // From the top, clockwise, as a reader traces a loop.
        let angle = (2.0 * std::f64::consts::PI * index as f64 / count) - (std::f64::consts::PI / 2.0);
        let centre_x = radius * angle.cos();
        let centre_y = radius * angle.sin();
        let width = widths[variable.name.as_str()];
        boxes.insert(
            variable.name.clone(),
            Rect::new(
                settled(centre_x - width / 2.0),
                settled(centre_y - height / 2.0),
                width,
                height,
            ),
        );
    }
    boxes
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

/// The order variables are seated around the ring: a depth-first walk of the links read as undirected, from the
/// first declared variable, earliest-declared neighbour first, continuing from the earliest variable left when a
/// component is exhausted.
pub fn ring_order(model: &Model) -> Vec<Variable> {
    let position: HashMap<&str, usize> = model
        .variables
        .iter()
        .enumerate()
        .map(|(i, v)| (v.name.as_str(), i))
        .collect();
    let mut neighbours: Vec<Vec<usize>> = vec![Vec::new(); model.variables.len()];
    for link in &model.links {
        let (Some(&from), Some(&to)) = (position.get(link.from.as_str()), position.get(link.to.as_str())) else {
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
    let mut placed = vec![false; model.variables.len()];
    let mut order = Vec::with_capacity(model.variables.len());
    for seed in 0..model.variables.len() {
        if placed[seed] {
            continue;
        }
        let mut stack = vec![seed];
        while let Some(current) = stack.pop() {
            if placed[current] {
                continue;
            }
            placed[current] = true;
            order.push(model.variables[current].clone());
            // Pushed in reverse so the earliest-declared neighbour is popped first.
            for &next in neighbours[current].iter().rev() {
                if !placed[next] {
                    stack.push(next);
                }
            }
        }
    }
    order
}
