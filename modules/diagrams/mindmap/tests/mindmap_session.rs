//! A mind map opened through `adp-diagram` with this module: what a node's menu offers and does, what the property
//! grid shows and writes, and every edit one undo away with the bytes put back. Ported from the standalone host's
//! `MindmapContextActionProvider.Tests.cs`, `MindmapContextPropertyProvider.Tests.cs`, `Commands.Tests.cs` and
//! `MindmapValidator.Tests.cs` at `8b6ccfc`.

mod common;

use std::fs;
use std::path::Path;

use adp_context::Prompt;
use adp_diagram::session::ActionOutcome;
use adp_diagram::workspace::DiagramWorkspace;
use adp_diagram::{DiagramProblemLocation, DiagramProblemSeverity};
use adp_diagram_mindmap::actions::{ADD_CHILD, ADD_SIBLING, DELETE, EDIT_NOTES, LINK, RENAME, TOGGLE_FOLD, UNLINK};
use adp_diagram_mindmap::validator;
use adp_history::Outcome;
use common::TestProject;

const ROOT: &str = "ID_1730044821";
const BACKEND: &str = "ID_411002937";
const CONTEXT_SERVICE: &str = "ID_88117420";
const ICOMMAND: &str = "ID_88117422";
const HIERARCHY: &str = "ID_88117425";

fn input(outcome: ActionOutcome) -> (String, String) {
    match outcome {
        ActionOutcome::Prompt(Prompt::Input {
            title, initial_value, ..
        }) => (title, initial_value),
        other => panic!("not an input prompt: {other:?}"),
    }
}

fn texts_under(project: &TestProject, id: &str) -> Vec<String> {
    let document = project.document();
    let node = document.find(id).unwrap();
    document.children(node).into_iter().map(|c| document.text(c)).collect()
}

// ---- what is offered -----------------------------------------------------------------------------------------------

#[test]
fn a_leaf_offers_everything_but_fold_and_unlink() {
    let project = TestProject::new();
    assert_eq!(
        project.offered(ICOMMAND),
        [ADD_CHILD, ADD_SIBLING, RENAME, DELETE, EDIT_NOTES, LINK]
    );
}

#[test]
fn the_root_offers_no_sibling_and_no_delete() {
    let project = TestProject::new();
    let offered = project.offered(ROOT);
    assert!(!offered.iter().any(|a| a == ADD_SIBLING));
    assert!(!offered.iter().any(|a| a == DELETE));
    assert!(offered.iter().any(|a| a == ADD_CHILD));
}

#[test]
fn a_linked_node_offers_unlink() {
    let project = TestProject::new();
    assert!(project.offered(CONTEXT_SERVICE).iter().any(|a| a == UNLINK));
}

#[test]
fn an_unknown_node_offers_nothing() {
    let project = TestProject::new();
    assert!(project.offered("ID_nope").is_empty());
}

// ---- what executing does -------------------------------------------------------------------------------------------

#[test]
fn rename_asks_with_the_current_text() {
    let mut project = TestProject::new();
    let (title, value) = input(project.execute(ICOMMAND, RENAME));
    assert_eq!(title, "Rename node");
    assert_eq!(value, "ICommand");
}

#[test]
fn notes_asks_with_the_current_notes() {
    let mut project = TestProject::new();
    let (title, value) = input(project.execute(ICOMMAND, EDIT_NOTES));
    assert_eq!((title.as_str(), value.as_str()), ("Notes", ""));
}

#[test]
fn add_child_creates_the_node_at_once_named_from_its_siblings_and_its_answer_renames_it() {
    let mut project = TestProject::new();
    let before = texts_under(&project, ROOT).len();
    let (title, name) = input(project.execute(ROOT, ADD_CHILD));
    assert_eq!(title, "Rename node");
    let children = texts_under(&project, ROOT);
    assert_eq!(
        children.len(),
        before + 1,
        "the node is there before the prompt is answered"
    );
    assert_eq!(children.last(), Some(&name));
    assert!(!name.trim().is_empty());
    assert!(project.can_undo());

    assert_eq!(project.answer(ROOT, ADD_CHILD, "Named"), ActionOutcome::Done);
    let children = texts_under(&project, ROOT);
    assert_eq!(
        children.len(),
        before + 1,
        "the answer names the node; it adds no second one"
    );
    assert_eq!(children.last().map(String::as_str), Some("Named"));
}

