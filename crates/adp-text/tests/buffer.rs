//! The text component's model (etalii.adp spec 030, research R6, task T029): editing at a cursor or a selection,
//! moving, undo inside the editor with typing grouped, line and column positions, and find.

use adp_text::{Buffer, Movement};

#[test]
fn typing_inserts_at_the_cursor_and_replaces_a_selection() {
    let mut buffer = Buffer::new("hello world");
    buffer.set_cursor(5);
    buffer.insert(",");
    assert_eq!(buffer.text(), "hello, world");
    buffer.select(7, 12);
    buffer.insert("there");
    assert_eq!(buffer.text(), "hello, there");
    assert_eq!(buffer.cursor(), 12);
}

#[test]
fn backspace_and_delete_remove_a_character_or_the_selection() {
    let mut buffer = Buffer::new("abc");
    buffer.set_cursor(3);
    buffer.delete_backward();
    assert_eq!(buffer.text(), "ab");
    buffer.set_cursor(0);
    buffer.delete_forward();
    assert_eq!(buffer.text(), "b");
    buffer.select(0, 1);
    buffer.delete_backward();
    assert_eq!(buffer.text(), "");
    buffer.delete_backward();
    assert_eq!(buffer.text(), "");
}

#[test]
fn typed_characters_undo_as_one_word_and_redo_restores_them() {
    let mut buffer = Buffer::new("");
    for c in ["h", "i", " ", "y", "o", "u"] {
        buffer.insert(c);
    }
    assert_eq!(buffer.text(), "hi you");
    buffer.undo();
    assert_eq!(buffer.text(), "hi ");
    buffer.undo();
    assert_eq!(buffer.text(), "");
    assert!(!buffer.undo());
    buffer.redo();
    buffer.redo();
    assert_eq!(buffer.text(), "hi you");
    assert_eq!(buffer.cursor(), 6);
}

#[test]
fn an_edit_after_undo_discards_redo() {
    let mut buffer = Buffer::new("a");
    buffer.set_cursor(1);
    buffer.insert("b");
    buffer.undo();
    buffer.insert("c");
    assert!(!buffer.redo());
    assert_eq!(buffer.text(), "ac");
}

#[test]
fn positions_convert_between_offsets_and_lines_and_columns() {
    let buffer = Buffer::new("one\r\ntwo\nthree");
    assert_eq!(buffer.line_count(), 3);
    assert_eq!(buffer.position(5), (1, 0));
    assert_eq!(buffer.offset(2, 3), 12);
    assert_eq!(buffer.line(1), "two");
    assert_eq!(buffer.line(0), "one");
}

#[test]
fn the_cursor_moves_by_character_word_line_and_document() {
    let mut buffer = Buffer::new("alpha beta\ngamma");
    buffer.set_cursor(0);
    buffer.move_cursor(Movement::WordRight, false);
    assert_eq!(buffer.cursor(), 6);
    buffer.move_cursor(Movement::Down, false);
    assert_eq!(buffer.position(buffer.cursor()), (1, 5));
    buffer.move_cursor(Movement::LineStart, false);
    assert_eq!(buffer.cursor(), 11);
    buffer.move_cursor(Movement::Left, false);
    assert_eq!(buffer.cursor(), 10);
    buffer.move_cursor(Movement::DocumentStart, true);
    assert_eq!(buffer.selection(), (0, 10));
    buffer.move_cursor(Movement::DocumentEnd, false);
    assert_eq!(buffer.cursor(), 16);
}

#[test]
fn find_wraps_and_honours_case() {
    let buffer = Buffer::new("Rust rust RUST");
    assert_eq!(buffer.find("rust", 0, false), Some((0, 4)));
    assert_eq!(buffer.find("rust", 1, false), Some((5, 9)));
    assert_eq!(buffer.find("rust", 1, true), Some((5, 9)));
    assert_eq!(buffer.find("rust", 6, true), Some((5, 9)));
    assert_eq!(buffer.find("go", 0, false), None);
}

#[test]
fn the_buffer_knows_whether_it_differs_from_what_was_saved() {
    let mut buffer = Buffer::new("x");
    assert!(!buffer.is_modified());
    buffer.insert("y");
    assert!(buffer.is_modified());
    buffer.mark_saved();
    assert!(!buffer.is_modified());
    buffer.undo();
    assert!(buffer.is_modified());
}
