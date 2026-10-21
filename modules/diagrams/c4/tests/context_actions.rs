//! What a person is offered on a C4 element or relationship, and what choosing it does (etalii.adp spec 030, task
//! T061): an action that does not apply is not offered at all rather than greyed out.
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/C4ContextActionProvider.Tests.cs`
//! at `8b6ccfc`, through the session of a `c4/container` registration of the module's own model. Not ported:
//! `OnlyRenameAndRelabelMarkTheirPromptsAsEditingTheLabelOnScreen`, because the desktop host's prompt carries no
//! `InlineLabelElementId` (the shell's inline editor is opened by the gesture, not named by the prompt).

use std::fs;
use std::path::{Path, PathBuf};

use adp_context::{ActionGroup, Prompt};
use adp_core::Catalogue;
use adp_diagram::{ActionOutcome, ActionTarget, DiagramSession, DiagramTools, DiagramWorkspace};
use adp_diagram_c4::actions::{self, EDIT_DESCRIPTION, RELABEL, RENAME, SET_PROTOCOL, SET_TECHNOLOGY};
use adp_disl::DislPluginFunctions;
use adp_disl::behavior::ids;

const MODEL: &str = r#"workspace "Bank" {
    model {
        u = person "Customer" "A customer."
        s = softwareSystem "Banking" "Does banking." {
            web = container "Web" "Serves." "React"
        }
        u -> web "Uses" "HTTPS"
    }
    views {
        container s "containers" {
            include *
        }
    }
}"#;

fn tools() -> DiagramTools {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    adp_diagram_c4::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
        DislPluginFunctions::none()
    }))
}

struct Project {
    _folder: tempfile::TempDir,
    root: PathBuf,
    body: PathBuf,
    tools: DiagramTools,
    workspace: DiagramWorkspace,
    session: DiagramSession,
}

impl Project {
    fn open() -> Project {
        let folder = tempfile::tempdir().unwrap();
        let root = folder.path().to_path_buf();
        let body = root.join("model.dsl");
        fs::write(&body, MODEL).unwrap();
        fs::write(
            root.join("model.adp"),
            "c4/container\nbody: model.dsl\nview: containers\n",
        )
        .unwrap();
        let mut tools = tools();
        let mut workspace = DiagramWorkspace::new(&root);
        let (_, session) = workspace.open(&mut tools, Path::new("model.adp")).unwrap();
        Project {
            _folder: folder,
            root,
            body,
            tools,
            workspace,
            session,
        }
    }

    fn action_ids(&self, element_id: &str) -> Vec<String> {
        ids_of(&self.session.actions(
            self.workspace.documents(),
            &ActionTarget::Element(element_id.to_owned()),
        ))
    }

    fn execute(&mut self, element_id: &str, action_id: &str) -> ActionOutcome {
        self.session.execute(
            &mut self.workspace,
            &ActionTarget::Element(element_id.to_owned()),
            action_id,
            &ids::fixed(&[]),
        )
    }

    fn commit(&mut self, element_id: &str, action_id: &str, value: &str) -> ActionOutcome {
        self.session.answer(
            &mut self.workspace,
            &ActionTarget::Element(element_id.to_owned()),
            action_id,
            value,
        )
    }

    fn text(&self) -> String {
        fs::read_to_string(&self.body).unwrap()
    }
}

fn ids_of(groups: &[ActionGroup]) -> Vec<String> {
    groups.iter().flat_map(|g| &g.actions).map(|a| a.id.clone()).collect()
}

fn relationship_id() -> String {
    adp_diagram_c4::parser::parse_text(MODEL).relationships[0].id()
}

#[test]
fn a_container_is_offered_a_technology() {
    let project = Project::open();
    let ids = project.action_ids("web");
    assert!(ids.contains(&RENAME.to_owned()));
    assert!(ids.contains(&EDIT_DESCRIPTION.to_owned()));
    assert!(ids.contains(&SET_TECHNOLOGY.to_owned()));
}

#[test]
fn a_person_is_not_offered_a_technology_because_c4_gives_them_none() {
    let project = Project::open();
    let ids = project.action_ids("u");
    assert!(ids.contains(&RENAME.to_owned()));
    assert!(!ids.contains(&SET_TECHNOLOGY.to_owned()));
}

#[test]
fn a_relationship_is_offered_a_relabel_and_a_protocol_and_not_a_rename() {
    let project = Project::open();
    assert_eq!(project.action_ids(&relationship_id()), vec![RELABEL, SET_PROTOCOL]);
}

#[test]
fn something_no_longer_in_the_model_is_offered_nothing() {
    let project = Project::open();
    assert!(project.action_ids("ghost").is_empty());
}

#[test]
fn rename_asks_for_the_name_starting_from_the_current_one() {
    let mut project = Project::open();
    let ActionOutcome::Prompt(Prompt::Input { initial_value, .. }) = project.execute("web", RENAME) else {
        panic!("a rename asks for the name");
    };
    assert_eq!(initial_value, "Web");
}

#[test]
fn relabel_writes_only_the_description_leaving_the_technology_that_is_drawn_beside_it() {
    let mut project = Project::open();
    let before = adp_diagram_c4::parser::parse_text(MODEL).relationships[0].clone();
    assert_ne!(before.technology, "");
    assert_eq!(
        project.commit(&before.id(), RELABEL, "Views accounts using"),
        ActionOutcome::Done
    );
    let after = adp_diagram_c4::parser::parse_text(&project.text()).relationships[0].clone();
    assert_eq!(after.description, "Views accounts using");
    assert_eq!(after.technology, before.technology);
}

#[test]
fn rename_commits_through_a_command_so_it_is_one_undo_away() {
    let mut project = Project::open();
    assert_eq!(project.commit("web", RENAME, "Web Application"), ActionOutcome::Done);
    assert!(project.text().contains("container \"Web Application\""));
    assert!(project.workspace.history().availability().can_undo);
}

#[test]
fn relabel_commits_through_a_command() {
    let mut project = Project::open();
    assert_eq!(
        project.commit(&relationship_id(), RELABEL, "Views accounts using"),
        ActionOutcome::Done
    );
    assert!(project.text().contains("\"Views accounts using\""));
}

#[test]
fn an_empty_name_is_refused_because_nothing_would_identify_the_element() {
    let refusal = actions::validate(RENAME, "   ").unwrap_err();
    assert!(refusal.to_lowercase().contains("needs a name"), "{refusal}");
}

#[test]
fn an_empty_description_is_accepted_because_an_incomplete_model_is_ordinary() {
    assert!(actions::validate(EDIT_DESCRIPTION, "").is_ok());
}

#[test]
fn an_action_that_does_not_apply_to_the_selection_is_refused_with_a_reason() {
    let mut project = Project::open();
    let ActionOutcome::Refused(reason) = project.execute(&relationship_id(), SET_TECHNOLOGY) else {
        panic!("a technology does not apply to a relationship");
    };
    assert!(reason.to_lowercase().contains("does not apply"), "{reason}");
}

/// Holds `path` open sharing nothing, so a read of it fails while the guard lives.
#[cfg(windows)]
fn lock(path: &Path) -> fs::File {
    use std::os::windows::fs::OpenOptionsExt;
    fs::OpenOptions::new().read(true).share_mode(0).open(path).unwrap()
}

#[cfg(windows)]
#[test]
fn a_document_that_could_not_be_read_is_refused_by_the_menus_and_the_grid_and_writes_nothing() {
    let folder = tempfile::tempdir().unwrap();
    let root = folder.path().to_path_buf();
    let body = root.join("model.dsl");
    fs::write(&body, MODEL).unwrap();
    fs::write(
        root.join("model.adp"),
        "c4/container\nbody: model.dsl\nview: containers\n",
    )
    .unwrap();
    let before = fs::read(&body).unwrap();
    let mut tools = tools();
    let mut workspace = DiagramWorkspace::new(&root);
    let guard = lock(&body);
    let (_, session) = workspace.open(&mut tools, Path::new("model.adp")).unwrap();

    let offered = session.actions(workspace.documents(), &ActionTarget::Canvas { x: 0.0, y: 0.0 });
    let web = ActionTarget::Element("web".to_owned());
    let renamed = session.execute(&mut workspace, &web, RENAME, &ids::fixed(&[]));
    let committed = session.answer(&mut workspace, &web, RENAME, "Renamed");
    let set = session.set_property(&mut workspace, &["web"], "c4.name", "Renamed");
    drop(guard);

    assert!(offered.is_empty());
    for outcome in [renamed, committed] {
        let ActionOutcome::Refused(reason) = outcome else {
            panic!("an unreadable document refuses");
        };
        assert!(reason.contains("could not be read"), "{reason}");
    }
    assert!(set.unwrap_err().contains("could not be read"));
    assert_eq!(fs::read(&body).unwrap(), before);
}

#[test]
fn a_file_that_is_not_a_dsl_document_is_answered_with_nothing_whatever_it_holds() {
    let mut project = Project::open();
    fs::write(project.root.join("model.txt"), MODEL).unwrap();
    assert!(
        project
            .workspace
            .open(&mut project.tools, Path::new("model.txt"))
            .is_err(),
        "no C4 type opens a .txt"
    );
}
