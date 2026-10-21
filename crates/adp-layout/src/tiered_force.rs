//! The `tiered-force` layout mode (etalii.adp spec 030, FR-018), ported from the standalone host's
//! `src/client/src/canvas/library/layout/tieredForceLayout.ts` at `8b6ccfc` (agent-activity-diagram Requirements 6.5
//! to 6.10).
//!
//! A layout that radiates outwards by element type: the first tier in the middle, each further tier on a wider ring,
//! connected elements drawn together and all elements held apart. It is a small force simulation written out here
//! rather than taken from a library for one reason: **the same input must give the same picture, on every machine and
//! in every host.** So nothing here is random, the number of steps is fixed, and every loop runs in the order of the
//! input.
//!
//! What it does, in order:
//!
//! 1. **Rings.** Each tier gets a radius: wide enough for what the tier holds, and at least one ring gap beyond the
//!    tier inside it.
//! 2. **A start.** Tier by tier outwards, an element starts on its ring at the mean direction of the elements it is
//!    connected to on the rings inside it, or at a golden-angle step when it is connected to none, so two unconnected
//!    elements never start on top of each other.
//! 3. **Forces**, for a fixed number of steps that cool as they go: a spring along every connection, a push between
//!    every pair, and a pull of each element back to its ring.
//! 4. **No overlap.** Whatever the forces left touching is pushed apart along the axis that needs the least movement,
//!    until nothing overlaps.
//!
//! A pinned element is where the model has it and stays there through all four; the others arrange themselves around
//! it. Given the placements a previous pass ended on, an element that was placed then starts from there and the
//! simulation runs cooler, so one change to the model moves the rest a little, not everything from nothing.

use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;

use crate::algorithm::{
    LayoutAlgorithm, LayoutDefinition, LayoutInput, LayoutMode, LayoutPlacement, Placements, Point,
};

/// The distance from one ring to the next, where no ring needs more.
const RING_GAP: f64 = 260.0;
/// The clear space kept between two elements, on a ring and by the overlap pass.
const CLEARANCE: f64 = 28.0;
const STEPS: usize = 240;
/// Steps taken from a previous pass's placements, and how hot they start.
const WARM_STEPS: usize = 60;
const WARM_HEAT: f64 = 0.08;
const SPRING: f64 = 0.06;
const REPULSION: f64 = 26000.0;
const RING_PULL: f64 = 0.09;
const MAX_STEP: f64 = 60.0;
const OVERLAP_PASSES: usize = 400;

/// The `tiered-force` mode.
#[derive(Debug, Clone, Copy, Default)]
pub struct TieredForceLayout;

impl LayoutAlgorithm for TieredForceLayout {
    fn mode(&self) -> LayoutMode {
        LayoutMode::TieredForce
    }

    fn place(
        &self,
        input: &LayoutInput,
        definition: &LayoutDefinition,
        previous: Option<&Placements>,
    ) -> Option<Placements> {
        Some(place(input, &definition.tiers, previous))
    }

    /// Ids, sizes, types, what is pinned and where, and the connections. An unpinned element's own position is not
    /// among them: this layout decides it, so the model moving it changes nothing.
    fn signature(&self, input: &LayoutInput) -> Option<String> {
        let elements: Vec<String> = input
            .elements
            .iter()
            .map(|element| {
                let pinned = if element.pinned {
                    format!("p{},{}", element.x, element.y)
                } else {
                    String::new()
                };
                [
                    element.id.clone(),
                    element.element_type.clone().unwrap_or_default(),
                    element.width.to_string(),
                    element.height.to_string(),
                    pinned,
                ]
                .join("|")
            })
            .collect();
        let connections: Vec<String> = input
            .connections
            .iter()
            .map(|connection| format!("{}>{}", connection.source_id, connection.target_id))
            .collect();
        Some(format!("{}#{}", elements.join(";"), connections.join(";")))
    }

    /// Where a point in the placed space stands in the model's: the same point, since nothing is transformed.
    fn inverse(&self, point: Point, _input: &LayoutInput, _definition: &LayoutDefinition) -> Option<Point> {
        Some(point)
    }
}

struct Node {
    id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    tier: usize,
    fixed: bool,
}

