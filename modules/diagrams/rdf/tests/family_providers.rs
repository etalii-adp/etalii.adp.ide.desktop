//! The family's context providers (etalii.adp spec 030, task T083), after the standalone host's
//! `RdfContextActionProvider.Tests.cs`, `RdfContextPropertyProvider.Tests.cs` and `RdfContextSourceResolver.Tests.cs`
//! at `8b6ccfc`: discovery per selection kind, prefix-validated dialogs, every real edit one undo away through the
//! project's history, every edit withheld under truncation with the stated sentence, the rows of a resource, a blank
//! node and an edge, and what a selection reads as. The providers are reached through the view, as the shell reaches
//! them.

mod common;

use std::fs;
use std::path::Path;
use std::rc::Rc;

use adp_context::Prompt;
use adp_core::catalogue::Catalogue;
use adp_diagram::session::{ActionOutcome, ActionTarget};
use adp_diagram::{DiagramSession, DiagramTools, DiagramWorkspace, ModuleView};
use adp_diagram_rdf::{actions, graph, local_name, projection, properties, term_input};
use adp_disl::DislPluginFunctions;

const CORPUS: &str = "@prefix ex: <http://example.org/> .\r\n\r\nex:alice ex:knows ex:bob ;\r\n    ex:name \"Alice\" .\r\n\r\nex:bob ex:name \"Bob\" .\r\n";

const PROPERTY_CORPUS: &str = "@prefix ex: <http://example.org/> .\r\n@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\r\n\r\nex:alice a ex:Person ;\r\n    rdfs:label \"Alice\" .\r\n\r\nex:bob ex:name \"Bob\" .\r\n";

fn tools() -> DiagramTools {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    adp_diagram_rdf::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
        DislPluginFunctions::none()
    }))
}

struct Project {
    _folder: tempfile::TempDir,
    root: std::path::PathBuf,
    workspace: DiagramWorkspace,
    session: DiagramSession,
}

impl Project {
    /// `content` as `graph.ttl`, opened as a bare data graph.
    fn new(content: &str) -> Self {
        let folder = tempfile::tempdir().unwrap();
        let root = folder.path().to_path_buf();
        fs::write(root.join("graph.ttl"), content).unwrap();
        let mut workspace = DiagramWorkspace::new(&root);
        let (_, session) = workspace.open(&mut tools(), Path::new("graph.ttl")).unwrap();
        Project {
            _folder: folder,
            root,
            workspace,
            session,
        }
    }

    fn view(&self) -> Rc<dyn ModuleView> {
        self.session.module_view().unwrap().clone()
    }

    fn body(&self) -> String {
        String::from_utf8(fs::read(self.root.join("graph.ttl")).unwrap()).unwrap()
    }

    fn action_ids(&self, target: ActionTarget) -> Vec<String> {
        self.view()
            .actions(self.workspace.documents(), &target)
            .into_iter()
            .flat_map(|g| g.actions)
            .map(|a| a.id)
            .collect()
    }

    fn execute(&mut self, target: &ActionTarget, action_id: &str) -> ActionOutcome {
        self.view().execute(&mut self.workspace, target, action_id)
    }

    fn answer(&mut self, target: &ActionTarget, action_id: &str, value: &str) -> ActionOutcome {
        self.view().answer(&mut self.workspace, target, action_id, value)
    }

    fn validate(&self, element_id: &str, action_id: &str, value: &str) -> Result<(), String> {
        let document = adp_diagram_rdf::store::get(self.workspace.documents(), &self.root.join("graph.ttl")).unwrap();
        let body = self.root.join("graph.ttl");
        let target = adp_diagram_rdf::family::Target {
            element_id,
            body_path: &body,
            origin: graph::ORIGIN,
        };
        actions::validate(document, &target, action_id, value)
    }

    fn undo(&mut self) {
        assert!(matches!(self.workspace.undo(), adp_history::Outcome::Done { .. }));
    }
}