#[test]
fn add_child_twice_without_renaming_does_not_produce_two_nodes_with_one_name() {
    let mut project = TestProject::new();
    let (_, first) = input(project.execute(ROOT, ADD_CHILD));
    let (_, second) = input(project.execute(ROOT, ADD_CHILD));
    assert_ne!(first, second);
}

#[test]
fn add_sibling_inserts_after_the_node_and_undo_puts_the_bytes_back() {
    let mut project = TestProject::new();
    let before = project.body();
    input(project.execute(CONTEXT_SERVICE, ADD_SIBLING));
    let siblings = texts_under(&project, BACKEND);
    assert_eq!(siblings[0], "Context service");
    assert_eq!(siblings.len(), 4);
    assert!(matches!(project.workspace.undo(), Outcome::Done { .. }));
    assert_eq!(project.body(), before);
}

#[test]
fn delete_on_a_leaf_removes_it_at_once_and_it_is_undoable() {
    let mut project = TestProject::new();
    let before = project.body();
    assert_eq!(project.execute(ICOMMAND, DELETE), ActionOutcome::Done);
    assert!(project.document().find(ICOMMAND).is_none());
    assert!(project.can_undo());
    project.workspace.undo();
    assert_eq!(project.body(), before);
}

#[test]
fn delete_on_a_branch_asks_first_naming_what_goes() {
    let mut project = TestProject::new();
    let ActionOutcome::Prompt(Prompt::Confirm { message, danger, .. }) = project.execute(BACKEND, DELETE) else {
        panic!("a branch is confirmed");
    };
    assert!(message.contains("9 nodes"), "{message}");
    assert!(danger);
    assert!(project.document().find(BACKEND).is_some());
    assert_eq!(project.answer(BACKEND, DELETE, ""), ActionOutcome::Done);
    assert!(project.document().find(BACKEND).is_none());
}

#[test]
fn toggle_fold_changes_view_state_only_and_records_nothing() {
    let mut project = TestProject::new();
    let before = project.body();
    assert_eq!(project.execute(BACKEND, TOGGLE_FOLD), ActionOutcome::Done);
    assert!(!project.can_undo());
    assert_eq!(project.body(), before);
}

#[test]
fn a_folded_branch_is_not_laid_out() {
    let mut project = TestProject::new();
    let placed = |project: &TestProject, id: &str| {
        project
            .session
            .render(project.workspace.documents())
            .unwrap()
            .into_iter()
            .find(|e| e.id == id)
            .is_some_and(|e| e.placed)
    };
    // The file folds Hierarchy, so its children start hidden; Backend's open.
    assert!(!placed(&project, "ID_88117426"));
    assert!(placed(&project, CONTEXT_SERVICE));
    project.execute(BACKEND, TOGGLE_FOLD);
    assert!(!placed(&project, CONTEXT_SERVICE));
    assert!(placed(&project, BACKEND));
}

#[test]
fn the_central_topic_is_at_the_origin() {
    let project = TestProject::new();
    let elements = project.session.render(project.workspace.documents()).unwrap();
    let root = elements.iter().find(|e| e.id == ROOT).unwrap();
    assert!(root.placed);
    assert_eq!((root.x, root.y), (0.0, 0.0));
}

#[test]
fn unlink_is_undoable() {
    let mut project = TestProject::new();
    assert_eq!(project.execute(CONTEXT_SERVICE, UNLINK), ActionOutcome::Done);
    let document = project.document();
    assert_eq!(document.link(document.find(CONTEXT_SERVICE).unwrap()), None);
    project.workspace.undo();
    let document = project.document();
    assert!(document.link(document.find(CONTEXT_SERVICE).unwrap()).is_some());
}

#[test]
fn link_offers_the_projects_files_to_pick_from() {
    let mut project = TestProject::new();
    fs::write(project.root().join("README.md"), "").unwrap();
    let ActionOutcome::Prompt(Prompt::Choice { options, .. }) = project.execute(ICOMMAND, LINK) else {
        panic!("a link is picked");
    };
    assert!(options.iter().any(|o| o.id == "README.md" && o.selectable));
    let docs = options.iter().find(|o| o.id == "docs").expect("the docs folder");
    assert!(docs.children.iter().any(|o| o.id == "docs/architecture.mm"));
}

// ---- what committing does ------------------------------------------------------------------------------------------

#[test]
fn committing_add_child_adds_an_undoable_node() {
    let mut project = TestProject::new();
    assert_eq!(project.answer(ICOMMAND, ADD_CHILD, "a child"), ActionOutcome::Done);
    assert_eq!(texts_under(&project, ICOMMAND), ["a child"]);
    assert!(project.can_undo());
}

