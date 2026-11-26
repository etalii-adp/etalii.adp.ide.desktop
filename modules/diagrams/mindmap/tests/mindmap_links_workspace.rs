//! A mind map in a workspace of documents that link to each other (etalii.adp spec 089; FBL section 17, mindmap.md
//! section 3.8): opening it checks its links, and what that changes is written with its first edit, in the same step
//! of the history, since opening writes nothing; its links that do not resolve are findings at the nodes that hold
//! them, and a linked node's `self.linked` says whether its link resolves.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use adp_cel::Value;
use adp_context::Prompt;
use adp_diagram::DiagramDocument;
use adp_diagram::document::DiagramChange;
use adp_diagram::link_actions::{LINKED_FROM, OPEN_LINKED};
use adp_diagram::problems::DiagramProblemLocation;
use adp_diagram::session::{ActionOutcome, ActionTarget, DiagramSession};
use adp_diagram::workspace::DiagramWorkspace;
use adp_diagram_mindmap::document::MindmapDocument;
use adp_disl::DislChange;

const OTHER: &str = "<map version=\"freeplane 1.11.5\">\n<node TEXT=\"Other\" ID=\"ID_o1\">\n<attribute NAME=\"adp-id\" VALUE=\"o7h3r0ne\"/>\n<node TEXT=\"Renamed target\" ID=\"ID_o2\"/>\n</node>\n</map>\n";

const IDEAS: &str = "<map version=\"freeplane 1.11.5\">\n<node TEXT=\"Ideas\" ID=\"ID_1\">\n<attribute NAME=\"adp-id\" VALUE=\"m8c1u4wa\"/>\n<node TEXT=\"Launch\" ID=\"ID_2\"/>\n<node TEXT=\"Start\" ID=\"ID_3\">\n<attribute NAME=\"adp-link\" VALUE=\"$m8c1u4wa@ideas.mm#ID_2.text=Start\"/>\n</node>\n<node TEXT=\"Old target\" ID=\"ID_4\">\n<attribute NAME=\"adp-link\" VALUE=\"$o7h3r0ne@other.mm#ID_o2.text=Old target\"/>\n</node>\n<node TEXT=\"Lost\" ID=\"ID_5\">\n<attribute NAME=\"adp-link\" VALUE=\"$qqqqqqqq@nowhere.mm#x1=Lost\"/>\n</node>\n<node TEXT=\"Bad\" ID=\"ID_6\">\n<attribute NAME=\"adp-link\" VALUE=\"other.mm#ID_o2\"/>\n</node>\n<node TEXT=\"Moved\" ID=\"ID_7\">\n<attribute NAME=\"adp-link\" VALUE=\"$o7h3r0ne@old-name.mm#ID_o2.text=Renamed target\"/>\n</node>\n</node>\n</map>\n";

struct Project {
    folder: tempfile::TempDir,
    tools: adp_diagram::DiagramTools,
    workspace: DiagramWorkspace,
    session: DiagramSession,
}

impl Project {
    fn new(ideas: &str) -> Project {
        let folder = tempfile::tempdir().unwrap();
        fs::create_dir_all(folder.path().join("maps")).unwrap();
        fs::write(folder.path().join("maps/ideas.mm"), ideas).unwrap();
        fs::write(folder.path().join("other.mm"), OTHER).unwrap();
        let mut tools = common::tools();
        let mut workspace = DiagramWorkspace::new(folder.path());
        let (_, session) = workspace
            .open(&mut tools, Path::new("maps/ideas.mm"))
            .expect("the map opens");
        Project {
            folder,
            tools,
            workspace,
            session,
        }
    }

    fn path(&self) -> PathBuf {
        std::path::absolute(self.folder.path().join("maps/ideas.mm")).unwrap()
    }

    fn check(&mut self) {
        let path = self.path();
        self.workspace
            .check_links(&mut self.tools, &path)
            .expect("the links are checked");
    }

    /// An edit of the person's own: the central topic renamed.
    fn edit(&mut self) {
        let mut attributes = adp_cel::Map::default();
        attributes.insert("text", Value::String("Ideas!".into()));
        let change = DiagramChange::Disl(DislChange::Set {
            element_id: "ID_1".into(),
            type_name: "Node".into(),
            attributes,
        });
        let edit = self.document().plan("Rename", &[change]).expect("planned");
        self.workspace.execute(edit).expect("the edit is made");
    }

    fn can_undo(&self) -> bool {
        self.workspace.history().availability().can_undo
    }

    fn body(&self) -> String {
        fs::read_to_string(self.path()).unwrap()
    }

    fn map(&self) -> MindmapDocument {
        MindmapDocument::parse(&self.body(), false).unwrap()
    }

    fn document(&self) -> &DiagramDocument {
        self.workspace.documents().get(&self.path()).expect("open")
    }

    fn link_of(&self, id: &str) -> Option<String> {
        let map = self.map();
        map.adp_link(map.find(id).unwrap())
    }

