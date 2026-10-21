//! The `.fdg` document: it reads, it splices, and an unchanged document comes back byte for byte, compared as bytes
//! rather than text, since a string comparison passes on a document whose CRLF endings became LF (ported from the
//! standalone host's `FdgDocument.Tests.cs` at `8b6ccfc`).

mod common;

use adp_diagram_functional_decomposition_graph::lines::{LineDocument, LineRange};
use adp_diagram_functional_decomposition_graph::model::{FdgElement, FdgModel, element_types, geometry};
use adp_diagram_functional_decomposition_graph::{parser, writer};
use common::{bytes, load};

const EVERY_FIXTURE: [&str; 5] = [
    "crlf-line-endings.fdg",
    "lf-line-endings.fdg",
    "no-trailing-newline.fdg",
    "malformed-entries.fdg",
    "not-yaml.fdg",
];

#[test]
fn an_unchanged_document_comes_back_byte_for_byte() {
    for fixture in EVERY_FIXTURE {
        let original = bytes(fixture);
        let document = LineDocument::parse(&String::from_utf8(original.clone()).unwrap());
        let _ = parser::parse(&document);
        assert_eq!(original, document.text().into_bytes(), "{fixture}");
    }
}

#[test]
fn the_crlf_and_lf_fixtures_are_still_different_bytes() {
    let crlf = bytes("crlf-line-endings.fdg");
    let lf = bytes("lf-line-endings.fdg");
    assert_ne!(crlf, lf);
    assert!(String::from_utf8(crlf).unwrap().contains("\r\n"));
    assert!(!String::from_utf8(lf).unwrap().contains('\r'));
}

#[test]
fn the_example_reads_every_entry_the_design_states() {
    let model = parser::parse(&load("crlf-line-endings.fdg"));

    assert_eq!(model.version, Some(FdgModel::CURRENT_VERSION));
    let ids: Vec<&str> = model.elements.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, ["task-list", "open-task", "note-offline"]);
    let connections: Vec<&str> = model.connections.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(connections, ["c-open"]);

    let list = &model.elements[0];
    assert_eq!(list.element_type, element_types::UI_ELEMENT);
    assert_eq!(list.name, "Task list");
    assert_eq!((list.x, list.y, list.width), (120.0, 80.0, 160.0));
}

/// Each entry's lines run from its dash to the last line that says something of it: the blank line after the first
/// element is not its own, and neither is the comment two entries above another.
#[test]
fn each_entry_knows_its_own_lines() {
    let model = parser::parse(&load("crlf-line-endings.fdg"));
    let ranges: Vec<LineRange> = model.elements.iter().map(|e| e.range).collect();
    assert_eq!(
        ranges,
        [LineRange::new(3, 9), LineRange::new(11, 16), LineRange::new(17, 25)]
    );
    assert_eq!(model.connections[0].range, LineRange::new(27, 32));
}

#[test]
fn only_a_comment_carries_its_own_height() {
    let model = parser::parse(&load("crlf-line-endings.fdg"));

    let action = model.element_of("open-task").unwrap();
    assert_eq!(action.height, None);
    assert_eq!(action.drawn_height(), geometry::SHARED_HEIGHT);

    let comment = model.element_of("note-offline").unwrap();
    assert_eq!(comment.height, Some(96.0));
    assert_eq!(comment.drawn_height(), 96.0);
}

#[test]
fn a_comments_text_keeps_its_newlines() {
    let model = parser::parse(&load("crlf-line-endings.fdg"));
    let comment = model.element_of("note-offline").unwrap();
    assert_eq!(
        comment.text.replace("\r\n", "\n"),
        "Everything here must work offline.\nSync happens when the device reconnects."
    );
}

#[test]
fn what_the_parser_does_not_understand_survives_and_is_reported() {
    let document = load("malformed-entries.fdg");
    let model = parser::parse(&document);

    let ids: Vec<&str> = model.elements.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, ["good", "odd", "commented"]);
    assert!(document.text().contains("colour: puce"));
    for expected in [
        "not an element type",
        "`colour` is not a key",
        "A Comment has no name",
        "is not a number",
    ] {
        assert!(
            model.problems.iter().any(|p| p.message.contains(expected)),
            "{expected}: {:?}",
            model.problems
        );
    }
}

#[test]
fn a_document_that_is_not_yaml_yields_a_model_and_a_problem() {
    let document = load("not-yaml.fdg");
    let outcome = parser::outcome(&document);

    assert!(outcome.model.elements.is_empty());
    assert!(outcome.model.connections.is_empty());
    assert!(!outcome.model.problems.is_empty());
    assert!(
        outcome
            .unreadable
            .as_deref()
            .is_some_and(|r| r.starts_with("The document could not be read as YAML: "))
    );
    assert!(document.text().contains("bad-indent"));
}

