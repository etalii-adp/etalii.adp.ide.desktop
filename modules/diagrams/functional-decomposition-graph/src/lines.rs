//! A hand-authored `.fdg` as the lines it is made of, edits applied by splicing one range of them, and the splicing
//! helpers its writer uses (etalii.adp spec 030, task T072).
//!
//! Ported from the standalone host's core at `8b6ccfc`: `src/backend/EtAlii.Adp.Documents/LineDocument.cs`,
//! `LineRange.cs`, `LineSplice.cs` and `DslNumber.cs`. Every line keeps its own terminator, so a document nothing has
//! spliced is its own bytes again; a line spliced in takes the terminator most of the document's lines use, a tie
//! (an empty document included) going to CRLF, the standalone repository's house style. Nothing here reads YAML: the
//! parser records which lines declare what, and the writer splices those ranges.

/// One line: its text without its terminator, and the terminator (`\n`, `\r\n`, or none for a last line without one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    pub ending: &'static str,
}

/// An inclusive range of zero-based line indices: the lines that declare one entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LineRange {
    pub start: usize,
    pub end: usize,
}

impl LineRange {
    pub fn new(start: usize, end: usize) -> Self {
        LineRange { start, end }
    }

    /// How many lines the range covers.
    pub fn len(&self) -> usize {
        self.end + 1 - self.start
    }

    /// Never empty: a range covers at least its first line.
    pub fn is_empty(&self) -> bool {
        false
    }
}

/// A document as its lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineDocument {
    lines: Vec<Line>,
    dominant_ending: &'static str,
}

impl LineDocument {
    /// Splits `text` into lines, each keeping its own terminator. A file that ends without a newline gives a last
    /// line with no terminator; one that ends with a newline gains no empty line.
    pub fn parse(text: &str) -> LineDocument {
        let mut lines = Vec::new();
        let (mut crlf, mut lf) = (0, 0);
        let mut start = 0;
        let bytes = text.as_bytes();
        for (index, byte) in bytes.iter().enumerate() {
            if *byte != b'\n' {
                continue;
            }
            let carriage_return = index > start && bytes[index - 1] == b'\r';
            let end = if carriage_return { index - 1 } else { index };
            lines.push(Line {
                text: text[start..end].to_owned(),
                ending: if carriage_return { "\r\n" } else { "\n" },
            });
            if carriage_return {
                crlf += 1;
            } else {
                lf += 1;
            }
            start = index + 1;
        }
        if start < text.len() {
            lines.push(Line {
                text: text[start..].to_owned(),
                ending: "",
            });
        }
        LineDocument {
            lines,
            dominant_ending: if lf > crlf { "\n" } else { "\r\n" },
        }
    }

    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    /// The terminator a newly inserted line adopts.
    pub fn dominant_ending(&self) -> &'static str {
        self.dominant_ending
    }

    /// The document as text: byte for byte what [`LineDocument::parse`] was given, for a document nothing spliced.
    pub fn text(&self) -> String {
        let mut text = String::new();
        for line in &self.lines {
            text.push_str(&line.text);
            text.push_str(line.ending);
        }
        text
    }

    fn guard(&self, range: LineRange) -> Result<(), String> {
        if range.end < range.start {
            return Err(format!("Line {} cannot precede line {}.", range.end, range.start));
        }
        if range.end >= self.lines.len() {
            return Err(format!(
                "Lines {}-{} are outside a document of {} lines.",
                range.start,
                range.end,
                self.lines.len()
            ));
        }
        Ok(())
    }

    /// Replaces the lines in `range` with `replacement`; the replacement's last line inherits the terminator the
    /// range's last line had.
    pub fn replace(&mut self, range: LineRange, replacement: &[String]) -> Result<(), String> {
        self.guard(range)?;
        let ending = self.lines[range.end].ending;
        let count = replacement.len();
        let lines: Vec<Line> = replacement
            .iter()
            .enumerate()
            .map(|(index, text)| Line {
                text: text.clone(),
                ending: if index == count - 1 {
                    ending
                } else {
                    self.dominant_ending
                },
            })
            .collect();
        self.lines.splice(range.start..=range.end, lines);
        Ok(())
    }

    /// Inserts lines before `index`. Appending to a last line without a terminator gives that line one and the new
    /// last line inherits the missing terminator, so an append and its undo come back byte for byte.
    pub fn insert(&mut self, index: usize, lines: &[String]) -> Result<(), String> {
        if index > self.lines.len() {
            return Err(format!(
                "Line {index} is outside a document of {} lines.",
                self.lines.len()
            ));
        }
        let appending_to_unterminated =
            index == self.lines.len() && self.lines.last().is_some_and(|l| l.ending.is_empty());
        if appending_to_unterminated && let Some(last) = self.lines.last_mut() {
            last.ending = self.dominant_ending;
        }
        let mut inserted: Vec<Line> = lines
            .iter()
            .map(|text| Line {
                text: text.clone(),
                ending: self.dominant_ending,
            })
            .collect();
        if appending_to_unterminated && let Some(last) = inserted.last_mut() {
            last.ending = "";
        }
        self.lines.splice(index..index, inserted);
        Ok(())
    }

    /// Removes the lines in `range`; when the last line goes, what it said about the file's ending passes to the
    /// line that takes its place.
    pub fn remove(&mut self, range: LineRange) -> Result<(), String> {
        self.guard(range)?;
        let removing_the_end = range.end == self.lines.len() - 1;
        let ending = self.lines[range.end].ending;
        self.lines.drain(range.start..=range.end);
        if removing_the_end && let Some(last) = self.lines.last_mut() {
            last.ending = ending;
        }
        Ok(())
    }
}