/// The tier an element's type is on; a type no tier names goes on the ring beyond the last.
fn tier_of(element_type: Option<&str>, tiers: &[Vec<String>]) -> usize {
    element_type
        .and_then(|kind| tiers.iter().position(|tier| tier.iter().any(|member| member == kind)))
        .unwrap_or(tiers.len())
}

fn ring_radii(nodes: &[Node], tier_count: usize) -> Vec<f64> {
    let mut radii: Vec<f64> = Vec::with_capacity(tier_count);
    for tier in 0..tier_count {
        let members: Vec<&Node> = nodes.iter().filter(|node| node.tier == tier).collect();
        // The circle that fits the tier's elements side by side with their clearance between them.
        let needed = members
            .iter()
            .fold(0.0, |sum, node| sum + node.width.max(node.height) + CLEARANCE)
            / (2.0 * PI);
        if tier == 0 {
            radii.push(if members.len() <= 1 { 0.0 } else { needed });
        } else {
            radii.push(needed.max(radii[tier - 1] + RING_GAP));
        }
    }
    radii
}

/// JavaScript's `Math.round`: halves go up, towards positive infinity.
fn js_round(value: f64) -> f64 {
    let floor = value.floor();
    if value - floor >= 0.5 { floor + 1.0 } else { floor }
}

fn place(input: &LayoutInput, tiers: &[Vec<String>], previous: Option<&Placements>) -> Placements {
    let golden_angle = PI * (3.0 - 5.0f64.sqrt());
    let mut nodes: Vec<Node> = input
        .elements
        .iter()
        .map(|element| Node {
            id: element.id.clone(),
            x: element.x,
            y: element.y,
            width: element.width,
            height: element.height,
            tier: tier_of(element.element_type.as_deref(), tiers),
            fixed: element.pinned,
        })
        .collect();
    let by_id: HashMap<&str, usize> = input
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| (element.id.as_str(), index))
        .collect();
    let links: Vec<(usize, usize)> = input
        .connections
        .iter()
        .filter_map(|connection| {
            let a = *by_id.get(connection.source_id.as_str())?;
            let b = *by_id.get(connection.target_id.as_str())?;
            (a != b).then_some((a, b))
        })
        .collect();
    let radii = ring_radii(&nodes, tiers.len() + 1);

    // A start: from a previous pass where there is one, else on the ring.
    let mut warm = false;
    let mut started: HashSet<String> = HashSet::new();
    for node in &mut nodes {
        let was = previous.and_then(|previous| previous.get(&node.id));
        if node.fixed {
            started.insert(node.id.clone());
        } else if let Some(was) = was {
            node.x = was.x;
            node.y = was.y;
            started.insert(node.id.clone());
            warm = true;
        }
    }
    let mut unconnected = 0usize;
    for (tier, &radius) in radii.iter().enumerate() {
        for index in 0..nodes.len() {
            if nodes[index].tier != tier || started.contains(&nodes[index].id) {
                continue;
            }
            // The mean direction of the neighbours that already have a place.
            let mut sum_x = 0.0;
            let mut sum_y = 0.0;
            for &(a, b) in &links {
                let other = if a == index {
                    Some(b)
                } else if b == index {
                    Some(a)
                } else {
                    None
                };
                if let Some(other) = other {
                    let other = &nodes[other];
                    if started.contains(&other.id) && (other.x != 0.0 || other.y != 0.0) {
                        let length = other.x.hypot(other.y);
                        sum_x += other.x / length;
                        sum_y += other.y / length;
                    }
                }
            }
            let angle = if sum_x != 0.0 || sum_y != 0.0 {
                sum_y.atan2(sum_x) + unconnected as f64 * 1e-3
            } else {
                unconnected as f64 * golden_angle
            };
            unconnected += 1;
            nodes[index].x = angle.cos() * radius;
            nodes[index].y = angle.sin() * radius;
            started.insert(nodes[index].id.clone());
        }
    }

    let (steps, heat) = if warm { (WARM_STEPS, WARM_HEAT) } else { (STEPS, 1.0) };
    let mut move_x = vec![0.0f64; nodes.len()];
    let mut move_y = vec![0.0f64; nodes.len()];
    for step in 0..steps {
        let alpha = heat * (1.0 - step as f64 / steps as f64);
        move_x.fill(0.0);
        move_y.fill(0.0);

        for i in 0..nodes.len() {
            for j in i + 1..nodes.len() {
                let mut dx = nodes[j].x - nodes[i].x;
                let mut dy = nodes[j].y - nodes[i].y;
                if dx == 0.0 && dy == 0.0 {
                    // Two elements on one point have no direction to part in; their order gives one.
                    dx = 1e-3 * (j - i) as f64;
                    dy = 1e-3;
                }
                let distance = dx.hypot(dy);
                let push = REPULSION / (distance * distance);
                move_x[i] -= (dx / distance) * push;
                move_y[i] -= (dy / distance) * push;
                move_x[j] += (dx / distance) * push;
                move_y[j] += (dy / distance) * push;
            }
        }

        for &(i, j) in &links {
            let dx = nodes[j].x - nodes[i].x;
            let dy = nodes[j].y - nodes[i].y;
            let hypot = dx.hypot(dy);
            // JavaScript's `||`: a zero, or not a number, falls back.
            let distance = if hypot == 0.0 || hypot.is_nan() { 1e-3 } else { hypot };
            // A connection wants its ends one ring gap apart per ring between them, and no closer than that.
            let rest = RING_GAP * 1.0f64.max(nodes[i].tier.abs_diff(nodes[j].tier) as f64);
            let pull = SPRING * (distance - rest);
            move_x[i] += (dx / distance) * pull;
            move_y[i] += (dy / distance) * pull;
            move_x[j] -= (dx / distance) * pull;
            move_y[j] -= (dy / distance) * pull;
        }

        for (i, node) in nodes.iter_mut().enumerate() {
            if node.fixed {
                continue;
            }
            let distance = node.x.hypot(node.y);
            let radius = radii[node.tier];
            if distance > 1e-6 {
                let to_ring = RING_PULL * (radius - distance);
                move_x[i] += (node.x / distance) * to_ring;
                move_y[i] += (node.y / distance) * to_ring;
            } else if radius > 0.0 {
                move_x[i] += RING_PULL * radius;
            }
            let length = move_x[i].hypot(move_y[i]);
            let scale = if length > MAX_STEP { MAX_STEP / length } else { 1.0 };
            node.x += move_x[i] * scale * alpha;
            node.y += move_y[i] * scale * alpha;
        }
    }

    separate(&mut nodes);

    let mut placements = Placements::new();
    for node in &nodes {
        if !node.fixed {
            // Rounded, so two machines that differ in the last bits of a cosine still draw one picture.
            let placement = LayoutPlacement::at(js_round(node.x * 100.0) / 100.0, js_round(node.y * 100.0) / 100.0);
            placements.insert(node.id.clone(), placement);
        }
    }
    placements
}

