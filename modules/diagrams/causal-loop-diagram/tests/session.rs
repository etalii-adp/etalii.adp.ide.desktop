//! A causal loop diagram edited through the desktop session with the module: the standalone host's
//! `CausalLoopContextActionsTests`, `CausalLoopContextPropertiesTests` and `CausalLoopSessionTests` at `8b6ccfc`, where
//! they reach what the module or the session does for this tool type beyond the specification (the arrangement, a
//! variable added at a point with its position in the same step, a link that claims the loops it closes, the polarity
//! row's spellings, what a drag may move), with the same expectations. What the specification's menus, rows and forms
//! answer alike for every tool type is `adp-diagram`'s to test, and the payloads of the gRPC contract are not the
//! desktop host's.

mod common;

use adp_diagram::{ActionOutcome, ActionTarget};
use common::{Diagram, open, open_with};

const CORPUS: &str = "causal-loop 1\r\nvariable population \"Population\"\r\nvariable births \"Births\"\r\nlink population -> births +\r\nlink births -> population +\r\nloop R1 \"Births beget births\" population births\r\n";

fn action(diagram: &Diagram, target: &ActionTarget, id: &str) -> Option<adp_context::Action> {
    diagram
        .session
        .actions(diagram.workspace.documents(), target)
        .into_iter()
        .flat_map(|g| g.actions)
        .find(|a| a.id == id)
}

fn canvas() -> ActionTarget {
    ActionTarget::Canvas { x: 0.0, y: 0.0 }
}

#[test]
fn a_placement_offers_a_new_variable_and_the_diagram_wide_arrangement() {
    let diagram = open(CORPUS);
    assert!(action(&diagram, &canvas(), "causal-loop.add-variable").is_some());
    assert!(
        action(&diagram, &canvas(), "causal-loop.arrange")
            .expect("offered")
            .is_available()
    );
    assert!(
        action(
            &diagram,
            &ActionTarget::Element("variable:population".into()),
            "causal-loop.arrange"
        )
        .is_none()
    );
}

#[test]
fn the_arrangement_is_unavailable_with_its_reason_on_a_diagram_with_nothing_to_arrange() {
    let diagram = open("causal-loop 1\r\nvariable alone \"Alone\"\r\n");
    let arrange = action(&diagram, &canvas(), "causal-loop.arrange").expect("offered");
    assert!(!arrange.is_available());
    assert!(arrange.unavailable_reason.contains("two variables"), "{arrange:?}");
}

#[test]
fn the_arrangement_runs_through_the_action_path_and_is_one_undo_away() {
    let mut diagram = open(CORPUS);
    let before = diagram.registration_text();
    assert_eq!(diagram.arrange(), ActionOutcome::Done);
    let text = diagram.registration_text();
    assert!(
        text.contains("  variable:births: ") && text.contains("  variable:population: "),
        "{text}"
    );
    assert_eq!(text.matches("  variable:").count(), 2);
    assert_eq!(diagram.text(), CORPUS);
    assert!(diagram.workspace.undo().is_done());
    assert_eq!(diagram.registration_text(), before);
}

#[test]
fn adding_a_variable_completes_with_a_calculated_name_without_asking_for_one() {
    let mut diagram = open(CORPUS);
    let outcome = diagram.execute(&ActionTarget::Canvas { x: 10.0, y: 20.0 }, "causal-loop.add-variable");
    assert_eq!(outcome, ActionOutcome::Done);
    assert!(diagram.model().variables.iter().any(|v| v.name == "variable1"));
    // Its position is stored under its id in the same step, in a new block the standalone host wrote in CRLF.
    assert_eq!(
        diagram.registration_text(),
        "systems/causal-loop-diagram\r\nlayout:\r\n  variable:variable1: 10 20\r\n"
    );
    assert_eq!(diagram.position("variable:variable1"), (10.0, 20.0));
    // One undo takes both back.
    assert!(diagram.workspace.undo().is_done());
    assert_eq!(diagram.text(), CORPUS);
    assert_eq!(diagram.registration_text(), "systems/causal-loop-diagram\r\n");
}

#[test]
fn a_new_layout_block_takes_crlf_whatever_the_registration_ends_its_lines_with() {
    let mut diagram = open_with(CORPUS, "systems/causal-loop-diagram\n");
    assert_eq!(
        diagram.execute(&ActionTarget::Canvas { x: 5.5, y: -3.0 }, "causal-loop.add-variable"),
        ActionOutcome::Done
    );
    assert_eq!(
        diagram.registration_text(),
        "systems/causal-loop-diagram\nlayout:\r\n  variable:variable1: 5.5 -3\r\n"
    );
}