#[test]
fn committing_rename_to_empty_is_accepted() {
    let mut project = TestProject::new();
    assert_eq!(project.answer(ICOMMAND, RENAME, ""), ActionOutcome::Done);
    let document = project.document();
    assert_eq!(document.text(document.find(ICOMMAND).unwrap()), "");
}

#[test]
fn committing_link_stores_it_map_relative() {
    let mut project = TestProject::new();
    assert_eq!(
        project.answer(ICOMMAND, LINK, "docs/architecture.mm"),
        ActionOutcome::Done
    );
    let document = project.document();
    assert_eq!(document.link(document.find(ICOMMAND).unwrap()), Some("architecture.mm"));
}

#[test]
fn committing_on_a_node_that_is_gone_fails() {
    let mut project = TestProject::new();
    assert_eq!(
        project.answer("ID_nope", RENAME, "x"),
        ActionOutcome::Refused("The node no longer exists.".into())
    );
}

// ---- the property grid ---------------------------------------------------------------------------------------------

#[test]
fn a_node_offers_its_text_notes_and_link() {
    let project = TestProject::new();
    let rows = project.session.properties(project.workspace.documents(), &[BACKEND]);
    let row = |id: &str| {
        rows.iter()
            .find(|r| r.id == id)
            .unwrap_or_else(|| panic!("no row {id}"))
    };
    assert_eq!(row("mindmap.text").value, "Backend");
    row("mindmap.notes");
    row("mindmap.link");
    let identifier = row("mindmap.identifier");
    assert_eq!(identifier.value, BACKEND);
    assert!(!identifier.read_only_reason.is_empty());
}

#[test]
fn collapsing_is_shown_but_not_editable() {
    let project = TestProject::new();
    let rows = project.session.properties(project.workspace.documents(), &[ROOT]);
    let folded = rows
        .iter()
        .find(|r| r.id == "mindmap.folded")
        .expect("the Collapsed row");
    assert!(
        folded.read_only_reason.contains("view setting"),
        "{}",
        folded.read_only_reason
    );
}

#[test]
fn setting_the_text_changes_the_document_and_is_one_undo_away() {
    let mut project = TestProject::new();
    let before = project.body();
    project
        .session
        .set_property(&mut project.workspace, &[BACKEND], "mindmap.text", "Backend services")
        .unwrap();
    let document = project.document();
    assert_eq!(document.text(document.find(BACKEND).unwrap()), "Backend services");
    project.workspace.undo();
    assert_eq!(project.body(), before);
}

#[test]
fn emptying_the_link_unlinks_the_node() {
    let mut project = TestProject::new();
    project
        .session
        .set_property(&mut project.workspace, &[BACKEND], "mindmap.link", "docs/other.md")
        .unwrap();
    project
        .session
        .set_property(&mut project.workspace, &[BACKEND], "mindmap.link", "")
        .unwrap();
    let document = project.document();
    assert_eq!(document.link(document.find(BACKEND).unwrap()), None);
}

#[test]
fn setting_a_property_of_a_node_that_is_gone_is_refused() {
    let mut project = TestProject::new();
    assert!(
        project
            .session
            .set_property(&mut project.workspace, &["ID_nonexistent"], "mindmap.text", "Anything")
            .is_err()
    );
}

// ---- moves ---------------------------------------------------------------------------------------------------------

#[test]
fn a_move_re_parents_the_branch_and_undo_restores_parent_and_place() {
    let mut project = TestProject::new();
    let before = project.body();
    project
        .session
        .move_element(&mut project.workspace, HIERARCHY, Some("ID_411002938"), 0)
        .unwrap();
    assert_eq!(texts_under(&project, "ID_411002938")[0], "Hierarchy");
    project.workspace.undo();
    assert_eq!(project.body(), before);
}

#[test]
fn a_move_into_its_own_branch_is_refused() {
    let mut project = TestProject::new();
    assert!(
        project
            .session
            .move_element(&mut project.workspace, BACKEND, Some(ICOMMAND), 0)
            .is_err()
    );
}

#[test]
fn a_move_of_the_root_is_refused() {
    let mut project = TestProject::new();
    assert!(
        project
            .session
            .move_element(&mut project.workspace, ROOT, Some(BACKEND), 0)
            .is_err()
    );
}

