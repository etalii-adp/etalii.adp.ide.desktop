//! Adding an element to a C4 model, as the toolbox's drop and the menu's "Add container…" do (etalii.adp spec 030,
//! task T061).
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/AddC4Element.Tests.cs` at
//! `8b6ccfc`: where a new element lands, C4's containment rule enforced at the gesture, a numbered identifier for a
//! second element of one name, and the block surgery an add makes on a parent declared on one line, undone byte for
//! byte without taking a comment the person wrote inside the block.

use std::fs;
use std::path::{Path, PathBuf};

use adp_core::Catalogue;
use adp_diagram::DiagramDocuments;
use adp_diagram::{DiagramProblemSeverity, DiagramSession, DiagramTools, DiagramWorkspace};
use adp_diagram_c4::commands::AddC4Element;
use adp_diagram_c4::lines::LineDocument;
use adp_diagram_c4::model::{C4ElementKind, C4Workspace};
use adp_diagram_c4::{parser, rules, store};
use adp_disl::DislPluginFunctions;
use adp_history::{Applied, Command, Outcome, Refusal};

/// The store's own save of a document edited beside the history, as the standalone test's `Save` was: it is not
/// recorded, so the next undo is the add's.
struct Save {
    body: PathBuf,
    document: LineDocument,
}

impl Command<DiagramDocuments> for Save {
    fn label(&self) -> String {
        "Save".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        store::save(documents, &self.body, &self.document).map_err(Refusal::new)?;
        Ok(Applied::final_change())
    }
}

const MODEL: &str = "workspace \"Bank\" {
    model {
        u = person \"Customer\" \"A customer.\"
        s = softwareSystem \"Banking\" \"Does banking.\" {
            web = container \"Web\" \"Serves pages.\" \"React\"
        }
        u -> web \"Uses\" \"HTTPS\"
    }
    views {
        container s \"containers\" {
            include *
        }
    }
}";

struct Project {
    _folder: tempfile::TempDir,
    body: PathBuf,
    workspace: DiagramWorkspace,
    _session: DiagramSession,
}

impl Project {
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
            _session: session,
        }
    }

    fn add(&mut self, kind: C4ElementKind, name: &str, parent_id: &str) -> Outcome {
        let command = AddC4Element {
            body_path: self.body.clone(),
            kind,
            name: name.into(),
            parent_id: parent_id.into(),
            identifier: String::new(),
        };
        self.workspace.execute_command(&command)
    }

    fn model(&self) -> C4Workspace {
        (*store::get(self.workspace.documents(), &self.body).unwrap().workspace).clone()
    }

    fn on_disk(&self) -> String {
        fs::read_to_string(&self.body).unwrap()
    }
}

fn refusal(outcome: Outcome) -> String {
    match outcome {
        Outcome::Refused(refusal) => refusal.0,
        Outcome::Done { .. } => panic!("expected a refusal"),
    }
}

#[test]
fn a_software_system_lands_at_the_top_of_the_model() {
    let mut project = Project::new();
    assert!(project.add(C4ElementKind::SoftwareSystem, "Payments", "").is_done());
    let model = project.model();
    let added: Vec<_> = model.elements.iter().filter(|e| e.name == "Payments").collect();
    assert_eq!(added.len(), 1);
    assert_eq!(added[0].kind, C4ElementKind::SoftwareSystem);
    assert_eq!(added[0].parent_id, None);
}

#[test]
fn a_container_lands_inside_the_system_it_was_dropped_on() {
    let mut project = Project::new();
    assert!(project.add(C4ElementKind::Container, "Api", "s").is_done());
    let model = project.model();
    let added = model.elements.iter().find(|e| e.name == "Api").unwrap();
    assert_eq!(added.parent_id.as_deref(), Some("s"));
    assert!(!added.technology.is_empty());
}

#[test]
fn a_component_gives_its_container_a_block_when_it_has_none() {
    let mut project = Project::new();
    assert!(!project.on_disk().contains("\"React\" {"));
    assert!(project.add(C4ElementKind::Component, "Controller", "web").is_done());
    let model = project.model();
    let added = model.elements.iter().find(|e| e.name == "Controller").unwrap();
    assert_eq!(added.parent_id.as_deref(), Some("web"));
    assert!(project.on_disk().contains("\"React\" {"));
}

#[test]
fn that_block_surgery_is_one_undo_away() {
    let mut project = Project::new();
    let before = project.on_disk();
    assert!(project.add(C4ElementKind::Component, "Controller", "web").is_done());
    assert!(project.workspace.undo().is_done());
    assert_eq!(project.on_disk(), before);
}

#[test]
fn an_add_changes_only_the_lines_it_needs() {
    let mut project = Project::new();
    let before = project.on_disk();
    let before: Vec<&str> = before.split('\n').collect();
    assert!(project.add(C4ElementKind::SoftwareSystem, "Payments", "").is_done());
    let after = project.on_disk();
    let after: Vec<&str> = after.split('\n').collect();
    assert_eq!(after.len(), before.len() + 1);
    assert!(before.iter().all(|line| after.contains(line)));
}

#[test]
fn a_container_dropped_on_nothing_is_refused_with_somewhere_to_go() {
    let mut project = Project::new();
    let error = refusal(project.add(C4ElementKind::Container, "Api", ""));
    assert!(error.to_lowercase().contains("software system"), "{error}");
}

#[test]
fn a_component_dropped_on_a_system_is_refused_and_says_what_it_wanted() {
    let mut project = Project::new();
    let error = refusal(project.add(C4ElementKind::Component, "Controller", "s"));
    assert!(error.to_lowercase().contains("container"), "{error}");
    assert!(!project.on_disk().contains("Controller"));
}

#[test]
fn an_unnamed_element_is_refused() {
    let mut project = Project::new();
    assert!(!project.add(C4ElementKind::SoftwareSystem, "   ", "").is_done());
}

#[test]
fn a_second_element_of_the_same_name_gets_its_own_identifier() {
    let mut project = Project::new();
    assert!(project.add(C4ElementKind::SoftwareSystem, "Payments", "").is_done());
    assert!(project.add(C4ElementKind::SoftwareSystem, "Payments", "").is_done());
    let model = project.model();
    assert!(model.find("payments").is_some());
    assert!(model.find("payments2").is_some());
}

#[test]
fn an_added_element_is_one_undo_away_and_redo_brings_it_back() {
    let mut project = Project::new();
    let before = project.on_disk();
    assert!(project.add(C4ElementKind::SoftwareSystem, "Payments", "").is_done());
    assert!(project.workspace.undo().is_done());
    assert_eq!(project.on_disk(), before);
    assert!(project.workspace.redo().is_done());
    assert!(project.model().find("payments").is_some());
}

#[test]
fn the_document_still_parses_after_an_add_and_an_undo() {
    let mut project = Project::new();
    assert!(project.add(C4ElementKind::Component, "Controller", "web").is_done());
    assert!(project.add(C4ElementKind::Container, "Api", "s").is_done());
    assert!(project.workspace.undo().is_done());
    let workspace = parser::parse_text(&project.on_disk());
    assert!(workspace.find("controller").is_some());
    assert!(workspace.find("api").is_none());
    assert!(
        !rules::validate(&workspace)
            .iter()
            .any(|p| p.severity == DiagramProblemSeverity::Error)
    );
}

#[test]
fn undoing_an_add_does_not_take_a_comment_the_user_wrote_inside_the_block() {
    let mut project = Project::new();
    assert!(project.add(C4ElementKind::Component, "Controller", "web").is_done());

    // The person writes a comment inside the block the add opened, and the store reads it as its own save.
    let mut document = store::get(project.workspace.documents(), &project.body)
        .unwrap()
        .lines();
    let opened = document
        .code_lines()
        .into_iter()
        .find(|line| line.code().contains("container \"Web\""))
        .unwrap();
    document
        .insert_line(
            opened.number + 1,
            "                // Components arrive here as the design settles.",
        )
        .unwrap();
    let save = Save {
        body: project.body.clone(),
        document,
    };
    assert!(project.workspace.execute_command(&save).is_done());

    assert!(project.workspace.undo().is_done());
    let on_disk = project.on_disk();
    assert!(
        on_disk.contains("// Components arrive here as the design settles."),
        "{on_disk}"
    );
    assert!(!on_disk.contains("Controller"), "{on_disk}");
}
