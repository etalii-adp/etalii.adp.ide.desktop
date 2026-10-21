//! Requirement 5's table and the eight things a document can get wrong, one fixture per rule id each carrying
//! exactly that breach, `rules-clean.fdg` the control with its navigation loop; the same at the panel's seam, with
//! each breach's severity and line; and the field-service example held to what its readme claims (ported from the
//! standalone host's `FdgRules.Tests.cs`, `FdgValidator.Tests.cs` and `FdgExample.Tests.cs` at `8b6ccfc`).

mod common;

use std::collections::HashMap;
use std::fs;

use adp_diagram::{DiagramProblemLocation, DiagramProblemSeverity};
use adp_diagram_functional_decomposition_graph::lines::LineDocument;
use adp_diagram_functional_decomposition_graph::model::{connection_types, element_types};
use adp_diagram_functional_decomposition_graph::relations;
use adp_diagram_functional_decomposition_graph::rules::{self, FdgBreach, rule_ids};
use common::{example_path, fixture_path, model};

const EVERY_RULE: [(&str, &str); 8] = [
    ("rule-forbidden-link.fdg", rule_ids::FORBIDDEN_LINK),
    ("rule-self-link.fdg", rule_ids::SELF_LINK),
    ("rule-second-parent.fdg", rule_ids::SECOND_PARENT),
    ("rule-second-shows.fdg", rule_ids::SECOND_SHOWS),
    ("rule-ownership-cycle.fdg", rule_ids::OWNERSHIP_CYCLE),
    ("rule-dangling-reference.fdg", rule_ids::DANGLING_REFERENCE),
    ("rule-duplicate-id.fdg", rule_ids::DUPLICATE_ID),
    ("rule-unreadable-entry.fdg", rule_ids::UNREADABLE_ENTRY),
];

fn breaches_in(fixture: &str) -> Vec<FdgBreach> {
    rules::breaches(&model(fixture))
}

fn problems_in(fixture: &str) -> Vec<adp_diagram::DiagramProblem> {
    rules::problems_of(&fs::read_to_string(fixture_path(fixture)).unwrap())
}

#[test]
fn each_fixture_reports_its_own_rule_and_no_other() {
    for (fixture, rule_id) in EVERY_RULE {
        let breaches = breaches_in(fixture);
        assert!(breaches.iter().any(|b| b.rule_id == rule_id), "{fixture}: {breaches:?}");
        assert!(breaches.iter().all(|b| b.rule_id == rule_id), "{fixture}: {breaches:?}");
    }
}

#[test]
fn each_breach_names_what_it_is_about() {
    for (fixture, rule_id) in EVERY_RULE {
        let breaches: Vec<FdgBreach> = breaches_in(fixture)
            .into_iter()
            .filter(|b| b.rule_id == rule_id)
            .collect();
        assert!(!breaches.is_empty(), "{fixture}");
        assert!(breaches.iter().all(|b| !b.message.trim().is_empty()), "{fixture}");
        // The unreadable-entry rule carries the parser's problems, about a line rather than an element.
        if rule_id != rule_ids::UNREADABLE_ENTRY {
            assert!(breaches.iter().all(|b| !b.elements.is_empty()), "{fixture}");
        }
    }
}

#[test]
fn the_clean_document_reports_nothing() {
    assert_eq!(breaches_in("rules-clean.fdg"), []);
}

#[test]
fn a_shows_loop_is_not_a_cycle() {
    let model = model("rules-clean.fdg");
    let shows: Vec<_> = model
        .connections
        .iter()
        .filter(|c| c.connection_type == connection_types::SHOWS)
        .collect();
    assert_eq!(shows.len(), 2);
    assert!(shows.iter().any(|c| c.from == "open" && c.to == "detail"));
    assert!(shows.iter().any(|c| c.from == "back" && c.to == "page"));
    assert!(relations::cycles_in(&model).is_empty());
}

