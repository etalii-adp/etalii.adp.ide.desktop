//! `arrange.selfOrganising` (ATD §13.3; its contract and fixtures are in `specifications/atd/capabilities/`): Meyer's
//! self-organizing graphs with every source of randomness replaced by a function of the input and the iteration index.
//! Moved (not rewritten) from the causal loop diagram's `self_organizing.rs`, which calls it here (etalii.adp spec 076,
//! T033), itself ported from the standalone host's `SelfOrganizingLayout.cs` and `SelfOrganizingResult.cs` at
//! `8b6ccfc`.
//!
//! The elements start on a phyllotaxis spiral in their order, the stimuli are a two-dimensional Halton sequence over
//! the rectangle around it, the best-matching unit's ties go to the earlier element, and the neighbourhood is measured
//! in hops over the relations read as undirected: an element in another component is not pulled at all. After a fixed
//! 2000 iterations the boxes still overlapping are pushed apart by the shared overlap resolver in at most `rounds`
//! rounds; where they cannot be, or the graph is past the budget, the arrangement is refused with the size it failed
//! at.

use std::collections::{HashMap, VecDeque};

use adp_layout::overlap_resolver;
use serde_json::{Value as Json, json};

use super::two_sided_tree::round2;

/// The element budget above which the arrangement declines to start.
pub const DEFAULT_BUDGET: usize = 1000;
/// How many rounds the separation pass gets.
pub const SEPARATION_ROUNDS: usize = 200;
/// The clear space kept between two boxes.
pub const DEFAULT_SEPARATION: f64 = 40.0;

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
/// The golden angle, in radians.
const GOLDEN_ANGLE: f64 = 2.399_963_229_728_653_3;

/// An element to arrange: its id, and its box's width; every box is one height.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrangedElement {
    pub id: String,
    pub width: f64,
}

/// The centre each element was placed at, in the elements' order, or why the arrangement declined.
pub type Arrangement = Result<Vec<(f64, f64)>, String>;

/// Arranges `elements`, every box `height` tall, `separation` clear of each other, over `relations` by the ids of their
/// ends, with `budget` and `rounds` as the standalone host's parameters.
pub fn compute(
    elements: &[ArrangedElement],
    relations: &[(String, String)],
    height: f64,
    separation: f64,
    budget: usize,
    rounds: usize,
) -> Arrangement {
    if elements.len() > budget {
        return Err(over_budget(elements.len(), budget));
    }
    if elements.is_empty() {
        return Ok(Vec::new());
    }
    let widths: Vec<f64> = elements.iter().map(|e| e.width).collect();
    let mut positions = initial_positions(elements.len(), &widths, height, separation);
    if elements.len() > 1 {
        organize(elements, relations, &mut positions, &widths, height, separation);
    }
    let remaining = separate(&mut positions, &widths, height, separation, rounds);
    if remaining > 0 {
        return Err(could_not_separate(elements.len(), remaining));
    }
    Ok(positions)
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
    elements: &[ArrangedElement],
    relations: &[(String, String)],
    positions: &mut [(f64, f64)],
    widths: &[f64],
    height: f64,
    separation: f64,
) {
    let hops = graph_distances(elements, relations);
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

/// Hop counts between every pair of elements over the relations read as undirected; -1 where unreachable.
pub fn graph_distances(elements: &[ArrangedElement], relations: &[(String, String)]) -> Vec<Vec<i32>> {
    let mut index: HashMap<&str, usize> = HashMap::with_capacity(elements.len());
    for (i, element) in elements.iter().enumerate() {
        // The last of an id given twice, as a dictionary assigned in order keeps it.
        index.insert(element.id.as_str(), i);
    }
    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); elements.len()];
    for (from, to) in relations {
        let (Some(&from), Some(&to)) = (index.get(from.as_str()), index.get(to.as_str())) else {
            continue;
        };
        if from == to {
            continue;
        }
        adjacency[from].push(to);
        adjacency[to].push(from);
    }
    (0..elements.len())
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

/// `arrange.selfOrganising` over `$defs/AnalysisInput`: its `elements` with their sizes, in order, and its `relations`;
/// one `move` per element to its centre, rounded to 0.01, or the refusal as one finding of severity `error`.
pub fn arrange(input: &Json, args: &Json) -> Result<Json, String> {
    Ok(match moves(input, args)? {
        Ok(moves) => {
            let actions: Vec<Json> = moves
                .into_iter()
                .map(|(id, x, y)| json!({"kind": "move", "target": id, "x": round2(x), "y": round2(y)}))
                .collect();
            json!({"actions": actions})
        }
        Err(message) => json!({"findings": [{"severity": "error", "message": message}]}),
    })
}

/// The moves of an arrangement: each element's id and the centre it moves to, or the arrangement's refusal.
pub type Moves = Result<Vec<(String, f64, f64)>, String>;

/// The centre each element of `input` moves to, in order, as the arrangement computes it, not rounded; or the refusal.
pub fn moves(input: &Json, args: &Json) -> Result<Moves, String> {
    let given = input["elements"].as_array().ok_or("The input has no `elements`.")?;
    let elements: Vec<ArrangedElement> = given
        .iter()
        .map(|e| ArrangedElement {
            id: e["id"].as_str().unwrap_or_default().to_owned(),
            width: e["width"].as_f64().unwrap_or(0.0),
        })
        .collect();
    let height = given.first().and_then(|e| e["height"].as_f64()).unwrap_or(0.0);
    let relations: Vec<(String, String)> = input["relations"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| Some((r["source"].as_str()?.to_owned(), r["target"].as_str()?.to_owned())))
        .collect();
    let count = |name: &str, default: usize| args[name].as_u64().map_or(default, |n| n as usize);
    let separation = args["separation"].as_f64().unwrap_or(DEFAULT_SEPARATION);
    Ok(compute(
        &elements,
        &relations,
        height,
        separation,
        count("budget", DEFAULT_BUDGET),
        count("rounds", SEPARATION_ROUNDS),
    )
    .map(|centres| {
        elements
            .iter()
            .zip(centres)
            .map(|(e, (x, y))| (e.id.clone(), x, y))
            .collect()
    }))
}
