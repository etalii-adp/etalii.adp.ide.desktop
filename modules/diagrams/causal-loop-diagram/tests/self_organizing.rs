//! Arrange diagram's layout: the standalone host's `SelfOrganizingLayoutTests` at `8b6ccfc`, with the same expectations,
//! over models read through the binding.
//!
//! Not ported: the two-process determinism test, which started the test host again as a child process; the layout
//! reads no clock, no random source and no hash order, and `the_layout_is_a_function_of_the_document_and_not_of_how_it_was_built`
//! holds it to bit-identical positions, as the parity golden `systems/causal-loop-diagram/on-call` holds it to the
//! standalone host's own.

mod common;

use std::collections::HashMap;

use adp_diagram_causal_loop::metrics::Metrics;
use adp_diagram_causal_loop::reading::Model;
use adp_diagram_causal_loop::self_organizing::{self, DEFAULT_BUDGET, SEPARATION_ROUNDS};
use adp_layout::Rect;
use common::model_of;

fn fixture() -> Model {
    model_of(
        &[
            "causal-loop 1",
            "variable population \"Population\"",
            "variable births \"Births\"",
            "variable crowding \"Crowding\"",
            "variable deaths \"Deaths\"",
            "variable morale \"Morale\"",
            "variable rumour \"Rumour\"",
            "link population -> births +",
            "link births -> population +",
            "link population -> crowding +",
            "link crowding -> deaths +",
            "link deaths -> population -",
            "link morale -> rumour +",
            "link rumour -> morale +",
        ]
        .map(str::to_owned),
    )
}

fn ring(count: usize) -> Model {
    let mut statements = vec!["causal-loop 1".to_owned()];
    statements.extend((0..count).map(|i| format!("variable v{i} \"Variable {i}\"")));
    statements.extend((0..count).map(|i| format!("link v{i} -> v{} +", (i + 1) % count)));
    model_of(&statements)
}

/// Every variable linked to every other: the shape that piles up.
fn dense(count: usize) -> Model {
    let mut statements = vec!["causal-loop 1".to_owned()];
    statements.extend((0..count).map(|i| format!("variable v{i} \"Variable {i}\"")));
    for first in 0..count {
        for second in first + 1..count {
            statements.push(format!("link v{first} -> v{second} +"));
        }
    }
    model_of(&statements)
}

fn compute(model: &Model) -> self_organizing::Arrangement {
    self_organizing::compute(model, &Metrics::default(), DEFAULT_BUDGET, SEPARATION_ROUNDS)
}

fn serialize(boxes: &HashMap<String, Rect>) -> String {
    let mut entries: Vec<_> = boxes.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    entries
        .into_iter()
        .map(|(id, b)| format!("{id} {:?} {:?} {:?} {:?}\n", b.x, b.y, b.width, b.height))
        .collect()
}

fn distance(first: &Rect, second: &Rect) -> f64 {
    let dx = first.center_x() - second.center_x();
    let dy = first.center_y() - second.center_y();
    (dx * dx + dy * dy).sqrt()
}

fn assert_apart(boxes: &[Rect], what: &str) {
    for first in 0..boxes.len() {
        for second in first + 1..boxes.len() {
            assert!(
                !boxes[first].overlaps(&boxes[second]),
                "{what}: {:?} and {:?}",
                boxes[first],
                boxes[second]
            );
        }
    }
}

// ---- a pure function of the document ---------------------------------------------------------------------------------

#[test]
fn the_layout_is_a_function_of_the_document_and_not_of_how_it_was_built() {
    let first = compute(&fixture()).unwrap();
    let second = compute(&fixture()).unwrap();
    assert_eq!(serialize(&first), serialize(&second));
}