/// The item indentation used when a document has no entry to copy.
pub const DEFAULT_ITEM_INDENT: &str = "  ";

/// The key indentation used when a document has no entry to copy.
pub const DEFAULT_KEY_INDENT: &str = "    ";

/// The leading whitespace of `text`, as .NET's `TrimStart` takes it.
pub fn indent_of(text: &str) -> &str {
    &text[..text.len() - text.trim_start().len()]
}

/// The index of the line carrying `key` within `range`, or none; a key on the item's own `- id:` line is found too.
pub fn find_key(document: &LineDocument, range: LineRange, key: &str) -> Option<usize> {
    let lines = document.lines();
    let wanted = format!("{key}:");
    (range.start..=range.end).take_while(|i| *i < lines.len()).find(|i| {
        let trimmed = lines[*i].text.trim_start();
        let trimmed = trimmed.strip_prefix("- ").unwrap_or(trimmed);
        trimmed.starts_with(&wanted)
    })
}

/// Replaces one key's value inside `range`, keeping the line's indentation and its dash, or adds the key directly
/// after the range's first line, indented as its neighbours are, when it is not there.
pub fn set_key(document: &mut LineDocument, range: LineRange, key: &str, value: &str) -> Result<(), String> {
    if let Some(index) = find_key(document, range, key) {
        let existing = document.lines()[index].text.clone();
        let indent = indent_of(&existing);
        let prefix = if existing.trim_start().starts_with("- ") {
            "- "
        } else {
            ""
        };
        return document.replace(
            LineRange::new(index, index),
            &[format!("{indent}{prefix}{key}: {value}")],
        );
    }
    let key_indent = key_indent_within(document, range);
    document.insert(range.start + 1, &[format!("{key_indent}{key}: {value}")])
}

/// Removes one key's line from within `range`, if it is there.
pub fn remove_key(document: &mut LineDocument, range: LineRange, key: &str) -> Result<(), String> {
    match find_key(document, range, key) {
        Some(index) => document.remove(LineRange::new(index, index)),
        None => Ok(()),
    }
}

