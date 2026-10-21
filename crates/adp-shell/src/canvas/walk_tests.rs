//! The canvas as the Story 2 walk of the desktop host found it wanting (`parity/walks.md`, findings F1 to F14), each
//! held to what the standalone host at `8b6ccfc` does: `DiagramCanvas.tsx`, `MindmapCanvas.tsx`, `TimelineCanvas.tsx`
//! and `C4Canvas.tsx`.

use super::*;

/// A notch of a mouse wheel over `at`, turned away from the person when `up`.
fn wheel(at: Pos2, up: bool, modifiers: Modifiers) -> Vec<Event> {
    vec![
        Event::PointerMoved(at),
        Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, if up { 1.0 } else { -1.0 }),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers,
        },
    ]
}

#[test]
fn f1_the_plain_wheel_zooms_about_the_pointer_by_one_step_a_notch() {
    let mut harness = Harness::dependency_graph();
    let before = harness.tab.view;
    let at = harness.screen_of("api-gateway");
    let under = harness.tab.view.to_canvas(harness.tab.surface, at);
    harness.frame(wheel(at, true, Modifiers::NONE));
    assert!(
        (harness.tab.view.zoom - before.zoom * super::super::ZOOM_STEP).abs() < 1e-9,
        "one notch in is one step of 1.25: {} from {}",
        harness.tab.view.zoom,
        before.zoom
    );
    let still = harness.tab.view.to_canvas(harness.tab.surface, at);
    assert!(
        (still.x - under.x).abs() < 1e-6 && (still.y - under.y).abs() < 1e-6,
        "about the pointer"
    );
    harness.frame(wheel(at, false, Modifiers::NONE));
    harness.frame(wheel(at, false, Modifiers::NONE));
    assert!((harness.tab.view.zoom - before.zoom / super::super::ZOOM_STEP).abs() < 1e-9);
    // Ctrl changes nothing: the wheel zooms by the same step.
    harness.frame(wheel(at, true, Modifiers::COMMAND));
    assert!((harness.tab.view.zoom - before.zoom).abs() < 1e-9);
    assert_eq!(harness.written(), harness.original);
}

#[test]
fn f3_the_view_stays_fitted_as_the_surface_changes_until_the_person_changes_it() {
    let mut harness = Harness::dependency_graph();
    let fitted = harness.tab.view;
    // The window grows on its second frame, as a side pane that lays itself out does.
    harness.screen = egui::vec2(1600.0, 1000.0);
    harness.frame(Vec::new());
    harness.frame(Vec::new());
    assert_ne!(
        harness.tab.view, fitted,
        "the view is fitted again to the larger surface"
    );
    let surface = harness.tab.surface;
    for id in ["checkout-web", "payments-db", "notification-service"] {
        assert!(surface.contains(harness.screen_of(id)), "{id} is in view");
    }
    // Once zoomed, a resize keeps the view where the person put it.
    let at = harness.screen_of("api-gateway");
    harness.frame(wheel(at, true, Modifiers::NONE));
    let zoomed = harness.tab.view;
    harness.screen = SCREEN;
    harness.frame(Vec::new());
    assert_eq!(harness.tab.view, zoomed);
    // Fit to view follows the fit again.
    harness.key(Key::Num0, Modifiers::NONE);
    assert_eq!(harness.tab.view, fitted);
}

#[test]
fn f4_enter_in_a_field_of_the_property_grid_commits_the_field_and_never_reaches_the_canvas() {
    let mut harness = Harness::dependency_graph();
    harness.grid = true;
    harness.tab.ids = Some(Box::new(ids::fixed(&["leaked-below", "leaked-edge"])));
    harness.frame(Vec::new());
    let gateway = harness.screen_of("api-gateway");
    harness.click(gateway, Modifiers::NONE);
    harness.settle();
    let right_of_canvas = harness.tab.surface.max.x;
    let field = harness
        .texts
        .iter()
        .find(|(text, rect)| text == "API gateway" && rect.min.x > right_of_canvas)
        .map(|(_, rect)| rect.center())
        .expect("the grid, right of the canvas, shows the label in its field");
    harness.click(field, Modifiers::NONE);
    harness.frame(vec![
        Event::Key {
            key: Key::End,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        },
        Event::Text(" edge".into()),
    ]);
    harness.key(Key::Enter, Modifiers::NONE);
    harness.settle();
    let written = harness.written();
    assert!(written.contains("label: API gateway edge"), "{written}");
    assert!(
        !written.contains("leaked-below"),
        "the Enter added no node below: {written}"
    );
}

#[test]
fn f13_the_toolbar_undo_says_why_it_was_refused_and_a_later_success_takes_it_away() {
    let mut harness = Harness::dependency_graph();
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.select(vec!["api-gateway".into()], adp_context::SelectMode::Replace);
    let label = crate::properties::rows(tab, &host.workspaces[root])
        .into_iter()
        .find(|r| r.label == "Label")
        .unwrap()
        .id;
    crate::properties::set(tab, host.workspace(root), &label, "Front door");
    // The body is changed by another program: undo stops there.
    let changed = harness
        .written()
        .replace("label: Orders database", "label: Order store");
    fs::write(&harness.body, &changed).unwrap();
    harness.host.reconcile(&harness.body);
    harness.tab.changed();
    harness.tab.cleared = false;
    harness.settle();
    let undo = harness.text("Undo").expect("the toolbar's Undo");
    harness.click(undo, Modifiers::NONE);
    let refusal = harness
        .tab
        .notice
        .clone()
        .unwrap_or_else(|| panic!("the refusal is said: {:?}", harness.written() == changed));
    assert!(refusal.contains("changed outside ADP"), "{refusal}");
    assert!(
        harness.ctx.has_requested_repaint(),
        "a frame is asked for, so the status bar shows it"
    );
    assert_eq!(harness.written(), changed, "and nothing is undone");

    // A later edit that goes through takes the refusal away.
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    crate::properties::set(tab, host.workspace(root), &label, "Main door");
    assert!(harness.tab.notice.is_none());
    assert!(harness.tab.cleared, "and says so to the status bar");
}