#[test]
fn an_edit_rewrites_only_its_own_entrys_lines() {
    let mut document = load("crlf-line-endings.fdg");
    let before: Vec<String> = document.lines().iter().map(|l| l.text.clone()).collect();
    let model = parser::parse(&document);

    let moved = model.element_of("open-task").unwrap().clone();
    writer::move_to(&mut document, &moved, 300.0, 400.0).unwrap();

    let after: Vec<String> = document.lines().iter().map(|l| l.text.clone()).collect();
    assert_eq!(before.len(), after.len());
    let changed: Vec<usize> = before
        .iter()
        .zip(&after)
        .enumerate()
        .filter(|(_, (b, a))| b != a)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(changed.len(), 2);
    assert!(
        changed
            .iter()
            .all(|line| (moved.range.start..=moved.range.end).contains(line))
    );
    let text = document.text();
    assert!(text.contains("x: 300"));
    assert!(text.contains("y: 400"));
    assert!(text.contains("# Comments and blank lines survive every edit."));
}

#[test]
fn an_edit_on_an_lf_document_does_not_introduce_carriage_returns() {
    let mut document = load("lf-line-endings.fdg");
    let model = parser::parse(&document);
    writer::move_to(&mut document, &model.elements[0], 1.0, 2.0).unwrap();
    assert!(!document.text().contains('\r'));
}

#[test]
fn the_empty_document_is_a_header_and_two_empty_sections() {
    let text = parser::empty_document("\n");
    assert_eq!(
        text,
        "functional-decomposition-graph: 1\nelements: []\nconnections: []\n"
    );

    let model = parser::parse(&LineDocument::parse(&text));
    assert_eq!(model.version, Some(FdgModel::CURRENT_VERSION));
    assert!(model.elements.is_empty());
    assert!(model.connections.is_empty());
    assert!(model.problems.is_empty());
}

#[test]
fn adding_to_an_empty_document_opens_the_flow_section() {
    let mut document = LineDocument::parse(&parser::empty_document("\n"));
    let model = parser::parse(&document);
    let added = FdgElement {
        id: "first".to_owned(),
        element_type: element_types::FUNCTION.to_owned(),
        name: "First".to_owned(),
        description: String::new(),
        text: String::new(),
        x: 10.0,
        y: 20.0,
        width: 100.0,
        height: None,
        range: LineRange::default(),
    };
    writer::add_element(&mut document, &model, &added).unwrap();

    let reread = parser::parse(&LineDocument::parse(&document.text()));
    let ids: Vec<&str> = reread.elements.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, ["first"]);
    assert!(reread.problems.is_empty(), "{:?}", reread.problems);
    assert!(!document.text().contains("elements: []"));
}

#[test]
fn removing_an_element_takes_every_connection_touching_it() {
    let mut document = load("crlf-line-endings.fdg");
    let model = parser::parse(&document);
    let target = model.element_of("open-task").unwrap();
    writer::remove_element(&mut document, &model, target).unwrap();

    let reread = parser::parse(&LineDocument::parse(&document.text()));
    let ids: Vec<&str> = reread.elements.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, ["task-list", "note-offline"]);
    assert!(reread.connections.is_empty());
}

#[test]
fn a_resize_below_the_minimum_is_clamped() {
    let mut document = load("crlf-line-endings.fdg");
    let model = parser::parse(&document);
    writer::resize(&mut document, &model.elements[0], 10.0, None).unwrap();

    let reread = parser::parse(&LineDocument::parse(&document.text()));
    assert_eq!(reread.elements[0].width, geometry::MINIMUM_WIDTH);
}

#[test]
fn a_comment_refuses_a_name_and_the_others_refuse_a_height() {
    let mut document = load("crlf-line-endings.fdg");
    let model = parser::parse(&document);
    let comment = model.element_of("note-offline").unwrap();
    let action = model.element_of("open-task").unwrap();

    let named = writer::set_name(&mut document, comment, "no");
    let heightened = writer::resize(&mut document, action, 200.0, Some(300.0));

    assert!(named.unwrap_err().contains("Comment"));
    assert!(heightened.unwrap_err().contains("share"));
}

/// A Comment's text written as a block scalar reads back with its newlines, and its lines stay inside its entry.
#[test]
fn a_comments_text_is_written_as_a_block_and_reads_back() {
    let mut document = load("lf-line-endings.fdg");
    let model = parser::parse(&document);
    let comment = model.element_of("note-offline").unwrap();
    writer::set_text(&mut document, comment, "Offline first.\nAlways.\nThree lines.").unwrap();

    let reread = parser::parse(&document);
    assert_eq!(
        reread.element_of("note-offline").unwrap().text,
        "Offline first.\nAlways.\nThree lines."
    );
    assert!(reread.problems.is_empty(), "{:?}", reread.problems);
    assert!(
        document
            .text()
            .contains("    text: |-\n      Offline first.\n      Always.\n      Three lines.\n    x: 420\n")
    );
}