#[test]
fn a_drawn_link_states_it_and_claims_the_loop_it_closes_with_no_dialog() {
    let mut diagram = open("causal-loop 1\r\n\r\nvariable a\r\nvariable b\r\nlink a -> b +\r\n");
    let outcome = diagram.execute(
        &ActionTarget::Connection {
            source: Some("variable:b".into()),
            target: Some("variable:a".into()),
        },
        "causal-loop.connect",
    );
    assert_eq!(outcome, ActionOutcome::Done);
    assert_eq!(
        diagram.text(),
        "causal-loop 1\r\n\r\nvariable a\r\nvariable b\r\nlink a -> b +\r\nlink b -> a +\r\nloop R1 \"\" a b\r\n"
    );
    assert!(diagram.workspace.undo().is_done());
    assert_eq!(
        diagram.text(),
        "causal-loop 1\r\n\r\nvariable a\r\nvariable b\r\nlink a -> b +\r\n"
    );
}

#[test]
fn adding_a_link_asks_for_the_other_end_then_writes_it() {
    let mut diagram = open(&format!("{CORPUS}variable deaths \"Deaths\"\r\n"));
    let target = ActionTarget::Element("variable:population".into());
    assert!(matches!(
        diagram.execute(&target, "causal-loop.add-link"),
        ActionOutcome::Prompt(_)
    ));
    assert_eq!(
        diagram.answer(&target, "causal-loop.add-link", "deaths"),
        ActionOutcome::Done
    );
    assert!(diagram.text().contains("link population -> deaths +"));
}

#[test]
fn an_answered_link_that_closes_a_cycle_claims_it_in_the_same_edit() {
    let mut diagram = open(&format!(
        "{CORPUS}variable deaths \"Deaths\"\r\nlink deaths -> population -\r\n"
    ));
    let target = ActionTarget::Element("variable:population".into());
    assert_eq!(
        diagram.answer(&target, "causal-loop.add-link", "deaths"),
        ActionOutcome::Done
    );
    let text = diagram.text();
    assert!(text.contains("link population -> deaths +"), "{text}");
    assert!(text.contains("loop B1 \"\" population deaths"), "{text}");
    assert!(diagram.workspace.undo().is_done());
    assert!(!diagram.text().contains("loop B1"));
}

#[test]
fn a_link_to_something_undeclared_is_refused_naming_it() {
    let mut diagram = open(CORPUS);
    let target = ActionTarget::Element("variable:population".into());
    assert!(matches!(
        diagram.answer(&target, "causal-loop.add-link", "nowhere"),
        ActionOutcome::Refused(reason) if reason.contains("'nowhere'") || reason.contains("No variable of that name")
    ));
    assert_eq!(diagram.text(), CORPUS);
}

#[test]
fn setting_a_polarity_from_the_grid_takes_it_as_the_document_writes_it() {
    let mut diagram = open(CORPUS);
    for (typed, written) in [("-", "-"), ("s", "+"), ("O", "-"), ("positive", "+")] {
        diagram
            .session
            .set_property(
                &mut diagram.workspace,
                &["link:population|births"],
                "causal-loop.link-polarity",
                typed,
            )
            .unwrap_or_else(|e| panic!("{typed}: {e}"));
        assert!(
            diagram
                .text()
                .contains(&format!("link population -> births {written}\r\n")),
            "{typed}: {}",
            diagram.text()
        );
    }
    let refused = diagram
        .session
        .set_property(
            &mut diagram.workspace,
            &["link:population|births"],
            "causal-loop.link-polarity",
            "sideways",
        )
        .unwrap_err();
    assert_eq!(refused, "'sideways' is not a polarity a link can state.");
}

#[test]
fn moving_a_link_or_a_loop_is_refused_because_neither_has_a_position() {
    let diagram = open(CORPUS);
    for id in ["link:population|births", "loop:R1"] {
        let mut diagram_mut = open(CORPUS);
        assert!(
            diagram_mut
                .session
                .move_element_to(&mut diagram_mut.workspace, id, 1.0, 2.0)
                .is_err(),
            "{id}"
        );
        assert_eq!(diagram_mut.registration_text(), "systems/causal-loop-diagram\r\n");
    }
    drop(diagram);
}

#[test]
fn a_reposition_stores_in_the_registration_and_never_touches_the_body() {
    let mut diagram = open(CORPUS);
    diagram
        .session
        .move_element_to(&mut diagram.workspace, "variable:births", 120.0, -45.5)
        .unwrap();
    assert_eq!(diagram.text(), CORPUS);
    assert_eq!(
        diagram.registration_text(),
        "systems/causal-loop-diagram\r\nlayout:\r\n  variable:births: 120 -45.5\r\n"
    );
    assert_eq!(diagram.position("variable:births"), (120.0, -45.5));
}
