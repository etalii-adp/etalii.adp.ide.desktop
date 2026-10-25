//! The reading of a `bullets` body into its kinds of line (FBL 0.7 section 4.8; the standalone host's
//! `Files/Bullets/{Bullet,BulletsParser,Heading,Columns}` at `a7fdf03`): list items nested by their indentation,
//! headings, statements, and the fences and thematic breaks that are none of these. It mirrors `bullets_of` in
//! etalii.adp's `.github/scripts/validate-examples.py`.

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;

use crate::families::lines::Statement;
use crate::families::{Entry, EntryId, EntryKind, Family};
use crate::text::{BodyText, Span};

static THEMATIC_BREAK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[ \t]*(?:(?:-[ \t]*){3,}|(?:\*[ \t]*){3,}|(?:_[ \t]*){3,})$").expect("the pattern compiles")
});
static FENCE_OPENING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(`{3,}|~{3,})").expect("the pattern compiles"));
static HEADING_MARK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(#{1,6})(?:[ \t]|$)").expect("the pattern compiles"));
static CLOSING_HASHES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:^|[ \t]+)#+$").expect("the pattern compiles"));

/// What a list item of the `bullets` family has besides its statement (FBL 0.7 section 4.8). As a statement its
/// content is the item's text, which a rule with `within` matches; its own span runs from its marker to the end of
/// its last line, and its line span adds the indentation before the marker and the last line's ending.
#[derive(Debug, Clone)]
pub(crate) struct Bullet {
    /// The column of the marker, a tab counting four columns.
    pub marker_column: usize,
    /// The column where the item's text begins, or the marker's column plus two when it has none.
    pub content_column: usize,
    /// The marker: `-`, `*` or `+`.
    pub marker: char,
    /// The whitespace before the marker, as written.
    pub indent_text: String,
    /// The 0-based lines of the continuation: from its first line to its last before the first nested item, blank
    /// lines between them included.
    pub continuation: Vec<usize>,
    /// Which of the continuation's lines are blank lines, read as empty lines.
    blank_lines: HashSet<usize>,
    /// The 0-based line its spans end on: its last continuation line or its last nested item's, whichever is later.
    last_line: usize,
    blanks: Vec<usize>,
    nested: bool,
}

impl Bullet {
    /// A line that is no item and stands while this item is the deepest open one.
    fn add_line(&mut self, line: usize) {
        if !self.nested {
            self.continuation.append(&mut self.blanks);
            self.continuation.push(line);
        }
        self.last_line = line;
    }

    /// A blank line while this item is the deepest open one: part of the continuation only when a line follows it
    /// there.
    fn add_blank(&mut self, line: usize) {
        if self.nested || self.continuation.is_empty() {
            return;
        }
        self.blanks.push(line);
        self.blank_lines.insert(line);
    }

    /// The span the continuation's value is written in (FBL 0.7 section 6.3): from the end of the item line, before
    /// its ending, to the end of the continuation's last line, before its ending; none without continuation.
    pub fn continuation_span(&self, text: &BodyText, first_line: usize) -> Option<Span> {
        let last = *self.continuation.last()?;
        Some(Span::new(
            text.lines()[first_line].content_end,
            text.lines()[last].content_end,
        ))
    }

    /// The continuation as text (FBL 0.7 section 4.8): its lines joined by LF, each without the whitespace before
    /// the content column, a blank line read as an empty one.
    pub fn continuation_text(&self, text: &BodyText) -> String {
        self.continuation
            .iter()
            .map(|&line| {
                if self.blank_lines.contains(&line) {
                    String::new()
                } else {
                    let l = text.lines()[line];
                    columns::without(text.text(l.start, l.content_end), self.content_column)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// A heading of a bullets body (FBL 0.7 section 4.8): its 0-based line, its level and its text.
#[derive(Debug, Clone)]
pub(crate) struct Heading {
    pub line: usize,
    pub level: usize,
    pub text: String,
}

/// Columns as the `bullets` family counts them (FBL 0.7 section 4.8): from 0, a tab four columns wherever it stands.
pub(crate) mod columns {
    /// The columns of a run of spaces and tabs.
    pub fn of(whitespace: &str) -> usize {
        whitespace.chars().map(|c| if c == '\t' { 4 } else { 1 }).sum()
    }

    /// `line` with the whitespace before `column` taken off: a line indented less loses all of it, and a tab the
    /// column cuts into is replaced by the spaces that remain of it.
    pub fn without(line: &str, column: usize) -> String {
        let mut at = 0;
        for (index, c) in line.char_indices() {
            if !matches!(c, ' ' | '\t') || at >= column {
                return line[index..].to_owned();
            }
            let width = if c == '\t' { 4 } else { 1 };
            if at + width > column {
                return format!("{}{}", " ".repeat(at + width - column), &line[index + 1..]);
            }
            at += width;
        }
        String::new()
    }

    /// `line` shifted by `shift` columns: right by writing spaces before it, left by [`without`].
    pub fn shift(line: &str, shift: isize) -> String {
        if shift >= 0 {
            format!("{}{line}", " ".repeat(shift.unsigned_abs()))
        } else {
            without(line, shift.unsigned_abs())
        }
    }

    /// The lines of `text` with their endings, each ending kept with its line.
    pub fn lines(text: &str) -> Vec<(&str, &str)> {
        let bytes = text.as_bytes();
        let mut lines = Vec::new();
        let (mut start, mut i) = (0, 0);
        while i < bytes.len() {
            if !matches!(bytes[i], b'\r' | b'\n') {
                i += 1;
                continue;
            }
            let length = if bytes[i] == b'\r' && bytes.get(i + 1) == Some(&b'\n') {
                2
            } else {
                1
            };
            lines.push((&text[start..i], &text[i..i + length]));
            i += length;
            start = i;
        }
        if start < text.len() {
            lines.push((&text[start..], ""));
        }
        lines
    }

    /// Text written as lines, each after a line ending: `value`'s lines, each indented with spaces to `column`, an
    /// empty line written empty.
    pub fn indented(value: &str, column: usize, newline: &str) -> String {
        let mut out = String::new();
        for line in value.split('\n') {
            out.push_str(newline);
            if !line.is_empty() {
                out.push_str(&" ".repeat(column));
                out.push_str(line);
            }
        }
        out
    }
}

/// What reading a bullets body gives besides its entries: its headings.
pub(crate) struct Parsed {
    pub headings: Vec<Heading>,
    /// Every item and statement, in document order.
    pub statements: Vec<EntryId>,
}

/// An open fence: its character, its length, and the item it stands in.
type Fence = (u8, usize, Option<EntryId>);

impl Family {
    /// Reads the body into items, statements and headings; every byte of a line that is not blank is in a leaf.
    pub(crate) fn read_bullets_body(&mut self) -> Parsed {
        let mut open: Vec<EntryId> = Vec::new();
        let mut fence: Option<Fence> = None;
        let mut parsed = Parsed {
            headings: Vec::new(),
            statements: Vec::new(),
        };
        for index in 0..self.text.lines().len() {
            let line = self.text.lines()[index];
            let start = if index == 0 {
                line.start.max(self.text.bom_length())
            } else {
                line.start
            };
            let mut first = start;
            while first < line.content_end && matches!(self.text.bytes()[first], b' ' | b'\t') {
                first += 1;
            }
            if first < line.content_end {
                self.leaves.push(Span::new(first, line.content_end));
            }
            let rest = self.text.text(first, line.content_end).to_owned();
            if let Some((character, length, owner)) = fence {
                if closes(&rest, character, length) {
                    fence = None;
                }
                if let Some(owner) = owner {
                    self.bullet_mut(owner).add_line(index);
                }
                continue;
            }
            if first == line.content_end {
                if let Some(&deepest) = open.last() {
                    self.bullet_mut(deepest).add_blank(index);
                }
                continue;
            }
            let indentation = columns::of(self.text.text(start, first));
            while open
                .last()
                .is_some_and(|&item| self.bullet(item).marker_column >= indentation)
            {
                open.pop();
            }
            let whole = self.text.text(start, line.content_end).to_owned();
            let r = rest.as_bytes();
            if matches!(r[0], b'-' | b'*' | b'+')
                && (r.len() == 1 || matches!(r[1], b' ' | b'\t'))
                && !THEMATIC_BREAK.is_match(&whole)
            {
                let item = self.bullet_item(index, start, first, indentation, &rest, open.last().copied());
                parsed.statements.push(item);
                open.push(item);
                continue;
            }
            let opens = FENCE_OPENING.captures(&rest).map(|c| (c[1].as_bytes()[0], c[1].len()));
            if let Some(&deepest) = open.last() {
                self.bullet_mut(deepest).add_line(index);
                if let Some((character, length)) = opens {
                    fence = Some((character, length, Some(deepest)));
                }
                continue;
            }
            let heading = if indentation <= 3 {
                HEADING_MARK.captures(&rest).map(|c| c[1].len())
            } else {
                None
            };
            if let Some(level) = heading {
                let title = rest[level..].trim_matches([' ', '\t']);
                let title = CLOSING_HASHES.replace(title, "");
                parsed.headings.push(Heading {
                    line: index,
                    level,
                    text: title.trim_matches([' ', '\t']).to_owned(),
                });
            } else if let Some((character, length)) = opens {
                fence = Some((character, length, None));
            } else if !THEMATIC_BREAK.is_match(&whole) {
                parsed
                    .statements
                    .push(self.bullets_statement(index, start, first, rest));
            }
        }
        let items: Vec<EntryId> = parsed
            .statements
            .iter()
            .copied()
            .filter(|&s| self.entries[s].statement().bullet.is_some())
            .collect();
        for &item in items.iter().rev() {
            self.close_bullet(item);
        }
        parsed
    }

    pub(crate) fn bullet(&self, entry: EntryId) -> &Bullet {
        self.entries[entry]
            .statement()
            .bullet
            .as_deref()
            .expect("the entry is an item")
    }

    fn bullet_mut(&mut self, entry: EntryId) -> &mut Bullet {
        match &mut self.entries[entry].kind {
            EntryKind::Statement(s) => s.bullet.as_deref_mut().expect("the entry is an item"),
            _ => panic!("the entry is not an item"),
        }
    }

    /// Ends the spans at the item's last line, once every line is read; nested items are closed first.
    fn close_bullet(&mut self, item: EntryId) {
        let first_line = self.entries[item].statement().first_line;
        let nested = self.entries[item]
            .children
            .iter()
            .map(|&child| self.bullet(child).last_line)
            .max()
            .unwrap_or(first_line);
        let bullet = self.bullet_mut(item);
        bullet.last_line = bullet.last_line.max(first_line).max(nested);
        let last = self.text.lines()[self.bullet(item).last_line];
        let entry = &mut self.entries[item];
        entry.own = entry.own.with_end(last.content_end);
        entry.line_span = entry.line_span.map(|l| l.with_end(last.end));
    }

    fn bullet_item(
        &mut self,
        index: usize,
        start: usize,
        first: usize,
        indentation: usize,
        rest: &str,
        parent: Option<EntryId>,
    ) -> EntryId {
        let line = self.text.lines()[index];
        let after = &rest[1..];
        let item_text = after.trim_start_matches([' ', '\t']);
        let whitespace = &after[..after.len() - item_text.len()];
        let text_start = first + 1 + whitespace.len();
        let content_column = if item_text.is_empty() {
            indentation + 2
        } else {
            indentation + 1 + columns::of(whitespace)
        };
        let own = Span::new(first, line.content_end);
        self.entries.push(Entry {
            own,
            line_span: Some(Span::new(start, line.end)),
            parent,
            children: Vec::new(),
            name: None,
            indent: first - start,
            kind: EntryKind::Statement(Box::new(Statement {
                first_line: index,
                content: item_text.to_owned(),
                content_start: text_start,
                opens: false,
                is_header: Default::default(),
                first_line_span: Span::new(text_start, line.content_end),
                bullet: Some(Box::new(Bullet {
                    marker_column: indentation,
                    content_column,
                    marker: rest.chars().next().expect("a marker"),
                    indent_text: self.text.text(start, first).to_owned(),
                    continuation: Vec::new(),
                    blank_lines: HashSet::new(),
                    last_line: index,
                    blanks: Vec::new(),
                    nested: false,
                })),
            })),
        });
        let item = self.entries.len() - 1;
        if let Some(parent) = parent {
            self.entries[parent].children.push(item);
            self.bullet_mut(parent).nested = true;
        }
        self.order.push(item);
        item
    }

    fn bullets_statement(&mut self, index: usize, start: usize, first: usize, rest: String) -> EntryId {
        let line = self.text.lines()[index];
        let own = Span::new(first, first + rest.trim_end_matches([' ', '\t']).len());
        self.entries.push(Entry {
            own,
            line_span: Some(Span::new(start, line.end)),
            parent: None,
            children: Vec::new(),
            name: None,
            indent: first - start,
            kind: EntryKind::Statement(Box::new(Statement {
                first_line: index,
                content: rest,
                content_start: first,
                opens: false,
                is_header: Default::default(),
                first_line_span: own,
                bullet: None,
            })),
        });
        let statement = self.entries.len() - 1;
        self.order.push(statement);
        statement
    }
}

/// Whether `rest` closes a fence of `length` `character`s: at least as many, and only whitespace after them.
fn closes(rest: &str, character: u8, length: usize) -> bool {
    let run = rest.bytes().take_while(|&b| b == character).count();
    run >= length && rest[run..].bytes().all(|b| matches!(b, b' ' | b'\t'))
}
