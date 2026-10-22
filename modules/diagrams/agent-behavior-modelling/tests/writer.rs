//! The Markdown writer (etalii.adp spec 030, task T074), ported from the standalone host's `AbmWriter.Tests.cs` and
//! `AbmEmptyDocument.Tests.cs` at `8b6ccfc`: a document nothing edited comes back byte for byte, and every edit is the
//! lines it touches and no others.

mod common;

use adp_diagram_agent_behavior_modelling::kinds;
use adp_diagram_agent_behavior_modelling::lines::LineDocument;
use adp_diagram_agent_behavior_modelling::model::Model;
use adp_diagram_agent_behavior_modelling::{parser, writer};
use common::{EXAMPLES, TestProject, example};

/// The writer tests' tree: CRLF, notes under a leaf, prose on both sides and no final newline (`AbmCorpus.WriterTree`).
pub const TREE: &str = "# Agent\r\n\r\nProse the module never touches.\r\n\r\n## Behavior\r\n\r\n- **Do in order:** Work\r\n  - **Check:** Ready\r\n  - **Try in order:** Options\r\n    - **Do:** First\r\n      Notes for first.\r\n    - **Do:** Second\r\n  - **Do:** Finish\r\n\r\n## After\r\n\r\nMore prose.";

fn load(text: &str) -> (LineDocument, Model) {
    let document = LineDocument::parse(text);
    let model = parser::parse(&document);
    (document, model)
}

#[test]
fn a_document_nothing_edited_comes_back_byte_for_byte() {
    for text in [
        TREE,
        "## Behavior\n- **Do in order:** Work\n  - **Do:** Act\n",
        "## Behavior\n\n- **Do in order:** Work\n\n  - **Do:** Act",
    ] {
        assert_eq!(load(text).0.text(), text);
    }
}

#[test]
fn set_label_rewrites_one_line() {
    let (mut document, model) = load(TREE);
    writer::set_label(&mut document, model.node("1.2.1").unwrap(), "Try the cache\nfirst").unwrap();
    assert_eq!(
        document.text(),
        TREE.replace("**Do:** First", "**Do:** Try the cache first")
    );
}

#[test]
fn set_kind_writes_the_keyword_and_a_retrys_count() {
    let (mut document, model) = load(TREE);
    writer::set_kind(&mut document, model.node("1.2").unwrap(), kinds::PARALLEL, 0).unwrap();
    writer::set_kind(&mut document, model.node("1.3").unwrap(), kinds::ASK, 0).unwrap();
    assert!(document.text().contains("  - **Do together:** Options\r\n"));
    assert!(document.text().contains("  - **Ask the user:** Finish\r\n"));
    assert_eq!(kinds::keyword_of(kinds::RETRY, 1), "Retry up to 1 time");
}

#[test]
fn set_kind_refuses_a_kind_that_cannot_hold_the_children() {
    let (mut document, model) = load(TREE);
    let to_leaf = writer::set_kind(&mut document, model.node("1.2").unwrap(), kinds::ACTION, 0);
    let to_decorator = writer::set_kind(&mut document, model.node("1.2").unwrap(), kinds::GUARD, 0);
    assert!(to_leaf.is_err() && to_decorator.is_err());
    assert_eq!(document.text(), TREE);
}

#[test]
fn set_notes_adds_replaces_and_removes() {
    let (mut document, model) = load(TREE);
    writer::set_notes(
        &mut document,
        model.node("1.3").unwrap(),
        "Say you are done.\n\nThen stop.",
    )
    .unwrap();
    assert!(
        document
            .text()
            .contains("  - **Do:** Finish\r\n    Say you are done.\r\n\r\n    Then stop.\r\n")
    );
    let reread = parser::parse(&document);
    writer::set_notes(&mut document, reread.node("1.2.1").unwrap(), "").unwrap();
    assert!(!document.text().contains("Notes for first."));
    assert_eq!(
        parser::parse(&document).node("1.3").unwrap().notes,
        "Say you are done.\n\nThen stop."
    );
}

#[test]
fn add_puts_a_child_where_its_siblings_are() {
    let (mut document, model) = load(TREE);
    let id = writer::add(&mut document, &model, model.node("1.2"), 1, kinds::CHECK, "Cached", 0).unwrap();
    assert_eq!(id, "1.2.2");
    let reread = parser::parse(&document);
    assert_eq!(reread.node("1.2.2").unwrap().label, "Cached");
    assert_eq!(reread.node("1.2.3").unwrap().label, "Second");
    assert!(
        document
            .text()
            .contains("      Notes for first.\r\n    - **Check:** Cached\r\n")
    );
}

