//! A supply chain diagram on the canvas with its module (etalii.adp spec 030, task T076): the standalone host's
//! `automotive` example, which places no node, opened through its registration from the parity fixtures, drawn on the
//! module's layered layout, its chain traced through a selection, a card dragged by its top-left, a flow drawn from one
//! card to another, a quantity stepped from the menu, and every edit undone through the project's history.

use std::fs;
use std::path::Path;

use adp_diagram::ActionTarget;
use adp_scene::scene::Owner;
use adp_scene::scene::Primitive;

use super::{Harness, texts_of};

pub(super) fn automotive() -> Harness {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/fixtures/etalii/supply-chain/automotive");
    Harness::open_from(
        &folder,
        &["automotive.adp", "automotive.supply"],
        "automotive.adp",
        "automotive.supply",
    )
}

/// The opacity of `id`'s body.
fn body_opacity(harness: &Harness, id: &str) -> f64 {
    harness
        .tab
        .scene()
        .unwrap()
        .primitives
        .iter()
        .find_map(|p| match p {
            Primitive::Shape(s) if s.owner == Owner::Element(id.to_owned()) => Some(s.opacity),
            _ => None,
        })
        .unwrap()
}

#[test]
fn a_supply_chain_opens_on_its_layered_layout_and_is_traced_dragged_connected_and_stepped_on_the_canvas() {
    let mut harness = automotive();
    assert!(harness.tab.session.module_view().is_some());
    harness.settle();

    // The cards, the frames and the flows, as the module laid them out.
    let texts = texts_of(harness.tab.scene().expect("a scene"));
    for label in [
        "SOURCE",
        "INTEGRATOR",
        "Vehicle assembly plant",
        "2.8 M vehicles",
        "Central Africa",
        "Europe",
    ] {
        assert!(texts.contains(&label), "{label} is drawn: {texts:?}");
    }
    // Goods flow to the right: a mine stands left of the plant it feeds.
    let mine = harness.tab.bounds_of("kolwezi-cobalt").expect("the mine is drawn");
    let plant = harness.tab.bounds_of("vehicle-plant").expect("the plant is drawn");
    assert!(mine.right() < plant.x, "{mine:?} {plant:?}");
    assert_eq!((192.0, 96.0), (plant.width, plant.height));

    // A selection traces the chain through it, and dims what is off it.
    harness
        .tab
        .select(vec!["pack-assembly".to_owned()], adp_context::SelectMode::Replace);
    harness.tab.changed();
    harness.settle();
    assert_eq!(1.0, body_opacity(&harness, "kolwezi-cobalt"));
    assert!(body_opacity(&harness, "thai-rubber") < 0.5);

    // A drag writes where the card's top-left went.
    let original = harness.written();
    let before = harness.tab.bounds_of("thai-rubber").unwrap();
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.move_by(host.workspace(root), &["thai-rubber".to_owned()], (40.0, 20.0));
    let moved = harness.written();
    assert!(
        moved.contains(&format!(
            "  - id: thai-rubber\n    x: {}\n    y: {}\n",
            before.x + 40.0,
            before.y + 20.0
        )),
        "{moved}"
    );

    // A flow drawn from one card to another, under a ShortGuid.
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.connect(
        host.workspace(root),
        "thai-rubber",
        Some("private-buyers"),
        adp_scene::Point::new(0.0, 0.0),
    );
    let connected = harness.written();
    assert!(
        connected.contains("    from: thai-rubber\n    to: private-buyers\n"),
        "{connected}"
    );
    let id_line = connected
        .lines()
        .rev()
        .find(|l| l.starts_with("  - id: "))
        .unwrap()
        .trim_start_matches("  - id: ")
        .to_owned();
    assert_eq!(25, id_line.len(), "{id_line}");

    // A quantity stepped from the menu.
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.run_action(
        host.workspace(root),
        ActionTarget::Element("vehicle-plant".into()),
        "supply-chain.increase",
    );
    assert!(harness.written().contains("    quantity: 2.9\n"));
    harness.tab.changed();
    harness.settle();
    assert!(texts_of(harness.tab.scene().unwrap()).contains(&"2.9 M vehicles"));

    // Three undos take every edit back, byte for byte.
    for _ in 0..3 {
        assert!(harness.workspace().undo().is_done());
    }
    assert_eq!(harness.written(), original);
    assert_eq!(fs::read_to_string(&harness.body).unwrap(), harness.original);

    harness.screenshot(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/screenshots/supply-chain.png"));
}