    fn text_of(&self, id: &str) -> String {
        let map = self.map();
        map.text(map.find(id).unwrap())
    }
}

#[test]
fn opening_writes_nothing_and_the_first_edit_rewrites_every_link_that_resolved_otherwise_in_its_own_step() {
    let mut project = Project::new(IDEAS);
    project.check();
    assert_eq!(project.body(), IDEAS, "opening writes nothing");
    assert!(!project.can_undo());
    assert!(project.workspace.holds_link_changes(&project.path()));
    project.edit();
    assert_eq!(project.text_of("ID_1"), "Ideas!");
    // A stale cached name, of a node of its own and of another map's.
    assert_eq!(
        project.link_of("ID_3").as_deref(),
        Some("$m8c1u4wa@ideas.mm#ID_2.text=Launch")
    );
    assert_eq!(project.text_of("ID_3"), "Launch");
    assert_eq!(
        project.link_of("ID_4").as_deref(),
        Some("$o7h3r0ne@other.mm#ID_o2.text=Renamed target")
    );
    assert_eq!(project.text_of("ID_4"), "Renamed target");
    // A renamed file, found by its id.
    assert_eq!(
        project.link_of("ID_7").as_deref(),
        Some("$o7h3r0ne@other.mm#ID_o2.text=Renamed target")
    );
    // What did not resolve is left as written.
    assert_eq!(project.link_of("ID_5").as_deref(), Some("$qqqqqqqq@nowhere.mm#x1=Lost"));
    assert_eq!(project.link_of("ID_6").as_deref(), Some("other.mm#ID_o2"));
    assert!(!project.workspace.holds_link_changes(&project.path()));
    // One step, which undo takes back to the bytes the map had.
    project.workspace.undo();
    assert_eq!(project.body(), IDEAS);
    assert!(!project.can_undo());
    // And redo puts both back.
    project.workspace.redo();
    assert_eq!(project.text_of("ID_1"), "Ideas!");
    assert_eq!(project.text_of("ID_3"), "Launch");
}

#[test]
fn a_map_whose_links_are_current_holds_nothing_and_its_edit_writes_only_itself() {
    let current = IDEAS
        .replace("ID_2.text=Start\"", "ID_2.text=Launch\"")
        .replace(
            "<node TEXT=\"Start\" ID=\"ID_3\">",
            "<node TEXT=\"Launch\" ID=\"ID_3\">",
        )
        .replace("ID_o2.text=Old target\"", "ID_o2.text=Renamed target\"")
        .replace("<node TEXT=\"Old target\"", "<node TEXT=\"Renamed target\"")
        .replace("@old-name.mm#", "@other.mm#");
    let mut project = Project::new(&current);
    project.check();
    assert!(!project.workspace.holds_link_changes(&project.path()));
    project.edit();
    assert_eq!(project.body(), current.replace("TEXT=\"Ideas\"", "TEXT=\"Ideas!\""));
}

#[test]
fn a_map_without_a_document_id_is_given_one_with_its_first_edit() {
    let without = IDEAS.replace("<attribute NAME=\"adp-id\" VALUE=\"m8c1u4wa\"/>\n", "");
    let mut project = Project::new(&without);
    project.check();
    assert_eq!(project.body(), without);
    project.edit();
    let id = project.map().document_id().expect("an id was written");
    assert_eq!(id.len(), 8);
    assert!(id.chars().all(|c| c.is_ascii_digit() || c.is_ascii_lowercase()), "{id}");
    assert!(
        project.body().starts_with(&format!(
            "<map version=\"freeplane 1.11.5\">\n<node TEXT=\"Ideas!\" ID=\"ID_1\">\n<attribute NAME=\"adp-id\" VALUE=\"{id}\"/>\n<node TEXT=\"Launch\" ID=\"ID_2\"/>"
        )),
        "{}",
        project.body()
    );
    assert_eq!(project.text_of("ID_4"), "Renamed target");
    project.workspace.undo();
    assert_eq!(project.body(), without);
}

#[test]
fn the_links_that_do_not_resolve_are_findings_at_the_nodes_that_hold_them() {
    let mut project = Project::new(IDEAS);
    project.check();
    let problems = project.session.problems(project.workspace.documents());
    let links: Vec<(String, String)> = problems
        .iter()
        .filter(|p| p.rule_id.starts_with("link."))
        .map(|p| {
            let at = match &p.location {
                Some(DiagramProblemLocation::Element(id)) => id.clone(),
                other => format!("{other:?}"),
            };
            (at, p.rule_id.clone())
        })
        .collect();
    assert_eq!(
        links,
        [
            ("ID_5".to_owned(), "link.broken-document".to_owned()),
            ("ID_6".to_owned(), "link.malformed".to_owned()),
        ]
    );
}

