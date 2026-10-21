//! The rules, each reported on the line it breaks, and none reported for a clean document. Ported from the standalone
//! host's `EtAlii.Adp.Diagram.Sankey.Tests/SankeyValidator.Tests.cs` at `8b6ccfc`.

mod common;

use std::fs;

use adp_diagram::{DiagramProblemLocation, DiagramProblemSeverity};
use adp_diagram_sankey::rules::{self, *};
use common::{fixture, load, open};

#[test]
fn every_rule_is_reported_on_the_broken_document() {
    let breaches = rules::validate(&load(&fixture("rules-broken.skv")));
    let mut ids: Vec<&str> = breaches.iter().map(|b| b.rule_id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(
        ids,
        [
            BACKWARD_FLOW,
            DANGLING_FLOW,
            DUPLICATE_ID,
            MISSING_ID,
            MISSING_VALUE,
            NEGATIVE_VALUE,
            SELF_FLOW,
            UNKNOWN_COLOR
        ]
    );
    assert!(
        breaches
            .iter()
            .any(|b| b.rule_id == DUPLICATE_ID && b.message.contains("written twice"))
    );
}

#[test]
fn an_unreadable_entry_is_a_warning_not_an_error() {
    let breaches = rules::validate(&load(&fixture("malformed-entries.skv")));
    assert!(!breaches.is_empty());
    assert!(
        breaches
            .iter()
            .filter(|b| b.rule_id == UNREADABLE_ENTRY)
            .all(|b| b.is_warning)
    );
}

#[test]
fn a_breach_reaches_the_panel_on_its_one_based_line() {
    let text = fs::read_to_string(fixture("rules-broken.skv")).unwrap();
    let self_flow_line = text.split('\n').position(|l| l.contains("from: b")).unwrap() + 1;
    let diagram = open(&fixture("rules-broken.skv"), "rules-broken");
    let problems = diagram.session.problems(diagram.workspace.documents());
    let self_flows: Vec<_> = problems.iter().filter(|p| p.rule_id == SELF_FLOW).collect();
    assert_eq!(self_flows.len(), 1);
    assert_eq!(self_flows[0].severity, DiagramProblemSeverity::Error);
    assert_eq!(
        self_flows[0].location,
        Some(DiagramProblemLocation::Line(self_flow_line as u32))
    );
    assert_eq!(
        problems.iter().find(|p| p.rule_id == BACKWARD_FLOW).unwrap().severity,
        DiagramProblemSeverity::Warning
    );
}

#[test]
fn a_clean_document_reports_nothing() {
    let mut clean = vec![fixture("lf-line-endings.skv")];
    clean.extend(common::every_example());
    for path in clean {
        let breaches = rules::validate(&load(&path));
        assert!(breaches.is_empty(), "{}: {breaches:?}", path.display());
    }
}
