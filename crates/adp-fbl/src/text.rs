//! A body's bytes with FBL's view of them (FBL section 2.6; the standalone host's `Span` and `Text/BodyText`): UTF-8
//! with or without a byte-order mark, lines and their endings (CRLF, LF or a lone CR), and conversion from a byte
//! offset to a 1-based line and a code-point column. Every offset is a UTF-8 byte offset into the whole file, the
//! mark included.

use std::fmt;

/// A range of UTF-8 byte offsets into a body, `end` exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }

    pub fn with_start(self, start: usize) -> Self {
        Span { start, end: self.end }
    }

    pub fn with_end(self, end: usize) -> Self {
        Span { start: self.start, end }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {})", self.start, self.end)
    }
}

/// One line: `start` to `content_end` is the line, up to `end` its ending.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextLine {
    pub start: usize,
    pub content_end: usize,
    pub end: usize,
    pub ending: &'static str,
}

/// A body's bytes, its lines, and whether it is valid UTF-8.
#[derive(Debug, Clone)]
pub struct BodyText {
    bytes: Vec<u8>,
    bom_length: usize,
    invalid_offset: Option<usize>,
    lines: Vec<TextLine>,
}

impl BodyText {
    pub fn new(bytes: Vec<u8>) -> Self {
        let bom_length = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) { 3 } else { 0 };
        let invalid_offset = std::str::from_utf8(&bytes[bom_length..])
            .err()
            .map(|e| bom_length + e.valid_up_to());
        let mut text = BodyText {
            bytes,
            bom_length,
            invalid_offset,
            lines: Vec::new(),
        };
        text.build_lines();
        text
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// 3 when the body starts with a UTF-8 byte-order mark, else 0. The mark belongs to no node.
    pub fn bom_length(&self) -> usize {
        self.bom_length
    }

    pub fn is_valid_utf8(&self) -> bool {
        self.invalid_offset.is_none()
    }

    pub fn invalid_offset(&self) -> usize {
        self.invalid_offset.unwrap_or(0)
    }

    pub fn lines(&self) -> &[TextLine] {
        &self.lines
    }

    /// The bytes as text. Only meaningful for valid UTF-8; a range that cuts a character gives what can be decoded.
    pub fn text(&self, start: usize, end: usize) -> &str {
        let slice = &self.bytes[start.min(self.bytes.len())..end.min(self.bytes.len())];
        match std::str::from_utf8(slice) {
            Ok(text) => text,
            Err(e) => std::str::from_utf8(&slice[..e.valid_up_to()]).unwrap_or(""),
        }
    }

    pub fn text_of(&self, span: Span) -> &str {
        self.text(span.start, span.end)
    }

    /// The ending that ends the most lines, CRLF winning a tie with LF; a lone CR counts as neither (FBL section
    /// 6.3). None when no line has an ending.
    pub fn dominant_ending(&self) -> Option<&'static str> {
        let (mut crlf, mut lf, mut cr) = (0, 0, 0);
        for line in &self.lines {
            match line.ending {
                "\r\n" => crlf += 1,
                "\n" => lf += 1,
                "\r" => cr += 1,
                _ => {}
            }
        }
        if crlf == 0 && lf == 0 {
            return if cr > 0 { Some("\r") } else { None };
        }
        Some(if crlf >= lf { "\r\n" } else { "\n" })
    }

    /// The line ending a splice at `offset` writes (FBL section 6.3).
    pub fn newline_at(&self, offset: usize, fallback: &str) -> String {
        let line = &self.lines[self.line_index_at(offset)];
        if !line.ending.is_empty() {
            return line.ending.to_owned();
        }
        self.dominant_ending().unwrap_or(fallback).to_owned()
    }

    /// The 0-based index of the line holding `offset`.
    pub fn line_index_at(&self, offset: usize) -> usize {
        let (mut lo, mut hi) = (0usize, self.lines.len() - 1);
        while lo < hi {
            let mid = (lo + hi).div_ceil(2);
            if self.lines[mid].start <= offset {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        lo
    }

    /// The 1-based line and code-point column of `offset`.
    pub fn position(&self, offset: usize) -> (usize, usize) {
        let index = self.line_index_at(offset);
        let mut start = self.lines[index].start;
        if index == 0 {
            start = start.max(self.bom_length);
        }
        let mut column = 1;
        let mut i = start;
        while i < offset && i < self.bytes.len() {
            if (self.bytes[i] & 0xC0) != 0x80 {
                column += 1;
            }
            i += 1;
        }
        (index + 1, column)
    }

    /// The number of code points in a span, for a finding's length.
    pub fn code_points(&self, start: usize, end: usize) -> usize {
        self.bytes[start.min(self.bytes.len())..end.min(self.bytes.len())]
            .iter()
            .filter(|b| (**b & 0xC0) != 0x80)
            .count()
    }

    fn build_lines(&mut self) {
        let bytes = &self.bytes;
        let mut start = 0;
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'\r' => {
                    if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                        self.lines.push(TextLine {
                            start,
                            content_end: i,
                            end: i + 2,
                            ending: "\r\n",
                        });
                        i += 2;
                    } else {
                        self.lines.push(TextLine {
                            start,
                            content_end: i,
                            end: i + 1,
                            ending: "\r",
                        });
                        i += 1;
                    }
                    start = i;
                }
                b'\n' => {
                    self.lines.push(TextLine {
                        start,
                        content_end: i,
                        end: i + 1,
                        ending: "\n",
                    });
                    i += 1;
                    start = i;
                }
                _ => i += 1,
            }
        }
        if start < bytes.len() || self.lines.is_empty() {
            self.lines.push(TextLine {
                start,
                content_end: bytes.len(),
                end: bytes.len(),
                ending: "",
            });
        }
    }
}

/// The number of UTF-8 bytes of `text`, as offsets count them.
pub fn byte_count(text: &str) -> usize {
    text.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_endings_and_positions_are_read_as_written() {
        let text = BodyText::new("\u{feff}a\r\nbé\nc\rd".as_bytes().to_vec());
        assert_eq!(text.bom_length(), 3);
        assert_eq!(text.lines().len(), 4);
        assert_eq!(text.lines()[0].ending, "\r\n");
        assert_eq!(text.dominant_ending(), Some("\r\n"));
        assert_eq!(text.position(3), (1, 1));
        assert_eq!(text.position(9), (2, 3));
        assert_eq!(text.newline_at(text.len(), "\n"), "\r\n");
        assert!(!BodyText::new(vec![b'a', 0xFF]).is_valid_utf8());
    }
}
