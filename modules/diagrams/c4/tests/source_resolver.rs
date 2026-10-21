//! What makes a C4 element or relationship selectable, and what a selection of it reads as (etalii.adp spec 030, task
//! T061).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/`
//! `C4ContextSourceResolver.Tests.cs` and the parts of the shared `EtAlii.Adp.TestSupport/DiagramElementSourceResolverContract.cs`
//! that are a module's, on the courier example selected through a registration of the context type. In the desktop
//! host a selection is `DiagramSession::select`. Not ported, as the context service's rather than the module's: a
//! canvas point or a connect gesture named by an id (`new:x,y`, a gesture id), which the desktop host passes as an
//! `ActionTarget`; a client path, a parent level or a file of another type that does not match; a tracked element
//! following a rename; a payload's bytes and `HasChildren`, which a desktop selection does not carry.

use std::fs;
use std::path::{Path, PathBuf};

use adp_core::Catalogue;
use adp_diagram::{DiagramSession, DiagramTools, DiagramViewport, DiagramWorkspace};
use adp_diagram_c4::mapper::{self, NODE_TYPE, RELATIONSHIP_TYPE};
use adp_diagram_c4::metrics::C4Metrics;
use adp_diagram_c4::parser;
use adp_disl::DislPluginFunctions;

fn courier() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/examples/reference/architecture/courier.dsl")
}

struct Project {
    _folder: tempfile::TempDir,
    body: PathBuf,
    workspace: DiagramWorkspace,
    session: DiagramSession,
}

/// The courier model, selected in a registration of the context type beside it.
fn open() -> Project {
    let folder = tempfile::tempdir().unwrap();
    let root = folder.path().to_path_buf();
    let body = root.join("courier.dsl");
    fs::copy(courier(), &body).unwrap();
    fs::write(root.join("courier.adp"), "c4/context\r\nbody: courier.dsl\r\n").unwrap();
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    let mut tools = adp_diagram_c4::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
        DislPluginFunctions::none()
    }));
    let mut workspace = DiagramWorkspace::new(&root);
    let (_, session) = workspace.open(&mut tools, Path::new("courier.adp")).unwrap();
    Project {
        _folder: folder,
        body,
        workspace,
        session,
    }
}

#[test]
fn an_element_resolves_to_what_it_shows() {
    let project = open();
    let selected = project
        .session
        .select(project.workspace.documents(), "tracking")
        .unwrap();
    assert_eq!(selected.path, vec!["Courier Tracking"]);
    assert_eq!(selected.text, "Courier Tracking");
    assert_eq!(selected.element.element_type, NODE_TYPE);
}

#[test]
fn an_unknown_id_is_refused_as_gone() {
    let project = open();
    assert!(
        project
            .session
            .select(project.workspace.documents(), "nothing-here")
            .is_none()
    );
}

#[test]
fn a_line_only_a_view_elevates_is_selectable_as_the_canvas_drew_it() {
    let project = open();
    let workspace = parser::parse_text(&fs::read_to_string(&project.body).unwrap());
    let model_ids: Vec<String> = workspace.relationships.iter().map(|r| r.id()).collect();
    let elevated = workspace
        .views
        .iter()
        .flat_map(|view| {
            mapper::visible(
                &workspace,
                view,
                DiagramViewport::UNBOUNDED,
                &C4Metrics::default(),
                None,
            )
        })
        .find(|e| e.element_type == RELATIONSHIP_TYPE && !model_ids.contains(&e.id))
        .expect("some view elevates a line");
    let selected = project
        .session
        .select(project.workspace.documents(), &elevated.id)
        .unwrap();
    assert!(selected.text.contains(" -> "), "{}", selected.text);
}