/// The indentation the keys inside `range` already use; for a one-line entry, the dash's own and two spaces.
pub fn key_indent_within(document: &LineDocument, range: LineRange) -> String {
    let lines = document.lines();
    for i in range.start + 1..=range.end {
        if i >= lines.len() {
            break;
        }
        let text = &lines[i].text;
        if !text.trim().is_empty() {
            return indent_of(text).to_owned();
        }
    }
    let first = lines.get(range.start).map(|l| l.text.as_str()).unwrap_or_default();
    format!("{}{DEFAULT_ITEM_INDENT}", indent_of(first))
}

/// The item indentation, the gap after the dash and the key indentation the first of `ranges` uses, or the defaults
/// when the document has no entry to copy.
pub fn indent_of_entries(document: &LineDocument, ranges: &[LineRange]) -> (String, String, String) {
    let Some(range) = ranges.first().copied() else {
        return (
            DEFAULT_ITEM_INDENT.to_owned(),
            " ".to_owned(),
            DEFAULT_KEY_INDENT.to_owned(),
        );
    };
    let dash = &document.lines()[range.start].text;
    let item_indent = indent_of(dash).to_owned();
    let after_dash = dash.trim_start();
    let mut gap = " ".to_owned();
    if let Some(rest) = after_dash.strip_prefix('-') {
        gap = indent_of(rest).to_owned();
        if gap.is_empty() {
            gap = " ".to_owned();
        }
    }
    (item_indent, gap, key_indent_within(document, range))
}

/// Where a new entry goes: after the last of `ranges`, or directly after the section key when there are none, a
/// section written flow-empty (`elements: []`) opened into a bare key first; none when the section key is absent.
pub fn insertion_point_for(document: &mut LineDocument, ranges: &[LineRange], section_key: &str) -> Option<usize> {
    if let Some(last) = ranges.last() {
        return Some(last.end + 1);
    }
    let at = document
        .lines()
        .iter()
        .position(|l| l.text.trim_start().starts_with(section_key))?;
    let text = document.lines()[at].text.clone();
    let value = text.trim_start()[section_key.len()..].trim();
    if value == "[]" || value == "[ ]" {
        let indent = indent_of(&text).to_owned();
        document
            .replace(LineRange::new(at, at), &[format!("{indent}{section_key}")])
            .ok()?;
    }
    Some(at + 1)
}

/// Quotes a value only where YAML needs it, so an ordinary label stays plain; a value with a line break is
/// double-quoted with each break as its escape.
pub fn quote(value: &str) -> String {
    if value.is_empty() {
        return "\"\"".to_owned();
    }
    let first = value.chars().next().unwrap_or(' ');
    let needs_quoting = value.contains(':')
        || value.contains('#')
        || value.starts_with('-')
        || value.starts_with(' ')
        || value.ends_with(' ')
        || value.starts_with('"')
        || value.starts_with('\'')
        || "[]{}&*!|>%@`?,".contains(first)
        || value.contains('\n')
        || value.contains('\r');
    if !needs_quoting {
        return value.to_owned();
    }
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\r', "\\r")
        .replace('\n', "\\n");
    format!("\"{escaped}\"")
}

/// A number as document text, as the standalone host's `DslNumber.Format` writes it (`0.####`, invariant culture):
/// at most four decimals, a whole number without a decimal point.
pub fn format_number(value: f64) -> String {
    if !value.is_finite() {
        return if value.is_nan() {
            "NaN".to_owned()
        } else if value > 0.0 {
            "∞".to_owned()
        } else {
            "-∞".to_owned()
        };
    }
    let rounded = (value * 10_000.0).round() / 10_000.0;
    let mut text = format!("{rounded:.4}");
    while text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    if text == "-0" { "0".to_owned() } else { text }
}

/// A number as .NET's default `double.ToString()` writes it in a sentence: the shortest text that reads back as it.
pub fn shortest(value: f64) -> String {
    format!("{value}")
}
