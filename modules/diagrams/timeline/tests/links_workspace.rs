//! A timeline in a workspace of documents that link to each other (etalii.adp spec 089; FBL section 17, timeline.md):
//! its top-level `id` is its document id and an entry with a `link` is a linked period or moment, labelled with its
//! target's name; opening it checks its links and writes nothing, its first edit writes what the check changed, and
//! its menu offers Add from another document here, Linked from (N) and Open linked element.

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

const OTHER: &str = "timeline: 1\r\nid: o7h3r0ne\r\nelements:\r\n  - id: beta\r\n    label: Beta release\r\n    begin: 2026-03-01\r\n    row: 0\r\n";

const RELEASE: &str = "timeline: 1\r\nid: t5h8w1rz\r\nelements:\r\n  - id: discovery\r\n    label: Discovery\r\n    begin: 2026-04-27\r\n    end: 2026-06-05\r\n    row: 1\r\n  - id: launch\r\n    label: Launch\r\n    begin: 2026-12-04\r\n    row: 2\r\n  - id: l1\r\n    link: $t5h8w1rz@release.tml#discovery.label=Start\r\n    begin: 2026-05-01\r\n    end: 2026-05-20\r\n    row: 3\r\n  - id: l2\r\n    link: $o7h3r0ne@other.tml#beta.label=Beta\r\n    begin: 2026-07-01\r\n    row: 4\r\n  - id: l3\r\n    link: $qqqqqqqq@nowhere.tml#x1=Lost\r\n    begin: 2026-08-01\r\n    row: 5\r\nconnections:\r\n  - id: c1\r\n    from: discovery\r\n    to: l2\r\n";

struct Project {
    folder: tempfile::TempDir,
    tools: adp_diagram::DiagramTools,
    workspace: DiagramWorkspace,
    session: DiagramSession,
}

impl Project {
    fn new(release: &str) -> Project {
        let folder = tempfile::tempdir().unwrap();
        fs::create_dir_all(folder.path().join("plan")).unwrap();
        fs::write(folder.path().join("plan/release.tml"), release).unwrap();
        fs::write(folder.path().join("other.tml"), OTHER).unwrap();
        let mut tools = common::tools();
        let mut workspace = DiagramWorkspace::new(folder.path());
        let (_, session) = workspace
            .open(&mut tools, Path::new("plan/release.tml"))
            .expect("the timeline opens");
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
        std::path::absolute(self.folder.path().join("plan/release.tml")).unwrap()
    }

    fn other(&self) -> PathBuf {
        std::path::absolute(self.folder.path().join("other.tml")).unwrap()
    }

    fn body(&self) -> String {
        fs::read_to_string(self.path()).unwrap()
    }

    fn document(&self) -> &DiagramDocument {
        self.workspace.documents().get(&self.path()).expect("open")
    }

