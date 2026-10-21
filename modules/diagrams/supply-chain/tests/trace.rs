//! The trace: upstream and downstream from the selection, the rest dimmed (the standalone host's
//! `SupplyChainTraceTests`), and the same trace read back from what the canvas is given.

mod common;

use adp_diagram_supply_chain::layout::SupplyChainLayout;
use adp_diagram_supply_chain::mapper;
use adp_diagram_supply_chain::trace::{DIMMED, DOWNSTREAM, RELATED, SELECTED, SupplyChainTrace, TraceGraph, UPSTREAM};
use common::{AUTOMOTIVE, read};

fn automotive() -> SupplyChainLayout {
    SupplyChainLayout::of(&read(AUTOMOTIVE))
}

#[test]
fn nothing_selected_marks_nothing() {
    let trace = SupplyChainTrace::through(&automotive(), None);
    assert!(!trace.is_active());
    assert_eq!("", trace.of("vehicle-plant"));
}

#[test]
fn a_node_lights_what_supplies_it_and_what_it_supplies_and_dims_the_rest() {
    let trace = SupplyChainTrace::through(&automotive(), Some("pack-assembly"));
    assert_eq!(SELECTED, trace.of("pack-assembly"));
    assert_eq!(UPSTREAM, trace.of("kolwezi-cobalt"));
    assert_eq!(UPSTREAM, trace.of("cell-gigafactory-to-pack-assembly"));
    assert_eq!(DOWNSTREAM, trace.of("vehicle-plant"));
    assert_eq!(DOWNSTREAM, trace.of("private-buyers"));
    // Off the chain both ways: tyres go into the same car, but not through the battery pack.
    assert_eq!(DIMMED, trace.of("thai-rubber"));
    assert_eq!(DIMMED, trace.of("tyre-plant-to-vehicle-plant"));
}

#[test]
fn a_flow_traces_up_from_its_supplier_and_down_from_its_consumer() {
    let trace = SupplyChainTrace::through(&automotive(), Some("steel-mill-to-body-shop"));
    assert_eq!(SELECTED, trace.of("steel-mill-to-body-shop"));
    assert_eq!(UPSTREAM, trace.of("steel-mill"));
    assert_eq!(UPSTREAM, trace.of("pilbara-iron"));
    assert_eq!(DOWNSTREAM, trace.of("body-shop"));
    assert_eq!(DOWNSTREAM, trace.of("company-fleets"));
    assert_eq!(DIMMED, trace.of("atacama-lithium"));
}

#[test]
fn a_group_traces_from_all_its_members_and_a_group_on_the_chain_is_related() {
    let trace = SupplyChainTrace::through(&automotive(), Some("central-africa"));
    assert_eq!(SELECTED, trace.of("central-africa"));
    assert_eq!(SELECTED, trace.of("kolwezi-cobalt"));
    assert_eq!(DOWNSTREAM, trace.of("cathode-plant"));
    assert_eq!(RELATED, trace.of("china"));
    assert_eq!(DIMMED, trace.of("australia"));
}

#[test]
fn an_id_that_names_nothing_marks_nothing() {
    assert!(!SupplyChainTrace::through(&automotive(), Some("nowhere")).is_active());
}

#[test]
fn the_trace_over_what_the_canvas_is_given_is_the_trace_over_the_layout() {
    let model = read(AUTOMOTIVE);
    let drawn = TraceGraph::of_elements(&mapper::all(&model));
    let layout = SupplyChainLayout::of(&model);
    for selected in ["pack-assembly", "steel-mill-to-body-shop", "central-africa", "europe"] {
        let (from_layout, from_drawn) = (
            SupplyChainTrace::through(&layout, Some(selected)),
            SupplyChainTrace::over(&drawn, Some(selected)),
        );
        for id in layout
            .nodes
            .iter()
            .map(|n| &n.id)
            .chain(layout.flows.iter().map(|f| &f.id))
            .chain(layout.groups.iter().map(|g| &g.id))
        {
            assert_eq!(from_layout.of(id), from_drawn.of(id), "{selected}: {id}");
        }
    }
}
