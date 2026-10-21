//! A second DISL and FBL tool type through the same session (etalii.adp spec 030, Story 2; tasks T056 and T057): the
//! standalone host's dependency graph example, opened from its body with the registration beside it, its place kept
//! in the body rather than the registration, edited from the property grid and from the canvas's context menu, and
//! undone, with every write checked byte for byte.
//!
//! The example is `src/diagrams/dependency-graph/examples/example-1/` of etalii.adp.ide.standalone at `8b6ccfc`,
//! copied unchanged to `tests/fixtures/dependency-graph/example-1/`.

use std::fs;
use std::path::{Path, PathBuf};

use adp_core::Catalogue;
use adp_diagram::session::{ActionOutcome, ActionTarget};
use adp_diagram::tool::DiagramTools;
use adp_diagram::workspace::DiagramWorkspace;
use adp_diagram::{DiagramDelta, ElementKind};
use adp_disl::DislPluginFunctions;
use adp_disl::behavior::ids;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dependency-graph/example-1")
}

fn tools() -> DiagramTools {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor");
    DiagramTools::new(Catalogue::load(&vendor), |_| DislPluginFunctions::none())
}

#[test]
fn a_body_opens_as_the_tool_type_of_its_extension_and_is_edited_in_its_own_lines() {
    let folder = tempfile::tempdir().unwrap();
    for name in ["services.adp", "services.dgr"] {
        fs::copy(fixtures().join(name), folder.path().join(name)).unwrap();
    }
    let body = folder.path().join("services.dgr");
    let original = fs::read_to_string(&body).unwrap();
    let mut tools = tools();
    let mut workspace = DiagramWorkspace::new(folder.path());

    let (opened, mut session) = workspace.open(&mut tools, Path::new("services.dgr")).unwrap();
    assert_eq!(opened.origin, "generic/dependencies");
    assert_eq!(opened.registration_path, Some(folder.path().join("services.adp")));

    let baseline = session.baseline(workspace.documents()).unwrap();
    let [DiagramDelta::Add(elements)] = baseline.as_slice() else {
        panic!("a baseline is one add, not {baseline:?}");
    };
    let gateway = elements.iter().find(|e| e.id == "api-gateway").expect("the gateway");
    assert_eq!(gateway.kind, ElementKind::Node);
    assert_eq!(gateway.attribute("label"), "API gateway");
    assert_eq!(
        (gateway.x, gateway.y, gateway.placed),
        (270.42873351852575, 4.0, true),
        "a dependency graph keeps its places in its body, by x and row"
    );
    assert!(elements.iter().any(|e| e.kind == ElementKind::Relation));
    assert!(!session.toolbox(workspace.documents()).is_empty());

    // The property grid writes the node's own line.
    let rows = session.properties(workspace.documents(), &["api-gateway"]);
    let label = rows.iter().find(|r| r.label == "Label").expect("a label row");
    session
        .set_property(&mut workspace, &["api-gateway"], &label.id, "Edge gateway")
        .unwrap();
    let renamed = original.replace("label: API gateway", "label: Edge gateway");
    assert_ne!(renamed, original);
    assert_eq!(fs::read_to_string(&body).unwrap(), renamed);

    // The canvas's menu adds a node where it was opened, under the id it is given.
    let canvas = ActionTarget::Canvas { x: 40.0, y: 3.0 };
    let add = session
        .actions(workspace.documents(), &canvas)
        .into_iter()
        .flat_map(|g| g.actions)
        .find(|a| a.id == "dependencies.add-element")
        .expect("the canvas offers a new node");
    assert_eq!(
        session.execute(&mut workspace, &canvas, &add.id, &ids::fixed(&["new-node"])),
        ActionOutcome::Done
    );
    let added = fs::read_to_string(&body).unwrap();
    let inserted = renamed.replacen(
        "    row: -4\r\nrelations:",
        "    row: -4\r\n  - id: new-node\r\n    label: New node\r\n    x: 40\r\n    row: 3\r\nrelations:",
        1,
    );
    assert_ne!(inserted, renamed);
    assert_eq!(added, inserted);
    workspace.take_changed();
    let change = session.on_document_changed(workspace.documents(), &body);
    assert!(matches!(change, adp_diagram::DocumentChange::Changed(_)), "{change:?}");

    // Undo, one edit at a time, back to the bytes it opened with.
    assert!(workspace.undo().is_done());
    assert_eq!(fs::read_to_string(&body).unwrap(), renamed);
    assert!(workspace.undo().is_done());
    assert_eq!(fs::read_to_string(&body).unwrap(), original);
}

