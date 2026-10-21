//! The layout: columns from the flows, bars and bands on one scale, the document's order within a column, and bands
//! that leave and reach a bar without crossing. Ported from the standalone host's
//! `EtAlii.Adp.Diagram.Sankey.Tests/SankeyLayout.Tests.cs` at `8b6ccfc`. Not ported:
//! `TheLayout_IsComputedOncePerModel`, since a layout here is computed when it is asked for and cached nowhere.

mod common;

use adp_diagram_sankey::layout::{self, SankeyLayout};
use adp_diagram_sankey::model::{SankeyModel, geometry};
use common::{fixture, load, parse};

fn small() -> SankeyModel {
    load(&fixture("lf-line-endings.skv"))
}

fn close(expected: f64, actual: f64, digits: i32) {
    let tolerance = 0.5 * 10f64.powi(-digits);
    assert!((expected - actual).abs() <= tolerance, "{expected} != {actual}");
}

#[test]
fn columns_follow_the_flows_from_the_sources_rightwards() {
    let layout = SankeyLayout::of(&small());
    let columns: Vec<usize> = ["a", "b", "m", "x", "y"].iter().map(|id| layout.columns[*id]).collect();
    assert_eq!(columns, [0, 0, 1, 2, 2]);
    assert_eq!(layout.column_order[0], ["a", "b"]);
    assert_eq!(layout.column_order[2], ["x", "y"]);
    for (id, bar) in &layout.boxes {
        assert_eq!(bar.x, layout::left_of(layout.columns[id]));
    }
}

#[test]
fn a_nodes_value_is_the_larger_of_what_flows_in_and_out() {
    let layout = SankeyLayout::of(&parse(
        "sankey: 1\nnodes:\n  - id: a\n  - id: b\n  - id: c\nflows:\n  - from: a\n    to: b\n    value: 5\n  - from: b\n    to: c\n    value: 3\n",
    ));
    assert_eq!(
        (layout.values["a"], layout.values["b"], layout.values["c"]),
        (5.0, 5.0, 3.0)
    );
}

#[test]
fn bars_and_bands_share_one_scale_and_the_fullest_column_fills_the_base_height() {
    let layout = SankeyLayout::of(&small());
    // Every column carries 10, so 10 units fill the base height.
    close(geometry::BASE_HEIGHT / 10.0, layout.scale, 6);
    close(6.0 * layout.scale, layout.boxes["a"].height, 2);
    close(4.0 * layout.scale, layout.boxes["b"].height, 2);
    close(10.0 * layout.scale, layout.boxes["m"].height, 2);
    close(7.0 * layout.scale, layout.bands["m->x"].thickness, 2);
}

#[test]
fn the_thickness_setting_scales_every_bar_and_band() {
    let thin = SankeyLayout::of(&small());
    let mut model = small();
    model.settings.thickness = 1.5;
    let thick = SankeyLayout::of(&model);
    close(thin.boxes["m"].height * 1.5, thick.boxes["m"].height, 2);
    close(thin.bands["a->m"].thickness * 1.5, thick.bands["a->m"].thickness, 2);
}

#[test]
fn the_documents_order_is_the_order_in_each_column_and_nodes_never_closer_than_the_gap() {
    let layout = SankeyLayout::of(&small());
    for column in &layout.column_order {
        for i in 1..column.len() {
            let above = layout.boxes[&column[i - 1]];
            let below = layout.boxes[&column[i]];
            assert!(
                below.y >= above.bottom() + geometry::NODE_GAP - 0.01,
                "{} is closer than the gap to {}",
                column[i],
                column[i - 1]
            );
        }
    }
}

#[test]
fn bands_leave_and_reach_a_bar_stacked_in_the_order_of_their_other_ends() {
    let layout = SankeyLayout::of(&small());
    // x is above y, so the band to x leaves the middle above the band to y, and they fill the bar between them.
    let to_x = layout.bands["m->x"];
    let to_y = layout.bands["m->y"];
    assert!(to_x.source_at < to_y.source_at);
    close(0.35, to_x.source_at, 3);
    close(0.85, to_y.source_at, 3);
    close(0.3, layout.bands["a->m"].target_at, 3);
    close(0.8, layout.bands["b->m"].target_at, 3);
    close(0.5, to_x.target_at, 3);
    assert!(layout.bands.values().all(|band| !band.backward));
}

