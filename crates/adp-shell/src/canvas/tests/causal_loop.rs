//! A causal loop diagram on the canvas with its module (etalii.adp spec 030, task T069): the standalone host's
//! `reference` example, which stores no position, opened through its registration from the parity fixtures, drawn on
//! the module's ring, edited from the context menu of the canvas and of a link, arranged, and every edit undone through
//! the project's history.

use std::fs;
use std::path::Path;

use adp_diagram::ActionTarget;

use eframe::egui::{Modifiers, PointerButton};

use super::{Harness, bodies, texts_of};

pub(super) fn reference() -> Harness {
    let folder =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/fixtures/systems/causal-loop-diagram/reference");
    Harness::open_from(
        &folder,
        &["reference.adp", "reference.cld"],
        "reference.adp",
        "reference.cld",
    )
}

#[test]
fn a_causal_loop_diagram_opens_on_its_ring_and_is_edited_and_arranged_on_the_canvas() {
    let mut harness = reference();
    harness.settle();
    let scene = harness.tab.scene().expect("a scene");
    assert_eq!(bodies(scene), 6, "every variable has its pill");
    let texts = texts_of(scene);
    for label in [
        "Population",
        "Births",
        "Food per capita",
        "R1 · Births beget births",
        "B4 · Food limits births",
    ] {
        assert!(texts.contains(&label), "{label} is drawn: {texts:?}");
    }

    // The ring: the first variable of the walk sits at the top, the others around it, none on another.
    let population = harness
        .tab
        .bounds_of("variable:population")
        .expect("population is drawn");
    assert!(population.centre().x.abs() < 1e-6, "{population:?}");
    assert!(population.centre().y < 0.0, "{population:?}");
    let births = harness.tab.bounds_of("variable:births").expect("births is drawn");
    assert!(births.centre().x > 0.0, "clockwise from the top: {births:?}");

    // A variable added on the canvas: its statement in the body and its position in a new `layout:` block, in CRLF
    // as the standalone host wrote it, as one edit.
    let registration = harness.root.join("reference.adp");
    let original_registration = fs::read_to_string(&registration).unwrap();
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.run_action(
        host.workspace(root),
        ActionTarget::Canvas { x: 300.0, y: -200.0 },
        "causal-loop.add-variable",
    );
    assert_eq!(
        fs::read_to_string(&registration).unwrap(),
        "systems/causal-loop-diagram\nlayout:\r\n  variable:variable1: 300 -200\r\n"
    );
    assert!(
        fs::read_to_string(&harness.body)
            .unwrap()
            .contains("variable foodPerCapita \"Food per capita\"\nvariable variable1 \"Variable 1\"\n")
    );
    harness.tab.changed();
    harness.settle();
    let added = harness
        .tab
        .bounds_of("variable:variable1")
        .expect("the new variable is drawn");
    assert_eq!((added.centre().x, added.centre().y), (300.0, -200.0));

    // A link's menu.
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.run_action(
        host.workspace(root),
        ActionTarget::Element("link:population|births".into()),
        "causal-loop.make-negative",
    );
    assert!(
        fs::read_to_string(&harness.body)
            .unwrap()
            .contains("link population -> births - weight=1.4 \"more people, more births\"")
    );

    // Arrange diagram: every variable's position stored, the body untouched.
    let before_arrange = fs::read_to_string(&harness.body).unwrap();
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.run_action(
        host.workspace(root),
        ActionTarget::Canvas { x: 0.0, y: 0.0 },
        "causal-loop.arrange",
    );
    let arranged = fs::read_to_string(&registration).unwrap();
    assert_eq!(arranged.matches("  variable:").count(), 7, "{arranged}");
    assert_eq!(fs::read_to_string(&harness.body).unwrap(), before_arrange);
    harness.tab.changed();
    harness.settle();
    assert!(harness.tab.bounds_of("variable:crowding").is_some());

    // Three undos take every edit back, byte for byte.
    for _ in 0..3 {
        assert!(harness.workspace().undo().is_done());
    }
    assert_eq!(fs::read_to_string(&registration).unwrap(), original_registration);
    assert_eq!(fs::read_to_string(&harness.body).unwrap(), harness.original);
}

#[test]
fn a_right_drag_from_one_variable_to_another_draws_a_link_where_the_specification_binds_it_to_the_right_button() {
    let mut harness = reference();
    harness.settle();
    let deaths = harness.screen_of("variable:deaths");
    let sanitation = harness.screen_of("variable:sanitation");
    harness.press(deaths, PointerButton::Secondary, Modifiers::NONE);
    assert!(
        harness.tab.arbiter.connects_on_right_drag,
        "causal-loop-diagram.dis binds the link to a right drag from the body"
    );
    harness.release(deaths, PointerButton::Secondary, Modifiers::NONE);
    assert!(harness.tab.menu.is_some(), "a right click still opens the menu");
    assert_eq!(harness.written(), harness.original);

    harness.drag(deaths, sanitation, PointerButton::Secondary, true);
    assert!(harness.tab.menu.is_none(), "the drag opens no menu");
    let written = harness.written();
    assert!(written.contains("link deaths -> sanitation"), "{written}");
    assert!(harness.workspace().undo().is_done());
    assert_eq!(harness.written(), harness.original);
}
