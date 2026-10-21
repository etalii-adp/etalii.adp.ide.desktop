//! The format: what the parser reads, what it reports instead of failing, and that every write is a splice which
//! leaves every line it did not mean to touch exactly as it was. Ported from the standalone host's
//! `EtAlii.Adp.Diagram.Sankey.Tests/SankeyDocument.Tests.cs` at `8b6ccfc`.

mod common;

use std::fs;

use adp_diagram_sankey::format;
use adp_diagram_sankey::lines::{LineDocument, LineRange};
use adp_diagram_sankey::model::{FROM_TARGET, SankeyNode, SankeySettings};
use adp_diagram_sankey::{parser, writer};
use common::{fixture, parse};

fn load(name: &str) -> LineDocument {
    LineDocument::parse(&fs::read_to_string(fixture(name)).unwrap())
}

const LINE_ENDINGS: [&str; 3] = [
    "crlf-line-endings.skv",
    "lf-line-endings.skv",
    "no-trailing-newline.skv",
];

#[test]
fn an_unchanged_document_comes_back_byte_identical() {
    for name in LINE_ENDINGS {
        let bytes = fs::read(fixture(name)).unwrap();
        let document = LineDocument::parse(&fs::read_to_string(fixture(name)).unwrap());
        parser::parse(&document);
        assert_eq!(bytes, document.text().into_bytes(), "{name}");
    }
}

#[test]
fn the_small_flow_reads_every_entry() {
    for name in LINE_ENDINGS {
        let model = parser::parse(&load(name));
        assert!(model.problems.is_empty(), "{name}: {:?}", model.problems);
        assert_eq!(model.version, Some(1));
        assert_eq!(model.settings.format, "{value} t");
        // Zero-based: `format:` is the third line.
        assert_eq!(model.settings.format_line, Some(2));
        assert_eq!(model.settings.flow_color, FROM_TARGET);
        assert_eq!(model.settings.thickness, 1.0);
        let nodes: Vec<&str> = model.nodes.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(nodes, ["a", "b", "m", "x", "y"]);
        let flows: Vec<String> = model.flows.iter().map(|f| f.id()).collect();
        assert_eq!(flows, ["a->m", "b->m", "m->x", "m->y"]);
        assert_eq!(
            (model.nodes[0].name.as_str(), model.nodes[0].color.as_str()),
            ("Source A", "blue")
        );
        assert_eq!(model.nodes[2].note, "all of it");
        assert_eq!(model.nodes[3].color, "#336699");
        assert_eq!((model.flows[2].value, model.flows[2].step), (Some(7.0), Some(0.5)));
        assert_eq!(model.flows[3].step, None);
    }
}

#[test]
fn malformed_entries_are_reported_and_the_rest_still_reads() {
    let model = parser::parse(&load("malformed-entries.skv"));
    for saying in [
        "`thickness: wide`",
        "`flow-color: sideways`",
        "`colour`",
        "`column: 1.5`",
        "not a mapping",
        "`value: lots`",
    ] {
        assert!(
            model.problems.iter().any(|p| p.message.contains(saying)),
            "{saying}: {:?}",
            model.problems
        );
    }
    let nodes: Vec<&str> = model.nodes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(nodes, ["a", "b"]);
    assert_eq!(model.nodes[1].column, None);
    assert_eq!(model.flows.len(), 1);
    assert_eq!(model.flows[0].value, None);
    assert_eq!(model.settings.thickness, SankeySettings::default().thickness);
    // A value refused is still a key written: the default is kept, and the key's line recorded.
    assert_eq!(model.settings.flow_color, FROM_TARGET);
    assert_eq!(model.settings.flow_color_line, Some(2));
}

#[test]
fn text_that_is_not_yaml_is_one_problem_and_an_empty_model() {
    let outcome = parser::outcome(&load("not-yaml.skv"));
    assert_eq!(outcome.model.problems.len(), 1);
    assert!(
        outcome.model.problems[0].message.contains("could not be read as YAML"),
        "{:?}",
        outcome.model.problems
    );
    assert!(outcome.model.nodes.is_empty());
    assert!(outcome.unreadable.is_some());
}

