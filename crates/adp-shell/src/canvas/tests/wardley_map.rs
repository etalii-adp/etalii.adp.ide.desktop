//! A Wardley map on the canvas with its module (etalii.adp spec 030, task T070): the standalone host's
//! `strategy-vocabulary` example and its identity sidecar, opened through its registration from the parity fixtures,
//! drawn on its square with the four evolution stages, a component selected and dragged, edited from its context menu
//! and the property grid, and every edit undone through the project's history.

use std::fs;
use std::path::Path;

use adp_diagram::ActionTarget;
use eframe::egui::{self, Modifiers, PointerButton};

use super::{Harness, texts_of};

/// The ids the example's sidecar gives Payment and Datacentre.
const PAYMENT: &str = "et0fqurcgd3ri1az051jupyvf";
const DATACENTRE: &str = "804r9rdikiiftb5uxc4dmz1sb";

pub(super) fn strategy_vocabulary() -> Harness {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/fixtures/wardley/map/strategy-vocabulary");
    Harness::open_from(
        &folder,
        &[
            "strategy-vocabulary.adp",
            "strategy-vocabulary.owm",
            "strategy-vocabulary.identities.json",
        ],
        "strategy-vocabulary.adp",
        "strategy-vocabulary.owm",
    )
}

#[test]
fn a_wardley_map_opens_on_its_axes_and_is_dragged_and_edited_on_the_canvas() {
    let mut harness = strategy_vocabulary();
    assert!(harness.tab.session.module_view().is_some(), "the module draws the map");
    harness.settle();
    let scene = harness.tab.scene().expect("a scene");
    let texts = texts_of(scene);
    for label in [
        "Genesis",
        "Custom Built",
        "Product (+rental)",
        "Commodity (+utility)",
        "Value chain",
        "Evolution",
        "Customer",
        "Payment",
        "buy",
        "inertia",
        "Pioneers",
        "Renewal is due in March",
        "Regulatory pressure",
    ] {
        assert!(texts.contains(&label), "{label} is drawn: {texts:?}");
    }

    // Each mark at its point on the square of 1000: Payment `[0.70, 0.72]` at maturity 0.72, visibility 0.70.
    let payment = harness.tab.bounds_of(PAYMENT).expect("Payment is drawn");
    assert!((payment.centre().x - 720.0).abs() < 1e-6, "{payment:?}");
    assert!((payment.centre().y - 300.0).abs() < 1e-6, "{payment:?}");

    // A click selects it, and the property grid shows its rows through the session.
    let at = harness.screen_of(PAYMENT);
    harness.click(at, Modifiers::NONE);
    assert_eq!(harness.tab.selection, vec![PAYMENT.to_owned()]);
    let rows = harness
        .tab
        .session
        .properties(harness.host.workspace(&harness.root).documents(), &[PAYMENT]);
    let row = |id: &str| rows.iter().find(|r| r.id == id).map(|r| r.value.clone());
    assert_eq!(row("wardley.name").as_deref(), Some("Payment"));
    assert_eq!(row("wardley.stage").as_deref(), Some("Commodity (+utility)"));
    assert_eq!(row("wardley.decorators").as_deref(), Some("buy"));

    // A drag rewrites the coordinate pair alone, past the map's edge clamped to it, as one undo; the sidecar is
    // written beside it.
    let zoom = harness.tab.view.zoom as f32;
    harness.drag(at, at + egui::vec2(2000.0 * zoom, 0.0), PointerButton::Primary, true);
    assert!(
        harness.written().contains("component Payment [0.7, 1] (buy)\n"),
        "{}",
        harness.written()
    );
    let sidecar = harness.root.join("strategy-vocabulary.identities.json");
    assert!(fs::read_to_string(&sidecar).unwrap().contains(PAYMENT));
    assert!(harness.workspace().undo().is_done());
    assert_eq!(harness.written(), harness.original);

    // The context menu: Datacentre's inertia cleared, and its evolution stopped.
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.run_action(
        host.workspace(root),
        ActionTarget::Element(DATACENTRE.into()),
        "wardley.toggle-inertia",
    );
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.run_action(
        host.workspace(root),
        ActionTarget::Element(DATACENTRE.into()),
        "wardley.clear-evolve",
    );
    let written = harness.written();
    assert!(written.contains("component Datacentre [0.15, 0.55]\n"), "{written}");
    assert!(!written.contains("evolve Datacentre"), "{written}");

    // The property grid: a decorator set by its row, the canvas drawing it.
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.session
        .set_property(host.workspace(root), &[PAYMENT], "wardley.decorators", "buy, market")
        .unwrap();
    assert!(
        harness
            .written()
            .contains("component Payment [0.70, 0.72] (buy) (market)\n")
    );
    harness.tab.changed();
    harness.settle();
    assert!(texts_of(harness.tab.scene().unwrap()).contains(&"buy · market"));

    // Three undos take every edit back, byte for byte.
    for _ in 0..3 {
        assert!(harness.workspace().undo().is_done());
    }
    assert_eq!(harness.written(), harness.original);

    harness.screenshot(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/screenshots/wardley-map.png"));
}
