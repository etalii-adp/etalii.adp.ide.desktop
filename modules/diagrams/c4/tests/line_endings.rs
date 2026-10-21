//! How C4's edits end their lines, read through the line document the standalone host's C4 module moved onto in
//! code-structure T128 (behaviour changes B-05 and B-16). Ported from the standalone host's
//! `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/C4LineEndings.Tests.cs` at `8b6ccfc`.

use adp_diagram_c4::lines::LineDocument;

#[test]
fn a_file_with_equal_counts_of_lf_and_crlf_round_trips_and_a_new_line_takes_crlf() {
    let text = "workspace {\n    model {\r\n        user = person \"User\"\n    }\r\n";
    let mut document = LineDocument::parse(text);
    let unchanged = document.text();
    document
        .insert_line(4, "        system = softwareSystem \"System\"")
        .unwrap();
    assert_eq!(unchanged, text);
    assert_eq!(
        document.text(),
        "workspace {\n    model {\r\n        user = person \"User\"\n        system = softwareSystem \"System\"\r\n    }\r\n"
    );
}

#[test]
fn removing_an_unterminated_last_line_leaves_the_file_unterminated() {
    let mut document = LineDocument::parse("workspace {\r\n}\r\nviews");
    document.remove_lines(3, 3).unwrap();
    assert_eq!(document.text(), "workspace {\r\n}");
}

#[test]
fn a_file_with_no_line_breaks_takes_crlf_whatever_the_platform() {
    let mut document = LineDocument::parse("workspace {");
    document.insert_line(2, "}").unwrap();
    assert_eq!(document.text(), "workspace {\r\n}");
}
