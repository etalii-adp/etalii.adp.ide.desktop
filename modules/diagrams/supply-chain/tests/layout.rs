//! The layered layout: goods flow left to right, groups hold their members, and nothing overlaps (the standalone
//! host's `SupplyChainLayoutTests`).

mod common;

use std::collections::HashSet;

use adp_diagram_supply_chain::layout::{LAYER_PITCH, SupplyChainLayout};
use adp_diagram_supply_chain::model::{SupplyChainModel, geometry, stages};
use common::{AUTOMOTIVE, GPU_MEMORY, read, read_text};

const EXAMPLES: [&str; 2] = [AUTOMOTIVE, GPU_MEMORY];

#[test]
fn every_flow_runs_left_to_right() {
    for example in EXAMPLES {
        let layout = SupplyChainLayout::of(&read(example));
        assert!(!layout.flows.is_empty());
        for flow in &layout.flows {
            let (from, to) = (layout.node_boxes[&flow.from], layout.node_boxes[&flow.to]);
            assert!(
                from.right() < to.x,
                "{example}: {} runs from {} to {}",
                flow.id,
                from.x,
                to.x
            );
        }
    }
}

#[test]
fn each_node_stands_no_earlier_than_its_stage_and_a_source_exactly_at_it() {
    for example in EXAMPLES {
        let layout = SupplyChainLayout::of(&read(example));
        let supplied: HashSet<&str> = layout.flows.iter().map(|f| f.to.as_str()).collect();
        for node in &layout.nodes {
            let column = layout.node_boxes[&node.id].x / LAYER_PITCH;
            let stage = f64::from(stages::rank_of(&node.node_type));
            assert!(
                column >= stage,
                "{} ({}) stands in column {column}",
                node.id,
                node.node_type
            );
            if !supplied.contains(node.id.as_str()) {
                assert_eq!(stage, column, "{}", node.id);
            }
        }
    }
}

/// Points along horizontal S curves from each stop to the next, as the canvas's flow path bends them.
fn sampled(stops: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut points = Vec::new();
    for pair in stops.windows(2) {
        let (start, end) = (pair[0], pair[1]);
        let direction = if end.0 >= start.0 { 1.0 } else { -1.0 };
        let pull = f64::max(48.0, (end.0 - start.0).abs() / 2.0);
        let (c1, c2) = ((start.0 + direction * pull, start.1), (end.0 - direction * pull, end.1));
        let mut t = 0.0;
        while t <= 1.0 {
            let u = 1.0 - t;
            points.push((
                u * u * u * start.0 + 3.0 * u * u * t * c1.0 + 3.0 * u * t * t * c2.0 + t * t * t * end.0,
                u * u * u * start.1 + 3.0 * u * u * t * c1.1 + 3.0 * u * t * t * c2.1 + t * t * t * end.1,
            ));
            t += 0.02;
        }
    }
    points
}

#[test]
fn no_flow_crosses_a_card() {
    for example in EXAMPLES {
        let layout = SupplyChainLayout::of(&read(example));
        for flow in &layout.flows {
            let (from, to) = (layout.node_boxes[&flow.from], layout.node_boxes[&flow.to]);
            let mut stops = vec![(from.right(), from.center_y())];
            for (x, y) in layout.lanes.get(&flow.id).cloned().unwrap_or_default() {
                stops.push((x - geometry::NODE_WIDTH / 2.0, y));
                stops.push((x + geometry::NODE_WIDTH / 2.0, y));
            }
            stops.push((to.x, to.center_y()));
            for (id, card) in layout
                .node_boxes
                .iter()
                .filter(|(id, _)| **id != flow.from && **id != flow.to)
            {
                for (x, y) in sampled(&stops) {
                    assert!(
                        !(x > card.x && x < card.right() && y > card.y && y < card.bottom()),
                        "{example}: {} crosses {id} at ({x:.0}, {y:.0})",
                        flow.id
                    );
                }
            }
        }
    }
}

#[test]
fn no_two_arranged_nodes_overlap() {
    for example in EXAMPLES {
        let boxes: Vec<_> = SupplyChainLayout::of(&read(example))
            .node_boxes
            .values()
            .copied()
            .collect();
        for (i, a) in boxes.iter().enumerate() {
            for b in &boxes[i + 1..] {
                assert!(!a.overlaps(b), "{a:?} overlaps {b:?}");
            }
        }
    }
}

#[test]
fn every_group_frame_holds_its_members() {
    for example in EXAMPLES {
        let layout = SupplyChainLayout::of(&read(example));
        assert!(!layout.groups.is_empty());
        for node in layout.nodes.iter().filter(|n| !n.group.is_empty()) {
            let (frame, card) = (layout.group_boxes[&node.group], layout.node_boxes[&node.id]);
            assert!(
                frame.x < card.x && frame.y < card.y && frame.right() > card.right() && frame.bottom() > card.bottom(),
                "{} is outside {}",
                node.id,
                node.group
            );
        }
    }
}

#[test]
fn a_cycle_is_laid_out_rather_than_looping_forever() {
    let model = read_text(
        "supply-chain: 1\nnodes:\n  - id: a\n    type: supplier\n  - id: b\n    type: manufacturer\n  - id: c\n    type: consumer\nflows:\n  - id: ab\n    from: a\n    to: b\n  - id: bc\n    from: b\n    to: c\n  - id: ca\n    from: c\n    to: a\n",
    );
    let layout = SupplyChainLayout::of(&model);
    assert_eq!(3, layout.node_boxes.len());
    assert!(layout.node_boxes["a"].x < layout.node_boxes["b"].x);
    assert!(layout.node_boxes["b"].x < layout.node_boxes["c"].x);
}

#[test]
fn an_authored_position_wins_over_the_arranged_one() {
    let layout = SupplyChainLayout::of(&read("lf-line-endings.supply"));
    assert_eq!(
        (800.0, 40.0),
        (layout.node_boxes["shop"].x, layout.node_boxes["shop"].y)
    );
    assert_ne!((800.0, 40.0), layout.arranged["shop"]);
}

#[test]
fn an_empty_document_lays_out_nothing() {
    let layout = SupplyChainLayout::of(&SupplyChainModel::default());
    assert!(layout.node_boxes.is_empty());
    assert!(layout.group_boxes.is_empty());
}