// ---- a map that is not one, and a host without the reader ----------------------------------------------------------

#[test]
fn an_edit_of_a_map_that_is_not_a_map_is_refused_naming_the_file() {
    let mut project = TestProject::with_body("just some text");
    let ActionOutcome::Refused(refusal) = project.execute(ROOT, RENAME) else {
        panic!("an unreadable map is never edited");
    };
    assert!(refusal.contains("architecture.mm"), "{refusal}");
    let problems = project.session.problems(project.workspace.documents());
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0].rule_id, "mindmap.not-a-map");
}

#[test]
fn without_the_module_the_map_is_unreadable_with_a_finding() {
    let folder = tempfile::tempdir().unwrap();
    fs::write(folder.path().join("map.mm"), common::corpus()).unwrap();
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    let mut tools = adp_diagram::DiagramTools::new(adp_core::Catalogue::load(&vendor), |_| {
        adp_disl::DislPluginFunctions::none()
    });
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (_, session) = workspace.open(&mut tools, Path::new("map.mm")).unwrap();
    let document = workspace.documents().get(&folder.path().join("map.mm")).unwrap();
    assert!(document.refusal().is_some_and(|r| r.contains("could not be read")));
    let problems = session.problems(workspace.documents());
    assert!(
        problems
            .iter()
            .any(|p| p.rule_id == "std.pluginMissing" && p.severity == DiagramProblemSeverity::Error),
        "{problems:?}"
    );
}

// ---- the validator -------------------------------------------------------------------------------------------------

fn validate(text: &str) -> Vec<adp_diagram::DiagramProblem> {
    let mut tools = common::tools();
    let tool = tools.get(adp_diagram_mindmap::ORIGIN).unwrap();
    validator::validate(&tool, text, "architecture.adp")
}

#[test]
fn the_genuine_freeplane_fixture_validates_clean() {
    assert!(validate(&common::corpus()).is_empty());
}

#[test]
fn a_map_that_will_not_parse_is_one_error_naming_the_file() {
    let problems = validate("<map version=\"freeplane 1.12.15\"><node TEXT=\"unclosed\"</map>");
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0].rule_id, "mindmap.not-a-map");
    assert_eq!(problems[0].severity, DiagramProblemSeverity::Error);
    assert!(problems[0].message.contains("architecture"), "{}", problems[0].message);
}

#[test]
fn a_file_that_is_not_a_map_at_all_is_the_same_error() {
    let problems = validate("just some text");
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].rule_id, "mindmap.not-a-map");
}

#[test]
fn an_unnamed_central_topic_is_a_warning_on_the_root() {
    let problems = validate(
        "<map version=\"freeplane 1.12.15\"><node TEXT=\"\" ID=\"ID_1\"><node TEXT=\"a child\" ID=\"ID_2\"/></node></map>",
    );
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0].rule_id, "mindmap.unnamed-root");
    assert_eq!(problems[0].severity, DiagramProblemSeverity::Warning);
    assert_eq!(
        problems[0].location,
        Some(DiagramProblemLocation::Element("ID_1".into()))
    );
}

#[test]
fn an_empty_ordinary_node_is_not_a_problem() {
    let problems = validate(
        "<map version=\"freeplane 1.12.15\"><node TEXT=\"root\" ID=\"ID_1\"><node TEXT=\"\" ID=\"ID_2\"/></node></map>",
    );
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn two_nodes_sharing_an_id_are_an_error() {
    let problems = validate(
        "<map version=\"freeplane 1.12.15\"><node TEXT=\"root\" ID=\"ID_1\"><node TEXT=\"first\" ID=\"ID_2\"/><node TEXT=\"second\" ID=\"ID_2\"/></node></map>",
    );
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0].rule_id, "mindmap.duplicate-id");
    assert_eq!(problems[0].severity, DiagramProblemSeverity::Error);
    for part in ["ID_2", "first", "second"] {
        assert!(problems[0].message.contains(part), "{}", problems[0].message);
    }
}

#[test]
fn several_rules_can_fire_together() {
    let problems = validate(
        "<map version=\"freeplane 1.12.15\"><node TEXT=\"\" ID=\"ID_1\"><node TEXT=\"first\" ID=\"ID_2\"/><node TEXT=\"second\" ID=\"ID_2\"/></node></map>",
    );
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(problems.iter().any(|p| p.rule_id == "mindmap.unnamed-root"));
    assert!(problems.iter().any(|p| p.rule_id == "mindmap.duplicate-id"));
}