#[test]
fn a_missing_header_is_reported() {
    let model = parse("nodes: []\n");
    assert!(model.problems.iter().any(|p| p.message.contains("does not begin with")));
}

#[test]
fn setting_a_number_changes_that_line_only() {
    let mut document = load("lf-line-endings.skv");
    let before: Vec<String> = document.lines().iter().map(|l| l.text.clone()).collect();
    let flow = parser::parse(&document).flows[0].clone();
    writer::set_number(&mut document, flow.range, "value", Some(12.5));
    let changed: Vec<usize> = (0..before.len()).filter(|i| before[*i] != document.line(*i)).collect();
    assert_eq!(changed.len(), 1);
    assert_eq!(document.line(changed[0]), "    value: 12.5");
    assert_eq!(parser::parse(&document).flows[0].value, Some(12.5));
}

#[test]
fn moving_a_node_moves_its_lines_beside_the_anchor_comments_included() {
    for (node, anchor, after, expected) in [
        ("y", "x", false, ["a", "b", "m", "y", "x"]),
        ("a", "b", true, ["b", "a", "m", "x", "y"]),
        ("b", "a", false, ["b", "a", "m", "x", "y"]),
        ("a", "y", true, ["b", "m", "x", "y", "a"]),
    ] {
        let mut document = load("lf-line-endings.skv");
        let model = parser::parse(&document);
        let count = document.lines().len();
        let find = |id: &str| model.nodes.iter().find(|n| n.id == id).unwrap().clone();
        writer::move_node(&mut document, &find(node), &find(anchor), after).unwrap();
        assert_eq!(document.lines().len(), count);
        let moved = parser::parse(&document);
        assert!(moved.problems.is_empty());
        let ids: Vec<&str> = moved.nodes.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids, expected, "{node} {anchor} {after}");
        assert!(
            document
                .lines()
                .iter()
                .any(|l| l.text == "    name: Source A   # a comment the writer must keep")
        );
    }
}

#[test]
fn adding_a_node_before_another_puts_it_there_in_the_documents_indentation() {
    let mut document = load("lf-line-endings.skv");
    let model = parser::parse(&document);
    let node = SankeyNode {
        id: "n".to_owned(),
        name: "New: node".to_owned(),
        color: String::new(),
        note: String::new(),
        format: String::new(),
        column: Some(3),
        description: String::new(),
        range: LineRange::default(),
    };
    writer::add_node(&mut document, &model, &node, Some(&model.nodes[3]));
    let added = parser::parse(&document);
    assert!(added.problems.is_empty());
    let ids: Vec<&str> = added.nodes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, ["a", "b", "m", "n", "x", "y"]);
    assert_eq!(
        (added.nodes[3].name.as_str(), added.nodes[3].column),
        ("New: node", Some(3))
    );
}

#[test]
fn a_text_reads_back_as_itself() {
    for value in ["{value} TWh", "[EU]", "yes", "12", "a: b", "#hash", "'quoted'"] {
        let mut document = LineDocument::parse("sankey: 1\nnodes:\n  - id: a\n    name: A\n");
        let node = parser::parse(&document).nodes[0].clone();
        writer::set_text(&mut document, node.range, "note", value, true);
        let read = parser::parse(&document);
        assert!(read.problems.is_empty(), "{value}: {:?}", read.problems);
        assert_eq!(read.nodes[0].note, value);
    }
}

#[test]
fn a_document_wide_key_is_written_under_the_header_when_the_document_states_none() {
    let mut document = LineDocument::parse("sankey: 1\nnodes: []\n");
    writer::set_root_key(&mut document, None, "thickness", "1.25");
    assert_eq!(document.text(), "sankey: 1\nthickness: 1.25\nnodes: []\n");
}

#[test]
fn a_value_is_written_in_its_format() {
    for (format, value, expected) in [
        ("€{value}M", 1302.5, "€1,302.5M"),
        ("({value})", 3.0, "(3)"),
        ("", 0.25, "0.25"),
        ("people", 7.0, "7 people"),
    ] {
        assert_eq!(format::write(format, value), expected);
    }
}