#[test]
fn add_the_first_child_starts_at_the_parents_text_column() {
    let (mut document, model) = load(TREE);
    writer::add(&mut document, &model, model.node("1"), -1, kinds::ACTION, "Last", 0).unwrap();
    let reread = parser::parse(&document);
    let under_a_leaf = writer::add(
        &mut document,
        &reread,
        reread.node("1.3"),
        -1,
        kinds::ACTION,
        "Under a leaf",
        0,
    );
    // Appended after the whole subtree, and a leaf takes no child.
    assert_eq!(parser::parse(&document).node("1.4").unwrap().label, "Last");
    assert!(under_a_leaf.is_err());
}

#[test]
fn add_the_first_root_starts_a_behavior_section_when_there_is_none() {
    let (mut document, model) = load("# Agent\r\n\r\nNo tree yet.\r\n");
    let id = writer::add(
        &mut document,
        &model,
        None,
        -1,
        kinds::SEQUENCE,
        "Handle the request",
        0,
    )
    .unwrap();
    assert_eq!(id, "1");
    assert_eq!(
        document.text(),
        "# Agent\r\n\r\nNo tree yet.\r\n\r\n## Behavior\r\n\r\n- **Do in order:** Handle the request\r\n"
    );
}

#[test]
fn add_the_first_root_goes_under_an_existing_heading() {
    let (mut document, model) = load("## Behavior\n\nNothing yet.\n");
    writer::add(&mut document, &model, None, -1, kinds::ACTION, "Start", 0).unwrap();
    let reread = parser::parse(&document);
    assert_eq!(reread.nodes.len(), 1);
    assert_eq!(reread.nodes[0].label, "Start");
}

#[test]
fn remove_takes_the_subtree_and_its_notes() {
    let (mut document, model) = load(TREE);
    writer::remove(&mut document, model.node("1.2").unwrap()).unwrap();
    assert_eq!(
        document.text(),
        TREE.replace(
            "  - **Try in order:** Options\r\n    - **Do:** First\r\n      Notes for first.\r\n    - **Do:** Second\r\n",
            ""
        )
    );
}

#[test]
fn move_earlier_among_siblings_swaps_the_subtrees() {
    let (mut document, model) = load(TREE);
    writer::move_node(&mut document, &model, model.node("1.2").unwrap(), model.node("1"), 0).unwrap();
    let reread = parser::parse(&document);
    let labels: Vec<&str> = reread
        .children_of(reread.node("1").unwrap())
        .iter()
        .map(|n| n.label.as_str())
        .collect();
    assert_eq!(labels, ["Options", "Ready", "Finish"]);
    assert_eq!(reread.node("1.1.1").unwrap().notes, "Notes for first.");
}

#[test]
fn move_under_another_parent_re_indents_the_subtree() {
    let (mut document, model) = load(TREE);
    writer::move_node(&mut document, &model, model.node("1.3").unwrap(), model.node("1.2"), -1).unwrap();
    assert!(
        document
            .text()
            .contains("    - **Do:** Second\r\n    - **Do:** Finish\r\n")
    );
    let reread = parser::parse(&document);
    writer::move_node(&mut document, &reread, reread.node("1.2").unwrap(), None, -1).unwrap();
    assert!(
        document
            .text()
            .contains("- **Try in order:** Options\r\n  - **Do:** First\r\n    Notes for first.\r\n")
    );
    assert_eq!(parser::parse(&document).roots().len(), 2);
}

#[test]
fn move_beneath_itself_is_refused() {
    let (mut document, model) = load(TREE);
    assert!(writer::move_node(&mut document, &model, model.node("1").unwrap(), model.node("1.2"), -1).is_err());
    assert_eq!(document.text(), TREE);
}

#[test]
fn move_to_where_it_already_is_is_refused() {
    let (mut document, model) = load(TREE);
    assert!(writer::move_node(&mut document, &model, model.node("1.2").unwrap(), model.node("1"), 2).is_err());
}

#[test]
fn a_new_model_has_one_root_and_explains_every_keyword_to_the_agent() {
    let text = writer::empty_document("Release notes writer", "\r\n");
    let model = common::parse(&text);
    assert!(text.starts_with("# Release notes writer\r\n"));
    assert_eq!(model.nodes.len(), 1, "the legend's list is prose, not part of the tree");
    assert_eq!(model.nodes[0].kind, kinds::SEQUENCE);
    for kind in kinds::ALL {
        assert!(text.contains(&format!("- **{}** {}.", kind.keyword, kind.meaning)));
    }
    let project = TestProject::with(&text, "etalii/agent-behavior-modelling\r\n");
    let problems = project.session.problems(project.workspace.documents());
    assert!(
        problems
            .iter()
            .all(|p| p.severity != adp_diagram::DiagramProblemSeverity::Error),
        "{problems:?}"
    );
}

#[test]
fn the_examples_explain_the_keywords_as_a_new_model_does() {
    let legend = writer::legend().join("\r\n");
    for name in EXAMPLES {
        // The examples are committed with LF: the legend is read in the file's own line endings.
        assert!(example(name).replace('\n', "\r\n").contains(&legend), "{name}");
    }
}
