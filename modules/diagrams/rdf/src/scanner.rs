//! The terminals Turtle shares with SPARQL, read from a text by byte offset (etalii.adp spec 030, task T083): the
//! whitespace and comments between tokens, strings, IRIs, the local part of a name and numbers, and what a string, an
//! IRI or a local name means once its escapes are decoded.
//!
//! Ported from the standalone host's core at `8b6ccfc`: `EtAlii.Adp.Documents/TerminalScanner.cs`,
//! `TerminalProblem.cs`, `NumericLiteral.cs` and `RdfEscapes.cs`. Each reader says where its terminal ends; the
//! tokenizer that called it makes the token, and its own error when one did not close.

/// Why a terminal could not be closed (`TerminalProblem`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalProblem {
    None,
    /// A terminal that may not span lines reached a line break first.
    LineEnded,
    /// The text ended first.
    FileEnded,
}

/// Where a terminal ends, and whether it closed (`TerminalScan`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalScan {
    pub end: usize,
    pub problem: TerminalProblem,
}

/// The datatype of a number with neither a fraction nor an exponent.
pub const INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
/// The datatype of a number with a fraction and no exponent.
pub const DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
/// The datatype of a number with an exponent.
pub const DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";

/// The character at `position` of `text`, when there is one.
pub fn char_at(text: &str, position: usize) -> Option<char> {
    text.get(position..).and_then(|rest| rest.chars().next())
}

/// Whether `character` may stand inside a name: a letter, a digit, `_` or `-` (`TerminalScanner.IsNameChar`).
pub fn is_name_char(character: char) -> bool {
    character.is_alphanumeric() || character == '_' || character == '-'
}

/// The offset of the next token: past whitespace, and past every `#` comment to its line's end.
pub fn skip_trivia(text: &str, mut position: usize) -> usize {
    while let Some(character) = char_at(text, position) {
        if character.is_whitespace() {
            position += character.len_utf8();
            continue;
        }
        if character == '#' {
            while let Some(c) = char_at(text, position) {
                if c == '\n' {
                    break;
                }
                position += c.len_utf8();
            }
            continue;
        }
        break;
    }
    position
}

/// Advances `position` past the character there, or by one byte at the end.
fn step(text: &str, position: usize) -> usize {
    position + char_at(text, position).map_or(1, char::len_utf8)
}

/// A string whose opening quote is at `start`: short, closed by the same quote on its own line, or long, opened and
/// closed by three. An escaped character never closes it, and a short string ends at a carriage return as it does at
/// a line feed.
pub fn string(text: &str, start: usize) -> TerminalScan {
    let bytes = text.as_bytes();
    let quote = bytes[start];
    let mut position = start;
    let is_long = position + 2 < bytes.len() && bytes[position + 1] == quote && bytes[position + 2] == quote;
    if is_long {
        position += 3;
        while position < bytes.len() {
            if bytes[position] == b'\\' {
                position = step(text, position + 1);
                continue;
            }
            if bytes[position] == quote
                && position + 2 < bytes.len()
                && bytes[position + 1] == quote
                && bytes[position + 2] == quote
            {
                return TerminalScan {
                    end: position + 3,
                    problem: TerminalProblem::None,
                };
            }
            position = step(text, position);
        }
        return TerminalScan {
            end: bytes.len().min(position),
            problem: TerminalProblem::FileEnded,
        };
    }

    position += 1;
    while position < bytes.len() {
        let byte = bytes[position];
        if byte == b'\\' {
            position = step(text, position + 1);
            continue;
        }
        if byte == quote {
            return TerminalScan {
                end: position + 1,
                problem: TerminalProblem::None,
            };
        }
        if byte == b'\n' || byte == b'\r' {
            return TerminalScan {
                end: position,
                problem: TerminalProblem::LineEnded,
            };
        }
        position = step(text, position);
    }
    TerminalScan {
        end: bytes.len().min(position),
        problem: TerminalProblem::FileEnded,
    }
}

/// An IRI whose `<` is at `start`, closed by `>` on its own line; a backslash escapes the character after it.
pub fn iri(text: &str, start: usize) -> TerminalScan {
    let bytes = text.as_bytes();
    let mut position = start + 1;
    while position < bytes.len() {
        let byte = bytes[position];
        if byte == b'>' {
            return TerminalScan {
                end: position + 1,
                problem: TerminalProblem::None,
            };
        }
        if byte == b'\\' {
            position += 1;
        }
        if byte == b'\n' || byte == b'\r' {
            return TerminalScan {
                end: position,
                problem: TerminalProblem::LineEnded,
            };
        }
        position = step(text, position);
    }
    TerminalScan {
        end: bytes.len().min(position),
        problem: TerminalProblem::FileEnded,
    }
}

