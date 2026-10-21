//! What the module's registrations open (etalii.adp spec 030, task T061): every C4 type with a canvas opens its
//! registrations through the one module, over the one shared reading of the body, and selects in it.
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/ServiceCollectionAddC4.Tests.cs`
//! at `8b6ccfc`, whose dependency-injection registrations are here `adp_diagram_c4::register`, and whose reload seam
//! is `DiagramWorkspace::reconcile`.

use std::fs;
use std::path::Path;

use adp_core::Catalogue;
use adp_diagram::document::DiskChange;
use adp_diagram::{DiagramTools, DiagramWorkspace};
use adp_diagram_c4::store;
use adp_disl::DislPluginFunctions;

const TYPES: [&str; 6] = [
    "system-landscape",
    "context",
    "container",
    "component",
    "dynamic",
    "deployment",
];

#[test]
fn every_type_has_an_element_resolver_for_its_files() {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    let courier =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/examples/reference/architecture/courier.dsl");
    for kind in TYPES {
        let folder = tempfile::tempdir().unwrap();
        let root = folder.path().to_path_buf();
        let body = root.join("courier.dsl");
        fs::copy(&courier, &body).unwrap();
        fs::write(root.join("courier.adp"), format!("c4/{kind}\r\nbody: courier.dsl\r\n")).unwrap();
        let mut tools = adp_diagram_c4::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
            DislPluginFunctions::none()
        }));
        assert!(tools.opened_by_module(&format!("c4/{kind}")), "{kind}");
        let mut workspace = DiagramWorkspace::new(&root);
        let (opened, session) = workspace.open(&mut tools, Path::new("courier.adp")).unwrap();
        assert_eq!(opened.origin, format!("c4/{kind}"));
        assert_eq!(opened.body_path, body);
        assert!(session.select(workspace.documents(), "tracking").is_some(), "{kind}");
    }
}

#[test]
fn every_type_has_one_reload_seam_over_the_one_store() {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    let courier =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/examples/reference/architecture/courier.dsl");
    for kind in TYPES {
        let folder = tempfile::tempdir().unwrap();
        let root = folder.path().to_path_buf();
        let body = root.join("courier.dsl");
        fs::copy(&courier, &body).unwrap();
        fs::write(
            root.join("courier.adp"),
            format!(
                "c4/{kind}
body: courier.dsl
"
            ),
        )
        .unwrap();
        let mut tools = adp_diagram_c4::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
            DislPluginFunctions::none()
        }));
        let mut workspace = DiagramWorkspace::new(&root);
        workspace.open(&mut tools, Path::new("courier.adp")).unwrap();
        workspace.take_changed();
        assert!(
            !store::get(workspace.documents(), &body)
                .unwrap()
                .workspace
                .elements
                .is_empty()
        );
        fs::remove_file(&body).unwrap();

        // The deletion reaches the store the sessions read, and ends its model.
        assert!(matches!(workspace.reconcile(&body), DiskChange::Deleted(_)), "{kind}");
        assert_eq!(workspace.take_changed(), vec![body.clone()], "{kind}");
        assert!(
            store::get(workspace.documents(), &body)
                .unwrap()
                .workspace
                .elements
                .is_empty()
        );
    }
}
