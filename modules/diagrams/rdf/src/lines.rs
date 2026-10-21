//! An RDF document as the lines it is made of, and the line splices its writers make (etalii.adp spec 030, task T083).
//!
//! Ported from the standalone host at `8b6ccfc`: the core's `EtAlii.Adp.Documents/LineDocument.cs`, `Line.cs`,
//! `LineRange.cs` and `LineIndex.cs`, which the RDF family's parser and writers read and splice through. Every line
//! keeps its own terminator, so a document nothing has spliced is its own bytes again, a line spliced in takes the
//! terminator most of the document's lines use (a tie, an empty document included, goes to CRLF), and how the file
//! ends is a property of the file rather than of its last line. Offsets are byte offsets into the text.

/// One line: its text without its terminator, and the terminator (`\n`, `\r\n`, or none for a last line without one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    pub ending: &'static str,
}

/// An inclusive range of zero-based line indices: the lines that declare one construct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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

    /// Never: a range covers at least its first line.
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
    /// Splits `text` into lines, each keeping its own terminator. A file that ends without a newline gives a last
    /// line with no terminator; one that ends with a newline gains no empty line. A tie between LF and CRLF, an empty
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

    /// The terminator most of the document's lines use, which a line spliced in takes.
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

    /// Replaces the lines of `range` with `replacement`, whose last line keeps the terminator the range's last line
    /// had and the others the dominant one.
    ///
    /// # Panics
    ///
    /// When `range` is not inside the document, as the standalone host throws: a clamped range splices the wrong line.
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

    /// Inserts `lines` before line `index`. Appended after a last line without a terminator, that line gains one and
    /// the new last line goes without, so an append and its undo give back the same bytes.
    ///
    /// # Panics
    ///
    /// When `index` is past the end of the document.
    pub fn insert(&mut self, index: usize, lines: &[String]) {
        assert!(
            index <= self.lines.len(),
            "Line {index} is outside a document of {} lines.",
            self.lines.len()
        );
        let appending_to_unterminated =
            index == self.lines.len() && self.lines.last().is_some_and(|last| last.ending.is_empty());
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

    /// Removes the lines of `range`. When the last line goes, the line that takes its place takes its terminator.
    ///
    /// # Panics
    ///
    /// When `range` is not inside the document.
    pub fn remove(&mut self, range: LineRange) {
        self.guard(range);
        let removing_the_end = range.end == self.lines.len() - 1;
        let ending = self.lines[range.end].ending;
        self.lines.drain(range.start..=range.end);
        if removing_the_end && let Some(last) = self.lines.last_mut() {
            last.ending = ending;
        }
    }

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
    /// Whether the line holds nothing but whitespace (`Line.IsBlank`).
    pub fn is_blank(&self) -> bool {
        self.text.trim().is_empty()
    }
}

impl LineDocument {
    /// Where each line starts in [`LineDocument::text`], for turning an offset into a line (`LineDocument.Index`).
    pub fn index(&self) -> LineIndex {
        let mut starts = vec![0];
        let mut offset = 0;
        for line in &self.lines {
            offset += line.text.len() + line.ending.len();
            if !line.ending.is_empty() {
                starts.push(offset);
            }
        }
        LineIndex { starts }
    }

    /// How many lines the document has.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Whether the document has no lines at all.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

/// Where each line of a text starts (`LineIndex.cs`): a line starts at offset 0 and after every `\n`, so a text that
/// ends in a line break has one start more than it has lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    /// The line starts of `text`.
    pub fn new(text: &str) -> Self {
        let mut starts = vec![0];
        for (index, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                starts.push(index + 1);
            }
        }
        LineIndex { starts }
    }

    /// The zero-based line holding `offset`; an offset past the end is on the last line.
    pub fn line_of(&self, offset: usize) -> usize {
        match self.starts.binary_search(&offset) {
            Ok(line) => line,
            Err(next) => next.saturating_sub(1),
        }
    }

    /// The offset at which the zero-based `line` starts.
    pub fn start_of(&self, line: usize) -> usize {
        self.starts[line]
    }
}
