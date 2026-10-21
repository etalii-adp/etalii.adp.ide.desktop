//! The ring a causal loop diagram opens on: the standalone host's `CausalLoopLayoutTests` at `8b6ccfc`, with the same
//! expectations, over models read through the binding; and the positions the session draws from it, with a stored
//! position winning for its variable (`CausalLoopSessionTests`' overlay).

mod common;

use adp_diagram_causal_loop::metrics::Metrics;
use adp_diagram_causal_loop::reading::Model;
use adp_diagram_causal_loop::ring;
use adp_layout::Rect;
use common::{model, model_of, open, open_with, ring as ring_of};

fn compute(model: &Model) -> std::collections::HashMap<String, Rect> {
    ring::compute(model, &Metrics::default())
}

fn order(model: &Model) -> Vec<String> {
    ring::ring_order(model).into_iter().map(|v| v.name).collect()
}

const CYCLE_A_C_B_D: [&str; 9] = [
    "causal-loop 1",
    "variable a \"A\"",
    "variable b \"B\"",
    "variable c \"C\"",
    "variable d \"D\"",
    "link a -> c +",
    "link c -> b +",
    "link b -> d +",
    "link d -> a +",
];

#[test]
fn every_declared_variable_is_placed() {
    let model = model_of(&ring_of(7));
    let boxes = compute(&model);
    assert!(!boxes.is_empty());
    assert_eq!(boxes.len(), model.variables.len());
    assert!(model.variables.iter().all(|v| boxes.contains_key(&v.name)));
}

#[test]
fn a_variable_written_twice_is_placed_once() {
    let model = model(&[
        "variable a \"First\"",
        "variable b \"B\"",
        "variable a \"Second\"",
        "link a -> b +",
    ]);
    let mut names: Vec<_> = compute(&model).into_keys().collect();
    names.sort();
    assert_eq!(names, ["a", "b"]);
}

#[test]
fn a_variable_genuinely_at_the_origin_is_distinguishable_from_an_unplaced_one() {
    let boxes = compute(&model(&["variable only \"Only\""]));
    let placed = boxes.get("only").expect("placed");
    assert_eq!((placed.x, placed.y), (0.0, 0.0));
    assert!(!boxes.contains_key("absent"));
}

#[test]
fn an_empty_document_places_nothing_rather_than_placing_something_at_the_origin() {
    assert!(compute(&Model::default()).is_empty());
}

#[test]
fn no_two_variables_overlap_at_any_size() {
    for count in [2, 3, 8, 25, 60] {
        let boxes: Vec<Rect> = compute(&model_of(&ring_of(count))).into_values().collect();
        assert_eq!(boxes.len(), count);
        for left in 0..boxes.len() {
            for right in left + 1..boxes.len() {
                assert!(
                    !boxes[left].overlaps(&boxes[right]),
                    "boxes {left} and {right} overlap at {count} variables"
                );
            }
        }
    }
}

#[test]
fn a_long_label_widens_its_own_box_and_the_ring_grows_to_keep_them_apart() {
    let boxes = compute(&model(&[
        "variable a \"a\"",
        "variable b \"an extremely long variable name that will not fit in the minimum width\"",
        "variable c \"c\"",
    ]));
    assert!(boxes["b"].width > boxes["a"].width);
    for left in ["a", "b", "c"] {
        for right in ["a", "b", "c"].into_iter().filter(|r| *r != left) {
            assert!(!boxes[left].overlaps(&boxes[right]));
        }
    }
}

#[test]
fn the_extent_stays_inside_the_ratio_bound() {
    for count in [2, 9, 40] {
        let boxes: Vec<Rect> = compute(&model_of(&ring_of(count))).into_values().collect();
        let width =
            boxes.iter().map(Rect::right).fold(f64::MIN, f64::max) - boxes.iter().map(|b| b.x).fold(f64::MAX, f64::min);
        let height = boxes.iter().map(Rect::bottom).fold(f64::MIN, f64::max)
            - boxes.iter().map(|b| b.y).fold(f64::MAX, f64::min);
        assert!(
            width.max(height) <= 4.0 * width.min(height),
            "{width} by {height} at {count} variables"
        );
    }
}

#[test]
fn the_layout_is_deterministic() {
    let model = model_of(&ring_of(12));
    assert_eq!(compute(&model), compute(&model));
}

#[test]
fn the_ring_runs_clockwise_from_the_top_so_a_reader_traces_it_the_way_they_read() {
    let boxes = compute(&model_of(&ring_of(4)));
    assert!(boxes["v0"].center_y() < boxes["v2"].center_y());
    assert!(boxes["v1"].center_x() > boxes["v3"].center_x());
    assert!(boxes["v0"].center_x().abs() < 0.001);
    assert!(boxes["v1"].center_y().abs() < 0.001);
}

#[test]
fn a_variable_with_no_label_is_sized_from_its_id() {
    let boxes = compute(&model(&["variable population", "variable b"]));
    assert!(boxes["population"].width > boxes["b"].width);
}

// ---- the ring follows the links, not the declaration order ----------------------------------------------------------