/// The offset past the local part of a name that starts at `position`: name characters, colons, `\` escapes and `%`
/// hex pairs, and a dot only when the name goes on after it.
pub fn local_name(text: &str, mut position: usize) -> usize {
    let length = text.len();
    while let Some(character) = char_at(text, position) {
        if character == '\\' && position + 1 < length {
            position = step(text, position + 1);
            continue;
        }
        if character == '%' && position + 2 < length {
            position = step(text, step(text, position + 1));
            continue;
        }
        if character == '.' {
            let next = char_at(text, position + 1).unwrap_or(' ');
            if is_name_char(next) || matches!(next, ':' | '%' | '\\') {
                position += 1;
                continue;
            }
            return position;
        }
        if is_name_char(character) || character == ':' {
            position += character.len_utf8();
            continue;
        }
        return position;
    }
    position.min(length)
}

/// The offset past a number that starts at `start`: an optional sign, digits, a fraction when a digit follows its
/// dot, and an exponent (`NumericLiteral.Scan`).
pub fn number(text: &str, start: usize) -> usize {
    let bytes = text.as_bytes();
    let mut position = start;
    if matches!(bytes[position], b'+' | b'-') {
        position += 1;
    }
    while position < bytes.len() && bytes[position].is_ascii_digit() {
        position += 1;
    }
    if position + 1 < bytes.len() && bytes[position] == b'.' && bytes[position + 1].is_ascii_digit() {
        position += 1;
        while position < bytes.len() && bytes[position].is_ascii_digit() {
            position += 1;
        }
    }
    if position < bytes.len() && matches!(bytes[position], b'e' | b'E') {
        position += 1;
        if position < bytes.len() && matches!(bytes[position], b'+' | b'-') {
            position += 1;
        }
        while position < bytes.len() && bytes[position].is_ascii_digit() {
            position += 1;
        }
    }
    position
}

/// The XML Schema type of the number written `lexical` (`NumericLiteral.Datatype`).
pub fn number_datatype(lexical: &str) -> &'static str {
    if lexical.contains(['e', 'E']) {
        DOUBLE
    } else if lexical.contains('.') {
        DECIMAL
    } else {
        INTEGER
    }
}

const SINGLE_LETTER_ESCAPES: &str = "tbnrf\"'\\";

/// Whether `code_point` is a Unicode scalar value.
fn character_of(code_point: u32) -> Option<char> {
    char::from_u32(code_point)
}

/// The first escape in the string token `raw` that Turtle refuses, as its byte offset in `raw` and a sentence saying
/// why; none when every escape is one the grammar knows (`RdfEscapes.Refusal`).
pub fn escape_refusal(raw: &str) -> Option<(usize, String)> {
    let chars: Vec<(usize, char)> = raw.char_indices().collect();
    let quote = chars.first()?.1;
    let is_long = chars.len() >= 6 && chars[1].1 == quote && chars[2].1 == quote;
    let end = chars.len() - if is_long { 3 } else { 1 };
    let mut i = if is_long { 3 } else { 1 };
    while i < end {
        if chars[i].1 != '\\' {
            i += 1;
            continue;
        }
        let letter = chars.get(i + 1).map_or('\0', |c| c.1);
        if SINGLE_LETTER_ESCAPES.contains(letter) {
            i += 2;
            continue;
        }
        if letter != 'u' && letter != 'U' {
            return Some((
                chars[i].0,
                format!("'\\{letter}' is not a string escape; a backslash is written '\\\\'."),
            ));
        }
        let digits = if letter == 'u' { 4 } else { 8 };
        let written_end = end.min(i + 2 + digits);
        let written: String = chars[i..written_end].iter().map(|c| c.1).collect();
        let hex: Option<u32> = (i + 2 + digits <= end)
            .then(|| {
                let text: String = chars[i + 2..i + 2 + digits].iter().map(|c| c.1).collect();
                if text.chars().all(|c| c.is_ascii_hexdigit()) {
                    u32::from_str_radix(&text, 16).ok()
                } else {
                    None
                }
            })
            .flatten();
        let Some(code_point) = hex else {
            return Some((
                chars[i].0,
                format!("'{written}' is not an escape: '\\{letter}' takes {digits} hex digits."),
            ));
        };
        if character_of(code_point).is_none() {
            return Some((chars[i].0, format!("'{written}' does not name a character.")));
        }
        i += 2 + digits;
    }
    None
}

