//! Arrange diagram through the desktop session: the standalone host's `ArrangeCausalLoopCommandTests` at `8b6ccfc`, with
//! the same expectations. The standalone test ran the command's handler and its inverse; here the action runs from the
//! canvas's menu through the module and the project's history undoes it.
//!
//! One differs in where it is asked: an empty diagram never reaches the layout, since the specification's menu makes
//! Arrange diagram unavailable below two variables, and the session refuses it with that entry's reason.

mod common;

use std::fs;

use adp_diagram::ActionOutcome;
use adp_diagram_causal_loop::metrics::Metrics;
use adp_diagram_causal_loop::self_organizing::{self, DEFAULT_BUDGET, SEPARATION_ROUNDS};
use common::{Diagram, open_with};

const BODY: &str = "causal-loop 1\r\n\r\n# the classic pair\r\nvariable population \"Population\"\r\nvariable births \"Births\"\r\nvariable crowding \"Crowding\"\r\n\r\nlink population -> births +\r\nlink births -> population +\r\nlink population -> crowding +\r\nlink crowding -> population -\r\n";

/// The standalone test's registration, its comment moved below the block: FBL section 8.1 reads the `layout:` block
/// among the headers, and what follows a comment is unbound content, where the standalone host's `RegistrationLayout`
/// looked for the block on any line.
const REGISTRATION: &str = "systems/causal-loop-diagram\r\nbody: feedback.cld\r\ntitle: Population feedback\r\nlayout:\r\n  variable:population: 10 20\r\n\r\n# an author's note that must survive\r\n";

fn diagram() -> Diagram {
    open_with(BODY, REGISTRATION)
}

/// The stored positions, by id.
fn stored(diagram: &Diagram) -> Vec<(String, f64, f64)> {
    let text = diagram.registration_text();
    text.lines()
        .skip_while(|l| l.trim_end() != "layout:")
        .skip(1)
        .take_while(|l| l.starts_with("  "))
        .filter_map(|l| {
            let (id, value) = l.trim().rsplit_once(": ")?;
            let mut parts = value.split(' ').map(|p| p.parse::<f64>().unwrap());
            Some((id.to_owned(), parts.next()?, parts.next()?))
        })
        .collect()
}

#[test]
fn the_arrangement_is_stored_as_authored_positions() {
    let mut diagram = diagram();
    assert_eq!(diagram.arrange(), ActionOutcome::Done);
    let ids: Vec<String> = stored(&diagram).into_iter().map(|(id, _, _)| id).collect();
    assert_eq!(ids, ["variable:births", "variable:crowding", "variable:population"]);
}

#[test]
fn the_positions_stored_are_the_centres_the_layout_computed() {
    let mut diagram = diagram();
    let expected =
        self_organizing::compute(&diagram.model(), &Metrics::default(), DEFAULT_BUDGET, SEPARATION_ROUNDS).unwrap();
    assert_eq!(diagram.arrange(), ActionOutcome::Done);
    let stored = stored(&diagram);
    assert!(!expected.is_empty());
    for (name, rect) in &expected {
        let (_, x, y) = stored
            .iter()
            .find(|(id, _, _)| *id == format!("variable:{name}"))
            .unwrap();
        assert!((rect.center_x() - x).abs() < 5e-4, "{name}");
        assert!((rect.center_y() - y).abs() < 5e-4, "{name}");
    }
    // And the session draws them there.
    let (x, y) = diagram.position("variable:births");
    assert!((x - expected["births"].center_x()).abs() < 5e-4 && (y - expected["births"].center_y()).abs() < 5e-4);
}

#[test]
fn the_body_is_not_touched() {
    let mut diagram = diagram();
    assert_eq!(diagram.arrange(), ActionOutcome::Done);
    assert_eq!(diagram.text(), BODY);
}

#[test]
fn everything_above_the_layout_block_survives() {
    let mut diagram = diagram();
    assert_eq!(diagram.arrange(), ActionOutcome::Done);
    let text = diagram.registration_text();
    assert!(text.contains("# an author's note that must survive"));
    assert!(text.contains("title: Population feedback"));
    assert!(text.starts_with("systems/causal-loop-diagram\r\nbody: feedback.cld\r\n"));
}