#[test]
fn an_ownership_loop_is_reported_once_with_its_members() {
    let cycles = relations::cycles_in(&model("rule-ownership-cycle.fdg"));
    assert_eq!(cycles, [vec!["f1".to_owned(), "f2".to_owned()]]);
}

#[test]
fn the_ownership_loop_breaks_no_cardinality_limit() {
    let breaches = breaches_in("rule-ownership-cycle.fdg");
    assert!(!breaches.iter().any(|b| b.rule_id == rule_ids::SECOND_PARENT));
    assert!(breaches.iter().any(|b| b.rule_id == rule_ids::OWNERSHIP_CYCLE));
    assert_eq!(breaches[0].message, "Ownership loops: f1 -> f2 -> f1.");
}

#[test]
fn the_table_is_exactly_the_five_relations() {
    let ids: Vec<&str> = relations::ALL.iter().map(|r| r.id).collect();
    assert_eq!(ids, ["ui-child", "owns-action", "owns-data", "owns-function", "shows"]);
    assert_eq!(relations::ALL.iter().filter(|r| r.is_ownership).count(), 4);
    assert!(!relations::by_id(connection_types::SHOWS).unwrap().is_ownership);
}

#[test]
fn no_relation_admits_a_comment() {
    for relation in &relations::ALL {
        assert!(!relation.sources.contains(&element_types::COMMENT));
        assert_ne!(relation.target, element_types::COMMENT);
    }
}

#[test]
fn the_limits_are_the_ones_the_table_states() {
    for relation in relations::ALL.iter().filter(|r| r.is_ownership) {
        assert_eq!(relation.max_into_target, Some(1));
    }
    let shows = relations::by_id(connection_types::SHOWS).unwrap();
    assert_eq!(shows.max_from_source, Some(1));
    assert_eq!(shows.max_into_target, None);
}

#[test]
fn a_document_that_breaks_a_rule_still_opens() {
    let model = model("rule-ownership-cycle.fdg");
    let ids: Vec<&str> = model.elements.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, ["f1", "f2"]);
    assert_eq!(model.connections.len(), 2);
}

/// Each breach fixture reaches the panel under its own rule id, and only that one, at the severity its rule has:
/// an unreadable entry keeps its lines, so it warns; a breach of the notation is an error.
#[test]
fn each_rule_fixture_reaches_the_panel_under_its_rule_id_and_severity() {
    for (fixture, rule_id) in EVERY_RULE {
        let problems = problems_in(fixture);
        assert!(!problems.is_empty(), "{fixture}");
        let expected = if rule_id == rule_ids::UNREADABLE_ENTRY {
            DiagramProblemSeverity::Warning
        } else {
            DiagramProblemSeverity::Error
        };
        for problem in problems {
            assert_eq!(problem.rule_id, rule_id, "{fixture}");
            assert!(!problem.message.trim().is_empty(), "{fixture}");
            assert_eq!(problem.severity, expected, "{fixture}");
        }
    }
}

#[test]
fn the_location_is_the_breachs_line_counted_from_one() {
    let problems = problems_in("rule-unreadable-entry.fdg");
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].location, Some(DiagramProblemLocation::Line(3)));
    assert_eq!(problems[0].message, "`narrow` is 12 wide; the minimum is 80.");
}

#[test]
fn a_document_that_is_not_yaml_is_reported_not_refused() {
    assert!(!problems_in("not-yaml.fdg").is_empty());
}

fn example() -> adp_diagram_functional_decomposition_graph::model::FdgModel {
    common::parse_file(&example_path())
}

#[test]
fn the_example_reads_without_a_problem_and_breaks_no_rule() {
    let model = example();
    assert_eq!(model.version, Some(1));
    assert!(!model.elements.is_empty());
    assert!(model.problems.is_empty(), "{:?}", model.problems);
    let document = LineDocument::parse(&fs::read_to_string(example_path()).unwrap());
    assert_eq!(rules::validate(&document), []);
}

