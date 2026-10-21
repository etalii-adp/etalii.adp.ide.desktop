//! Every C4 edit is a command with an inverse, run through the project's history, and changes only the line it
//! touches (etalii.adp spec 030, task T061).
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/C4Commands.Tests.cs` at
//! `8b6ccfc`: a rename, a description, a technology (refused on a person), a relationship's label and protocol, a
//! refusal for an element already gone, the bytes undo restores, the change every view hears, a write that cannot
//! land, and a drag whose layout cannot be saved.

use std::fs;
use std::path::{Path, PathBuf};

use adp_core::Catalogue;
use adp_diagram::{DiagramSession, DiagramTools, DiagramWorkspace};
use adp_diagram_c4::commands::{
    MoveC4Element, SetElementDescription, SetElementName, SetElementTechnology, SetRelationshipDescription,
    SetRelationshipTechnology,
};
use adp_diagram_c4::{parser, store};
use adp_disl::DislPluginFunctions;
use adp_history::{Command, Outcome};

const MODEL: &str = "workspace \"Bank\" {
    model {
        u = person \"Customer\" \"A customer.\"
        s = softwareSystem \"Banking\" \"Does banking.\" {
            web = container \"Web\" \"Serves.\" \"React\"
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
    root: PathBuf,
    body: PathBuf,
    workspace: DiagramWorkspace,
    _session: DiagramSession,
}

impl Project {
    fn new() -> Project {
        let folder = tempfile::tempdir().unwrap();
        let root = folder.path().to_path_buf();
        let body = root.join("model.dsl");
        fs::write(&body, MODEL).unwrap();
        let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
        let mut tools = adp_diagram_c4::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
            DislPluginFunctions::none()
        }));
        let mut workspace = DiagramWorkspace::new(&root);
        let (opened, session) = workspace.open(&mut tools, Path::new("model.dsl")).unwrap();
        let body = opened.body_path;
        Project {
            _folder: folder,
            root,
            body,
            workspace,
            _session: session,
        }
    }

    fn on_disk(&self) -> String {
        fs::read_to_string(&self.body).unwrap()
    }

    fn execute(&mut self, command: &dyn Command<adp_diagram::DiagramDocuments>) -> Outcome {
        self.workspace.execute_command(command)
    }

    fn name(&self, id: &str, name: &str) -> SetElementName {
        SetElementName {
            body_path: self.body.clone(),
            element_id: id.into(),
            name: name.into(),
        }
    }

    fn description(&self, id: &str, description: &str) -> SetElementDescription {
        SetElementDescription {
            body_path: self.body.clone(),
            element_id: id.into(),
            description: description.into(),
        }
    }

    fn relationship_id(&self) -> String {
        let workspace = &store::get(self.workspace.documents(), &self.body).unwrap().workspace;
        assert_eq!(workspace.relationships.len(), 1);
        workspace.relationships[0].id()
    }
}

fn refusal(outcome: Outcome) -> String {
    match outcome {
        Outcome::Refused(refusal) => refusal.0,
        Outcome::Done { .. } => panic!("expected a refusal"),
    }
}

fn differing_lines(before: &str, after: &str) -> i64 {
    let a: Vec<&str> = before.split('\n').collect();
    let b: Vec<&str> = after.split('\n').collect();
    if a.len() != b.len() {
        return -1;
    }
    a.iter().zip(&b).filter(|(x, y)| x != y).count() as i64
}

#[test]
fn rename_changes_the_name_and_nothing_else_on_the_line() {
    let mut project = Project::new();
    let before = project.on_disk();
    let command = project.name("web", "Web Application");
    assert!(project.execute(&command).is_done());
    let after = project.on_disk();
    assert!(
        after.contains("container \"Web Application\" \"Serves.\" \"React\""),
        "{after}"
    );
    assert_eq!(differing_lines(&before, &after), 1);
}

#[test]
fn rename_undoes_to_the_exact_bytes_it_started_from() {
    let mut project = Project::new();
    let before = project.on_disk();
    let command = project.name("web", "Renamed");
    assert!(project.execute(&command).is_done());
    assert!(project.workspace.undo().is_done());
    assert_eq!(project.on_disk(), before);
}

#[test]
fn set_description_changes_the_description() {
    let mut project = Project::new();
    let command = project.description("u", "A retail banking customer.");
    assert!(project.execute(&command).is_done());
    assert!(
        project
            .on_disk()
            .contains("person \"Customer\" \"A retail banking customer.\"")
    );
}

#[test]
fn set_technology_on_a_container_works() {
    let mut project = Project::new();
    let command = SetElementTechnology {
        body_path: project.body.clone(),
        element_id: "web".into(),
        technology: "Java and Spring MVC".into(),
    };
    assert!(project.execute(&command).is_done());
    assert!(project.on_disk().contains("\"Serves.\" \"Java and Spring MVC\""));
}

#[test]
fn set_technology_on_a_person_is_refused_because_c4_gives_them_none() {
    let mut project = Project::new();
    let before = project.on_disk();
    let command = SetElementTechnology {
        body_path: project.body.clone(),
        element_id: "u".into(),
        technology: "React".into(),
    };
    let error = refusal(project.execute(&command));
    assert!(error.to_lowercase().contains("no technology"), "{error}");
    assert_eq!(project.on_disk(), before);
}

#[test]
fn set_relationship_description_relabels_the_relationship() {
    let mut project = Project::new();
    let command = SetRelationshipDescription {
        body_path: project.body.clone(),
        relationship_id: project.relationship_id(),
        description: "Views accounts using".into(),
    };
    assert!(project.execute(&command).is_done());
    assert!(
        project
            .on_disk()
            .contains("u -> web \"Views accounts using\" \"HTTPS\"")
    );
}

#[test]
fn set_relationship_technology_sets_the_protocol() {
    let mut project = Project::new();
    let command = SetRelationshipTechnology {
        body_path: project.body.clone(),
        relationship_id: project.relationship_id(),
        technology: "JSON/HTTPS".into(),
    };
    assert!(project.execute(&command).is_done());
    assert!(project.on_disk().contains("u -> web \"Uses\" \"JSON/HTTPS\""));
}

#[test]
fn an_edit_to_something_already_deleted_is_refused_rather_than_throwing() {
    let mut project = Project::new();
    let command = project.name("ghost", "Anything");
    let error = refusal(project.execute(&command));
    assert!(error.contains("ghost"), "{error}");
}

#[test]
fn every_edit_leaves_the_document_parseable_and_the_other_lines_untouched() {
    let mut project = Project::new();
    let before = project.on_disk();
    let rename = project.name("s", "Internet Banking");
    let describe = project.description("web", "Delivers the app.");
    assert!(project.execute(&rename).is_done());
    assert!(project.execute(&describe).is_done());
    let after = project.on_disk();
    assert_eq!(differing_lines(&before, &after), 2);
    let workspace = parser::parse_text(&after);
    assert_eq!(workspace.find("s").unwrap().name, "Internet Banking");
    assert_eq!(workspace.find("web").unwrap().description, "Delivers the app.");
}

#[test]
fn undoing_both_edits_restores_the_original_bytes() {
    let mut project = Project::new();
    let before = project.on_disk();
    let rename = project.name("s", "Internet Banking");
    let describe = project.description("web", "Delivers the app.");
    assert!(project.execute(&rename).is_done());
    assert!(project.execute(&describe).is_done());
    assert!(project.workspace.undo().is_done());
    assert!(project.workspace.undo().is_done());
    assert_eq!(project.on_disk(), before);
}

#[test]
fn an_edit_reaches_the_sessions_on_that_document() {
    let mut project = Project::new();
    project.workspace.take_changed();
    let command = project.name("web", "Renamed");
    assert!(project.execute(&command).is_done());
    assert_eq!(project.workspace.take_changed(), vec![project.body.clone()]);
}

/// Opens `path` the way an external editor holding it does: sharing only `share`, so a replace of it is denied.
#[cfg(windows)]
fn hold(path: &Path, share: u32) -> fs::File {
    use std::os::windows::fs::OpenOptionsExt;
    fs::OpenOptions::new().read(true).share_mode(share).open(path).unwrap()
}

#[cfg(windows)]
const FILE_SHARE_READ: u32 = 1;

#[test]
#[cfg(windows)]
fn a_write_that_cannot_land_is_reported_rather_than_answered_with_success() {
    let mut project = Project::new();
    let _holder = hold(&project.body, FILE_SHARE_READ);
    let command = project.name("web", "Web Application");
    let error = refusal(project.execute(&command));
    assert!(error.contains("could not be written"), "{error}");
}

#[test]
#[cfg(windows)]
fn a_drag_whose_layout_cannot_be_saved_succeeds_but_says_so() {
    let mut project = Project::new();
    let first = MoveC4Element {
        body_path: project.body.clone(),
        view_key: "SystemContext".into(),
        element_id: "web".into(),
        x: 10.0,
        y: 20.0,
    };
    assert!(project.execute(&first).is_done());
    let sidecar = project.root.join("model.layout.json");
    assert!(sidecar.exists(), "expected a sidecar at {}", sidecar.display());
    let _holder = hold(&sidecar, FILE_SHARE_READ);
    let second = MoveC4Element {
        x: 30.0,
        y: 40.0,
        ..first
    };
    match project.execute(&second) {
        Outcome::Done { warning, .. } => assert!(warning.contains("reopened"), "{warning}"),
        Outcome::Refused(refusal) => panic!("refused: {}", refusal.0),
    }
}