#[test]
fn a_linked_node_reads_as_linked_while_its_link_resolves_and_not_while_it_does_not() {
    let mut project = Project::new(IDEAS);
    project.check();
    let document = project.document();
    document.bind_views();
    let diagram = &document.model().diagram;
    let linked = |id: &str| diagram.element_by_id(id).unwrap().linked().is_some();
    assert!(linked("ID_3"));
    assert!(linked("ID_4"));
    assert!(!linked("ID_5"));
    assert!(!linked("ID_6"));
}

impl Project {
    fn run(&mut self, node: &str, action: &str) -> ActionOutcome {
        self.session.execute(
            &mut self.workspace,
            &ActionTarget::Element(node.to_owned()),
            action,
            &common::CountingIds::default(),
        )
    }

    fn reply(&mut self, node: &str, action: &str, value: &str) -> ActionOutcome {
        self.session.answer_with_ids(
            &mut self.workspace,
            &ActionTarget::Element(node.to_owned()),
            action,
            value,
            &common::CountingIds::default(),
        )
    }

    fn labels(&self, node: &str) -> Vec<String> {
        self.session
            .actions(self.workspace.documents(), &ActionTarget::Element(node.to_owned()))
            .into_iter()
            .flat_map(|g| g.actions)
            .map(|a| a.label)
            .collect()
    }
}

fn other_path(project: &Project) -> PathBuf {
    std::path::absolute(project.folder.path().join("other.mm")).unwrap()
}

#[test]
fn a_node_others_link_to_lists_them_as_linked_from_and_each_opens_the_node_that_holds_it() {
    let mut project = Project::new(IDEAS);
    project.check();
    assert!(
        project.labels("ID_2").contains(&"Linked from (1)".to_owned()),
        "{:?}",
        project.labels("ID_2")
    );
    assert!(!project.labels("ID_1").iter().any(|l| l.starts_with("Linked from")));
    let ActionOutcome::Prompt(Prompt::Choice { options, .. }) = project.run("ID_2", LINKED_FROM) else {
        panic!("the backlinks are listed");
    };
    assert_eq!(options.len(), 1);
    assert!(options[0].label.starts_with("Start"), "{}", options[0].label);
    let ActionOutcome::Open { path, element_id } = project.reply("ID_2", LINKED_FROM, &options[0].id) else {
        panic!("the backlink opens");
    };
    assert_eq!((path, element_id.as_deref()), (project.path(), Some("ID_3")));
}

#[test]
fn a_linked_node_whose_link_resolves_opens_its_target_and_a_broken_one_offers_nothing_to_open() {
    let mut project = Project::new(IDEAS);
    project.check();
    assert!(project.labels("ID_4").contains(&"Open linked element".to_owned()));
    assert!(!project.labels("ID_5").contains(&"Open linked element".to_owned()));
    let ActionOutcome::Open { path, element_id } = project.run("ID_4", OPEN_LINKED) else {
        panic!("the target opens");
    };
    assert_eq!((path, element_id.as_deref()), (other_path(&project), Some("ID_o2")));
}

#[test]
fn add_from_another_document_lists_the_workspaces_linkable_nodes_and_adds_the_picked_one_as_the_last_child() {
    let mut project = Project::new(IDEAS);
    project.check();
    let ActionOutcome::Prompt(Prompt::Choice { options, .. }) = project.run("ID_1", "mindmap.add-linked") else {
        panic!("the lookup opens");
    };
    let headings: Vec<&str> = options.iter().map(|o| o.label.as_str()).collect();
    assert_eq!(headings, ["maps/ideas.mm", "other.mm"]);
    let target = options[1]
        .children
        .iter()
        .find(|o| o.label == "Renamed target")
        .expect("the other map's node");
    assert_eq!(target.id, "$o7h3r0ne@other.mm#ID_o2=Renamed target");
    assert!(matches!(
        project.reply("ID_1", "mindmap.add-linked", &target.id),
        ActionOutcome::Done
    ));
    let map = project.map();
    let root = map.root();
    let last = *map.children(root).last().unwrap();
    assert_eq!(map.text(last), "Renamed target");
    assert_eq!(map.adp_link(last).as_deref(), Some(target.id.as_str()));
    // The first edit wrote the load-time changes with it, in the same step.
    assert_eq!(project.text_of("ID_3"), "Launch");
    project.workspace.undo();
    assert_eq!(project.body(), IDEAS);
}

#[test]
fn deleting_a_node_others_link_to_says_how_many_links_it_breaks() {
    let mut project = Project::new(IDEAS);
    project.check();
    let ActionOutcome::Prompt(Prompt::Confirm { message, .. }) = project.run("ID_2", "mindmap.delete") else {
        panic!("the delete asks first");
    };
    assert!(message.contains("1 link"), "{message}");
    // A node nothing links to goes without asking, as before.
    assert!(matches!(project.run("ID_6", "mindmap.delete"), ActionOutcome::Done));
}
