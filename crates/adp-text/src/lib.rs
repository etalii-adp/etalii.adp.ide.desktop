//! The model of the editors' text component (etalii.adp spec 030, research R6, task T029): a rope with one cursor and
//! its selection, editing, moving, undo inside the editor with typing grouped word by word as VS Code groups it, line
//! and column positions, and find. Offsets are in characters. The shell draws it; this crate knows no toolkit.

use ropey::Rope;

/// Where a cursor movement goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Movement {
    Left,
    Right,
    Up,
    Down,
    WordLeft,
    WordRight,
    LineStart,
    LineEnd,
    DocumentStart,
    DocumentEnd,
}

/// One change: `removed` replaced by `inserted` at `at`, with the selection before and after.
#[derive(Debug, Clone)]
struct Edit {
    at: usize,
    removed: String,
    inserted: String,
    before: (usize, usize),
    after: (usize, usize),
}

#[derive(Debug, Clone)]
struct Group {
    id: u64,
    edits: Vec<Edit>,
    /// Whether this group is still collecting typed characters.
    typing: bool,
}

/// An editable text.
#[derive(Debug, Clone)]
pub struct Buffer {
    rope: Rope,
    /// The selection's anchor and head; equal when nothing is selected. The head is the cursor.
    anchor: usize,
    head: usize,
    /// The column a vertical movement aims for.
    goal_column: Option<usize>,
    undo: Vec<Group>,
    redo: Vec<Group>,
    next_id: u64,
    saved: u64,
}

