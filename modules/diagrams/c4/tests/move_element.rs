//! Dragging an element on a C4 diagram: it places the element in the view's layout sidecar and never re-parents it
//! (etalii.adp spec 030, task T061).
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/C4MoveElement.Tests.cs` at
//! `8b6ccfc`: the position recorded beside the document and the document left alone, a drag one undo away (the first
//! one handing the element back to the layout), the views told, the next layout honouring the drop, and a re-parent
//! refused for what it is, whatever its argument looks like.

use std::fs;
use std::path::{Path, PathBuf};

use adp_core::Catalogue;
use adp_diagram::{DiagramSession, DiagramTools, DiagramWorkspace};
use adp_diagram_c4::metrics::C4Metrics;
use adp_diagram_c4::{layout, sidecar, store};
use adp_disl::DislPluginFunctions;

const MODEL: &str = "workspace \"Bank\" {
    model {
        a = softwareSystem \"A\" \"desc\"
        b = softwareSystem \"B\" \"desc\"
        a -> b \"Calls\" \"HTTPS\"
    }
    views {
        systemLandscape \"all\" {
            include *
        }
    }
}";

struct Project {
    _folder: tempfile::TempDir,
    body: PathBuf,
    workspace: DiagramWorkspace,
    session: DiagramSession,
}

impl Project {
    /// The body opened on its own, as the standalone test's landscape session opened it: it shows the first view.
    fn new() -> Project {
        let folder = tempfile::tempdir().unwrap();
        fs::write(folder.path().join("model.dsl"), MODEL).unwrap();
        let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
        let mut tools = adp_diagram_c4::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
            DislPluginFunctions::none()
        }));
        let mut workspace = DiagramWorkspace::new(folder.path());
        let (opened, session) = workspace.open(&mut tools, Path::new("model.dsl")).unwrap();
        Project {
            _folder: folder,
            body: opened.body_path,
            workspace,
            session,
        }
    }

    fn move_to(&mut self, id: &str, x: f64, y: f64) -> Result<(), String> {
        self.session
            .move_elements(&mut self.workspace, &[(id.to_owned(), x, y)])
    }

    fn authored(&self) -> sidecar::Positions {
        sidecar::read(&self.body, "all")
    }
}

#[test]
fn a_drag_records_the_position_in_the_sidecar_and_leaves_the_document_alone() {
    let mut project = Project::new();
    let before = fs::read_to_string(&project.body).unwrap();
    project.move_to("a", 250.0, 400.0).unwrap();
    assert_eq!(project.authored().get("a"), Some((250.0, 400.0)));
    assert_eq!(fs::read_to_string(&project.body).unwrap(), before);
}

#[test]
fn a_drag_is_one_undo_away() {
    let mut project = Project::new();
    project.move_to("a", 250.0, 400.0).unwrap();
    assert!(project.workspace.undo().is_done());
    assert!(!project.authored().keys().any(|k| k == "a"));
}

#[test]
fn undoing_a_second_drag_restores_the_first_position() {
    let mut project = Project::new();
    project.move_to("a", 100.0, 100.0).unwrap();
    project.move_to("a", 250.0, 400.0).unwrap();
    assert!(project.workspace.undo().is_done());
    assert_eq!(project.authored().get("a"), Some((100.0, 100.0)));
}

#[test]
fn a_drag_tells_the_sessions_so_the_other_tab_redraws() {
    let mut project = Project::new();
    project.workspace.take_changed();
    project.move_to("a", 250.0, 400.0).unwrap();
    assert_eq!(project.workspace.take_changed(), vec![project.body.clone()]);
}

#[test]
fn a_dropped_position_is_honoured_by_the_next_layout() {
    let mut project = Project::new();
    project.move_to("a", 250.0, 400.0).unwrap();
    let workspace = &store::get(project.workspace.documents(), &project.body)
        .unwrap()
        .workspace;
    let authored = project.authored();
    let laid_out = layout::compute(workspace, &workspace.views[0], &C4Metrics::default(), Some(&authored));
    let a = laid_out.boxes.get("a").unwrap();
    assert_eq!((a.x, a.y), (250.0, 400.0));
}

#[test]
fn dropping_one_element_onto_another_is_refused_because_containment_is_the_models_to_say() {
    let mut project = Project::new();
    let error = project
        .session
        .move_element(&mut project.workspace, "a", Some("b"), -1)
        .unwrap_err();
    assert!(error.to_lowercase().contains("not what contains it"), "{error}");
    assert!(project.authored().is_empty());
}

#[test]
fn dragging_something_no_longer_in_the_model_is_refused() {
    let mut project = Project::new();
    let error = project.move_to("ghost", 10.0, 10.0).unwrap_err();
    assert!(error.contains("ghost"), "{error}");
}

#[test]
fn a_re_parenting_request_is_refused_for_what_it_is_not_as_a_bad_coordinate_pair() {
    let mut project = Project::new();
    let error = project
        .session
        .move_element(&mut project.workspace, "a", Some("b"), -1)
        .unwrap_err();
    assert!(error.to_lowercase().contains("not what contains it"), "{error}");
    assert!(project.authored().is_empty());
}

#[test]
fn a_re_parenting_request_is_refused_even_when_it_looks_like_coordinates() {
    let mut project = Project::new();
    let error = project
        .session
        .move_element(&mut project.workspace, "a", Some("12,34"), -1)
        .unwrap_err();
    assert!(error.to_lowercase().contains("not what contains it"), "{error}");
    assert!(project.authored().is_empty());
}