#[test]
fn one_undo_restores_the_registration_byte_for_byte() {
    let mut diagram = diagram();
    assert_eq!(diagram.arrange(), ActionOutcome::Done);
    assert_ne!(diagram.registration_text(), REGISTRATION);
    assert!(diagram.workspace.undo().is_done());
    assert_eq!(diagram.registration_text(), REGISTRATION);
    assert_eq!(diagram.position("variable:population"), (10.0, 20.0));
    // And redoing the undo arranges again.
    let arranged = {
        assert!(diagram.workspace.redo().is_done());
        diagram.registration_text()
    };
    assert_ne!(arranged, REGISTRATION);
}

#[test]
fn arranging_twice_writes_the_same_registration() {
    let mut diagram = diagram();
    assert_eq!(diagram.arrange(), ActionOutcome::Done);
    let first = diagram.registration_text();
    let mut again = self::diagram();
    assert_eq!(again.arrange(), ActionOutcome::Done);
    assert_eq!(again.registration_text(), first);
    // Arranging the arranged diagram changes nothing.
    assert_eq!(diagram.arrange(), ActionOutcome::Done);
    assert_eq!(diagram.registration_text(), first);
}

#[test]
fn a_diagram_the_layout_refuses_is_refused_in_the_layouts_own_words_and_writes_nothing() {
    let mut text = String::from("causal-loop 1\r\n");
    for index in 0..1200 {
        text.push_str(&format!("variable v{index} \"V{index}\"\r\n"));
    }
    let mut diagram = open_with(&text, REGISTRATION);
    let expected = self_organizing::over_budget(1200, DEFAULT_BUDGET);
    assert_eq!(diagram.arrange(), ActionOutcome::Refused(expected));
    assert!(matches!(diagram.arrange(), ActionOutcome::Refused(r) if r.contains("1200 variables")));
    assert_eq!(diagram.registration_text(), REGISTRATION);
}

#[test]
fn a_diagram_with_no_variables_is_refused_rather_than_arranged_to_nothing() {
    let mut diagram = open_with("causal-loop 1\r\n", REGISTRATION);
    assert_eq!(
        diagram.arrange(),
        ActionOutcome::Refused("There is nothing to arrange until this diagram has two variables.".to_owned())
    );
    assert_eq!(diagram.registration_text(), REGISTRATION);
}

#[test]
fn a_missing_registration_is_refused_rather_than_created() {
    let mut diagram = diagram();
    fs::remove_file(&diagram.registration).unwrap();
    assert_eq!(
        diagram.arrange(),
        ActionOutcome::Refused(adp_diagram_causal_loop::REGISTRATION_GONE.to_owned())
    );
    assert!(!diagram.registration.exists());
}

#[test]
fn an_unreadable_body_is_refused_without_touching_the_registration() {
    let mut diagram = open_with("this is not a causal loop diagram at all\r\n", REGISTRATION);
    assert!(matches!(diagram.arrange(), ActionOutcome::Refused(_)));
    assert_eq!(diagram.registration_text(), REGISTRATION);
}

#[test]
fn a_diagram_opened_without_a_registration_has_nowhere_to_store_an_arrangement() {
    let folder = tempfile::tempdir().unwrap();
    fs::write(folder.path().join("feedback.cld"), BODY).unwrap();
    let mut workspace = adp_diagram::DiagramWorkspace::new(folder.path());
    let (_, mut session) = workspace
        .open(&mut common::tools(), std::path::Path::new("feedback.cld"))
        .unwrap();
    session.baseline(workspace.documents()).unwrap();
    let outcome = session.execute(
        &mut workspace,
        &adp_diagram::ActionTarget::Canvas { x: 0.0, y: 0.0 },
        adp_diagram_causal_loop::ARRANGE,
        &adp_disl::behavior::ids::fixed(&[]),
    );
    assert_eq!(
        outcome,
        ActionOutcome::Refused(adp_diagram_causal_loop::NO_REGISTRATION.to_owned())
    );
}