impl Buffer {
    pub fn new(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            anchor: 0,
            head: 0,
            goal_column: None,
            undo: Vec::new(),
            redo: Vec::new(),
            next_id: 1,
            saved: 0,
        }
    }

    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    pub fn len(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    pub fn cursor(&self) -> usize {
        self.head
    }

    /// The selection as (start, end), start ≤ end.
    pub fn selection(&self) -> (usize, usize) {
        (self.anchor.min(self.head), self.anchor.max(self.head))
    }

    pub fn set_cursor(&mut self, offset: usize) {
        let offset = offset.min(self.len());
        self.anchor = offset;
        self.head = offset;
        self.goal_column = None;
        self.end_typing();
    }

    pub fn select(&mut self, anchor: usize, head: usize) {
        self.anchor = anchor.min(self.len());
        self.head = head.min(self.len());
        self.goal_column = None;
        self.end_typing();
    }

    pub fn select_all(&mut self) {
        self.select(0, self.len());
    }

    pub fn selected_text(&self) -> String {
        let (start, end) = self.selection();
        self.rope.slice(start..end).to_string()
    }

    /// Types or pastes text over the selection.
    pub fn insert(&mut self, text: &str) {
        let (start, end) = self.selection();
        let typed = text.chars().count() == 1 && start == end;
        let breaks_word = typed && self.continues_typing(start, text);
        self.replace(start, end, text, typed && breaks_word);
    }

    pub fn delete_backward(&mut self) {
        let (start, end) = self.selection();
        if start != end {
            self.replace(start, end, "", false);
        } else if start > 0 {
            self.replace(start - 1, start, "", false);
        }
    }

    pub fn delete_forward(&mut self) {
        let (start, end) = self.selection();
        if start != end {
            self.replace(start, end, "", false);
        } else if start < self.len() {
            self.replace(start, start + 1, "", false);
        }
    }

    /// Replaces the range with text as one undoable step, leaving the cursor after it.
    pub fn replace_range(&mut self, start: usize, end: usize, text: &str) {
        self.replace(start.min(self.len()), end.min(self.len()), text, false);
    }

    /// Undoes the last group of changes; returns whether there was one.
    pub fn undo(&mut self) -> bool {
        let Some(group) = self.undo.pop() else { return false };
        for edit in group.edits.iter().rev() {
            let end = edit.at + edit.inserted.chars().count();
            self.rope.remove(edit.at..end);
            self.rope.insert(edit.at, &edit.removed);
            (self.anchor, self.head) = edit.before;
        }
        self.redo.push(Group { typing: false, ..group });
        self.goal_column = None;
        true
    }

    /// Redoes the last undone group; returns whether there was one.
    pub fn redo(&mut self) -> bool {
        let Some(group) = self.redo.pop() else { return false };
        for edit in &group.edits {
            let end = edit.at + edit.removed.chars().count();
            self.rope.remove(edit.at..end);
            self.rope.insert(edit.at, &edit.inserted);
            (self.anchor, self.head) = edit.after;
        }
        self.undo.push(group);
        self.goal_column = None;
        true
    }

    /// Whether the text differs from what was last saved, by the history rather than by comparing text.
    pub fn is_modified(&self) -> bool {
        self.undo.last().map_or(0, |g| g.id) != self.saved
    }

    pub fn mark_saved(&mut self) {
        self.saved = self.undo.last().map_or(0, |g| g.id);
        self.end_typing();
    }

    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    /// A line's text without its terminator.
    pub fn line(&self, index: usize) -> String {
        let line = self.rope.line(index).to_string();
        line.trim_end_matches(['\n', '\r']).to_owned()
    }

    /// The (line, column) of an offset, both from 0.
    pub fn position(&self, offset: usize) -> (usize, usize) {
        let offset = offset.min(self.len());
        let line = self.rope.char_to_line(offset);
        (line, offset - self.rope.line_to_char(line))
    }

    /// The offset of a (line, column), the column limited to the line's length.
    pub fn offset(&self, line: usize, column: usize) -> usize {
        let line = line.min(self.line_count().saturating_sub(1));
        self.rope.line_to_char(line) + column.min(self.line(line).chars().count())
    }

    pub fn move_cursor(&mut self, movement: Movement, extend: bool) {
        let (line, column) = self.position(self.head);
        let vertical = matches!(movement, Movement::Up | Movement::Down);
        let goal = self.goal_column.unwrap_or(column);
        let target = match movement {
            Movement::Left => {
                let (start, end) = self.selection();
                if start != end && !extend {
                    start
                } else {
                    self.head.saturating_sub(1)
                }
            }
            Movement::Right => {
                let (start, end) = self.selection();
                if start != end && !extend {
                    end
                } else {
                    (self.head + 1).min(self.len())
                }
            }
            Movement::Up if line == 0 => 0,
            Movement::Up => self.offset(line - 1, goal),
            Movement::Down if line + 1 >= self.line_count() => self.len(),
            Movement::Down => self.offset(line + 1, goal),
            Movement::WordLeft => self.word_left(self.head),
            Movement::WordRight => self.word_right(self.head),
            Movement::LineStart => self.rope.line_to_char(line),
            Movement::LineEnd => self.rope.line_to_char(line) + self.line(line).chars().count(),
            Movement::DocumentStart => 0,
            Movement::DocumentEnd => self.len(),
        };
        self.head = target;
        if !extend {
            self.anchor = target;
        }
        self.goal_column = if vertical { Some(goal) } else { None };
        self.end_typing();
    }

    /// The first match of `query` at or after `from`, wrapping to the start; as (start, end).
    pub fn find(&self, query: &str, from: usize, case_sensitive: bool) -> Option<(usize, usize)> {
        if query.is_empty() {
            return None;
        }
        let fold = |s: &str| if case_sensitive { s.to_owned() } else { s.to_lowercase() };
        let haystack: Vec<char> = fold(&self.text()).chars().collect();
        let needle: Vec<char> = fold(query).chars().collect();
        if needle.len() > haystack.len() {
            return None;
        }
        let matches_at = |i: usize| haystack[i..i + needle.len()] == needle[..];
        let last = haystack.len() - needle.len();
        (from.min(last + 1)..=last)
            .chain(0..from.min(last + 1))
            .find(|&i| matches_at(i))
            .map(|i| (i, i + needle.len()))
    }

    fn replace(&mut self, start: usize, end: usize, text: &str, starts_word: bool) {
        let removed = self.rope.slice(start..end).to_string();
        let before = (self.anchor, self.head);
        self.rope.remove(start..end);
        self.rope.insert(start, text);
        let cursor = start + text.chars().count();
        self.anchor = cursor;
        self.head = cursor;
        self.goal_column = None;
        let edit = Edit {
            at: start,
            removed,
            inserted: text.to_owned(),
            before,
            after: (cursor, cursor),
        };
        let typed = text.chars().count() == 1 && start == end;
        self.redo.clear();
        match self.undo.last_mut() {
            Some(group) if group.typing && typed && !starts_word => group.edits.push(edit),
            _ => {
                let id = self.next_id;
                self.next_id += 1;
                self.undo.push(Group {
                    id,
                    edits: vec![edit],
                    typing: typed,
                });
            }
        }
    }

    /// Whether typing `text` at `at` would start a new word after whitespace, which starts a new undo group, or
    /// does not follow on from the last typed character.
    fn continues_typing(&self, at: usize, text: &str) -> bool {
        let Some(group) = self.undo.last() else { return true };
        let Some(last) = group.edits.last() else { return true };
        if !group.typing || last.at + last.inserted.chars().count() != at {
            return true;
        }
        let previous_is_space = last.inserted.chars().all(char::is_whitespace);
        let this_is_space = text.chars().all(char::is_whitespace);
        previous_is_space && !this_is_space
    }

    fn end_typing(&mut self) {
        if let Some(group) = self.undo.last_mut() {
            group.typing = false;
        }
    }

    fn char_at(&self, offset: usize) -> Option<char> {
        (offset < self.len()).then(|| self.rope.char(offset))
    }

    fn word_right(&self, mut at: usize) -> usize {
        let is_word = |c: char| c.is_alphanumeric() || c == '_';
        while self.char_at(at).is_some_and(is_word) {
            at += 1;
        }
        while self.char_at(at).is_some_and(|c| !is_word(c) && c != '\n') {
            at += 1;
        }
        if at == self.head && self.char_at(at) == Some('\n') {
            at += 1;
        }
        at
    }

    fn word_left(&self, mut at: usize) -> usize {
        let is_word = |c: char| c.is_alphanumeric() || c == '_';
        while at > 0 && self.char_at(at - 1).is_some_and(|c| !is_word(c)) {
            at -= 1;
        }
        while at > 0 && self.char_at(at - 1).is_some_and(is_word) {
            at -= 1;
        }
        at
    }
}
