//! Positions between the server's byte offsets and the client's lines and characters, in the encoding the two
//! agreed: UTF-16 code units by default, or UTF-8 bytes, or code points (UTF-32).

use serde_json::{Value as Json, json};

/// How a client counts characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    Utf16,
    Utf32,
}

impl Encoding {
    pub fn name(self) -> &'static str {
        match self {
            Encoding::Utf8 => "utf-8",
            Encoding::Utf16 => "utf-16",
            Encoding::Utf32 => "utf-32",
        }
    }

    fn width(self, c: char) -> usize {
        match self {
            Encoding::Utf8 => c.len_utf8(),
            Encoding::Utf16 => c.len_utf16(),
            Encoding::Utf32 => 1,
        }
    }
}

/// The byte offset of an LSP position (`{line, character}`, both from 0).
pub fn offset(text: &str, position: &Json, encoding: Encoding) -> usize {
    let line = position["line"].as_u64().unwrap_or(0) as usize;
    let character = position["character"].as_u64().unwrap_or(0) as usize;
    let mut start = 0;
    for _ in 0..line {
        match text[start..].find('\n') {
            Some(i) => start += i + 1,
            None => return text.len(),
        }
    }
    let line_text = &text[start..text[start..].find('\n').map(|i| start + i).unwrap_or(text.len())];
    let mut counted = 0;
    for (i, c) in line_text.char_indices() {
        if counted >= character {
            return start + i;
        }
        counted += encoding.width(c);
    }
    start + line_text.len()
}

/// The LSP position of a byte offset.
pub fn position(text: &str, offset: usize, encoding: Encoding) -> Json {
    let offset = offset.min(text.len());
    let line_start = text[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line = text[..line_start].matches('\n').count();
    let character: usize = text[line_start..offset].chars().map(|c| encoding.width(c)).sum();
    json!({"line": line, "character": character})
}

pub fn range(text: &str, start: usize, end: usize, encoding: Encoding) -> Json {
    json!({"start": position(text, start, encoding), "end": position(text, end, encoding)})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_counts_units_and_utf32_code_points() {
        let text = "a\n•x😀y";
        let at_y = text.find('y').unwrap();
        assert_eq!(
            position(text, at_y, Encoding::Utf16),
            json!({"line": 1, "character": 4})
        );
        assert_eq!(
            position(text, at_y, Encoding::Utf32),
            json!({"line": 1, "character": 3})
        );
        assert_eq!(offset(text, &json!({"line": 1, "character": 4}), Encoding::Utf16), at_y);
    }
}