#[test]
fn a_band_between_two_single_nodes_runs_level() {
    // d starts centred in its column, below c, and is drawn towards b.
    let layout = SankeyLayout::of(&parse(
        "sankey: 1\nnodes:\n  - id: a\n  - id: b\n  - id: c\n  - id: d\nflows:\n  - from: a\n    to: c\n    value: 9\n  - from: b\n    to: d\n    value: 1\n",
    ));
    for (from, to) in [("a", "c"), ("b", "d")] {
        let band = layout.bands[&format!("{from}->{to}")];
        let leaves = layout.boxes[from].y + band.source_at * layout.boxes[from].height;
        let arrives = layout.boxes[to].y + band.target_at * layout.boxes[to].height;
        assert!((leaves - arrives).abs() <= 0.5, "{from}->{to}: {leaves} {arrives}");
    }
}

#[test]
fn a_stated_column_wins_and_a_cycle_is_broken_at_the_flow_that_closes_it() {
    let layout = SankeyLayout::of(&parse(
        "sankey: 1\nnodes:\n  - id: a\n  - id: b\n  - id: c\n  - id: lone\n    column: 4\nflows:\n  - from: a\n    to: b\n    value: 2\n  - from: b\n    to: c\n    value: 2\n  - from: c\n    to: a\n    value: 1\n",
    ));
    let columns: Vec<usize> = ["a", "b", "c", "lone"].iter().map(|id| layout.columns[*id]).collect();
    assert_eq!(columns, [0, 1, 2, 3]);
    assert!(layout.bands["c->a"].backward);
    assert!(!layout.bands["a->b"].backward);
}

#[test]
fn a_node_that_states_its_column_stays_there_whatever_flows_into_it() {
    // c states the first column though b, one column right of a, flows into it.
    let layout = SankeyLayout::of(&parse(
        "sankey: 1\nnodes:\n  - id: a\n  - id: b\n  - id: c\n    column: 1\n  - id: d\nflows:\n  - from: a\n    to: b\n    value: 1\n  - from: b\n    to: c\n    value: 1\n  - from: c\n    to: d\n    value: 1\n",
    ));
    let columns: Vec<usize> = ["a", "b", "c", "d"].iter().map(|id| layout.columns[*id]).collect();
    // c keeps its column, and d follows c rather than b.
    assert_eq!(columns, [0, 1, 0, 1]);
}

#[test]
fn what_cannot_be_drawn_is_left_out_and_the_rest_is_laid_out() {
    let layout = SankeyLayout::of(&parse(
        "sankey: 1\nnodes:\n  - name: no id\n  - id: a\n  - id: a\n  - id: b\nflows:\n  - from: a\n    to: ghost\n    value: 1\n  - from: a\n    to: a\n    value: 1\n  - from: a\n    to: b\n",
    ));
    let nodes: Vec<&str> = layout.nodes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(nodes, ["a", "b"]);
    let flows: Vec<String> = layout.flows.iter().map(|f| f.id()).collect();
    assert_eq!(flows, ["a->b"]);
    assert_eq!(layout.bands["a->b"].thickness, geometry::MINIMUM_THICKNESS);
    assert_eq!(layout.boxes["a"].height, geometry::MINIMUM_NODE_HEIGHT);
}

#[test]
fn an_empty_document_has_an_empty_layout() {
    let layout = SankeyLayout::of(&SankeyModel::default());
    assert!(layout.boxes.is_empty());
    assert!(layout.column_order.is_empty());
}

#[test]
fn a_drop_is_in_the_nearest_column() {
    for (x, column) in [(0.0, 0), (150.0, 0), (200.0, 1), (-500.0, 0), (970.0, 3)] {
        assert_eq!(layout::column_at(x), column, "{x}");
    }
}

/// The company example's bars where the standalone host draws them, to the last written decimal: the recorder's
/// `--elements` of `parity/scripts/etalii/sankey/brightwater-coffee.json`.
#[test]
fn the_company_example_is_laid_out_where_the_standalone_host_lays_it_out() {
    let layout = SankeyLayout::of(&load(&common::brightwater_coffee()));
    for (id, x, y) in [
        ("cafes", 12.0, 147.2),
        ("cost-of-revenue", 652.0, 489.25),
        ("general", 1292.0, 413.94),
        ("gross-profit", 652.0, 173.25),
        ("licensing", 12.0, 653.6),
        ("marketing", 1292.0, 241.14),
        ("net-profit", 1292.0, 79.53999999999999),
        ("operating-expenses", 972.0, 255.71),
        ("operating-profit", 972.0, 90.91),
        ("packaged", 12.0, 407.2),
        ("research", 1292.0, 327.54),
        ("revenue", 332.0, 317.28),
        ("tax", 1292.0, 160.33999999999997),
        ("wholesale", 12.0, 561.6),
    ] {
        let bar = layout.boxes[id];
        assert_eq!((bar.centre_x(), bar.centre_y()), (x, y), "{id}");
    }
}
