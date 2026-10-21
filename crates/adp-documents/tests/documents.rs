//! Text files and tabs (etalii.adp spec 030, Story 1 scenario 5, FR-014, FR-020, task T027), as the standalone host's
//! `TextFileBuffer` tests state them.

use adp_documents::{Closing, Tabs, TextFile};

fn file(bytes: &[u8]) -> (tempfile::TempDir, std::path::PathBuf) {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("notes.md");
    std::fs::write(&path, bytes).unwrap();
    (folder, path)
}

#[test]
fn an_unchanged_file_saves_back_byte_for_byte_mixed_endings_and_bom_included() {
    let original = b"\xEF\xBB\xBFfirst\r\nsecond\nthird\r\nlast";
    let (_folder, path) = file(original);
    let mut text = TextFile::open(&path).unwrap();
    assert_eq!(text.encoding_name(), "UTF-8 with BOM");
    assert_eq!(text.line_ending_style(), "Mixed (mostly CRLF)");
    let content = text.content().to_owned();
    text.save(&content).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), original);
}

#[test]
fn edited_lines_keep_their_own_terminators_and_new_ones_take_the_dominant_style() {
    let (_folder, path) = file(b"a\r\nb\r\nc\n");
    let text = TextFile::open(&path).unwrap();
    assert_eq!(text.line_ending_style(), "Mixed (mostly CRLF)");
    assert_eq!(text.render("A\nb\nc\nd\n"), b"A\r\nb\r\nc\nd\r\n");
    assert_eq!(text.render("a\nb"), b"a\r\nb\r\n");
}

#[test]
fn a_file_without_a_final_newline_still_has_none() {
    let (_folder, path) = file(b"one\ntwo");
    let text = TextFile::open(&path).unwrap();
    assert_eq!(text.line_ending_style(), "LF");
    assert_eq!(text.render("one\ntwo changed"), b"one\ntwo changed");
}

#[test]
fn binary_and_non_utf8_files_are_refused_with_a_reason() {
    let (_a, binary) = file(b"PNG\0\x01\x02");
    assert!(
        TextFile::open(&binary)
            .unwrap_err()
            .contains("looks like a binary file")
    );
    let (_b, latin1) = file(b"caf\xE9\n");
    assert!(TextFile::open(&latin1).unwrap_err().contains("is not UTF-8 or ASCII"));
    let (_c, missing) = file(b"");
    std::fs::remove_file(&missing).unwrap();
    assert_eq!(TextFile::open(&missing).unwrap_err(), "'notes.md' does not exist.");
}

#[test]
fn a_document_opens_in_a_tab_or_focuses_the_tab_it_already_has() {
    let mut tabs = Tabs::default();
    let a = tabs.open("docs/a.md", "editor/markdown");
    let b = tabs.open("b.mm", "freeplane/mindmap");
    assert_eq!(tabs.active().unwrap().id, b);
    assert_eq!(tabs.open("docs/a.md", "editor/markdown"), a);
    assert_eq!(tabs.active().unwrap().id, a);
    // The same file with another tool is another tab.
    assert_ne!(tabs.open("docs/a.md", "editor/plain"), a);
    assert_eq!(tabs.tabs().len(), 3);
}

#[test]
fn a_tab_with_unsaved_changes_shows_it_and_asks_before_closing() {
    let mut tabs = Tabs::default();
    let a = tabs.open("a.md", "editor/markdown");
    let b = tabs.open("b.md", "editor/markdown");
    tabs.set_dirty(a, true);
    assert_eq!(tabs.tabs()[0].title(), "● a.md");
    assert!(matches!(tabs.close(a, false), Closing::Ask(_)));
    assert_eq!(tabs.tabs().len(), 2);
    assert_eq!(tabs.close(a, true), Closing::Closed);
    assert_eq!(tabs.active().unwrap().id, b);
    assert_eq!(tabs.close(b, false), Closing::Closed);
    assert!(tabs.active().is_none());
    assert_eq!(tabs.close(b, false), Closing::NotOpen);
}

#[test]
fn tabs_follow_a_rename_of_their_file_or_folder() {
    let mut tabs = Tabs::default();
    tabs.open("docs/a.md", "editor/markdown");
    tabs.open("docs.md", "editor/markdown");
    tabs.renamed("docs", "notes");
    assert_eq!(tabs.tabs()[0].path, "notes/a.md");
    assert_eq!(tabs.tabs()[1].path, "docs.md");
}