#[test]
fn a_drag_of_two_nodes_a_connect_and_an_inline_rename_are_one_undo_each() {
    let folder = tempfile::tempdir().unwrap();
    for name in ["services.adp", "services.dgr"] {
        fs::copy(fixtures().join(name), folder.path().join(name)).unwrap();
    }
    let body = folder.path().join("services.dgr");
    let registration = folder.path().join("services.adp");
    let original = fs::read_to_string(&body).unwrap();
    let original_registration = fs::read_to_string(&registration).unwrap();
    let mut tools = tools();
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (_, mut session) = workspace.open(&mut tools, Path::new("services.adp")).unwrap();
    session.baseline(workspace.documents()).unwrap();

    // Two nodes dragged together: their x and row in the body, and the registration untouched.
    session
        .move_elements(
            &mut workspace,
            &[("api-gateway".into(), 300.0, 3.0), ("orders-db".into(), 1100.5, 1.0)],
        )
        .unwrap();
    let dragged = original
        .replace("x: 270.42873351852575\r\n    row: 4", "x: 300\r\n    row: 3")
        .replace("x: 1080\r\n    row: 2", "x: 1100.5\r\n    row: 1");
    assert_eq!(fs::read_to_string(&body).unwrap(), dragged);
    assert_eq!(fs::read_to_string(&registration).unwrap(), original_registration);
    let moved = workspace.documents().get(&body).unwrap().positions();
    assert_eq!(moved["api-gateway"], (300.0, 3.0));

    // A connect gesture, under the id it is given.
    session
        .connect(
            &mut workspace,
            "DependsOn",
            "checkout-web",
            "orders-db",
            &ids::fixed(&["r-web-db"]),
        )
        .unwrap();
    let connected = fs::read_to_string(&body).unwrap();
    assert!(connected.starts_with(&dragged));
    assert_eq!(
        &connected[dragged.len()..],
        "  - id: r-web-db\r\n    from: checkout-web\r\n    to: orders-db\r\n"
    );

    // An inline rename, asked for and answered.
    let node = ActionTarget::Element("checkout-web".into());
    let rename = session
        .actions(workspace.documents(), &node)
        .into_iter()
        .flat_map(|g| g.actions)
        .find(|a| a.id == "dependencies.rename")
        .expect("a node can be renamed");
    let ActionOutcome::Prompt(adp_context::Prompt::Input { initial_value, .. }) =
        session.execute(&mut workspace, &node, &rename.id, &ids::fixed(&[]))
    else {
        panic!("a rename asks for the new label");
    };
    assert_eq!(initial_value, "Checkout web");
    assert_eq!(
        session.answer(&mut workspace, &node, &rename.id, "Storefront"),
        ActionOutcome::Done
    );
    let renamed = connected.replace("label: Checkout web", "label: Storefront");
    assert_eq!(fs::read_to_string(&body).unwrap(), renamed);

    for expected in [&connected, &dragged, &original] {
        assert!(workspace.undo().is_done());
        assert_eq!(&fs::read_to_string(&body).unwrap(), expected);
    }
}

#[test]
fn a_connect_gesture_draws_the_relation_types_whose_ends_take_both_nodes() {
    let folder = tempfile::tempdir().unwrap();
    for name in ["services.adp", "services.dgr"] {
        fs::copy(fixtures().join(name), folder.path().join(name)).unwrap();
    }
    let mut tools = tools();
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (_, session) = workspace.open(&mut tools, Path::new("services.dgr")).unwrap();
    let documents = workspace.documents();
    assert_eq!(
        session.relation_types_between(documents, "checkout-web", Some("orders-db")),
        ["DependsOn"]
    );
    assert_eq!(
        session.relation_types_between(documents, "checkout-web", None),
        ["DependsOn"]
    );
    assert!(
        session
            .relation_types_between(documents, "checkout-web", Some("no-such-node"))
            .is_empty()
    );
    // A line released on empty canvas makes its target with the toolbox's node, and the line to it, as one undo.
    let body = folder.path().join("services.dgr");
    let original = fs::read_to_string(&body).unwrap();
    session
        .connect_to_new(
            &mut workspace,
            "DependsOn",
            "checkout-web",
            40.0,
            3.0,
            &ids::fixed(&["made", "to-made"]),
        )
        .unwrap();
    let written = fs::read_to_string(&body).unwrap();
    assert!(written.contains("  - id: made\r\n"), "{written}");
    assert!(written.contains("from: checkout-web\r\n    to: made"), "{written}");
    assert!(workspace.undo().is_done());
    assert_eq!(fs::read_to_string(&body).unwrap(), original);
}
