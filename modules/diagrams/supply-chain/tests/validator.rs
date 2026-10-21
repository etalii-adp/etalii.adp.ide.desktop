//! The rules, each reported on the line it breaks, and none for a clean document (the standalone host's
//! `SupplyChainValidatorTests`), and what the canvas is sent for a document (`SupplyChainElementMapperTests`).

mod common;

use adp_diagram::{DiagramProblemLocation, DiagramProblemSeverity, DiagramViewport};
use adp_diagram_supply_chain::mapper;
use adp_diagram_supply_chain::trace::SupplyChainTrace;
use adp_diagram_supply_chain::validator::{self, *};
use common::{AUTOMOTIVE, GPU_MEMORY, fixture, open_fixture, read, read_text};

#[test]
fn every_rule_is_reported_on_the_broken_document() {
    let breaches = validator::validate(&read("rules-broken.supply"));
    let mut rules: Vec<&str> = breaches.iter().map(|b| b.rule_id).collect();
    rules.sort_unstable();
    rules.dedup();
    assert_eq!(
        vec![
            DANGLING_FLOW,
            DUPLICATE_ID,
            EMPTY_GROUP,
            MISSING_ID,
            SELF_FLOW,
            UNKNOWN_GROUP
        ],
        rules
    );
}

#[test]
fn a_breach_reaches_the_panel_on_its_one_based_line() {
    let text = fixture("rules-broken.supply");
    let self_flow_line = text.split('\n').position(|l| l.contains("id: loop")).unwrap() + 1;
    let diagram = open_fixture("rules-broken.supply");

    let problems = diagram.session.problems(diagram.workspace.documents());

    let self_flow: Vec<_> = problems.iter().filter(|p| p.rule_id == SELF_FLOW).collect();
    assert_eq!(1, self_flow.len());
    assert_eq!(DiagramProblemSeverity::Error, self_flow[0].severity);
    assert_eq!(
        Some(DiagramProblemLocation::Line(self_flow_line as u32)),
        self_flow[0].location
    );
}

#[test]
fn a_clean_document_reports_nothing() {
    for name in ["lf-line-endings.supply", AUTOMOTIVE, GPU_MEMORY] {
        assert_eq!(
            Vec::<SupplyChainBreach>::new(),
            validator::validate(&read(name)),
            "{name}"
        );
    }
}

#[test]
fn a_cards_share_is_of_the_whole_document_not_of_what_the_view_holds() {
    // A small mine in view, and one four times its size far outside it.
    let model = read_text(
        "supply-chain: 1\nnodes:\n  - id: small\n    type: raw-material\n    quantity: 25\n    unit: t\n    x: 0\n    y: 0\n  - id: large\n    type: raw-material\n    quantity: 100\n    unit: t\n    x: 5000\n    y: 5000\n",
    );
    let delivered = mapper::visible(
        &model,
        DiagramViewport::new(-10.0, -10.0, 300.0, 200.0),
        &SupplyChainTrace::none(),
    );
    assert_eq!(1, delivered.len());
    assert_eq!("0.25", delivered[0].attribute("share"));
}
