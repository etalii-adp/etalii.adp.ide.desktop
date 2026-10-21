//! A functional decomposition graph on the canvas with its module (etalii.adp spec 030, task T072): the standalone
//! host's field-service example, opened through its registration from the parity fixtures, drawn from what the module
//! hands over, selected, dragged by its top-left, connected by a drag from an element's anchor to another element,
//! given a new element from the palette, renamed through its prompt and edited in its rows, and every edit undone
//! through the project's history.

use std::path::Path;

use adp_diagram::ActionTarget;
use adp_scene::Point;
use adp_scene::scene::{Primitive, Role};
use eframe::egui::{self, Modifiers, PointerButton};

use super::{Harness, texts_of};

fn field_service() -> Harness {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../parity/fixtures/etalii/functional-decomposition-graph/field-service");
    Harness::open_from(
        &folder,
        &["field-service.adp", "field-service.fdg"],
        "field-service.adp",
        "field-service.fdg",
    )
}

fn model(harness: &Harness) -> adp_diagram_functional_decomposition_graph::model::FdgModel {
    adp_diagram_functional_decomposition_graph::parser::parse(
        &adp_diagram_functional_decomposition_graph::lines::LineDocument::parse(&harness.written()),
    )
}

#[test]
fn a_functional_decomposition_graph_opens_draws_and_is_edited_on_the_canvas() {
    let mut harness = field_service();
    assert!(harness.tab.session.module_view().is_some());
    harness.settle();

    // What the module hands over: every element in its shape, every name and comment line, every connection's name.
    let scene = harness.tab.scene().expect("the graph is drawn");
    let bodies = scene
        .primitives
        .iter()
        .filter(|p| matches!(p, Primitive::Shape(s) if s.role == Role::Body))
        .count();
    assert_eq!(bodies, 19, "every element has its body");
    let edges = scene
        .primitives
        .iter()
        .filter(|p| matches!(p, Primitive::Edge(_)))
        .count();
    assert_eq!(edges, 18, "every connection has its curve");
    let texts = texts_of(scene);
    for expected in [
        "Planning",
        "Task detail",
        "Everything on these screens must",
        "work offline.",
        "opens",
        "one per task",
    ] {
        assert!(texts.contains(&expected), "{expected} is drawn: {texts:?}");
    }
    assert!(harness.text("Planning").is_some(), "the name is painted");
    // Each element in the box the module gave it: Planning's top-left (600, 40), 140 by 48.
    let planning = harness.tab.bounds_of("planning").unwrap();
    assert_eq!(
        (planning.x, planning.y, planning.width, planning.height),
        (600.0, 40.0, 140.0, 48.0)
    );

    // A click selects, and the rows are the module's.
    let at = harness.screen_of("planning");
    harness.click(at, Modifiers::NONE);
    assert_eq!(harness.tab.selection, vec!["planning".to_owned()]);
    let rows = harness
        .tab
        .session
        .properties(harness.host.workspace(&harness.root).documents(), &["planning"]);
    assert_eq!(rows[0].id, "fdg.name");
    assert_eq!(rows[0].value, "Planning");

    // A drag writes the new top-left, one undo away.
    let zoom = harness.tab.view.zoom;
    harness.drag(at, at + egui::vec2(60.0, 30.0), PointerButton::Primary, true);
    let moved = model(&harness).element_of("planning").unwrap().clone();
    assert!(
        (moved.x - (600.0 + 60.0 / zoom)).abs() < 0.01,
        "{} at zoom {zoom}",
        moved.x
    );
    assert!(
        (moved.y - (40.0 + 30.0 / zoom)).abs() < 0.01,
        "{} at zoom {zoom}",
        moved.y
    );
    assert!(harness.workspace().undo().is_done());
    assert_eq!(harness.written(), harness.original);
    harness.tab.changed();
    harness.settle();

    // A drag from Tick step's anchor onto the Step list draws the one relation the pair admits: Shows.
    let tick = harness.tab.bounds_of("tick-step").unwrap();
    let anchor = harness
        .tab
        .view
        .to_screen(harness.tab.surface, Point::new(tick.right(), tick.centre().y));
    let step_list = harness.screen_of("step-list");
    harness.drag(anchor, step_list, PointerButton::Primary, true);
    let drawn = model(&harness);
    let shows: Vec<_> = drawn
        .connections
        .iter()
        .filter(|c| c.from == "tick-step" && c.to == "step-list")
        .collect();
    assert_eq!(shows.len(), 1, "{:?}", harness.tab.notice);
    assert_eq!(shows[0].connection_type, "shows");
    assert_eq!(shows[0].id.len(), 25, "a ShortGuid");
    assert!(harness.workspace().undo().is_done());
    assert_eq!(harness.written(), harness.original);

    // A drop from the palette adds its type centred where it landed, named as the next of its kind.
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.drop_tool(host.workspace(root), "fdg.toolbox.function", Point::new(1300.0, 700.0));
    let added = model(&harness);
    let function = added.elements.last().unwrap();
    assert_eq!(function.name, "New function");
    assert_eq!((function.x, function.y), (1220.0, 676.0));
    assert!(harness.workspace().undo().is_done());

    // A rename through its prompt, and a row in the grid, each one edit.
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.run_action(
        host.workspace(root),
        ActionTarget::Element("task-list".to_owned()),
        "fdg.rename",
    );
    // The name is edited in place, over the element's label, from what it is.
    let editor = harness.tab.editor.as_mut().expect("a rename opens the inline editor");
    assert_eq!(editor.element, "task-list");
    assert_eq!(editor.text, "Task list");
    editor.text = "Today".to_owned();
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.commit_editor(host.workspace(root));
    assert!(harness.written().contains("    name: Today\n"));
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.session
        .set_property(host.workspace(root), &["note-offline"], "fdg.height", "120")
        .unwrap();
    assert!(harness.written().contains("    height: 120\n"));
    harness.tab.changed();
    harness.settle();
    assert!(texts_of(harness.tab.scene().unwrap()).contains(&"Today"));
    assert_eq!(harness.tab.bounds_of("note-offline").unwrap().height, 120.0);

    assert!(harness.workspace().undo().is_done());
    assert!(harness.workspace().undo().is_done());
    assert_eq!(harness.written(), harness.original);
}