/// Pushes overlapping rectangles apart until none overlaps; a pinned one never moves.
fn separate(nodes: &mut [Node]) {
    for _ in 0..OVERLAP_PASSES {
        let mut moved = false;
        for i in 0..nodes.len() {
            for j in i + 1..nodes.len() {
                let (head, tail) = nodes.split_at_mut(j);
                let a = &mut head[i];
                let b = &mut tail[0];
                if a.fixed && b.fixed {
                    continue;
                }
                let overlap_x = (a.width + b.width) / 2.0 + CLEARANCE - (b.x - a.x).abs();
                let overlap_y = (a.height + b.height) / 2.0 + CLEARANCE - (b.y - a.y).abs();
                if overlap_x <= 0.0 || overlap_y <= 0.0 {
                    continue;
                }
                moved = true;
                // Along the axis that needs less; on a tie, and for two on one point, by their order (`j` is always
                // after `i`).
                let along_x = overlap_x <= overlap_y;
                let sign = if along_x {
                    if b.x >= a.x { 1.0 } else { -1.0 }
                } else if b.y >= a.y {
                    1.0
                } else {
                    -1.0
                };
                let amount = (if along_x { overlap_x } else { overlap_y }) + 0.01;
                let share_a = if a.fixed {
                    0.0
                } else if b.fixed {
                    1.0
                } else {
                    0.5
                };
                let share_b = if b.fixed {
                    0.0
                } else if a.fixed {
                    1.0
                } else {
                    0.5
                };
                if along_x {
                    a.x -= sign * amount * share_a;
                    b.x += sign * amount * share_b;
                } else {
                    a.y -= sign * amount * share_a;
                    b.y += sign * amount * share_b;
                }
            }
        }
        if !moved {
            return;
        }
    }
}
