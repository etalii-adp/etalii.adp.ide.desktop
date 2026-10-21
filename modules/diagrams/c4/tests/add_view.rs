//! Adding a second view to a model that already exists, and the registration that opens it: one C4 model, several
//! views (etalii.adp spec 030, task T061).
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/AddC4View.Tests.cs` at
//! `8b6ccfc`: the view appended and its `.adp` written, every other line kept, the scope each kind takes, the add
//! undone with its registration, a second view of one key and a model with no system refused, and the new view laid
//! out.

use std::fs;
use std::path::{Path, PathBuf};

use adp_core::Catalogue;
use adp_diagram::{DiagramSession, DiagramTools, DiagramWorkspace};
use adp_diagram_c4::commands::AddC4View;
use adp_diagram_c4::metrics::C4Metrics;
use adp_diagram_c4::model::{C4ViewKind, C4Workspace};
use adp_diagram_c4::{layout, parser, store};
use adp_disl::DislPluginFunctions;
use adp_history::Outcome;

const MODEL: &str = "workspace \"Bank\" {
    model {
        u = person \"Customer\" \"A customer.\"
        s = softwareSystem \"Banking\" \"Does banking.\" {
            web = container \"Web\" \"Serves.\" \"React\"
        }
        u -> web \"Uses\" \"HTTPS\"
    }
    views {
        systemContext s \"context\" {
            include *
        }
    }
}";

struct Project {
    _folder: tempfile::TempDir,
    root: PathBuf,
    body: PathBuf,
    workspace: DiagramWorkspace,
    _session: DiagramSession,
}

impl Project {
    fn new() -> Project {
        let folder = tempfile::tempdir().unwrap();
        fs::write(folder.path().join("banking.dsl"), MODEL).unwrap();
        let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
        let mut tools = adp_diagram_c4::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
            DislPluginFunctions::none()
        }));
        let mut workspace = DiagramWorkspace::new(folder.path());
        let (opened, session) = workspace.open(&mut tools, Path::new("banking.dsl")).unwrap();
        Project {
            root: opened.body_path.parent().unwrap().to_path_buf(),
            body: opened.body_path,
            _folder: folder,
            workspace,
            _session: session,
        }
    }

    fn command(&self, body: &Path, origin: &str, kind: C4ViewKind, key: &str) -> AddC4View {
        AddC4View {
            body_path: body.to_path_buf(),
            registration_path: self.root.join(format!("{key}.adp")),
            origin: origin.into(),
            view_kind: kind,
            view_key: key.into(),
            body_relative_path: body.file_name().unwrap().to_string_lossy().into_owned(),
        }
    }

    fn add(&mut self, origin: &str, kind: C4ViewKind, key: &str) -> Outcome {
        let command = self.command(&self.body.clone(), origin, kind, key);
        self.workspace.execute_command(&command)
    }

    fn add_containers(&mut self) -> Outcome {
        self.add("c4/container", C4ViewKind::Container, "containers")
    }

    fn model(&self) -> C4Workspace {
        (*store::get(self.workspace.documents(), &self.body).unwrap().workspace).clone()
    }

    fn on_disk(&self) -> String {
        fs::read_to_string(&self.body).unwrap()
    }
}

#[test]
fn adding_a_view_appends_it_to_the_document_and_creates_the_registration_that_opens_it() {
    let mut project = Project::new();
    assert!(project.add_containers().is_done());
    let model = project.model();
    let view = model.find_view("containers").unwrap();
    assert_eq!(view.kind, C4ViewKind::Container);
    assert_eq!(view.scope_id.as_deref(), Some("s"));
    let registration = fs::read_to_string(project.root.join("containers.adp")).unwrap();
    assert!(registration.starts_with("c4/container\n"), "{registration}");
    assert!(registration.contains("body: banking.dsl"));
    assert!(registration.contains("view: containers"));
}

#[test]
fn adding_a_view_leaves_the_rest_of_the_document_exactly_as_it_was() {
    let mut project = Project::new();
    let before = project.on_disk();
    let before: Vec<&str> = before.lines().collect();
    assert!(project.add_containers().is_done());
    let after = project.on_disk();
    let after: Vec<&str> = after.lines().collect();
    assert_eq!(after.len(), before.len() + 4);
    assert_eq!(&after[..10], &before[..10]);
}

#[test]
fn the_existing_view_still_opens_after_a_second_is_added() {
    let mut project = Project::new();
    assert!(project.add_containers().is_done());
    let model = project.model();
    assert!(model.find_view("context").is_some());
    assert_eq!(model.views.len(), 2);
}

#[test]
fn adding_a_view_undoes_removing_both_the_block_and_the_registration() {
    let mut project = Project::new();
    let before = project.on_disk();
    assert!(project.add_containers().is_done());
    assert!(project.workspace.undo().is_done());
    assert_eq!(project.on_disk(), before);
    assert!(!project.root.join("containers.adp").exists());
}

#[test]
fn adding_a_view_twice_under_one_key_is_refused() {
    let mut project = Project::new();
    assert!(project.add_containers().is_done());
    match project.add_containers() {
        Outcome::Refused(refusal) => assert!(refusal.0.to_lowercase().contains("already has a view"), "{}", refusal.0),
        Outcome::Done { .. } => panic!("a second view of one key was added"),
    }
}

#[test]
fn a_landscape_view_is_added_with_no_scope() {
    let mut project = Project::new();
    assert!(
        project
            .add("c4/system-landscape", C4ViewKind::SystemLandscape, "landscape")
            .is_done()
    );
    let model = project.model();
    let view = model.find_view("landscape").unwrap();
    assert_eq!(view.kind, C4ViewKind::SystemLandscape);
    assert_eq!(view.scope_id, None);
}

#[test]
fn a_component_view_is_scoped_to_a_container_not_a_system() {
    let mut project = Project::new();
    assert!(
        project
            .add("c4/component", C4ViewKind::Component, "components")
            .is_done()
    );
    assert_eq!(
        project.model().find_view("components").unwrap().scope_id.as_deref(),
        Some("web")
    );
}

#[test]
fn a_dynamic_view_is_added_with_no_include_all() {
    let mut project = Project::new();
    assert!(project.add("c4/dynamic", C4ViewKind::Dynamic, "scenario").is_done());
    let document = project.on_disk();
    let block = &document[document.find("dynamic s \"scenario\"").unwrap()..];
    assert!(!block[..block.find('}').unwrap()].contains("include *"));
}

#[test]
fn adding_a_view_to_a_model_with_no_system_is_refused_rather_than_writing_something_broken() {
    let mut project = Project::new();
    let empty = project.root.join("empty.dsl");
    fs::write(&empty, "workspace {\n    model {\n    }\n    views {\n    }\n}\n").unwrap();
    let mut command = project.command(&empty, "c4/container", C4ViewKind::Container, "containers");
    command.registration_path = project.root.join("x.adp");
    match project.workspace.execute_command(&command) {
        Outcome::Refused(refusal) => assert!(refusal.0.to_lowercase().contains("no software system"), "{}", refusal.0),
        Outcome::Done { .. } => panic!("a view of nothing was added"),
    }
    assert!(!project.root.join("x.adp").exists());
}

#[test]
fn the_added_document_still_parses_and_the_new_view_lays_out() {
    let mut project = Project::new();
    assert!(project.add_containers().is_done());
    let workspace = parser::parse_text(&project.on_disk());
    let view = workspace.find_view("containers").unwrap();
    let laid_out = layout::compute(&workspace, view, &C4Metrics::default(), None);
    assert!(laid_out.boxes.contains("web"));
    assert_eq!(laid_out.boundaries.len(), 1);
}
