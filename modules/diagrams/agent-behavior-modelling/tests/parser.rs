//! The Markdown reader (etalii.adp spec 030, task T074), ported from the standalone host's `AbmParser.Tests.cs` at
//! `8b6ccfc`: the tree with its kinds, labels and places, notes, the one Behavior section, keywords written by hand, an
//! item without a keyword, tab indentation, thematic breaks, and the four examples.

mod common;

use std::collections::BTreeSet;

use adp_diagram_agent_behavior_modelling::kinds;
use adp_diagram_agent_behavior_modelling::lines::LineRange;
use adp_diagram_agent_behavior_modelling::model::{Model, rule_ids};
use common::{EXAMPLES, TestProject, example};

fn parse(lines: &[&str]) -> Model {
    common::parse(&(lines.join("\n") + "\n"))
}

#[test]
fn reads_the_tree_kinds_labels_and_places() {
    let model = parse(&[
        "# Agent",
        "",
        "## Behavior",
        "",
        "- **Try in order:** Help",
        "  - **Check:** The user asked",
        "  - **Do in order:** Work",
        "    - **Do:** Act",
        "    - **Retry up to 2 times:** Test",
        "      - **Ask the user:** Which test?",
    ]);
    let ids: Vec<&str> = model.nodes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, ["1", "1.1", "1.2", "1.2.1", "1.2.2", "1.2.2.1"]);
    let kinds: Vec<&str> = model.nodes.iter().map(|n| n.kind.as_str()).collect();
    assert_eq!(
        kinds,
        [
            kinds::FALLBACK,
            kinds::CHECK,
            kinds::SEQUENCE,
            kinds::ACTION,
            kinds::RETRY,
            kinds::ASK
        ]
    );
    let labels: Vec<&str> = model.nodes.iter().map(|n| n.label.as_str()).collect();
    assert_eq!(labels, ["Help", "The user asked", "Work", "Act", "Test", "Which test?"]);
    assert_eq!(model.node("1.2.2").unwrap().retry_count, 2);
    assert_eq!(model.node("1").unwrap().child_ids, ["1.1", "1.2"]);
    assert_eq!(model.node("1").unwrap().subtree_end, 9);
    assert!(model.problems.is_empty());
}

#[test]
fn reads_notes_without_their_indentation_and_keeps_them_out_of_the_tree() {
    let model = parse(&[
        "## Behavior",
        "- **Do in order:** Work",
        "  First line of the notes.",
        "",
        "    Indented further.",
        "  - **Do:** Act",
    ]);
    let root = model.node("1").unwrap();
    assert_eq!(root.notes, "First line of the notes.\n\n  Indented further.");
    assert_eq!(root.notes_range, Some(LineRange::new(2, 4)));
    assert_eq!(root.child_ids, ["1.1"]);
}

#[test]
fn reads_only_the_behavior_section_and_not_a_heading_inside_a_code_block() {
    let model = parse(&[
        "# Agent",
        "- not a node: the list before the section",
        "```",
        "## Behavior",
        "- **Do:** inside a code block",
        "```",
        "## Behaviour",
        "- **Do:** The real one",
        "## Other",
        "- **Do:** After the section",
    ]);
    assert_eq!(model.nodes.len(), 1);
    assert_eq!(model.nodes[0].label, "The real one");
    assert_eq!(model.section_line, Some(6));
    assert_eq!(model.section_end, 7);
}

#[test]
fn reads_the_keywords_written_by_hand() {
    for line in ["- **Do**: Act", "* **do:** Act", "+ **Do:**Act"] {
        let model = parse(&["## Behavior", line]);
        assert_eq!(model.nodes.len(), 1, "{line}");
        let node = &model.nodes[0];
        assert_eq!(node.kind, kinds::ACTION, "{line}");
        assert_eq!(node.label, "Act", "{line}");
        assert!(node.has_keyword, "{line}");
    }
}

#[test]
fn an_item_without_a_keyword_is_read_as_a_do_and_reported() {
    let model = parse(&["## Behavior", "- Just do the thing"]);
    assert_eq!(model.nodes.len(), 1);
    let node = &model.nodes[0];
    assert_eq!(node.kind, kinds::ACTION);
    assert!(!node.has_keyword);
    assert_eq!(node.label, "Just do the thing");
    assert_eq!(model.problems.len(), 1);
    assert_eq!(model.problems[0].rule_id, rule_ids::NO_KEYWORD);
}

#[test]
fn a_tab_indented_child_is_a_child() {
    let model = parse(&["## Behavior", "- **Do in order:** Work", "\t- **Do:** Act"]);
    assert_eq!(model.node("1.1").unwrap().parent_id.as_deref(), Some("1"));
}

/// A line of three or more marks of one kind, `-`, `*` or `_`, optionally separated by spaces, is a CommonMark
/// thematic break and not a list item, so it is no node.
#[test]
fn a_thematic_break_is_no_node() {
    for line in [
        "---", "- - -", "* * *", "***", "-  -  -", "- - - -", "-\t-\t-", "   * * *",
    ] {
        assert!(parse(&["## Behavior", line]).nodes.is_empty(), "{line:?}");
    }
}

#[test]
fn a_file_without_a_section_has_no_tree() {
    let model = parse(&["# Notes", "- **Do:** Something"]);
    assert_eq!(model.section_line, None);
    assert!(model.nodes.is_empty());
}

#[test]
fn every_example_reads_without_a_problem() {
    for name in EXAMPLES {
        let model = common::parse(&example(name));
        assert!(model.nodes.len() > 5, "{name} read only {} nodes", model.nodes.len());
        let project = TestProject::example(name);
        let problems = project.session.problems(project.workspace.documents());
        assert!(problems.is_empty(), "{name}: {problems:?}");
    }
}

#[test]
fn the_examples_use_every_kind() {
    let used: BTreeSet<String> = EXAMPLES
        .iter()
        .flat_map(|name| common::parse(&example(name)).nodes)
        .map(|node| node.kind)
        .collect();
    let every: BTreeSet<String> = kinds::ALL.iter().map(|k| k.id.to_owned()).collect();
    assert_eq!(used, every);
}