/// A string token's value: its quotes, one or three, stripped, and its escapes decoded (`RdfEscapes.String`).
pub fn string_value(raw: &str) -> String {
    let chars: Vec<char> = raw.chars().collect();
    let quote = chars[0];
    let is_long = chars.len() >= 6 && chars[1] == quote && chars[2] == quote;
    let inner: &[char] = if is_long {
        &chars[3..chars.len() - 3]
    } else {
        &chars[1..chars.len() - 1]
    };
    if !inner.contains(&'\\') {
        return inner.iter().collect();
    }
    let mut builder = String::with_capacity(inner.len());
    let mut i = 0;
    while i < inner.len() {
        if inner[i] != '\\' || i + 1 >= inner.len() {
            builder.push(inner[i]);
            i += 1;
            continue;
        }
        i += 1;
        match inner[i] {
            't' => builder.push('\t'),
            'b' => builder.push('\u{8}'),
            'n' => builder.push('\n'),
            'r' => builder.push('\r'),
            'f' => builder.push('\u{c}'),
            'u' | 'U' => {
                let length = if inner[i] == 'u' { 4 } else { 8 };
                i = append_code_point(&mut builder, inner, i, length);
            }
            other => builder.push(other),
        }
        i += 1;
    }
    builder
}

/// The inside of a `<...>` token with its numeric escapes decoded (`RdfEscapes.Iri`).
pub fn iri_value(raw: &str) -> String {
    numeric(&raw[1..raw.len() - 1])
}

/// A prefixed name's local part with its reserved-character escapes dropped (`RdfEscapes.Local`).
pub fn local_value(raw: &str) -> String {
    if !raw.contains('\\') {
        return raw.to_owned();
    }
    let chars: Vec<char> = raw.chars().collect();
    let mut builder = String::with_capacity(raw.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            i += 1;
        }
        builder.push(chars[i]);
        i += 1;
    }
    builder
}

fn numeric(value: &str) -> String {
    if !value.contains('\\') {
        return value.to_owned();
    }
    let chars: Vec<char> = value.chars().collect();
    let mut builder = String::with_capacity(value.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() && matches!(chars[i + 1], 'u' | 'U') {
            let length = if chars[i + 1] == 'u' { 4 } else { 8 };
            i += 1;
            i = append_code_point(&mut builder, &chars, i, length) + 1;
            continue;
        }
        builder.push(chars[i]);
        i += 1;
    }
    builder
}

/// Appends the character the `digits` hex digits after `index` name, returning the index of the last one; or the
/// letter at `index` itself when they name none.
fn append_code_point(builder: &mut String, source: &[char], index: usize, digits: usize) -> usize {
    if index + digits < source.len() {
        let text: String = source[index + 1..=index + digits].iter().collect();
        if text.chars().all(|c| c.is_ascii_hexdigit())
            && let Some(character) = u32::from_str_radix(&text, 16).ok().and_then(character_of)
        {
            builder.push(character);
            return index + digits;
        }
    }
    builder.push(source[index]);
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_decode_their_escapes() {
        assert_eq!(string_value("\"a\\tb\""), "a\tb");
        assert_eq!(string_value("\"\\u00e9\""), "é");
        assert_eq!(string_value("\"\"\"x\"y\"\"\""), "x\"y");
    }

    #[test]
    fn an_unknown_escape_is_refused() {
        assert!(
            escape_refusal("\"a\\qb\"")
                .unwrap()
                .1
                .starts_with("'\\q' is not a string escape")
        );
        assert!(
            escape_refusal("\"\\u12zz\"")
                .unwrap()
                .1
                .starts_with("'\\u12zz' is not an escape")
        );
        assert_eq!(escape_refusal("\"ok\\n\""), None);
    }

    #[test]
    fn a_local_name_stops_before_a_terminating_dot() {
        assert_eq!(local_name("ex:a.b .", 3), 6);
        assert_eq!(local_name("ex:ab.", 3), 5);
    }

    #[test]
    fn numbers_take_their_datatype_from_their_form() {
        assert_eq!(number("12.5e3 ", 0), 6);
        assert_eq!(number_datatype("12.5e3"), DOUBLE);
        assert_eq!(number_datatype("1.5"), DECIMAL);
        assert_eq!(number("5. ", 0), 1);
        assert_eq!(number_datatype("5"), INTEGER);
    }
}
