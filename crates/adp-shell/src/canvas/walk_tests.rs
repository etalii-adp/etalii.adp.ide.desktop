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

// ---- the mind map ----------------------------------------------------------------------------------------------------

const CENTRAL_TOPIC: &str = "ID_5eb2r180uk2p4ys1q0tkiwil6";
const PERSISTENCE: &str = "ID_51ljf5yl355i4knxrpnzmbiar";
const OVERFLOW: &str = "ID_0jkyc0uyotnmbcen585kev647";
const CONSTRAINTS: &str = "ID_43s3dbv7dz34umql6dwza7lwb";
const INPUT_HANDLING: &str = "ID_1z8lmh0t5m59d4qdgnded2jtd";
const CONSTRAINT_LEAVES: [&str; 5] = [
    "ID_7o7qnujtgpeb9b1eqe9xokzgn",
    "ID_ez1aq7z5vdbb6be63bx0l45pf",
    "ID_dc4nmzrb52ldb33c5jl8fl8hy",
    "ID_70c25zb7brn5jb4yhf7q5kk1q",
    "ID_ac6qkn600eah6ncfjkp1fppvb",
];

impl Harness {
    /// The parity corpus's mind map, by its body, as the walk opened it.
    fn mindmap() -> Harness {
        let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/fixtures/freeplane/mindmap/example-1");
        Harness::open_from(
            &folder,
            &["mindmap.mm", "mindmap.adp", "design.mm"],
            "mindmap.mm",
            "mindmap.mm",
        )
    }

    /// The map as it is on disk.
    fn map(&self) -> adp_diagram_mindmap::document::MindmapDocument {
        adp_diagram_mindmap::document::MindmapDocument::parse(&self.written(), false).unwrap()
    }

    /// The texts of the children of `id`, as the map on disk has them.
    fn children_of(&self, id: &str) -> Vec<String> {
        let map = self.map();
        let node = map.find(id).unwrap_or_else(|| panic!("{id} is in the map"));
        map.children(node).into_iter().map(|c| map.text(c)).collect()
    }

    fn registration(&self) -> String {
        fs::read_to_string(self.root.join("mindmap.adp")).unwrap()
    }
}

#[test]
fn f2_a_node_dragged_onto_another_becomes_its_last_child_and_no_position_is_written() {
    let mut harness = Harness::mindmap();
    let registration = harness.registration();
    let from = harness.screen_of(OVERFLOW);
    let to = harness.screen_of(PERSISTENCE);
    harness.drag(from, to, PointerButton::Primary, true);
    assert_eq!(
        harness.children_of(PERSISTENCE).last().map(String::as_str),
        Some("Overflow handling"),
        "{:?}",
        harness.tab.notice
    );
    assert!(
        !harness
            .children_of(CONSTRAINTS)
            .contains(&"Overflow handling".to_owned())
    );
    assert_eq!(harness.registration(), registration, "the drag writes no position");
    assert!(harness.workspace().undo().is_done());
    assert_eq!(harness.written(), harness.original, "one undo puts the bytes back");

    // Dropped on empty canvas, or on its own place, the drag simply ends.
    harness.frame(Vec::new());
    let from = harness.screen_of(OVERFLOW);
    harness.drag(from, from + egui::vec2(0.0, 2000.0), PointerButton::Primary, true);
    harness.drag(from, from + egui::vec2(6.0, 6.0), PointerButton::Primary, true);
    assert_eq!(harness.written(), harness.original);
    assert_eq!(harness.registration(), registration);
}

#[test]
fn f7_the_toolbox_node_dropped_on_a_node_adds_a_child_under_it() {
    let mut harness = Harness::mindmap();
    harness.tab.ids = Some(Box::new(ids::fixed(&["ID_dropped"])));
    let node = harness
        .tab
        .session
        .toolbox(harness.host.workspaces[&harness.root].documents())
        .first()
        .expect("the toolbox has Node")
        .id
        .clone();
    let before = harness.children_of(PERSISTENCE).len();
    let at = harness.tab.bounds_of(PERSISTENCE).unwrap().centre();
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.drop_tool(host.workspace(root), &node, at);
    assert_eq!(
        harness.children_of(PERSISTENCE).len(),
        before + 1,
        "{:?}",
        harness.tab.notice
    );
    assert_eq!(harness.tab.selection, ["ID_dropped"], "what the drop made is selected");
    assert_eq!(
        harness.tab.editor.as_ref().map(|e| e.element.as_str()),
        Some("ID_dropped"),
        "and its name is asked in place"
    );

    // Over empty canvas there is no node, and nothing happens.
    harness.tab.editor = None;
    let written = harness.written();
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.drop_tool(host.workspace(root), &node, Point::new(-5000.0, -5000.0));
    assert_eq!(harness.written(), written);
}

