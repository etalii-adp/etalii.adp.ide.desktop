//! What Arrange diagram does: Meyer's self-organizing graphs with every source of randomness replaced by a function
//! of the document and the iteration index (the standalone host's `SelfOrganizingLayout.cs` and
//! `SelfOrganizingResult.cs` at `8b6ccfc`; the specification's layout `selfOrganizing`, the plugin
//! `net.etalii.adp.systems.cld`).
//!
//! The variables start on a phyllotaxis spiral in model order, the stimuli are a two-dimensional Halton sequence over
//! the rectangle around it, the best-matching unit's ties go to the earlier variable, and the neighbourhood is
//! measured in hops over the links read as undirected: a variable in another component is not pulled at all. After a
//! fixed 2000 iterations the boxes still overlapping are pushed apart by the shared overlap resolver in at most 200
//! rounds; where they cannot be, or the diagram is past the budget, the arrangement is refused with the size it
//! failed at. The same document gives the same positions in every process.

use std::collections::{HashMap, VecDeque};

use adp_layout::Rect;
use adp_layout::overlap_resolver;

use crate::metrics::Metrics;
use crate::reading::{Model, Variable};

/// The drawn-element budget above which the layout declines to start.
pub const DEFAULT_BUDGET: usize = 1000;

/// How many stimuli are presented.
const ITERATIONS: usize = 2000;
/// The learning rate at the first iteration and at the last.
const INITIAL_RATE: f64 = 0.9;
const FINAL_RATE: f64 = 0.01;
/// The neighbourhood radius, in hops, at the first iteration and at the last.
const INITIAL_RADIUS: f64 = 3.0;
const FINAL_RADIUS: f64 = 0.35;
/// What a pair is pushed beyond half its overlap, so two boxes just cleared do not touch.
const NUDGE: f64 = 0.5;
/// How many rounds the separation pass gets.
pub const SEPARATION_ROUNDS: usize = 200;
/// The golden angle, in radians.
const GOLDEN_ANGLE: f64 = 2.399_963_229_728_653_3;

/// Where each variable was placed, by name, or why the layout declined.
pub type Arrangement = Result<HashMap<String, Rect>, String>;

/// Arranges `model`'s variables, with `budget` and `rounds` as the standalone host's parameters.
pub fn compute(model: &Model, metrics: &Metrics, budget: usize, rounds: usize) -> Arrangement {
    let variables = &model.variables;
    if variables.len() > budget {
        return Err(over_budget(variables.len(), budget));
    }
    if variables.is_empty() {
        return Ok(HashMap::new());
    }
    let widths: Vec<f64> = variables.iter().map(|v| metrics.width_of(v.display())).collect();
    let height = metrics.height();
    let mut positions = initial_positions(variables.len(), &widths, height, metrics.separation);
    if variables.len() > 1 {
        organize(model, variables, &mut positions, &widths, height, metrics.separation);
    }
    let remaining = separate(&mut positions, &widths, height, metrics.separation, rounds);
    if remaining > 0 {
        return Err(could_not_separate(variables.len(), remaining));
    }
    let mut boxes = HashMap::with_capacity(variables.len());
    for (index, variable) in variables.iter().enumerate() {
        boxes.insert(
            variable.name.clone(),
            Rect::new(
                positions[index].0 - widths[index] / 2.0,
                positions[index].1 - height / 2.0,
                widths[index],
                height,
            ),
        );
    }
    Ok(boxes)
}

/// Why a diagram of `size` variables past the `budget` is not arranged: both numbers, so the refusal names a fact
/// about the document.
pub fn over_budget(size: usize, budget: usize) -> String {
    format!(
        "This diagram has {size} variables, which is more than the {budget} this layout arranges. It has been left as it is."
    )
}

/// Why a diagram of `size` variables whose boxes `overlaps` pairs still overlap is not arranged.
pub fn could_not_separate(size: usize, overlaps: usize) -> String {
    format!(
        "This layout could not arrange {size} variables without {overlaps} of them overlapping, so the diagram has been left as it is."
    )
}

/// Pushes overlapping variables apart, every box one height; how many pairs still overlap when the rounds are spent.
pub fn separate(positions: &mut [(f64, f64)], widths: &[f64], height: f64, separation: f64, rounds: usize) -> usize {
    let sizes: Vec<(f64, f64)> = widths.iter().map(|w| (*w, height)).collect();
    overlap_resolver::separate(positions, &sizes, separation, rounds, NUDGE)
}