fn element(id: &str) -> ActionTarget {
    ActionTarget::Element(id.to_owned())
}

#[test]
fn discovery_per_kind_matches_the_selection() {
    let project = Project::new(CORPUS);
    let resource = project.action_ids(element("res:http://example.org/alice"));
    assert!(resource.contains(&actions::RENAME_RESOURCE.to_owned()));
    assert!(resource.contains(&actions::REMOVE_RESOURCE.to_owned()));
    assert!(
        project
            .action_ids(element(
                "edge:res:http://example.org/alice|http://example.org/knows|res:http://example.org/bob"
            ))
            .contains(&actions::REMOVE_EDGE.to_owned())
    );
    assert!(
        project
            .action_ids(ActionTarget::Canvas { x: 100.0, y: 200.0 })
            .contains(&actions::ADD_RESOURCE.to_owned())
    );
    assert!(
        project
            .action_ids(ActionTarget::Connection {
                source: Some("res:http://example.org/alice".to_owned()),
                target: Some("res:http://example.org/bob".to_owned()),
            })
            .contains(&actions::CONNECT.to_owned())
    );
    // A vanished element discovers nothing to do.
    assert!(project.action_ids(element("res:http://example.org/nobody")).is_empty());
}

#[test]
fn the_remove_entry_says_how_many_statements_go_with_it() {
    let project = Project::new(CORPUS);
    let labels: Vec<String> = project
        .view()
        .actions(project.workspace.documents(), &element("res:http://example.org/alice"))
        .into_iter()
        .flat_map(|g| g.actions)
        .map(|a| a.label)
        .collect();
    assert_eq!(labels, ["Rename…", "Remove (with 2 statements)"]);
}

#[test]
fn rename_asks_validates_and_commits_one_undo_away() {
    let mut project = Project::new(CORPUS);
    let target = element("res:http://example.org/bob");
    match project.execute(&target, actions::RENAME_RESOURCE) {
        ActionOutcome::Prompt(Prompt::Input { initial_value, .. }) => assert_eq!(initial_value, "ex:bob"),
        other => panic!("expected an input, got {other:?}"),
    }
    let unknown = project
        .validate("res:http://example.org/bob", actions::RENAME_RESOURCE, "nope:bob")
        .unwrap_err();
    assert!(unknown.contains("nope"));
    assert!(
        project
            .validate("res:http://example.org/bob", actions::RENAME_RESOURCE, "ex:alice")
            .is_err()
    );

    assert_eq!(
        project.answer(&target, actions::RENAME_RESOURCE, "ex:robert"),
        ActionOutcome::Done
    );
    let text = project.body();
    assert!(text.contains("ex:robert ex:name"));
    assert!(!text.contains("ex:bob"));

    project.undo();
    assert_eq!(project.body(), CORPUS);
}

#[test]
fn remove_resource_says_how_many_statements_go_with_it_then_commits() {
    let mut project = Project::new(CORPUS);
    let target = element("res:http://example.org/alice");
    match project.execute(&target, actions::REMOVE_RESOURCE) {
        ActionOutcome::Prompt(Prompt::Confirm { message, danger, .. }) => {
            assert!(message.contains("2 statements"));
            assert!(danger);
        }
        other => panic!("expected a confirmation, got {other:?}"),
    }
    assert_eq!(
        project.answer(&target, actions::REMOVE_RESOURCE, ""),
        ActionOutcome::Done
    );
    assert!(!project.body().contains("alice"));
    project.undo();
    assert_eq!(project.body(), CORPUS);
}

#[test]
fn the_relation_gesture_asks_for_the_predicate_and_states_the_triple() {
    let mut project = Project::new(CORPUS);
    let target = ActionTarget::Connection {
        source: Some("res:http://example.org/bob".to_owned()),
        target: Some("res:http://example.org/alice".to_owned()),
    };
    assert!(matches!(
        project.execute(&target, actions::CONNECT),
        ActionOutcome::Prompt(Prompt::Input { .. })
    ));
    assert_eq!(
        project.answer(&target, actions::CONNECT, "ex:knows"),
        ActionOutcome::Done
    );
    assert!(project.body().contains("ex:knows ex:alice"));
}

