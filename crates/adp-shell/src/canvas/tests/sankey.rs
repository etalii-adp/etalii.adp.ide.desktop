//! A Sankey diagram on the canvas with its module (etalii.adp spec 030, task T077): the standalone host's
//! `brightwater-coffee` example opened through its registration from the parity fixtures, its bars and bands drawn
//! where the module laid them out, a bar dragged down its column, a flow drawn between two bars and stepped from its
//! menu, and every edit undone through the project's history.

use std::fs;
use std::path::Path;

use adp_diagram::ActionTarget;
use adp_scene::Point;
use adp_scene::scene::{Owner, Primitive, Role};
use eframe::egui::{self, Modifiers, PointerButton};

use super::{Harness, bodies, texts_of};

pub(super) fn company() -> Harness {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/fixtures/etalii/sankey/brightwater-coffee");
    Harness::open_from(
        &folder,
        &["brightwater-coffee.adp", "brightwater-coffee.skv"],
        "brightwater-coffee.adp",
        "brightwater-coffee.skv",
    )
}

fn bands(scene: &adp_scene::Scene) -> usize {
    scene
        .primitives
        .iter()
        .filter(|p| matches!(p, Primitive::Shape(s) if s.role == Role::Body && matches!(s.owner, Owner::Relation(_))))
        .count()
}

#[test]
fn a_sankey_diagram_opens_with_its_bars_and_bands_and_is_dragged_connected_and_stepped_on_the_canvas() {
    let mut harness = company();
    assert!(harness.tab.session.module_view().is_some());
    harness.settle();
    let scene = harness.tab.scene().expect("a scene");
    assert_eq!(bodies(scene), 14, "every node has its bar");
    assert_eq!(bands(scene), 13, "every flow has its band");
    let texts = texts_of(scene);
    for label in [
        "Cafés",
        "€1,840M",
        "+9% Y/Y",
        "Revenue",
        "€3,500M",
        "(€140M)",
        "12% margin",
    ] {
        assert!(texts.contains(&label), "{label} is drawn: {texts:?}");
    }
    // Every bar at the centre the module gave it, where the parity script's moves start from.
    let cafes = harness.tab.bounds_of("cafes").expect("cafes is drawn");
    assert_eq!((cafes.centre().x, cafes.centre().y), (12.0, 147.2));
    assert_eq!((cafes.width, cafes.height), (24.0, 294.4));

    // A click selects a bar, and the property grid shows its rows through the session.
    let at = harness.screen_of("cafes");
    harness.click(at, Modifiers::NONE);
    assert_eq!(harness.tab.selection, vec!["cafes".to_owned()]);
    let rows = harness
        .tab
        .session
        .properties(harness.host.workspace(&harness.root).documents(), &["cafes"]);
    assert_eq!((rows[0].id.as_str(), rows[0].value.as_str()), ("sankey.name", "Cafés"));

    // A drag down the column, past the packaged coffee, moves the cafés' entry after it, and nothing else.
    let packaged = harness.screen_of("packaged");
    harness.drag(
        at,
        egui::pos2(at.x + 30.0, packaged.y + 4.0),
        PointerButton::Primary,
        true,
    );
    let written = harness.written();
    assert!(
        written.find("  - id: packaged").unwrap() < written.find("  - id: cafes").unwrap(),
        "{written}"
    );
    assert_eq!(written.len(), harness.original.len());
    harness.tab.changed();
    harness.settle();
    let moved = harness.tab.bounds_of("cafes").expect("cafes is drawn");
    assert_eq!(moved.x, cafes.x, "a node keeps its column");
    assert!(moved.y > cafes.y, "{moved:?}");

    // A flow drawn from one bar to another: Add flow, carrying on what the source has not passed on.
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.connect(
        host.workspace(root),
        "net-profit",
        Some("research"),
        Point::new(0.0, 0.0),
    );
    assert!(
        harness
            .written()
            .ends_with("  - from: net-profit\r\n    to: research\r\n    value: 420\r\n"),
        "{}",
        harness.written()
    );

    // A flow's menu steps its value.
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.run_action(
        host.workspace(root),
        ActionTarget::Element("cafes->revenue".into()),
        "sankey.increase",
    );
    assert!(harness.written().contains("    to: revenue\r\n    value: 1940\r\n"));
    harness.tab.changed();
    harness.settle();
    assert!(texts_of(harness.tab.scene().unwrap()).contains(&"€1,940M"));

    // Three undos take every edit back, byte for byte.
    for _ in 0..3 {
        assert!(harness.workspace().undo().is_done());
    }
    assert_eq!(fs::read_to_string(&harness.body).unwrap(), harness.original);
}