#[test]
fn every_element_type_and_every_relation_appears_in_the_example() {
    let model = example();
    for element_type in element_types::ALL {
        assert!(
            model.elements.iter().any(|e| e.element_type == element_type),
            "{element_type}"
        );
    }
    for relation in &relations::ALL {
        assert!(
            model.connections.iter().any(|c| c.connection_type == relation.id),
            "{}",
            relation.id
        );
    }
}

#[test]
fn a_function_owns_a_function_and_a_data_element_owns_a_data_element() {
    let model = example();
    let type_of: HashMap<&str, &str> = model
        .elements
        .iter()
        .map(|e| (e.id.as_str(), e.element_type.as_str()))
        .collect();
    assert!(
        model
            .connections
            .iter()
            .any(|c| c.connection_type == connection_types::OWNS_FUNCTION
                && type_of[c.from.as_str()] == element_types::FUNCTION
                && type_of[c.to.as_str()] == element_types::FUNCTION)
    );
    assert!(
        model
            .connections
            .iter()
            .any(|c| c.connection_type == connection_types::OWNS_DATA
                && type_of[c.from.as_str()] == element_types::DATA_ELEMENT
                && type_of[c.to.as_str()] == element_types::DATA_ELEMENT)
    );
}

#[test]
fn the_navigation_round_trip_is_there_and_is_not_an_ownership_cycle() {
    let model = example();
    let round_trip = [
        ("task-list", connection_types::UI_CHILD, "task-row"),
        ("task-row", connection_types::OWNS_ACTION, "open-task"),
        ("open-task", connection_types::SHOWS, "task-detail"),
        ("task-detail", connection_types::OWNS_ACTION, "back-to-list"),
        ("back-to-list", connection_types::SHOWS, "task-list"),
    ];
    for (from, relation, to) in round_trip {
        assert!(
            model
                .connections
                .iter()
                .any(|c| c.from == from && c.connection_type == relation && c.to == to),
            "{from} {relation} {to}"
        );
    }
    assert!(relations::cycles_in(&model).is_empty());
}

#[test]
fn the_example_carries_names_descriptions_and_comments() {
    let model = example();
    assert!(model.connections.iter().any(|c| !c.name.is_empty()));
    assert!(model.elements.iter().any(|e| !e.description.is_empty()));
    assert!(model.connections.iter().any(|c| !c.description.is_empty()));
    assert_eq!(model.elements.iter().filter(|e| e.is_comment()).count(), 2);
}

/// The client's `relationOnto`: a drag from one element's body to another's draws the one relation whose ends admit
/// the pair, and none for a pair no relation admits (`fdgConnect.test.tsx`).
#[test]
fn the_relation_a_drag_draws_is_the_one_the_pair_admits() {
    use element_types::{ACTION, COMMENT, DATA_ELEMENT, FUNCTION, UI_ELEMENT};
    let onto = |from, to| relations::onto(from, to).map(|r| r.id);
    assert_eq!(onto(UI_ELEMENT, UI_ELEMENT), Some(connection_types::UI_CHILD));
    assert_eq!(onto(UI_ELEMENT, ACTION), Some(connection_types::OWNS_ACTION));
    assert_eq!(onto(ACTION, DATA_ELEMENT), Some(connection_types::OWNS_DATA));
    assert_eq!(onto(DATA_ELEMENT, FUNCTION), Some(connection_types::OWNS_FUNCTION));
    assert_eq!(onto(FUNCTION, FUNCTION), Some(connection_types::OWNS_FUNCTION));
    assert_eq!(onto(ACTION, UI_ELEMENT), Some(connection_types::SHOWS));
    assert_eq!(onto(ACTION, ACTION), None);
    assert_eq!(onto(FUNCTION, DATA_ELEMENT), None);
    assert_eq!(onto(COMMENT, UI_ELEMENT), None);
    assert_eq!(onto(UI_ELEMENT, COMMENT), None);
}
