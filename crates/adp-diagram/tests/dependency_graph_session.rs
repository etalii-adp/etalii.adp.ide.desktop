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
    assert!(!gateway.placed, "a dependency graph keeps its places in its body");
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