#[test]
fn f10_f11_a_node_added_by_key_is_drawn_at_once_selected_and_named_in_place_over_itself() {
    let mut harness = Harness::mindmap();
    harness.tab.ids = Some(Box::new(ids::fixed(&["ID_walkchild"])));
    let at = harness.screen_of(INPUT_HANDLING);
    harness.click(at, Modifiers::NONE);
    harness.key(Key::Insert, Modifiers::NONE);
    assert!(
        harness.map().find("ID_walkchild").is_some(),
        "Insert adds a child: {:?}",
        harness.tab.notice
    );
    let drawn = harness
        .tab
        .bounds_of("ID_walkchild")
        .expect("the new child is drawn in the frame that added it");
    assert_eq!(harness.tab.selection, ["ID_walkchild"], "and selected");
    let editor = harness.tab.editor.clone().expect("its name is asked in place");
    assert_eq!(editor.element, "ID_walkchild");
    assert!(
        drawn.contains(editor.area.centre()),
        "over the new child, not over its parent: {:?} in {drawn:?}",
        editor.area
    );
    harness.frame(Vec::new());
    harness.frame(vec![Event::Text(" walked".into())]);
    harness.key(Key::Enter, Modifiers::NONE);
    let map = harness.map();
    let text = map.text(map.find("ID_walkchild").unwrap());
    assert!(text.ends_with(" walked"), "the name typed is written: {text}");
}

#[test]
fn f12_space_and_the_menu_collapse_and_expand_a_branch_for_this_viewer_only() {
    let mut harness = Harness::mindmap();
    let target = adp_diagram::ActionTarget::Element(CONSTRAINTS.into());
    let labels = |harness: &Harness| -> Vec<String> {
        harness
            .tab
            .session
            .actions(harness.host.workspaces[&harness.root].documents(), &target)
            .into_iter()
            .flat_map(|g| g.actions)
            .map(|a| a.label)
            .collect()
    };
    assert!(
        labels(&harness).contains(&"Collapse".to_owned()),
        "{:?}",
        labels(&harness)
    );
    let at = harness.screen_of(CONSTRAINTS);
    harness.click(at, Modifiers::NONE);
    harness.key(Key::Space, Modifiers::NONE);
    assert!(
        harness.tab.bounds_of(CONSTRAINT_LEAVES[0]).is_none(),
        "Space collapses the branch: {:?}",
        harness.tab.notice
    );
    assert!(
        labels(&harness).contains(&"Expand".to_owned()),
        "{:?}",
        labels(&harness)
    );
    harness.key(Key::Space, Modifiers::NONE);
    assert!(harness.tab.bounds_of(CONSTRAINT_LEAVES[0]).is_some(), "and expands it");
    assert_eq!(harness.written(), harness.original, "folding writes nothing");
    assert!(
        !harness.workspace().history().availability().can_undo,
        "nor lands on the history"
    );
}

#[test]
fn f14_delete_with_several_selected_deletes_all_of_them_as_one_undo_or_none() {
    let mut harness = Harness::mindmap();
    let at = harness.screen_of(CONSTRAINT_LEAVES[0]);
    harness.click(at, Modifiers::NONE);
    harness.tab.select(
        CONSTRAINT_LEAVES.iter().map(|s| (*s).to_owned()).collect(),
        adp_context::SelectMode::Replace,
    );
    harness.key(Key::Delete, Modifiers::NONE);
    let map = harness.map();
    for leaf in CONSTRAINT_LEAVES {
        assert!(map.find(leaf).is_none(), "{leaf} is deleted: {:?}", harness.tab.notice);
    }
    assert!(harness.workspace().undo().is_done());
    assert_eq!(harness.written(), harness.original, "one undo puts all five back");

    // A selection with a branch in it asks first, then deletes them all, the branch's leaves with it.
    harness.tab.changed();
    harness.frame(Vec::new());
    harness.tab.select(
        vec![CONSTRAINTS.into(), CONSTRAINT_LEAVES[0].into(), PERSISTENCE.into()],
        adp_context::SelectMode::Replace,
    );
    harness.key(Key::Delete, Modifiers::NONE);
    assert!(harness.tab.prompt.is_some(), "a branch is confirmed first");
    assert_eq!(harness.written(), harness.original, "and nothing goes before it is");
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.answer(host.workspace(root), Some(""));
    let map = harness.map();
    assert!(map.find(CONSTRAINTS).is_none() && map.find(PERSISTENCE).is_none());
    assert!(harness.workspace().undo().is_done());
    assert_eq!(harness.written(), harness.original, "one undo puts every one back");

    // With the central topic among them, none is deleted.
    harness.tab.changed();
    harness.frame(Vec::new());
    harness.tab.select(
        vec![CONSTRAINT_LEAVES[0].into(), CENTRAL_TOPIC.into()],
        adp_context::SelectMode::Replace,
    );
    harness.key(Key::Delete, Modifiers::NONE);
    assert_eq!(harness.written(), harness.original, "{:?}", harness.tab.notice);
}
