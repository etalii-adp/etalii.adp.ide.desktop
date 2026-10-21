//! Editing one argument of one line and leaving every other character exactly as it was, which is what makes a
//! rename a one-line diff (c4-diagrams Requirement 3.2). Ported from the standalone host's
//! `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/C4Tokens.Tests.cs` at `8b6ccfc`.

use adp_diagram_c4::tokens::{replace_argument, split_with_spans};

fn values(line: &str) -> Vec<String> {
    split_with_spans(line).into_iter().map(|t| t.value).collect()
}

#[test]
fn split_with_spans_keeps_a_quoted_argument_whole_however_much_whitespace_it_has() {
    let tokens =
        split_with_spans("        s = softwareSystem \"Internet Banking System\" \"Lets customers view accounts.\"");
    let values: Vec<&str> = tokens.iter().map(|t| t.value.as_str()).collect();
    assert_eq!(
        values,
        [
            "s",
            "=",
            "softwareSystem",
            "Internet Banking System",
            "Lets customers view accounts."
        ]
    );
    assert!(tokens[3].quoted);
    assert!(!tokens[2].quoted);
}

#[test]
fn split_with_spans_reports_positions_that_index_back_into_the_line() {
    let line = "    web = container \"Web App\" \"Serves.\" \"React\"";
    let tokens = split_with_spans(line);
    let name = &tokens[3];
    assert_eq!(&line[name.start..name.start + name.length], "\"Web App\"");
}

#[test]
fn split_with_spans_stops_at_a_comment() {
    assert_eq!(
        values("    u = person \"User\" // the person who uses it"),
        ["u", "=", "person", "User"]
    );
}

#[test]
fn replace_argument_changes_only_that_argument() {
    let line = "    web = container \"Web App\" \"Serves.\" \"React\"";
    let cases = [
        (0, "Renamed", "    web = container \"Renamed\" \"Serves.\" \"React\""),
        (
            1,
            "New text.",
            "    web = container \"Web App\" \"New text.\" \"React\"",
        ),
        (2, "Kotlin", "    web = container \"Web App\" \"Serves.\" \"Kotlin\""),
    ];
    for (argument, value, expected) in cases {
        assert_eq!(replace_argument(line, 2, argument, value), expected);
    }
}

#[test]
fn replace_argument_keeps_the_trailing_brace() {
    assert_eq!(
        replace_argument("    s = softwareSystem \"Old\" \"desc\" {", 2, 0, "New"),
        "    s = softwareSystem \"New\" \"desc\" {"
    );
}

#[test]
fn replace_argument_keeps_a_trailing_comment() {
    assert_eq!(
        replace_argument("    u = person \"User\" \"desc\" // who uses it", 2, 0, "Customer"),
        "    u = person \"Customer\" \"desc\" // who uses it"
    );
}

#[test]
fn replace_argument_appends_an_argument_the_line_does_not_have_yet() {
    assert_eq!(
        replace_argument("    web = container \"Web App\" \"Serves.\"", 2, 2, "React"),
        "    web = container \"Web App\" \"Serves.\" \"React\""
    );
}

#[test]
fn replace_argument_pads_the_gap_so_appended_arguments_keep_their_meaning() {
    assert_eq!(
        replace_argument("    web = container \"Web App\"", 2, 2, "React"),
        "    web = container \"Web App\" \"\" \"React\""
    );
}

#[test]
fn replace_argument_appending_before_a_brace_keeps_the_brace_last() {
    assert_eq!(
        replace_argument("    s = softwareSystem \"System\" {", 2, 1, "A description."),
        "    s = softwareSystem \"System\" \"A description.\" {"
    );
}

#[test]
fn replace_argument_escapes_a_quote_in_the_value() {
    assert_eq!(
        replace_argument("    s = softwareSystem \"System\"", 2, 0, "the \"good\" one"),
        "    s = softwareSystem \"the \\\"good\\\" one\""
    );
}

#[test]
fn replace_argument_on_a_line_with_no_identifier_counts_from_the_keyword() {
    assert_eq!(
        replace_argument("    person \"User\" \"desc\"", 0, 0, "Customer"),
        "    person \"Customer\" \"desc\""
    );
}