#[test]
fn the_stimulus_sequence_is_the_radical_inverse_of_its_index() {
    for (index, radix, expected) in [
        (1, 2, 0.5),
        (2, 2, 0.25),
        (3, 2, 0.75),
        (4, 2, 0.125),
        (1, 3, 1.0 / 3.0),
        (2, 3, 2.0 / 3.0),
        (3, 3, 1.0 / 9.0),
    ] {
        assert!((self_organizing::halton(index, radix) - expected).abs() < 1e-12);
    }
}

#[test]
fn the_stimulus_sequence_covers_the_interval_evenly() {
    let mut buckets = [0; 10];
    for index in 1..=1000 {
        buckets[(self_organizing::halton(index, 2) * 10.0) as usize] += 1;
    }
    assert!(buckets.iter().all(|count| (95..=105).contains(count)), "{buckets:?}");
}

#[test]
fn the_initial_positions_are_a_spread_and_not_a_pile() {
    let widths = vec![120.0; 40];
    let positions = self_organizing::initial_positions(40, &widths, 30.0, 40.0);
    assert_eq!(positions[0], (0.0, 0.0));
    let mut distinct: Vec<(i64, i64)> = positions
        .iter()
        .map(|(x, y)| ((x * 1e6).round() as i64, (y * 1e6).round() as i64))
        .collect();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(distinct.len(), positions.len());
}

// ---- graph distance, which is what makes it a graph layout ------------------------------------------------------------

#[test]
fn graph_distance_is_counted_in_hops_over_undirected_links() {
    let model = fixture();
    let hops = self_organizing::graph_distances(&model, &model.variables);
    let index = |name: &str| model.variables.iter().position(|v| v.name == name).unwrap();
    assert_eq!(hops[index("population")][index("population")], 0);
    assert_eq!(hops[index("population")][index("births")], 1);
    assert_eq!(hops[index("crowding")][index("births")], 2);
    assert_eq!(hops[index("deaths")][index("population")], 1);
    assert_eq!(hops[index("population")][index("deaths")], 1);
    assert_eq!(hops[index("population")][index("morale")], -1);
}

#[test]
fn a_disconnected_component_is_not_dragged_toward_the_other_one() {
    let boxes = compute(&fixture()).unwrap();
    let within_pair = distance(&boxes["morale"], &boxes["rumour"]);
    let across = distance(&boxes["morale"], &boxes["population"]).min(distance(&boxes["rumour"], &boxes["population"]));
    assert!(
        within_pair < across,
        "The unconnected pair ({within_pair:.0} apart) should stay closer to each other than to the other component ({across:.0})."
    );
}

#[test]
fn linked_variables_end_up_nearer_than_unlinked_ones() {
    let boxes = compute(&ring(12)).unwrap();
    let neighbour = distance(&boxes["v0"], &boxes["v1"]);
    let opposite = distance(&boxes["v0"], &boxes["v6"]);
    assert!(neighbour < opposite, "{neighbour:.0} against {opposite:.0}");
}

// ---- the separation pass, and what it refuses -------------------------------------------------------------------------

#[test]
fn at_every_size_no_two_boxes_intersect() {
    for count in [4, 12, 40, 120] {
        let boxes: Vec<Rect> = compute(&ring(count)).unwrap().into_values().collect();
        assert_eq!(boxes.len(), count);
        assert_apart(&boxes, &format!("two of {count} variables overlap"));
    }
}

#[test]
fn the_extent_ratio_is_not_what_guards_readability() {
    let boxes: Vec<Rect> = compute(&dense(24)).unwrap().into_values().collect();
    let width =
        boxes.iter().map(Rect::right).fold(f64::MIN, f64::max) - boxes.iter().map(|b| b.x).fold(f64::MAX, f64::min);
    let height =
        boxes.iter().map(Rect::bottom).fold(f64::MIN, f64::max) - boxes.iter().map(|b| b.y).fold(f64::MAX, f64::min);
    let ratio = width.max(height) / width.min(height);
    assert!((1.0..=3.0).contains(&ratio), "{ratio}");
    assert!(!boxes.is_empty());
    assert_apart(&boxes, "a near-square arrangement still overlaps");
}