    fn rename_launch(&mut self) {
        let mut attributes = adp_cel::Map::default();
        attributes.insert("label", Value::String("Go live".into()));
        let change = adp_diagram::document::DiagramChange::Disl(adp_disl::DislChange::Set {
            element_id: "launch".into(),
            type_name: "Moment".into(),
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
fn a_timeline_reads_its_document_id_and_its_linked_periods_and_moments_labelled_with_their_targets_names() {
    let project = Project::new(RELEASE);
    let document = project.document();
    assert_eq!(document.fbl().document_id.as_deref(), Some("t5h8w1rz"));
    document.bind_views();
    let diagram = &document.model().diagram;
    let of = |id: &str| diagram.element_by_id(id).unwrap();
    assert_eq!(of("l1").type_name(), "LinkedPeriod");
    assert_eq!(of("l2").type_name(), "LinkedMoment");
    assert_eq!(of("launch").type_name(), "Moment");
    // The target's current name, not the stale one the link caches; a broken link keeps the cached one.
    assert_eq!(of("l1").value_of("label"), Value::String("Discovery".into()));
    assert_eq!(of("l2").value_of("label"), Value::String("Beta release".into()));
    assert_eq!(of("l3").value_of("label"), Value::String("Lost".into()));
    assert!(of("l1").linked().is_some());
    assert!(of("l3").linked().is_none());
    // A connection to a linked moment resolves.
    let connection = of("c1");
    assert_eq!(connection.target().map(|t| t.id()).as_deref(), Some("l2"));
}

#[test]
fn opening_writes_nothing_and_the_first_edit_refreshes_the_stale_links_in_the_same_step() {
    let mut project = Project::new(RELEASE);
    assert_eq!(project.body(), RELEASE);
    assert!(project.workspace.holds_link_changes(&project.path()));
    project.rename_launch();
    let expected = RELEASE
        .replace("label: Launch", "label: Go live")
        .replace("#discovery.label=Start", "#discovery.label=Discovery")
        .replace("#beta.label=Beta\r\n", "#beta.label=Beta release\r\n");
    assert_eq!(project.body(), expected);
    project.workspace.undo();
    assert_eq!(project.body(), RELEASE);
}

#[test]
fn a_timeline_without_a_document_id_is_given_one_beside_its_schema_marker_with_its_first_edit() {
    let without = RELEASE.replace("id: t5h8w1rz\r\n", "");
    let mut project = Project::new(&without);
    project.rename_launch();
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
fn a_timeline_without_links_reads_and_writes_as_before() {
    let plain =
        "timeline: 1\r\nelements:\r\n  - id: launch\r\n    label: Launch\r\n    begin: 2026-12-04\r\n    row: 2\r\n";
    let mut timeline = common::open(plain);
    let mut attributes = adp_cel::Map::default();
    attributes.insert("label", Value::String("Go live".into()));
    let change = adp_diagram::document::DiagramChange::Disl(adp_disl::DislChange::Set {
        element_id: "launch".into(),
        type_name: "Moment".into(),
        attributes,
    });
    let path = std::path::absolute(&timeline.body).unwrap();
    let edit = timeline
        .workspace
        .documents()
        .get(&path)
        .unwrap()
        .plan("Rename", &[change])
        .unwrap();
    timeline.workspace.execute(edit).unwrap();
    assert_eq!(timeline.text(), plain.replace("Launch", "Go live"));
}

#[test]
fn the_links_that_do_not_resolve_are_findings_at_the_elements_that_hold_them() {
    let project = Project::new(RELEASE);
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
fn an_element_others_link_to_lists_them_and_a_linked_one_opens_its_target() {
    let mut project = Project::new(RELEASE);
    assert!(
        project
            .labels(&element("discovery"))
            .contains(&"Linked from (1)".to_owned())
    );
    let ActionOutcome::Prompt(Prompt::Choice { options, .. }) = project.run(&element("discovery"), LINKED_FROM) else {
        panic!("the backlinks are listed");
    };
    let ActionOutcome::Open { path, element_id } = project.reply(&element("discovery"), LINKED_FROM, &options[0].id)
    else {
        panic!("the backlink opens");
    };
    assert_eq!((path, element_id.as_deref()), (project.path(), Some("l1")));
    assert!(
        project
            .labels(&element("l2"))
            .contains(&"Open linked element".to_owned())
    );
    assert!(
        !project
            .labels(&element("l3"))
            .contains(&"Open linked element".to_owned())
    );
    let ActionOutcome::Open { path, element_id } = project.run(&element("l2"), OPEN_LINKED) else {
        panic!("the target opens");
    };
    assert_eq!((path, element_id.as_deref()), (project.other(), Some("beta")));
}

#[test]
fn add_from_another_document_here_adds_a_linked_moment_where_the_menu_was_opened() {
    let mut project = Project::new(RELEASE);
    let x = 1_790_000_000.0; // 2026-09-21, some time in the day
    let canvas = ActionTarget::Canvas { x, y: 6.0 * 60.0 };
    let ActionOutcome::Prompt(Prompt::Choice { options, .. }) = project.run(&canvas, "timeline.add-linked") else {
        panic!("the lookup opens");
    };
    let headings: Vec<&str> = options.iter().map(|o| o.label.as_str()).collect();
    assert_eq!(headings, ["other.tml", "plan/release.tml"]);
    let beta = &options[0].children[0];
    assert_eq!(beta.id, "$o7h3r0ne@other.tml#beta.label=Beta release");
    assert!(matches!(
        project.reply(&canvas, "timeline.add-linked", &beta.id),
        ActionOutcome::Done
    ));
    let body = project.body();
    assert!(
        body.ends_with("  - id: n1\r\n    link: $o7h3r0ne@other.tml#beta.label=Beta release\r\n    begin: 2026-09-21\r\n    row: 6\r\nconnections:\r\n  - id: c1\r\n    from: discovery\r\n    to: l2\r\n"),
        "{body}"
    );
    // The stale links were refreshed in the same step.
    assert!(body.contains("#discovery.label=Discovery\r\n"), "{body}");
    project.workspace.undo();
    assert_eq!(project.body(), RELEASE);
}

#[test]
fn deleting_an_element_others_link_to_says_how_many_links_it_breaks() {
    let mut project = Project::new(RELEASE);
    let ActionOutcome::Prompt(Prompt::Confirm { message, .. }) = project.run(&element("discovery"), "timeline.remove")
    else {
        panic!("the delete asks first");
    };
    assert!(message.contains("1 link"), "{message}");
}
