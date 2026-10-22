//! An agent activity diagram on the canvas with its module (etalii.adp spec 030, task T075): the standalone host's
//! `two-projects` example, which locks no element, opened through its registration from the parity fixtures, drawn on
//! the module's rings, a card dragged and so locked where it was let go, an agent dropped from the palette, joined to a
//! specification by a line drawn from it and renamed through its prompt, a line the rules refuse, and every edit undone
//! through the project's history.

use std::fs;
use std::path::Path;

use adp_diagram::ActionTarget;
use eframe::egui::PointerButton;

use super::{Harness, texts_of};

pub(super) fn two_projects() -> Harness {
    let folder =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/fixtures/etalii/agent-activity-diagram/two-projects");
    Harness::open_from(
        &folder,
        &["two-projects.adp", "two-projects.aad"],
        "two-projects.adp",
        "two-projects.aad",
    )
}

#[test]
fn an_agent_activity_diagram_opens_on_its_rings_and_is_dragged_dropped_connected_and_renamed_on_the_canvas() {
    let mut harness = two_projects();
    assert!(harness.tab.session.module_view().is_some());
    harness.settle();

    // The cards with their statuses, kinds, folders and task groups.
    let texts = texts_of(harness.tab.scene().expect("a scene"));
    for label in [
        "etalii.adp",
        "Agent activity diagram",
        "Progressing",
        "Module thread",
        "Fractal",
        "Local machine",
        "Default",
        "▾ Progressing (1)",
        "▸ Pending (1)",
    ] {
        assert!(texts.contains(&label), "{label} is drawn: {texts:?}");
    }
    // Projects in the middle, environments outermost.
    let centre = |harness: &Harness, id: &str| {
        let bounds = harness.tab.bounds_of(id).unwrap_or_else(|| panic!("{id} is drawn"));
        let c = bounds.centre();
        (c.x * c.x + c.y * c.y).sqrt()
    };
    assert!(centre(&harness, "p-ide") < centre(&harness, "e-cloud"));

    // A drag locks the card where its centre was let go.
    let original = harness.written();
    let before = harness.tab.bounds_of("a-module").unwrap().centre();
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.move_by(host.workspace(root), &["a-module".to_owned()], (40.0, 20.0));
    let moved = harness.written();
    assert!(
        moved.contains(&format!(
            "view:\r\n  placements:\r\n    - element: a-module\r\n      x: {}\r\n      y: {}\r\n",
            (before.x + 40.0).round_ties_even(),
            (before.y + 20.0).round_ties_even()
        )),
        "{moved}"
    );
    harness.tab.changed();
    harness.settle();

    // An agent dropped from the palette, under a ShortGuid, locked where it was dropped.
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.run_action(
        host.workspace(root),
        ActionTarget::Canvas { x: 0.0, y: -600.0 },
        "add-agent",
    );
    let dropped = harness.written();
    let id = dropped
        .lines()
        .skip_while(|l| *l != "agents:")
        .take_while(|l| *l != "locations:")
        .filter_map(|l| l.strip_prefix("  - id: "))
        .last()
        .unwrap()
        .to_owned();
    assert_eq!(25, id.len(), "{id}");
    assert!(dropped.contains(&format!("    - element: {id}\r\n      x: 0\r\n      y: -600\r\n")));
    harness.tab.changed();
    harness.settle();

    // A line drawn from it to a specification sets its key; a line the rules refuse writes nothing.
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.connect(
        host.workspace(root),
        &id,
        Some("s-conversion"),
        adp_scene::Point::new(0.0, 0.0),
    );
    assert!(
        harness.written().contains(&format!(
            "  - id: {id}\r\n    name: New agent\r\n    specification: s-conversion\r\n"
        )),
        "{}",
        harness.written()
    );
    let connected = harness.written();
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.connect(
        host.workspace(root),
        "p-ide",
        Some("e-fractal"),
        adp_scene::Point::new(0.0, 0.0),
    );
    assert_eq!(connected, harness.written());

    // Renamed in place, over its own label.
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.run_action(host.workspace(root), ActionTarget::Element(id.clone()), "editLabel");
    let editor = harness.tab.editor.as_mut().expect("a rename opens the inline editor");
    assert_eq!(
        (editor.element.as_str(), editor.text.as_str()),
        (id.as_str(), "New agent")
    );
    editor.text = "Conversion thread".to_owned();
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.commit_editor(host.workspace(root));
    assert!(harness.written().contains("    name: Conversion thread\r\n"));
    harness.tab.changed();
    harness.settle();
    assert!(texts_of(harness.tab.scene().unwrap()).contains(&"Conversion thread"));

    // Four undos take every edit back, byte for byte.
    for step in 0..4 {
        let undone = harness.workspace().undo();
        assert!(undone.is_done(), "undo {step}: {undone:?}");
    }
    assert_eq!(harness.written(), original);
    assert_eq!(fs::read_to_string(&harness.body).unwrap(), harness.original);

    harness
        .screenshot(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/screenshots/agent-activity-diagram.png"));
}

#[test]
fn a_right_drag_from_one_card_to_another_draws_the_relation_the_pair_allows() {
    let mut harness = two_projects();
    harness.settle();
    let conversion = harness.screen_of("s-conversion");
    let agent = harness.screen_of("a-definitions");
    // a-definitions works on s-fbl: the line is refused with the rule, and nothing is written.
    harness.drag(conversion, agent, PointerButton::Secondary, true);
    assert!(
        harness.tab.arbiter.connects_on_right_drag,
        "the module connects on a right drag (`aadDefinition.ts`, `connectOnRightDrag: true`)"
    );
    assert!(harness.tab.menu.is_none(), "the drag opens no menu");
    assert_eq!(harness.written(), harness.original);
    assert!(
        harness
            .tab
            .notice
            .as_deref()
            .is_some_and(|n| n.contains("already joined")),
        "{:?}",
        harness.tab.notice
    );
    // An agent dropped from the palette, and a right drag from it to a specification sets its key.
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.run_action(
        host.workspace(root),
        ActionTarget::Canvas { x: 0.0, y: -700.0 },
        "add-agent",
    );
    let id = harness
        .written()
        .lines()
        .skip_while(|l| *l != "agents:")
        .take_while(|l| *l != "locations:")
        .filter_map(|l| l.strip_prefix("  - id: "))
        .last()
        .unwrap()
        .to_owned();
    harness.tab.notice = None;
    harness.tab.changed();
    harness.tab.fit();
    harness.settle();
    let dropped = harness.screen_of(&id);
    let conversion = harness.screen_of("s-conversion");
    harness.drag(dropped, conversion, PointerButton::Secondary, true);
    assert!(
        harness.written().contains(&format!(
            "  - id: {id}\r\n    name: New agent\r\n    specification: s-conversion\r\n"
        )),
        "{:?}\n{}",
        harness.tab.notice,
        harness.written()
    );
}