#[test]
fn add_resource_at_a_placement_states_the_typed_resource() {
    let mut project = Project::new(CORPUS);
    let target = ActionTarget::Canvas { x: 10.0, y: 20.0 };
    assert_eq!(
        project.answer(&target, actions::ADD_RESOURCE, "ex:carol"),
        ActionOutcome::Done
    );
    // The rdfs namespace is undeclared, so the type is written in full - never invented.
    assert!(
        project
            .body()
            .contains("ex:carol a <http://www.w3.org/2000/01/rdf-schema#Resource> .")
    );
}

#[test]
fn add_prefix_parses_the_one_line_declaration() {
    let mut project = Project::new(CORPUS);
    assert!(
        project
            .validate("new:0,0", actions::ADD_PREFIX, "not a declaration")
            .is_err()
    );
    let target = ActionTarget::Canvas { x: 0.0, y: 0.0 };
    assert_eq!(
        project.answer(&target, actions::ADD_PREFIX, "foaf: <http://xmlns.com/foaf/0.1/>"),
        ActionOutcome::Done
    );
    assert!(project.body().contains("@prefix foaf: <http://xmlns.com/foaf/0.1/> ."));
}

#[test]
fn an_answer_that_resolves_to_nothing_does_not_apply() {
    let mut project = Project::new(CORPUS);
    let target = ActionTarget::Canvas { x: 0.0, y: 0.0 };
    assert_eq!(
        project.answer(&target, actions::ADD_RESOURCE, "nope:carol"),
        ActionOutcome::Refused("'rdf.add-resource' does not apply to this selection.".to_owned())
    );
    assert_eq!(project.body(), CORPUS);
}

#[test]
fn an_action_the_target_does_not_offer_is_not_available() {
    let mut project = Project::new(CORPUS);
    assert_eq!(
        project.execute(&element("res:http://example.org/alice"), actions::ADD_PREFIX),
        ActionOutcome::Refused("That action is not available for this item.".to_owned())
    );
}

