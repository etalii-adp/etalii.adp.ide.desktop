//! A Databricks file as the lines it is made of, with every edit a splice of whole lines (etalii.adp spec 030, task
//! T081).
//!
//! Ported from the standalone host's core at `8b6ccfc`: `src/backend/EtAlii.Adp.Documents/LineDocument.cs`, `Line.cs`
//! and `LineRange.cs`, which the Databricks module edits through. Every line keeps its own terminator, so a document
//! nothing has spliced is its own bytes again, and a line spliced in takes the terminator most of the document's lines
//! use, CRLF on a tie.

/// One line: its text without its terminator, and the terminator (`\n`, `\r\n`, or none for a last line without one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    pub ending: &'static str,
}

/// An inclusive range of 0-based line indices: the lines that declare one entry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct LineRange {
    pub start: usize,
    pub end: usize,
}

impl LineRange {
    pub const fn new(start: usize, end: usize) -> Self {
        LineRange { start, end }
    }

    /// How many lines the range covers.
    pub fn len(&self) -> usize {
        self.end + 1 - self.start
    }

    /// Never: a range holds at least its first line.
    pub fn is_empty(&self) -> bool {
        false
    }
}

/// A hand-authored file as its lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineDocument {
    lines: Vec<Line>,
    dominant_ending: &'static str,
}

impl LineDocument {
    /// Splits `text` into lines, each keeping its own terminator. A file that ends without a newline gives a last line
    /// with no terminator; one that ends with a newline gains no empty line. A tie between LF and CRLF, an empty
    /// document included, goes to CRLF, the standalone repository's house style.
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

    /// The text of line `index`.
    pub fn line(&self, index: usize) -> &str {
        &self.lines[index].text
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

    /// Replaces the lines of `range` with `replacement`, whose last line keeps the terminator the range's last line
    /// had and the others the dominant one.
    pub fn replace(&mut self, range: LineRange, replacement: &[String]) {
        self.guard(range);
        let ending = self.lines[range.end].ending;
        let count = replacement.len();
        let lines: Vec<Line> = replacement
            .iter()
            .enumerate()
            .map(|(index, text)| Line {
                text: text.clone(),
                ending: if index + 1 == count {
                    ending
                } else {
                    self.dominant_ending
                },
            })
            .collect();
        self.lines.splice(range.start..=range.end, lines);
    }

    /// Inserts `lines` before `index`. Appending after a last line without a terminator gives that line the dominant
    /// one and the new last line none, so an append and its undo come back byte for byte.
    ///
    /// # Panics
    ///
    /// When `index` is past the end, as the standalone host throws there.
    pub fn insert(&mut self, index: usize, lines: &[String]) {
        assert!(
            index <= self.lines.len(),
            "Line {index} is outside a document of {} lines.",
            self.lines.len()
        );
        let appending_to_unterminated =
            index == self.lines.len() && self.lines.last().is_some_and(|line| line.ending.is_empty());
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
    }

    /// Removes the lines of `range`; when the last line goes, its terminator passes to the line that takes its place.
    pub fn remove(&mut self, range: LineRange) {
        self.guard(range);
        let removing_the_end = range.end == self.lines.len() - 1;
        let ending = self.lines[range.end].ending;
        self.lines.drain(range.start..=range.end);
        if removing_the_end && let Some(last) = self.lines.last_mut() {
            last.ending = ending;
        }
    }

    /// Refuses a range that is not entirely inside the document, rather than clamping it: a clamped range splices the
    /// wrong line.
    fn guard(&self, range: LineRange) {
        assert!(
            range.end >= range.start,
            "Line {} cannot precede line {}.",
            range.end,
            range.start
        );
        assert!(
            range.end < self.lines.len(),
            "Lines {}-{} are outside a document of {} lines.",
            range.start,
            range.end,
            self.lines.len()
        );
    }
}

impl Line {
    /// Whether the line carries nothing but whitespace (`Line.IsBlank`).
    pub fn is_blank(&self) -> bool {
        self.text.trim().is_empty()
    }
}
