//! An `.owm` held as its lines, and the 1-based line edits the module makes on it (etalii.adp spec 030, task T070),
//! ported from the standalone host's `WardleyLines.Tests.cs` and `WardleyLineEndings.Tests.cs` at `8b6ccfc`. A line the
//! document does not have is refused with `false` here, where the standalone host threw.

mod common;

use adp_diagram_wardley_map::lines::LineDocument;
use adp_diagram_wardley_map::writer;
use common::{corpus, fixture};

#[test]
fn corpus_is_not_empty_so_an_empty_folder_cannot_look_like_a_passing_suite() {
    assert!(!corpus().is_empty());
}

#[test]
fn to_text_returns_the_input_byte_for_byte_when_nothing_was_edited() {
    for name in corpus() {
        let text = fixture(&name);
        assert_eq!(LineDocument::parse(&text).text(), text, "{name}");
    }
}

#[test]
fn code_lines_are_numbered_from_one() {
    for name in corpus() {
        let document = LineDocument::parse(&fixture(&name));
        for (index, line) in document.lines().iter().enumerate() {
            assert_eq!(document.line_at(index as u32 + 1), Some(line.text.as_str()), "{name}");
        }
        assert_eq!(document.line_at(0), None);
        assert_eq!(document.line_at(document.len() as u32 + 1), None);
    }
}

#[test]
fn parse_preserves_crlf_and_the_lf_pair_is_genuinely_different() {
    let crlf = fixture("crlf-line-endings.owm");
    let lf = fixture("lf-line-endings.owm");
    let from_crlf = LineDocument::parse(&crlf);
    let from_lf = LineDocument::parse(&lf);
    assert_eq!(from_crlf.dominant_ending(), "\r\n");
    assert_eq!(from_lf.dominant_ending(), "\n");
    assert_ne!(crlf, lf);
    let texts = |d: &LineDocument| d.lines().iter().map(|l| l.text.clone()).collect::<Vec<_>>();
    assert_eq!(texts(&from_crlf), texts(&from_lf));
}

#[test]
fn parse_remembers_a_missing_final_newline() {
    let text = fixture("no-trailing-newline.owm");
    let document = LineDocument::parse(&text);
    assert_eq!(document.lines().last().unwrap().ending, "");
    assert_eq!(document.text(), text);
}

#[test]
fn parse_keeps_blank_line_runs_rather_than_compacting_them() {
    let text = fixture("blank-line-runs.owm");
    let document = LineDocument::parse(&text);
    assert!(document.lines().iter().any(|l| l.text.is_empty()));
    assert_eq!(document.text(), text);
}

#[test]
fn replace_line_changes_exactly_one_line_and_leaves_every_other_byte_alone() {
    let mut document = LineDocument::parse(&fixture("comments-everywhere.owm"));
    let before = document.lines().to_vec();
    let target = 6;
    assert!(writer::replace_line(
        &mut document,
        target,
        "component Cup of Tea [0.79, 0.42] // a trailing comment on an element"
    ));
    let after = document.lines().to_vec();
    assert_eq!(before.len(), after.len());
    for index in 0..after.len() {
        if index == target as usize - 1 {
            assert_ne!(before[index], after[index]);
        } else {
            assert_eq!(before[index], after[index]);
        }
    }
}

#[test]
fn replace_line_keeps_the_documents_own_newline_rather_than_the_platforms_one() {
    let mut document = LineDocument::parse(&fixture("crlf-line-endings.owm"));
    writer::replace_line(&mut document, 1, "title Edited");
    assert!(document.text().contains("title Edited\r\n"));
    assert!(!document.text().contains("title Edited\n\n"));
}

#[test]
fn insert_line_places_the_text_at_that_number_and_shifts_the_rest_down() {
    let mut document = LineDocument::parse(&fixture("minimal.owm"));
    let second = document.lines()[1].clone();
    writer::insert_line(&mut document, 2, "// inserted");
    assert_eq!(document.lines()[1].text, "// inserted");
    assert_eq!(document.lines()[2], second);
}

#[test]
fn insert_line_accepts_one_past_the_end_so_appending_is_not_a_special_case() {
    let mut document = LineDocument::parse(&fixture("minimal.owm"));
    let count = document.len();
    assert!(writer::insert_line(&mut document, count as u32 + 1, "// appended"));
    assert_eq!(document.len(), count + 1);
    assert_eq!(document.lines()[count].text, "// appended");
}

#[test]
fn remove_line_removes_that_line_and_shifts_the_rest_up() {
    let mut document = LineDocument::parse(&fixture("tea-shop.owm"));
    let count = document.len();
    let following = document.lines()[5].clone();
    writer::remove_line(&mut document, 5);
    assert_eq!(document.len(), count - 1);
    assert_eq!(document.lines()[4], following);
}

#[test]
fn replace_line_refuses_a_line_the_document_does_not_have() {
    for number in [0, 99] {
        let mut document = LineDocument::parse(&fixture("minimal.owm"));
        let before = document.text();
        assert!(!writer::replace_line(&mut document, number, "x"), "{number}");
        assert_eq!(document.text(), before);
    }
}

// ---- WardleyLineEndings.Tests.cs ------------------------------------------------------------------------------------

#[test]
fn append_to_a_file_with_as_many_lf_as_crlf_ends_the_new_line_with_crlf_and_leaves_the_last_line_alone() {
    let mut document = LineDocument::parse("title Tea\r\nanchor Business [0.95, 0.63]\n");
    writer::append(&mut document, "component Cup [0.79, 0.61]");
    assert_eq!(
        document.text(),
        "title Tea\r\nanchor Business [0.95, 0.63]\ncomponent Cup [0.79, 0.61]\r\n"
    );
}

#[test]
fn append_to_a_single_unterminated_line_ends_that_line_with_crlf_and_leaves_the_new_one_unterminated() {
    let mut document = LineDocument::parse("title Tea");
    writer::append(&mut document, "component Cup [0.79, 0.61]");
    assert_eq!(document.text(), "title Tea\r\ncomponent Cup [0.79, 0.61]");
}

#[test]
fn remove_line_of_an_unterminated_last_line_leaves_the_new_last_line_unterminated() {
    let mut document = LineDocument::parse("title Tea\ncomponent Cup [0.79, 0.61]");
    writer::remove_line(&mut document, 2);
    assert_eq!(document.text(), "title Tea");
}