#[test]
fn when_the_boxes_cannot_be_separated_the_layout_refuses_naming_the_size() {
    let mut positions = vec![(0.0, 0.0); 40];
    let widths = vec![120.0; 40];
    let remaining = self_organizing::separate(&mut positions, &widths, 30.0, 40.0, SEPARATION_ROUNDS);
    let refusal = self_organizing::could_not_separate(40, remaining);
    assert!(refusal.contains("40 variables"), "{refusal}");
}

#[test]
fn the_clearance_is_measured_from_the_boxes_and_not_from_the_separation_alone() {
    let metrics = Metrics {
        font_size: 100.0,
        separation: 10.0,
        ..Metrics::default()
    };
    assert!(metrics.height() > metrics.separation);
    let boxes: Vec<Rect> = self_organizing::compute(&ring(16), &metrics, DEFAULT_BUDGET, SEPARATION_ROUNDS)
        .unwrap()
        .into_values()
        .collect();
    assert_eq!(boxes.len(), 16);
    assert_apart(&boxes, "tall boxes overlap");
}

#[test]
fn the_refusal_is_reachable_through_the_layout_itself() {
    let model = dense(30);
    let refusal = self_organizing::compute(&model, &Metrics::default(), DEFAULT_BUDGET, 1).unwrap_err();
    assert!(refusal.contains("30 variables"), "{refusal}");
    assert!(compute(&model).is_ok());
}

#[test]
fn the_separation_pass_terminates_even_when_it_cannot_succeed() {
    let mut positions = vec![(0.0, 0.0); 200];
    let widths = vec![4000.0; 200];
    // It came back at all, which is the claim, and honest about what is left.
    let remaining = self_organizing::separate(&mut positions, &widths, 30.0, 40.0, SEPARATION_ROUNDS);
    assert!(remaining <= 200 * 199 / 2);
}

#[test]
fn the_separation_pass_moves_boxes_the_same_way_every_time() {
    let widths = vec![120.0; 30];
    let start: Vec<(f64, f64)> = (0..30).map(|i| ((i % 3) as f64, (i % 5) as f64)).collect();
    let (mut first, mut second) = (start.clone(), start);
    self_organizing::separate(&mut first, &widths, 30.0, 40.0, SEPARATION_ROUNDS);
    self_organizing::separate(&mut second, &widths, 30.0, 40.0, SEPARATION_ROUNDS);
    assert_eq!(first, second);
}

// ---- the budget, and the shapes that need no arranging -----------------------------------------------------------------

#[test]
fn a_document_over_the_budget_is_refused_with_its_size_rather_than_arranged_slowly() {
    let refusal = self_organizing::compute(&ring(40), &Metrics::default(), 25, SEPARATION_ROUNDS).unwrap_err();
    assert!(refusal.contains("40 variables"), "{refusal}");
    assert!(refusal.contains("25"), "{refusal}");
}

#[test]
fn an_empty_document_is_arranged_with_nothing_in_it() {
    assert!(compute(&model_of(&["causal-loop 1".to_owned()])).unwrap().is_empty());
}

#[test]
fn a_single_variable_stays_at_the_origin_rather_than_drifting_toward_the_stimuli() {
    let boxes = compute(&model_of(&[
        "causal-loop 1".to_owned(),
        "variable alone \"Alone\"".to_owned(),
    ]))
    .unwrap();
    assert_eq!(boxes.len(), 1);
    let only = &boxes["alone"];
    assert!(only.center_x().abs() < 1e-9 && only.center_y().abs() < 1e-9);
}

#[test]
fn every_declared_variable_is_placed() {
    let model = ring(20);
    let boxes = compute(&model).unwrap();
    assert!(!boxes.is_empty());
    assert!(model.variables.iter().all(|v| boxes.contains_key(&v.name)));
}
