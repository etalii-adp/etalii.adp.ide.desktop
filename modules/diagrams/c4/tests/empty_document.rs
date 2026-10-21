//! The empty document each C4 type starts from: not an empty file, since every view but the landscape is a view of
//! something the document has to carry (etalii.adp spec 030, task T061).
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/C4EmptyDocument.Tests.cs`
//! at `8b6ccfc`: a workspace with a model and views for every kind, balanced braces, each kind's own view and scope,
//! a file name made a DSL identifier and a quote escaped. The standalone host's `c4/code` refused through its empty
//! document factory, a dependency injection registration; here its registration is refused when it is opened, with
//! the same sentence.

use std::fs;
use std::path::Path;

use adp_core::Catalogue;
use adp_diagram::{DiagramTools, DiagramWorkspace};
use adp_diagram_c4::model::C4ViewKind;
use adp_diagram_c4::toolbox::{self, CODE_UNAVAILABLE};
use adp_disl::DislPluginFunctions;

const EVERY_VIEW_KIND: [C4ViewKind; 6] = [
    C4ViewKind::SystemLandscape,
    C4ViewKind::SystemContext,
    C4ViewKind::Container,
    C4ViewKind::Component,
    C4ViewKind::Dynamic,
    C4ViewKind::Deployment,
];

fn create(kind: C4ViewKind) -> String {
    toolbox::empty_document(kind, "payments")
}

#[test]
fn every_kind_produces_a_workspace_with_a_model_and_views() {
    for kind in EVERY_VIEW_KIND {
        let document = create(kind);
        assert!(document.starts_with("workspace \"payments\" {"), "{kind:?}");
        assert!(document.contains("    model {"), "{kind:?}");
        assert!(document.contains("    views {"), "{kind:?}");
        assert!(document.ends_with("}\n"), "{kind:?}");
    }
}

#[test]
fn every_kind_produces_balanced_braces() {
    for kind in EVERY_VIEW_KIND {
        let document = create(kind);
        assert_eq!(document.matches('{').count(), document.matches('}').count(), "{kind:?}");
    }
}

#[test]
fn each_kind_declares_its_own_view_scoped_to_what_that_kind_requires() {
    let cases = [
        (C4ViewKind::SystemLandscape, "systemLandscape \"landscape\""),
        (C4ViewKind::SystemContext, "systemContext payments \"context\""),
        (C4ViewKind::Container, "container payments \"containers\""),
        (C4ViewKind::Component, "component application \"components\""),
        (C4ViewKind::Dynamic, "dynamic payments \"scenario\""),
        (
            C4ViewKind::Deployment,
            "deployment payments \"Production\" \"deployment\"",
        ),
    ];
    for (kind, expected) in cases {
        assert!(create(kind).contains(expected), "{kind:?}: {expected}");
    }
}

#[test]
fn a_landscape_has_no_focal_system_which_is_what_distinguishes_it_from_a_context_diagram() {
    assert!(!create(C4ViewKind::SystemLandscape).contains("systemLandscape payments"));
}

#[test]
fn a_component_view_reaches_one_level_deeper_because_it_is_scoped_to_a_container() {
    let document = create(C4ViewKind::Component);
    assert!(document.contains("container \"Application\""));
    assert!(document.contains("\"Technology\""));
}

#[test]
fn a_deployment_view_gets_an_environment_to_bind_to() {
    assert!(create(C4ViewKind::Deployment).contains("deploymentEnvironment \"Production\""));
}

#[test]
fn a_dynamic_view_starts_with_no_interactions_rather_than_including_everything() {
    let document = create(C4ViewKind::Dynamic);
    assert!(document.contains("dynamic payments \"scenario\""));
    assert!(!document.contains("include *"));
}

#[test]
fn a_file_name_becomes_a_valid_dsl_identifier() {
    let cases = [
        ("my map", "my_map"),
        ("2024-review", "_2024_review"),
        ("payments!", "payments"),
        ("", "system"),
        ("   ", "system"),
        ("***", "system"),
    ];
    for (base_name, expected) in cases {
        let document = toolbox::empty_document(C4ViewKind::SystemContext, base_name);
        assert!(
            document.contains(&format!("        {expected} = softwareSystem ")),
            "{base_name:?}: {document}"
        );
    }
}

#[test]
fn a_name_with_a_quote_is_escaped_rather_than_breaking_the_document() {
    let document = toolbox::empty_document(C4ViewKind::SystemContext, "the \"good\" one");
    assert!(document.contains("\\\"good\\\""), "{document}");
}

#[test]
fn the_code_type_refuses_and_says_why() {
    let folder = tempfile::tempdir().unwrap();
    fs::write(folder.path().join("code-level.adp"), "c4/code\n").unwrap();
    fs::write(folder.path().join("code-level.dsl"), "workspace {\n}\n").unwrap();
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    let mut tools = adp_diagram_c4::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
        DislPluginFunctions::none()
    }));
    let mut workspace = DiagramWorkspace::new(folder.path());
    let refusal = workspace
        .open(&mut tools, Path::new("code-level.adp"))
        .err()
        .expect("c4/code opens nothing");
    assert_eq!(refusal, CODE_UNAVAILABLE);
}
