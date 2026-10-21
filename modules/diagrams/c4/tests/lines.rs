//! The textual guarantee everything else rests on: a document ADP did not change comes back byte-identical, and one
//! it did change differs only in the lines the edit touched (c4-diagrams Requirements 3.1, 3.2). Ported from the
//! standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/C4Lines.Tests.cs` at `8b6ccfc`.

use std::fs;
use std::path::Path;

use adp_diagram_c4::lines::{C4Line, LineDocument};

const CORPUS: [&str; 12] = [
    "minimal.dsl",
    "comments-everywhere.dsl",
    "crlf-line-endings.dsl",
    "lf-line-endings.dsl",
    "no-trailing-newline.dsl",
    "unusual-indentation.dsl",
    "unmodelled-constructs.dsl",
    "deployment-nested.dsl",
    "dynamic-interactions.dsl",
    "big-bank-plc.dsl",
    "aws-deployment.dsl",
    "crowded-component-view.dsl",
];

fn read(name: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)).unwrap()
}

#[test]
fn every_fixture_round_trips_byte_for_byte() {
    for name in CORPUS {
        let text = read(name);
        assert_eq!(LineDocument::parse(&text).text(), text, "{name}");
    }
}

#[test]
fn every_fixture_keeps_its_own_line_endings() {
    for name in CORPUS {
        let text = read(name);
        let expected = if text.contains("\r\n") { "\r\n" } else { "\n" };
        assert_eq!(LineDocument::parse(&text).dominant_ending(), expected, "{name}");
    }
}

#[test]
fn a_file_without_a_trailing_newline_still_has_none_after_a_round_trip() {
    let document = LineDocument::parse(&read("no-trailing-newline.dsl"));
    assert_eq!(document.lines().last().unwrap().ending, "");
    assert!(!document.text().ends_with('\n'));
}

#[test]
fn an_edit_changes_only_the_line_it_touches() {
    let text = read("minimal.dsl");
    let mut document = LineDocument::parse(&text);
    let target = document
        .code_lines()
        .into_iter()
        .find(|line| line.code().contains("softwareSystem"))
        .unwrap();
    document
        .replace_line(target.number, &target.text.replace("\"System\"", "\"Renamed\""))
        .unwrap();
    let edited = document.text();
    let before: Vec<&str> = text.split('\n').collect();
    let after: Vec<&str> = edited.split('\n').collect();
    assert_eq!(before.len(), after.len());
    let differing: Vec<_> = before.iter().zip(&after).filter(|(a, b)| a != b).collect();
    assert_eq!(differing.len(), 1);
    assert!(differing[0].1.contains("Renamed"));
}

#[test]
fn degenerate_texts_round_trip_too() {
    for text in ["", "one line", "a\nb", "a\r\nb\r\n", "\n", "a\n\n\nb\n"] {
        assert_eq!(LineDocument::parse(text).text(), text, "{text:?}");
    }
}

#[test]
fn mixed_line_endings_reuse_the_dominant_one_and_the_others_survive_untouched() {
    let text = "a\r\nb\r\nc\nd\r\n";
    let document = LineDocument::parse(text);
    assert_eq!(document.dominant_ending(), "\r\n");
    assert_eq!(document.lines().len(), 4);
    assert_eq!(document.text(), text);
}

#[test]
fn a_document_with_mixed_line_endings_round_trips_byte_for_byte() {
    for text in ["a\r\nb\r\nc\nd\r\n", "a\nb\nc\r\nd\n", "a\r\nb\n", "a\nb\r\n"] {
        assert_eq!(LineDocument::parse(text).text(), text, "{text:?}");
    }
}

#[test]
fn editing_one_line_of_a_mixed_document_leaves_every_other_terminator_alone() {
    let mut document = LineDocument::parse("a\r\nb\nc\r\n");
    document.replace_line(1, "changed").unwrap();
    assert_eq!(document.text(), "changed\r\nb\nc\r\n");
}

#[test]
fn a_line_inserted_into_a_mixed_document_takes_the_dominant_terminator() {
    let mut document = LineDocument::parse("a\r\nb\nc\r\n");
    document.insert_line(2, "inserted").unwrap();
    assert_eq!(document.text(), "a\r\ninserted\r\nb\nc\r\n");
}

#[test]
fn a_lines_code_drops_comments_but_not_what_looks_like_one_inside_a_name() {
    let cases = [
        ("model {", "model {"),
        ("    model { // trailing", "model {"),
        ("  # whole line", ""),
        (
            "person \"A\" \"Uses // not a comment\"",
            "person \"A\" \"Uses // not a comment\"",
        ),
        ("person \"A\" \"A # hash\"", "person \"A\" \"A # hash\""),
        ("url \"https://example.com\"", "url \"https://example.com\""),
    ];
    for (text, expected) in cases {
        let line = C4Line {
            text: text.to_owned(),
            number: 1,
        };
        assert_eq!(line.code(), expected, "{text:?}");
    }
}

#[test]
fn the_comments_fixture_has_code_on_precisely_the_lines_that_are_not_comments() {
    let document = LineDocument::parse(&read("comments-everywhere.dsl"));
    let lines = document.code_lines();
    assert!(lines.first().unwrap().is_blank());
    assert!(lines.iter().rev().find(|l| !l.text.is_empty()).unwrap().is_blank());
    assert!(lines.iter().any(|l| l.code().starts_with("workspace ")));
    assert!(lines.iter().any(|l| l.code() == "model {"));
}
