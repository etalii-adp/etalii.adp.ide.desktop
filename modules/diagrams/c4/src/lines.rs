//! A Structurizr DSL document as the lines it is made of, with edits applied by splicing whole lines (etalii.adp spec
//! 030, task T061).
//!
//! Ported from the standalone host at `8b6ccfc`: the core's `EtAlii.Adp.Documents/LineDocument.cs` and
//! `LineRange.cs`, which the C4 module moved onto in code-structure T128, and the module's
//! `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4/_Model/C4Line.cs` and `C4Lines.cs`. Every line keeps its own
//! terminator, so a document nothing has spliced is its own bytes again, and a line that is spliced in takes the
//! terminator most of the document's lines use.

use crate::tokens;

/// One line of a document: its text without its terminator, and the terminator (`\n`, `\r\n`, or none for a last
/// line without one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    pub ending: &'static str,
}

/// One line of DSL with its 1-based number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct C4Line {
    pub text: String,
    pub number: u32,
}

impl C4Line {
    /// The line with its comment and surrounding whitespace removed: what the parser reads.
    pub fn code(&self) -> &str {
        tokens::strip_comment(&self.text).trim()
    }

    /// Whether the line carries nothing but whitespace and a comment.
    pub fn is_blank(&self) -> bool {
        self.code().is_empty()
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

    /// The lines paired with their 1-based numbers.
    pub fn code_lines(&self) -> Vec<C4Line> {
        self.lines
            .iter()
            .enumerate()
            .map(|(index, line)| C4Line {
                text: line.text.clone(),
                number: index as u32 + 1,
            })
            .collect()
    }

    /// Replaces the lines `start..=end` (0-based) with `replacement`, whose last line keeps the terminator the range's
    /// last line had and the others the dominant one.
    pub fn replace(&mut self, start: usize, end: usize, replacement: &[String]) -> Result<(), String> {
        self.guard(start, end)?;
        let ending = self.lines[end].ending;
        let count = replacement.len();
        let lines = replacement.iter().enumerate().map(|(index, text)| Line {
            text: text.clone(),
            ending: if index == count - 1 {
                ending
            } else {
                self.dominant_ending
            },
        });
        let lines: Vec<Line> = lines.collect();
        self.lines.splice(start..=end, lines);
        Ok(())
    }

    /// Inserts `lines` before the 0-based `index`. Appending after a last line without a terminator gives that line
    /// the dominant one and the new last line none, so an append and its undo come back byte for byte.
    pub fn insert(&mut self, index: usize, lines: &[String]) -> Result<(), String> {
        if index > self.lines.len() {
            return Err(format!(
                "Line {index} is outside a document of {} lines.",
                self.lines.len()
            ));
        }
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
        Ok(())
    }

    /// Removes the lines `start..=end` (0-based); when the last line goes, its terminator passes to the line that
    /// takes its place.
    pub fn remove(&mut self, start: usize, end: usize) -> Result<(), String> {
        self.guard(start, end)?;
        let removing_the_end = end == self.lines.len() - 1;
        let ending = self.lines[end].ending;
        self.lines.drain(start..=end);
        if removing_the_end && let Some(last) = self.lines.last_mut() {
            last.ending = ending;
        }
        Ok(())
    }

    /// Replaces line `number` (1-based) with `text`; every other line keeps the bytes it arrived as.
    pub fn replace_line(&mut self, number: u32, text: &str) -> Result<(), String> {
        let index = (number as usize).checked_sub(1).ok_or("Line 0 is not a line.")?;
        self.replace(index, index, &[text.to_owned()])
    }

    /// Inserts `text` so that it becomes line `number` (1-based).
    pub fn insert_line(&mut self, number: u32, text: &str) -> Result<(), String> {
        let index = (number as usize).checked_sub(1).ok_or("Line 0 is not a line.")?;
        self.insert(index, &[text.to_owned()])
    }

    /// Removes lines `from` to `to` inclusive, 1-based.
    pub fn remove_lines(&mut self, from: u32, to: u32) -> Result<(), String> {
        let start = (from as usize).checked_sub(1).ok_or("Line 0 is not a line.")?;
        let end = (to as usize).checked_sub(1).ok_or("Line 0 is not a line.")?;
        self.remove(start, end)
    }

    /// Refuses a range that is not entirely inside the document, rather than clamping it: a clamped range splices the
    /// wrong line.
    fn guard(&self, start: usize, end: usize) -> Result<(), String> {
        if end < start {
            return Err(format!("Line {end} cannot precede line {start}."));
        }
        if end >= self.lines.len() {
            return Err(format!(
                "Lines {start}-{end} are outside a document of {} lines.",
                self.lines.len()
            ));
        }
        Ok(())
    }
}
