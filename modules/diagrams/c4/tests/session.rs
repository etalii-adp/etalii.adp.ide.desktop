//! One C4 diagram, one view, and the premise the C4 types rest on: two views over one document stay in step, because
//! they are two views of one model rather than two drawings (etalii.adp spec 030, task T061).
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/C4Session.Tests.cs` at
//! `8b6ccfc`. A session there subscribed to the store's change event; a desktop session is told of a change by the
//! shell (`DiagramSession::on_document_changed`), so a push is that call's answer. Not ported:
//! `DisposingASession_StopsItListeningToTheDocument`, because a desktop session subscribes to nothing.

use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_core::Catalogue;
use adp_diagram::change_handler::DocumentChange;
use adp_diagram::{
    DiagramDelta, DiagramDocuments, DiagramElement, DiagramSession, DiagramTools, DiagramViewport, DiagramWorkspace,
    ModuleView,
};
use adp_diagram_c4::mapper::{NODE_TYPE, VIEW_TYPE};
use adp_diagram_c4::view::C4DiagramView;
use adp_diagram_c4::{sidecar, store};
use adp_disl::DislPluginFunctions;
use adp_history::{Applied, Command, Refusal};

const MODEL: &str = r#"workspace "Bank" {
    model {
        u = person "Customer" "A customer."
        s = softwareSystem "Banking" "Does banking." {
            web = container "Web" "Serves." "React"
        }
        u -> web "Uses" "HTTPS"
    }
    views {
        systemContext s "context" {
            include *
        }
        container s "containers" {
            include *
        }
    }
}"#;

/// Reads a body again from disk, as the shell's watcher asks the store to.
struct Reload(PathBuf);

impl Command<DiagramDocuments> for Reload {
    fn label(&self) -> String {
        "Reload".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        store::reload(documents, &self.0);
        Ok(Applied::final_change())
    }
}

/// Opens a body in the project's documents.
struct Load(PathBuf);

impl Command<DiagramDocuments> for Load {
    fn label(&self) -> String {
        "Load".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        store::get_or_load(documents, &self.0);
        Ok(Applied::final_change())
    }
}

/// Tells every view of a body to show it again.
struct Touch(PathBuf);

impl Command<DiagramDocuments> for Touch {
    fn label(&self) -> String {
        "Touch".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        store::touch(documents, &self.0);
        Ok(Applied::final_change())
    }
}

struct Project {
    _folder: tempfile::TempDir,
    root: PathBuf,
    tools: DiagramTools,
    workspace: DiagramWorkspace,
}

impl Project {
    fn new() -> Project {
        let folder = tempfile::tempdir().unwrap();
        let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
        let root = folder.path().to_path_buf();
        Project {
            workspace: DiagramWorkspace::new(&root),
            root,
            _folder: folder,
            tools: adp_diagram_c4::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
                DislPluginFunctions::none()
            })),
        }
    }

    fn write_model(&self, name: &str, text: &str) -> PathBuf {
        let path = self.root.join(name);
        fs::write(&path, text).unwrap();
        path
    }

    fn write_registration(&self, name: &str, origin: &str, body: &str, view: Option<&str>) -> PathBuf {
        let path = self.root.join(name);
        let view = view.map_or(String::new(), |v| format!("view: {v}\n"));
        fs::write(&path, format!("{origin}\nbody: {body}\n{view}")).unwrap();
        path
    }

    fn open(&mut self, path: &Path) -> DiagramSession {
        self.workspace.open(&mut self.tools, path).unwrap().1
    }

    fn run(&mut self, command: &dyn Command<DiagramDocuments>) {
        assert!(self.workspace.execute_command(command).is_done());
    }
}

fn added(deltas: &[DiagramDelta]) -> Vec<DiagramElement> {
    deltas
        .iter()
        .filter_map(|d| match d {
            DiagramDelta::Add(elements) => Some(elements.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

fn removed(deltas: &[DiagramDelta]) -> Vec<String> {
    deltas
        .iter()
        .filter_map(|d| match d {
            DiagramDelta::Remove(ids) => Some(ids.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

fn view_key(session: &DiagramSession, documents: &DiagramDocuments) -> Option<String> {
    session
        .render(documents)
        .unwrap()
        .into_iter()
        .find(|e| e.element_type == VIEW_TYPE)
        .map(|e| e.attribute("viewKey").to_owned())
}

#[test]
fn a_registration_naming_a_view_opens_that_view() {
    let mut project = Project::new();
    project.write_model("model.dsl", MODEL);
    let adp = project.write_registration("containers.adp", "c4/container", "model.dsl", Some("containers"));
    let mut session = project.open(&adp);
    assert_eq!(
        view_key(&session, project.workspace.documents()).as_deref(),
        Some("containers")
    );
    // The container view shows the container; the context view would not.
    let baseline = session.baseline(project.workspace.documents()).unwrap();
    assert!(added(&baseline).iter().any(|e| e.id == "web"));
}

#[test]
fn a_document_opened_without_a_registration_shows_its_first_view() {
    let mut project = Project::new();
    let body = project.write_model("model.dsl", MODEL);
    let session = project.open(&body);
    assert_eq!(
        view_key(&session, project.workspace.documents()).as_deref(),
        Some("context")
    );
}

#[test]
fn a_registration_naming_a_view_the_document_does_not_declare_shows_nothing_rather_than_the_wrong_one() {
    let mut project = Project::new();
    project.write_model("model.dsl", MODEL);
    let adp = project.write_registration("ghost.adp", "c4/context", "model.dsl", Some("ghost"));
    let mut session = project.open(&adp);
    assert_eq!(view_key(&session, project.workspace.documents()), None);
    assert!(added(&session.baseline(project.workspace.documents()).unwrap()).is_empty());
}

#[test]
fn the_baseline_carries_the_views_furniture_as_well_as_its_elements() {
    let mut project = Project::new();
    let body = project.write_model("model.dsl", MODEL);
    let mut session = project.open(&body);
    let elements = added(&session.baseline(project.workspace.documents()).unwrap());
    assert!(elements.iter().any(|e| e.element_type == NODE_TYPE));
    assert!(elements.iter().any(|e| e.element_type == VIEW_TYPE));
}

#[test]
fn an_edit_through_one_view_reaches_a_session_on_another_view_of_the_same_model() {
    let mut project = Project::new();
    let body = project.write_model("model.dsl", MODEL);
    let context_adp = project.write_registration("context.adp", "c4/context", "model.dsl", Some("context"));
    let containers_adp = project.write_registration("containers.adp", "c4/container", "model.dsl", Some("containers"));
    let mut context = project.open(&context_adp);
    let mut containers = project.open(&containers_adp);
    context.baseline(project.workspace.documents()).unwrap();
    containers.baseline(project.workspace.documents()).unwrap();

    // Edit through the containers view.
    containers
        .set_property(&mut project.workspace, &["s"], "c4.name", "Renamed")
        .unwrap();
    assert_eq!(project.workspace.take_changed(), vec![body.clone()]);

    let DocumentChange::Changed(deltas) = context.on_document_changed(project.workspace.documents(), &body) else {
        panic!("the context view is told");
    };
    let renamed = added(&deltas).into_iter().find(|e| e.id == "s").unwrap();
    assert_eq!(renamed.attribute("name"), "Renamed");
}

#[test]
fn a_document_change_that_deletes_an_element_removes_it_from_the_canvas() {
    let mut project = Project::new();
    let body = project.write_model("model.dsl", MODEL);
    let mut session = project.open(&body);
    assert!(
        added(&session.baseline(project.workspace.documents()).unwrap())
            .iter()
            .any(|e| e.id == "u")
    );

    let kept: Vec<&str> = MODEL
        .split('\n')
        .filter(|line| !line.contains("u = person") && !line.contains("u -> web"))
        .collect();
    fs::write(&body, kept.join("\n")).unwrap();
    assert!(!fs::read_to_string(&body).unwrap().contains("person"));
    project.run(&Reload(body.clone()));

    let DocumentChange::Changed(deltas) = session.on_document_changed(project.workspace.documents(), &body) else {
        panic!("the session is told");
    };
    assert!(removed(&deltas).contains(&"u".to_owned()));
}

#[test]
fn a_change_to_another_document_is_not_this_sessions_business() {
    let mut project = Project::new();
    let body = project.write_model("model.dsl", MODEL);
    let other = project.write_model("other.dsl", MODEL);
    let mut session = project.open(&body);
    session.baseline(project.workspace.documents()).unwrap();
    project.run(&Load(other.clone()));
    // This view changed without telling it: a position written straight into its sidecar.
    assert_eq!(sidecar::write(&body, "context", "u", 4321.0, 1234.0), "");

    project.run(&Touch(other.clone()));
    let for_the_other = session.on_document_changed(project.workspace.documents(), &other);
    project.run(&Touch(body.clone()));
    let for_its_own = session.on_document_changed(project.workspace.documents(), &body);

    assert!(matches!(for_the_other, DocumentChange::OtherDocument));
    let DocumentChange::Changed(deltas) = for_its_own else {
        panic!("its own document's change moves the person");
    };
    assert!(added(&deltas).iter().any(|e| e.id == "u"));
}

#[test]
fn update_view_narrowing_to_nothing_removes_what_left() {
    let mut project = Project::new();
    let body = project.write_model("model.dsl", MODEL);
    let mut session = project.open(&body);
    session.baseline(project.workspace.documents()).unwrap();
    let deltas = session
        .update_view(
            project.workspace.documents(),
            DiagramViewport::new(100000.0, 100000.0, 200000.0, 200000.0),
        )
        .unwrap();
    let removed = removed(&deltas);
    assert!(removed.contains(&"u".to_owned()));
    assert!(removed.contains(&"s".to_owned()));
}

#[test]
fn update_view_widening_again_brings_them_back() {
    let mut project = Project::new();
    let body = project.write_model("model.dsl", MODEL);
    let mut session = project.open(&body);
    session.baseline(project.workspace.documents()).unwrap();
    session
        .update_view(
            project.workspace.documents(),
            DiagramViewport::new(100000.0, 100000.0, 200000.0, 200000.0),
        )
        .unwrap();
    let deltas = session
        .update_view(project.workspace.documents(), DiagramViewport::UNBOUNDED)
        .unwrap();
    assert!(added(&deltas).iter().any(|e| e.id == "s"));
    assert!(removed(&deltas).is_empty());
}

#[test]
fn a_document_that_does_not_exist_yet_opens_empty_rather_than_failing() {
    // The view itself, as the standalone host's session factory opened it: the desktop host's open of a registration
    // whose body is missing refuses before a view exists, as it does for every tool type.
    let mut project = Project::new();
    let missing = project.root.join("not-yet.dsl");
    project.run(&Load(missing.clone()));
    let view: Rc<dyn ModuleView> = Rc::new(C4DiagramView::new("c4/context", &missing, None));
    let mut session = DiagramSession::of_module(&missing, view);
    assert!(
        added(&session.baseline(project.workspace.documents()).unwrap()).is_empty(),
        "nothing is drawn"
    );
}