/// Where each variable starts: a phyllotaxis spiral in model order, spaced by the boxes' size.
pub fn initial_positions(count: usize, widths: &[f64], height: f64, separation: f64) -> Vec<(f64, f64)> {
    let widest = if widths.is_empty() {
        0.0
    } else {
        widths.iter().copied().fold(f64::MIN, f64::max)
    };
    let spacing = (widest + height + separation * 2.0) / 2.0;
    (0..count)
        .map(|index| {
            let radius = spacing * (index as f64).sqrt();
            let angle = index as f64 * GOLDEN_ANGLE;
            (radius * angle.cos(), radius * angle.sin())
        })
        .collect()
}

fn organize(
    model: &Model,
    variables: &[Variable],
    positions: &mut [(f64, f64)],
    widths: &[f64],
    height: f64,
    separation: f64,
) {
    let hops = graph_distances(model, variables);
    let (min_x, min_y, max_x, max_y) = extent(positions);
    let widest = widths.iter().copied().fold(f64::MIN, f64::max);
    let padding = ((widest + height) / 2.0) + separation;
    let minimum_x = min_x - padding;
    let minimum_y = min_y - padding;
    let span_x = (max_x - min_x + padding * 2.0).max(1.0);
    let span_y = (max_y - min_y + padding * 2.0).max(1.0);
    for iteration in 0..ITERATIONS {
        let progress = iteration as f64 / ITERATIONS as f64;
        let rate = INITIAL_RATE * (FINAL_RATE / INITIAL_RATE).powf(progress);
        let radius = INITIAL_RADIUS * (FINAL_RADIUS / INITIAL_RADIUS).powf(progress);
        let falloff = 2.0 * radius * radius;
        let stimulus_x = minimum_x + halton(iteration + 1, 2) * span_x;
        let stimulus_y = minimum_y + halton(iteration + 1, 3) * span_y;
        let winner = best_matching_unit(positions, stimulus_x, stimulus_y);
        for index in 0..positions.len() {
            let distance = hops[winner][index];
            if distance < 0 {
                continue;
            }
            let distance = f64::from(distance);
            let influence = rate * (-(distance * distance) / falloff).exp();
            positions[index] = (
                positions[index].0 + influence * (stimulus_x - positions[index].0),
                positions[index].1 + influence * (stimulus_y - positions[index].1),
            );
        }
    }
}

/// The variable nearest the point, ties going to the earlier.
fn best_matching_unit(positions: &[(f64, f64)], x: f64, y: f64) -> usize {
    let mut winner = 0;
    let mut best = f64::MAX;
    for (index, (px, py)) in positions.iter().enumerate() {
        let dx = px - x;
        let dy = py - y;
        let distance = dx * dx + dy * dy;
        if distance < best {
            best = distance;
            winner = index;
        }
    }
    winner
}

/// Hop counts between every pair of variables over the links read as undirected; -1 where unreachable.
pub fn graph_distances(model: &Model, variables: &[Variable]) -> Vec<Vec<i32>> {
    let mut index: HashMap<&str, usize> = HashMap::with_capacity(variables.len());
    for (i, variable) in variables.iter().enumerate() {
        // The last of a name written twice, as a dictionary assigned in order keeps it.
        index.insert(variable.name.as_str(), i);
    }
    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); variables.len()];
    for link in &model.links {
        let (Some(&from), Some(&to)) = (index.get(link.from.as_str()), index.get(link.to.as_str())) else {
            continue;
        };
        if from == to {
            continue;
        }
        adjacency[from].push(to);
        adjacency[to].push(from);
    }
    (0..variables.len())
        .map(|source| breadth_first(&adjacency, source))
        .collect()
}

fn breadth_first(adjacency: &[Vec<usize>], source: usize) -> Vec<i32> {
    let mut distances = vec![-1; adjacency.len()];
    distances[source] = 0;
    let mut queue = VecDeque::from([source]);
    while let Some(current) = queue.pop_front() {
        for &neighbour in &adjacency[current] {
            if distances[neighbour] < 0 {
                distances[neighbour] = distances[current] + 1;
                queue.push_back(neighbour);
            }
        }
    }
    distances
}

/// The `index`-th value of the radical-inverse (Halton) sequence in `radix`, in `[0, 1)`.
pub fn halton(mut index: usize, radix: usize) -> f64 {
    let mut result = 0.0;
    let mut fraction = 1.0 / radix as f64;
    while index > 0 {
        result += fraction * (index % radix) as f64;
        index /= radix;
        fraction /= radix as f64;
    }
    result
}

fn extent(positions: &[(f64, f64)]) -> (f64, f64, f64, f64) {
    positions.iter().fold(
        (f64::MAX, f64::MAX, f64::MIN, f64::MIN),
        |(min_x, min_y, max_x, max_y), (x, y)| (min_x.min(*x), min_y.min(*y), max_x.max(*x), max_y.max(*y)),
    )
}
