//! The shipped examples: each draws every node and flow it declares, and the figures their readmes state are the
//! figures the diagram shows. Ported from the standalone host's `EtAlii.Adp.Diagram.Sankey.Tests/SankeyExamples.Tests.cs`
//! at `8b6ccfc`, over the examples of `src/diagrams/sankey/examples/` copied into `tests/fixtures/examples/`.

mod common;

use adp_diagram_sankey::layout::SankeyLayout;
use adp_diagram_sankey::mapper;
use adp_diagram_sankey::model::geometry;
use common::{brightwater_coffee, every_example, load, recent_graduates, uk_energy};

#[test]
fn an_example_draws_everything_it_declares_reading_left_to_right() {
    for path in every_example() {
        let model = load(&path);
        let layout = SankeyLayout::of(&model);
        let elements = mapper::all(&model);
        assert!(model.problems.is_empty(), "{}", path.display());
        assert_eq!(model.nodes.len(), layout.nodes.len());
        assert_eq!(model.flows.len(), layout.flows.len());
        assert_eq!(model.nodes.len() + model.flows.len(), elements.len());
        assert!(layout.bands.values().all(|band| !band.backward));
    }
}

#[test]
fn an_example_keeps_its_nodes_apart_in_every_column() {
    for path in every_example() {
        let layout = SankeyLayout::of(&load(&path));
        for column in &layout.column_order {
            for i in 1..column.len() {
                assert!(
                    layout.boxes[&column[i]].y >= layout.boxes[&column[i - 1]].bottom() + geometry::NODE_GAP - 0.05,
                    "{} overlaps {}",
                    column[i],
                    column[i - 1]
                );
            }
        }
    }
}

#[test]
fn the_company_balances_from_segments_to_net_profit() {
    let layout = SankeyLayout::of(&load(&brightwater_coffee()));
    assert_eq!(layout.values["revenue"], 3500.0);
    assert_eq!(layout.values["net-profit"], 420.0);
    assert_eq!(layout.column_order.len(), 5);
}

#[test]
fn the_energy_example_joins_the_split_nuclear_links_into_the_original_figure() {
    let model = load(&uk_energy());
    assert_eq!(model.nodes.len(), 48);
    assert_eq!(model.flows.len(), 68);
    assert_eq!(
        model.flow_of("nuclear->thermal-generation").unwrap().value,
        Some(839.978)
    );
}

#[test]
fn the_graduates_example_counts_everyone_once() {
    let model = load(&recent_graduates());
    let layout = SankeyLayout::of(&model);
    // What reaches the employed is what leaves them, by the kind of job: the unclassified remainder closes the gap the
    // survey leaves.
    let inflow: f64 = model
        .flows
        .iter()
        .filter(|f| f.to == "employed")
        .map(|f| f.value.unwrap())
        .sum();
    let outflow: f64 = model
        .flows
        .iter()
        .filter(|f| f.from == "employed")
        .map(|f| f.value.unwrap())
        .sum();
    assert!((inflow - outflow).abs() < 0.05, "{inflow} {outflow}");
    assert_eq!(layout.column_order.len(), 3);
    assert_eq!(layout.column_order[0].len(), 16);
}
