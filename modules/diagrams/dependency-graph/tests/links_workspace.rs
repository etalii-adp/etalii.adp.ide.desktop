//! A dependency graph in a workspace of documents that link to each other (etalii.adp spec 089; FBL section 17,
//! dependency-graph.md): its top-level `id` is its document id and an entry with a `link` is a linked node, labelled
//! with its target's name; opening it checks its links and writes nothing, its first edit writes what the check
//! changed, and its menu offers Add from another document here, Linked from (N) and Open linked element.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use adp_cel::Value;
use adp_context::Prompt;
use adp_diagram::DiagramDocument;
use adp_diagram::link_actions::{LINKED_FROM, OPEN_LINKED};
use adp_diagram::problems::DiagramProblemLocation;
use adp_diagram::session::{ActionOutcome, ActionTarget, DiagramSession};
use adp_diagram::workspace::DiagramWorkspace;
use adp_disl::behavior::ids;

const OTHER: &str = "dependencies: 1\r\nid: o7h3r0ne\r\nelements:\r\n  - id: db\r\n    label: Orders database\r\n    x: 100\r\n    row: 0\r\n";

const SERVICES: &str = "dependencies: 1\r\nid: d7n2k4qp\r\nelements:\r\n  - id: api\r\n    label: API gateway\r\n    x: 270\r\n    row: 4\r\n  - id: auth\r\n    label: Identity service\r\n    x: 800\r\n    row: 4\r\n  - id: l1\r\n    link: $d7n2k4qp@services.dgr#api.label=Gateway\r\n    x: 1700\r\n    row: 4\r\n  - id: l2\r\n    link: $o7h3r0ne@other.dgr#db.label=Database\r\n    x: 1700\r\n    row: 6\r\n  - id: l3\r\n    link: $xxxxxxxx@gone.dgr#a1=Gone\r\n    x: 2000\r\n    row: 6\r\nrelations:\r\n  - id: r1\r\n    from: api\r\n    to: l2\r\n";

struct Project {
    folder: tempfile::TempDir,
    tools: adp_diagram::DiagramTools,
    workspace: DiagramWorkspace,
    session: DiagramSession,
}

impl Project {
    fn new(services: &str) -> Project {
        let folder = tempfile::tempdir().unwrap();
        fs::create_dir_all(folder.path().join("teams")).unwrap();
        fs::write(folder.path().join("teams/services.dgr"), services).unwrap();
        fs::write(folder.path().join("other.dgr"), OTHER).unwrap();
        let mut tools = common::tools();
        let mut workspace = DiagramWorkspace::new(folder.path());
        let (_, session) = workspace
            .open(&mut tools, Path::new("teams/services.dgr"))
            .expect("the graph opens");
        let mut project = Project {
            folder,
            tools,
            workspace,
            session,
        };
        let path = project.path();
        project
            .workspace
            .check_links(&mut project.tools, &path)
            .expect("the links are checked");
        project
    }

    fn path(&self) -> PathBuf {
        std::path::absolute(self.folder.path().join("teams/services.dgr")).unwrap()
    }

    fn other(&self) -> PathBuf {
        std::path::absolute(self.folder.path().join("other.dgr")).unwrap()
    }

    fn body(&self) -> String {
        fs::read_to_string(self.path()).unwrap()
    }

    fn document(&self) -> &DiagramDocument {
        self.workspace.documents().get(&self.path()).expect("open")
    }

    fn rename_auth(&mut self) {
        let mut attributes = adp_cel::Map::default();
        attributes.insert("label", Value::String("Login".into()));
        let change = adp_diagram::document::DiagramChange::Disl(adp_disl::DislChange::Set {
            element_id: "auth".into(),
            type_name: "Node".into(),
            attributes,
        });
        let edit = self.document().plan("Rename", &[change]).expect("planned");
        self.workspace.execute(edit).expect("the edit is made");
    }

    fn run(&mut self, target: &ActionTarget, action: &str) -> ActionOutcome {
        self.session
            .execute(&mut self.workspace, target, action, &ids::fixed(&["n1"]))
    }

    fn reply(&mut self, target: &ActionTarget, action: &str, value: &str) -> ActionOutcome {
        self.session
            .answer_with_ids(&mut self.workspace, target, action, value, &ids::fixed(&["n1"]))
    }

    fn labels(&self, target: &ActionTarget) -> Vec<String> {
        self.session
            .actions(self.workspace.documents(), target)
            .into_iter()
            .flat_map(|g| g.actions)
            .map(|a| a.label)
            .collect()
    }
}

fn element(id: &str) -> ActionTarget {
    ActionTarget::Element(id.to_owned())
}