#[test]
fn the_ring_follows_the_links_rather_than_the_order_they_were_declared_in() {
    assert_eq!(order(&model(&CYCLE_A_C_B_D)), ["a", "c", "b", "d"]);
}

#[test]
fn the_computed_positions_follow_the_ring_order_and_not_the_declaration_order() {
    let boxes = compute(&model(&CYCLE_A_C_B_D));
    assert_eq!(boxes.len(), 4);
    let mut seated: Vec<(&String, f64)> = boxes
        .iter()
        .map(|(name, b)| (name, b.center_y().atan2(b.center_x())))
        .collect();
    seated.sort_by(|a, b| a.1.total_cmp(&b.1));
    let seated: Vec<&str> = seated.into_iter().map(|(n, _)| n.as_str()).collect();
    let start = seated.iter().position(|n| *n == "a").unwrap();
    let order: Vec<&str> = (0..seated.len()).map(|o| seated[(start + o) % seated.len()]).collect();
    assert_eq!(order, ["a", "c", "b", "d"]);
}

#[test]
fn a_link_seats_its_ends_together_whichever_way_it_points() {
    let model = model(&[
        "causal-loop 1",
        "variable a \"A\"",
        "variable b \"B\"",
        "variable c \"C\"",
        "link c -> a +",
        "link c -> b +",
    ]);
    assert_eq!(order(&model), ["a", "c", "b"]);
}

#[test]
fn a_branch_takes_its_earliest_declared_neighbour_first() {
    let model = model(&[
        "causal-loop 1",
        "variable a \"A\"",
        "variable b \"B\"",
        "variable c \"C\"",
        "link a -> c +",
        "link a -> b +",
    ]);
    assert_eq!(order(&model), ["a", "b", "c"]);
}

#[test]
fn most_links_join_neighbours_on_the_ring() {
    let model = model(&[
        "causal-loop 1",
        "variable incidents \"Incidents\"",
        "variable onCallLoad \"Load\"",
        "variable fatigue \"Fatigue\"",
        "variable mistakes \"Mistakes\"",
        "variable attrition \"Attrition\"",
        "variable teamSize \"Team\"",
        "link incidents -> onCallLoad +",
        "link onCallLoad -> fatigue +",
        "link fatigue -> mistakes +",
        "link mistakes -> incidents +",
        "link fatigue -> attrition +",
        "link attrition -> teamSize -",
        "link teamSize -> onCallLoad -",
    ]);
    let order = order(&model);
    let seat = |name: &str| order.iter().position(|n| n == name).unwrap();
    assert!(!model.links.is_empty());
    let adjacent = model
        .links
        .iter()
        .filter(|link| {
            let gap = seat(&link.from).abs_diff(seat(&link.to));
            gap.min(order.len() - gap) == 1
        })
        .count();
    assert!(
        adjacent * 2 >= model.links.len(),
        "Only {adjacent} of {} links join ring neighbours; the rest cut across.",
        model.links.len()
    );
}

#[test]
fn the_ring_order_is_a_function_of_the_document() {
    let text = [
        "causal-loop 1",
        "variable a \"A\"",
        "variable b \"B\"",
        "variable c \"C\"",
        "link a -> b +",
        "link b -> c +",
        "link c -> a +",
    ];
    assert_eq!(order(&model(&text)), order(&model(&text)));
}

#[test]
fn every_variable_takes_a_seat_even_with_no_links_at_all() {
    let model = model(&[
        "causal-loop 1",
        "variable a \"A\"",
        "variable b \"B\"",
        "variable c \"C\"",
    ]);
    assert_eq!(order(&model), ["a", "b", "c"]);
}

#[test]
fn disconnected_components_keep_their_own_stretch_of_the_ring() {
    let model = model(&[
        "causal-loop 1",
        "variable a \"A\"",
        "variable x \"X\"",
        "variable b \"B\"",
        "variable y \"Y\"",
        "link a -> b +",
        "link b -> a +",
        "link x -> y +",
        "link y -> x +",
    ]);
    assert_eq!(order(&model), ["a", "b", "x", "y"]);
}

// ---- what the session draws ------------------------------------------------------------------------------------------

#[test]
fn a_document_opens_on_the_ring_drawn_at_the_centres_of_its_boxes() {
    let diagram = open(&common::body(&CYCLE_A_C_B_D));
    let boxes = compute(&diagram.model());
    for name in ["a", "b", "c", "d"] {
        assert_eq!(
            diagram.position(&format!("variable:{name}")),
            (boxes[name].center_x(), boxes[name].center_y())
        );
    }
}

#[test]
fn a_stored_position_wins_for_its_variable_and_a_stale_one_overrides_nothing() {
    let diagram = open_with(
        &common::body(&CYCLE_A_C_B_D),
        "systems/causal-loop-diagram\r\nlayout:\r\n  variable:c: 10 20\r\n  variable:gone: 1 2\r\n",
    );
    let boxes = compute(&diagram.model());
    assert_eq!(diagram.position("variable:c"), (10.0, 20.0));
    assert_eq!(
        diagram.position("variable:a"),
        (boxes["a"].center_x(), boxes["a"].center_y())
    );
}