#[test]
fn under_truncation_every_edit_is_withheld_with_the_stated_sentence() {
    let mut text = String::from("@prefix ex: <http://example.org/> .\r\n");
    for i in 0..=projection::DEFAULT_BUDGET {
        text.push_str(&format!("ex:s{i} ex:name \"n{i}\" .\r\n"));
    }
    let project = Project::new(&text);
    assert!(project.action_ids(element("res:http://example.org/s0")).is_empty());
    let document = adp_diagram_rdf::store::get(project.workspace.documents(), &project.root.join("graph.ttl")).unwrap();
    let body = project.root.join("graph.ttl");
    let target = adp_diagram_rdf::family::Target {
        element_id: "res:http://example.org/s0",
        body_path: &body,
        origin: graph::ORIGIN,
    };
    match actions::execute(document, &target, actions::REMOVE_RESOURCE) {
        adp_diagram_rdf::family::Run::Refused(message) => assert!(message.contains("withheld")),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn a_resource_shows_identity_read_only_and_its_documentation_editable() {
    let project = Project::new(PROPERTY_CORPUS);
    let rows = project
        .view()
        .properties(project.workspace.documents(), "res:http://example.org/alice");
    let iri = rows.iter().find(|r| r.id == properties::IRI).unwrap();
    assert_eq!(iri.value, "http://example.org/alice");
    assert!(iri.is_read_only());
    assert_eq!(
        rows.iter().find(|r| r.id == properties::TYPES).unwrap().value,
        "ex:Person"
    );
    let label = rows.iter().find(|r| r.id == properties::LABEL).unwrap();
    assert_eq!(label.value, "Alice");
    assert!(!label.is_read_only());
}

#[test]
fn setting_an_existing_label_rewrites_exactly_the_literal_token() {
    let mut project = Project::new(PROPERTY_CORPUS);
    let view = project.view();
    view.set_property(
        &mut project.workspace,
        &["res:http://example.org/alice"],
        properties::LABEL,
        "Alicia",
    )
    .unwrap();
    assert!(project.body().contains("rdfs:label \"Alicia\" ."));
    project.undo();
    assert_eq!(project.body(), PROPERTY_CORPUS);
}

#[test]
fn setting_a_missing_comment_states_the_triple() {
    let mut project = Project::new(PROPERTY_CORPUS);
    let view = project.view();
    view.set_property(
        &mut project.workspace,
        &["res:http://example.org/bob"],
        properties::COMMENT,
        "Who Bob is",
    )
    .unwrap();
    assert!(project.body().contains("rdfs:comment \"Who Bob is\" ."));
}

#[test]
fn a_blank_node_shows_its_boundary_and_an_edge_its_predicate() {
    let mut project = Project::new(&format!("{PROPERTY_CORPUS}_:x ex:name \"Anon\" .\r\n"));
    let view = project.view();
    let blank = view.properties(project.workspace.documents(), "blank:0");
    assert_eq!(blank.len(), 1);
    assert!(blank[0].read_only_reason.contains("blank node"));

    let edge = view.properties(
        project.workspace.documents(),
        "edge:res:http://example.org/alice|http://example.org/knows|res:http://example.org/bob",
    );
    assert!(edge.is_empty());

    assert!(
        view.set_property(&mut project.workspace, &["blank:0"], properties::LABEL, "nope")
            .is_err()
    );
}

#[test]
fn a_statement_edge_shows_its_predicate_read_only() {
    let project = Project::new(CORPUS);
    let rows = project.view().properties(
        project.workspace.documents(),
        "edge:res:http://example.org/alice|http://example.org/knows|res:http://example.org/bob",
    );
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].label, "Predicate");
    assert_eq!(rows[0].value, "ex:knows");
    assert!(rows[0].is_read_only());
}

#[test]
fn a_selection_reads_as_its_prefixed_name() {
    let project = Project::new(&common::fixture("constructs.ttl"));
    let view = project.view();
    let (path, text) = view
        .describe(project.workspace.documents(), "res:http://example.org/alice")
        .unwrap();
    assert_eq!(path, ["ex:alice"]);
    assert_eq!(text, "ex:alice");
    assert!(
        view.describe(project.workspace.documents(), "res:http://example.org/nobody")
            .is_none()
    );
}

#[test]
fn a_term_is_read_the_way_an_author_writes_it() {
    let model = common::model(CORPUS);
    assert_eq!(
        term_input::resolve(&model, " ex:carol ").unwrap(),
        "http://example.org/carol"
    );
    assert_eq!(term_input::resolve(&model, "<urn:isbn:1>").unwrap(), "urn:isbn:1");
    assert_eq!(
        term_input::resolve(&model, "http://example.org/x").unwrap(),
        "http://example.org/x"
    );
    assert_eq!(
        term_input::resolve(&model, "<>").unwrap_err(),
        "The brackets are empty."
    );
    assert_eq!(
        term_input::resolve(&model, "a b").unwrap_err(),
        "An IRI cannot contain spaces."
    );
    assert_eq!(
        term_input::resolve(&model, "carol").unwrap_err(),
        "'carol' is neither a full IRI nor a prefixed name. Write ex:carol, or a full IRI."
    );
}

#[test]
fn a_base_name_folds_to_a_local_name() {
    assert_eq!(local_name("My Graph.data"), "My_Graph_data");
    assert_eq!(local_name("__x__"), "x");
    assert_eq!(local_name("..."), "untitled");
    assert_eq!(local_name("café"), "caf");
}
