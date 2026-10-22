//! A pipeline as its lines: every fixture back byte for byte, line endings kept, and splices that touch only their
//! own lines (the standalone host's `PipelineDocumentTests`).

mod common;

use std::panic::{AssertUnwindSafe, catch_unwind};

use adp_diagram_azure_devops_pipeline::lines::{LineDocument, LineRange};
use common::{all_fixtures, fixture, fixtures};

fn texts(document: &LineDocument) -> Vec<String> {
    document
        .lines()
        .iter()
        .map(|l| format!("{}{}", l.text, l.ending))
        .collect()
}

#[test]
fn every_fixture_round_trips_byte_for_byte() {
    for name in all_fixtures() {
        let original = std::fs::read(fixtures().join(&name)).unwrap();
        let document = LineDocument::parse(&String::from_utf8(original.clone()).unwrap());
        assert_eq!(original, document.text().into_bytes(), "{name}");
    }
}

#[test]
fn a_crlf_fixture_keeps_its_carriage_returns() {
    let mut document = LineDocument::parse(&fixture("edge-crlf.yml"));
    let count = document.lines().len();
    assert!(document.lines()[..count - 1].iter().all(|l| l.ending == "\r\n"));
    document.insert(1, &["inserted".to_owned()]);
    assert_eq!("\r\n", document.lines()[1].ending);
}

#[test]
fn a_file_without_a_trailing_newline_does_not_gain_one() {
    let text = "trigger:\n  - main";
    let document = LineDocument::parse(text);
    assert_eq!(text, document.text());
    assert_eq!("", document.lines().last().unwrap().ending);
}

#[test]
fn a_file_with_a_trailing_newline_does_not_gain_a_phantom_line() {
    let text = "trigger:\n  - main\n";
    let document = LineDocument::parse(text);
    assert_eq!(2, document.lines().len());
    assert_eq!(text, document.text());
}

#[test]
fn a_mixed_ending_file_keeps_each_line_as_it_was() {
    let text = "a: 1\r\nb: 2\nc: 3\r\n";
    let document = LineDocument::parse(text);
    assert_eq!(text, document.text());
    let endings: Vec<&str> = document.lines().iter().map(|l| l.ending).collect();
    assert_eq!(vec!["\r\n", "\n", "\r\n"], endings);
}

#[test]
fn replace_changes_its_own_lines_and_nothing_else() {
    let mut document = LineDocument::parse("a: 1\nb: 2\nc: 3\n");
    document.replace(LineRange::new(1, 1), &["b: replaced".to_owned()]);
    assert_eq!("a: 1\nb: replaced\nc: 3\n", document.text());
}

#[test]
fn replace_over_several_lines_keeps_the_surrounding_ones() {
    let mut document = LineDocument::parse("keep\nold one\nold two\nkeep too\n");
    document.replace(LineRange::new(1, 2), &["new".to_owned()]);
    assert_eq!("keep\nnew\nkeep too\n", document.text());
}

#[test]
fn replace_of_the_last_line_of_a_file_without_a_trailing_newline_still_has_none() {
    let mut document = LineDocument::parse("a: 1\nb: 2");
    document.replace(LineRange::new(1, 1), &["b: replaced".to_owned()]);
    assert_eq!("a: 1\nb: replaced", document.text());
}

#[test]
fn insert_at_the_end_of_a_file_without_a_trailing_newline_does_not_run_the_lines_together() {
    let mut document = LineDocument::parse("a: 1\nb: 2");
    document.insert(2, &["c: 3".to_owned()]);
    assert_eq!("a: 1\nb: 2\nc: 3", document.text());
}

#[test]
fn appending_then_removing_leaves_an_unterminated_file_exactly_as_it_was() {
    let original = "a: 1\nb: 2";
    let mut document = LineDocument::parse(original);
    document.insert(2, &["c: 3".to_owned(), "d: 4".to_owned()]);
    document.remove(LineRange::new(2, 3));
    assert_eq!(original, document.text());
}

#[test]
fn appending_to_a_terminated_file_keeps_it_terminated() {
    let mut document = LineDocument::parse("a: 1\nb: 2\n");
    document.insert(2, &["c: 3".to_owned()]);
    assert_eq!("a: 1\nb: 2\nc: 3\n", document.text());
}

#[test]
fn insert_adopts_the_documents_own_ending() {
    let mut document = LineDocument::parse("a: 1\r\nb: 2\r\n");
    document.insert(1, &["inserted".to_owned()]);
    assert_eq!("a: 1\r\ninserted\r\nb: 2\r\n", document.text());
}

#[test]
fn remove_takes_its_lines_and_leaves_the_rest() {
    let mut document = LineDocument::parse("a: 1\nb: 2\nc: 3\n");
    document.remove(LineRange::new(1, 1));
    assert_eq!("a: 1\nc: 3\n", document.text());
}

#[test]
fn a_splice_into_a_fixture_leaves_every_other_byte_alone() {
    let mut document = LineDocument::parse(&fixture("edge-comments.yml"));
    let target = document
        .lines()
        .iter()
        .position(|l| l.text.contains("displayName: Build"))
        .unwrap();
    let before = texts(&document);
    document.replace(
        LineRange::new(target, target),
        &["          displayName: Renamed   # trailing again".to_owned()],
    );
    let after = texts(&document);
    assert_eq!(before.len(), after.len());
    for index in 0..before.len() {
        if index == target {
            assert_ne!(before[index], after[index]);
        } else {
            assert_eq!(before[index], after[index]);
        }
    }
}

#[test]
fn replace_outside_the_document_is_refused() {
    let mut document = LineDocument::parse("a: 1\n");
    let refused = catch_unwind(AssertUnwindSafe(|| {
        document.replace(LineRange::new(5, 5), &["x".to_owned()]);
    }));
    assert!(refused.is_err());
}

#[test]
fn a_tied_file_gives_an_inserted_line_crlf() {
    let mut document = LineDocument::parse(&fixture("edge-tied-endings.yml"));
    let endings: Vec<&str> = document.lines().iter().map(|l| l.ending).collect();
    assert_eq!(
        endings.iter().filter(|e| **e == "\n").count(),
        endings.iter().filter(|e| **e == "\r\n").count()
    );
    document.insert(1, &["  - release/*".to_owned()]);
    assert_eq!("\r\n", document.lines()[1].ending);
}

#[test]
fn a_range_ending_the_line_before_it_starts_is_refused() {
    let text = "a: 1\nb: 2\nc: 3\n";
    let mut document = LineDocument::parse(text);
    let replaced = catch_unwind(AssertUnwindSafe(|| {
        document.replace(LineRange::new(2, 1), &["x: 9".to_owned()]);
    }));
    assert!(replaced.is_err());
    let removed = catch_unwind(AssertUnwindSafe(|| document.remove(LineRange::new(2, 1))));
    assert!(removed.is_err());
    assert_eq!(text, document.text());
}