#[test]
fn a_graph_reads_its_document_id_and_its_linked_nodes_labelled_with_their_targets_names() {
    let project = Project::new(SERVICES);
    let document = project.document();
    assert_eq!(document.fbl().document_id.as_deref(), Some("d7n2k4qp"));
    document.bind_views();
    let diagram = &document.model().diagram;
    let of = |id: &str| diagram.element_by_id(id).unwrap();
    assert_eq!(of("l1").type_name(), "LinkedNode");
    assert_eq!(of("api").type_name(), "Node");
    assert_eq!(of("l1").value_of("label"), Value::String("API gateway".into()));
    assert_eq!(of("l2").value_of("label"), Value::String("Orders database".into()));
    assert_eq!(of("l3").value_of("label"), Value::String("Gone".into()));
    assert!(of("l2").linked().is_some());
    assert!(of("l3").linked().is_none());
    // A dependency on a linked node resolves.
    assert_eq!(of("r1").target().map(|t| t.id()).as_deref(), Some("l2"));
}

#[test]
fn opening_writes_nothing_and_the_first_edit_refreshes_the_stale_links_in_the_same_step() {
    let mut project = Project::new(SERVICES);
    assert_eq!(project.body(), SERVICES);
    assert!(project.workspace.holds_link_changes(&project.path()));
    project.rename_auth();
    let expected = SERVICES
        .replace("label: Identity service", "label: Login")
        .replace("#api.label=Gateway", "#api.label=API gateway")
        .replace("#db.label=Database", "#db.label=Orders database");
    assert_eq!(project.body(), expected);
    project.workspace.undo();
    assert_eq!(project.body(), SERVICES);
}

#[test]
fn a_graph_without_a_document_id_is_given_one_beside_its_schema_marker_with_its_first_edit() {
    let without = SERVICES.replace("id: d7n2k4qp\r\n", "");
    let mut project = Project::new(&without);
    project.rename_auth();
    let body = project.body();
    let id = body
        .lines()
        .nth(1)
        .and_then(|line| line.strip_prefix("id: "))
        .unwrap_or_else(|| panic!("{body}"));
    assert_eq!(id.len(), 8, "{id}");
    assert!(id.chars().all(|c| c.is_ascii_digit() || c.is_ascii_lowercase()), "{id}");
    project.workspace.undo();
    assert_eq!(project.body(), without);
}

#[test]
fn the_links_that_do_not_resolve_are_findings_at_the_nodes_that_hold_them() {
    let project = Project::new(SERVICES);
    let problems = project.session.problems(project.workspace.documents());
    let links: Vec<(String, String)> = problems
        .iter()
        .filter(|p| p.rule_id.starts_with("link."))
        .map(|p| match &p.location {
            Some(DiagramProblemLocation::Element(id)) => (id.clone(), p.rule_id.clone()),
            other => (format!("{other:?}"), p.rule_id.clone()),
        })
        .collect();
    assert_eq!(links, [("l3".to_owned(), "link.broken-document".to_owned())]);
}

#[test]
fn a_node_others_link_to_lists_them_and_a_linked_one_opens_its_target() {
    let mut project = Project::new(SERVICES);
    assert!(project.labels(&element("api")).contains(&"Linked from (1)".to_owned()));
    let ActionOutcome::Prompt(Prompt::Choice { options, .. }) = project.run(&element("api"), LINKED_FROM) else {
        panic!("the backlinks are listed");
    };
    let ActionOutcome::Open { path, element_id } = project.reply(&element("api"), LINKED_FROM, &options[0].id) else {
        panic!("the backlink opens");
    };
    assert_eq!((path, element_id.as_deref()), (project.path(), Some("l1")));
    assert!(
        !project
            .labels(&element("l3"))
            .contains(&"Open linked element".to_owned())
    );
    let ActionOutcome::Open { path, element_id } = project.run(&element("l2"), OPEN_LINKED) else {
        panic!("the target opens");
    };
    assert_eq!((path, element_id.as_deref()), (project.other(), Some("db")));
}

#[test]
fn add_from_another_document_here_adds_a_linked_node_where_the_canvas_was_clicked() {
    let mut project = Project::new(SERVICES);
    let canvas = ActionTarget::Canvas {
        x: 1500.0,
        y: 8.0 * 60.0,
    };
    let ActionOutcome::Prompt(Prompt::Choice { options, .. }) = project.run(&canvas, "dependencies.add-linked") else {
        panic!("the lookup opens");
    };
    let headings: Vec<&str> = options.iter().map(|o| o.label.as_str()).collect();
    assert_eq!(headings, ["other.dgr", "teams/services.dgr"]);
    let db = &options[0].children[0];
    assert_eq!(db.id, "$o7h3r0ne@other.dgr#db=Orders database");
    assert!(matches!(
        project.reply(&canvas, "dependencies.add-linked", &db.id),
        ActionOutcome::Done
    ));
    let body = project.body();
    assert!(
        body.contains("    row: 6\r\n  - id: n1\r\n    link: $o7h3r0ne@other.dgr#db=Orders database\r\n    x: 1500\r\n    row: 8\r\nrelations:\r\n"),
        "{body}"
    );
    project.workspace.undo();
    assert_eq!(project.body(), SERVICES);
}

#[test]
fn deleting_a_node_others_link_to_says_how_many_links_it_breaks() {
    let mut project = Project::new(SERVICES);
    let ActionOutcome::Prompt(Prompt::Confirm { message, .. }) = project.run(&element("api"), "dependencies.remove")
    else {
        panic!("the delete asks first");
    };
    assert!(message.contains("1 link"), "{message}");
}
