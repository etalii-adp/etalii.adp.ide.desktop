//! The terminals SPARQL shares with Turtle, read from a query's characters by offset, and what a string or a local
//! name means once its escapes are decoded (etalii.adp spec 030, task T084).
//!
//! Ported from the standalone host's core at `8b6ccfc`: `src/backend/EtAlii.Adp.Documents/TerminalScanner.cs`,
//! `NumericLiteral.cs`, `LineIndex.cs` and `RdfEscapes.cs`, the parts the SPARQL tokenizer and parser use. The text is
//! a slice of characters, where the standalone host's was a string of UTF-16 code units: an offset only ever slices
//! the text or names a line, so the two count alike for every character of the Basic Multilingual Plane, and a
//! character beyond it is, as there, neither a letter nor a digit.

/// The datatype of a number with neither a fraction nor an exponent.
pub const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
/// The datatype of a number with a fraction and no exponent.
pub const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
/// The datatype of a number with an exponent.
pub const XSD_DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";

/// How a string ended (`TerminalProblem`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalProblem {
    None,
    /// A short string met the end of its line.
    LineEnded,
    /// The text ended first.
    FileEnded,
}

fn in_plane(character: char) -> bool {
    u32::from(character) <= 0xFFFF
}

/// `char.IsLetter`.
pub fn is_letter(character: char) -> bool {
    in_plane(character) && character.is_alphabetic()
}

/// `char.IsLetterOrDigit`.
pub fn is_letter_or_digit(character: char) -> bool {
    in_plane(character) && character.is_alphanumeric()
}

/// `char.IsWhiteSpace`.
pub fn is_white_space(character: char) -> bool {
    in_plane(character) && character.is_whitespace()
}

/// Whether `character` may stand inside a name: a letter, a digit, `_` or `-` (`TerminalScanner.IsNameChar`).
pub fn is_name_char(character: char) -> bool {
    is_letter_or_digit(character) || character == '_' || character == '-'
}

/// The offset of the next token: past whitespace, and past every `#` comment to its line's end.
pub fn skip_trivia(text: &[char], mut position: usize) -> usize {
    while position < text.len() {
        let character = text[position];
        if is_white_space(character) {
            position += 1;
            continue;
        }
        if character == '#' {
            while position < text.len() && text[position] != '\n' {
                position += 1;
            }
            continue;
        }
        break;
    }
    position
}

/// A string whose opening quote is at `start`: where it ends, and how.
pub fn string(text: &[char], start: usize) -> (usize, TerminalProblem) {
    let quote = text[start];
    let mut position = start;
    let is_long = position + 2 < text.len() && text[position + 1] == quote && text[position + 2] == quote;
    if is_long {
        position += 3;
        while position < text.len() {
            if text[position] == '\\' {
                position += 2;
                continue;
            }
            if text[position] == quote
                && position + 2 < text.len()
                && text[position + 1] == quote
                && text[position + 2] == quote
            {
                return (position + 3, TerminalProblem::None);
            }
            position += 1;
        }
        return (position, TerminalProblem::FileEnded);
    }
    position += 1;
    while position < text.len() {
        let character = text[position];
        if character == '\\' {
            position += 2;
            continue;
        }
        if character == quote {
            return (position + 1, TerminalProblem::None);
        }
        if character == '\n' || character == '\r' {
            return (position, TerminalProblem::LineEnded);
        }
        position += 1;
    }
    (position, TerminalProblem::FileEnded)
}

/// The offset past the local part of a name that starts at `position`: name characters, colons, `\` escapes and `%`
/// hex pairs, and a dot only when the name goes on after it.
pub fn local_name(text: &[char], mut position: usize) -> usize {
    while position < text.len() {
        let character = text[position];
        if character == '\\' && position + 1 < text.len() {
            position += 2;
            continue;
        }
        if character == '%' && position + 2 < text.len() {
            position += 3;
            continue;
        }
        if character == '.' {
            let next = text.get(position + 1).copied().unwrap_or(' ');
            if is_name_char(next) || matches!(next, ':' | '%' | '\\') {
                position += 1;
                continue;
            }
            return position;
        }
        if is_name_char(character) || character == ':' {
            position += 1;
            continue;
        }
        return position;
    }
    position
}

/// The offset past a number that starts at `start`: an optional sign, digits, a fraction when a digit follows its
/// dot, and an exponent whose digits are not required (`NumericLiteral.Scan`).
pub fn number(text: &[char], start: usize) -> usize {
    let mut position = start;
    if matches!(text[position], '+' | '-') {
        position += 1;
    }
    while position < text.len() && text[position].is_ascii_digit() {
        position += 1;
    }
    if position + 1 < text.len() && text[position] == '.' && text[position + 1].is_ascii_digit() {
        position += 1;
        while position < text.len() && text[position].is_ascii_digit() {
            position += 1;
        }
    }
    if position < text.len() && matches!(text[position], 'e' | 'E') {
        position += 1;
        if position < text.len() && matches!(text[position], '+' | '-') {
            position += 1;
        }
        while position < text.len() && text[position].is_ascii_digit() {
            position += 1;
        }
    }
    position
}

/// The XML Schema type of the number written `lexical` (`NumericLiteral.Datatype`).
pub fn number_datatype(lexical: &str) -> &'static str {
    if lexical.contains(['e', 'E']) {
        XSD_DOUBLE
    } else if lexical.contains('.') {
        XSD_DECIMAL
    } else {
        XSD_INTEGER
    }
}

/// Where each line of a text starts (`LineIndex`).
#[derive(Debug, Clone)]
pub struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    pub fn new(text: &[char]) -> LineIndex {
        let mut starts = vec![0];
        for (index, character) in text.iter().enumerate() {
            if *character == '\n' {
                starts.push(index + 1);
            }
        }
        LineIndex { starts }
    }

    /// The zero-based line holding `offset`; an offset past the end is on the last line.
    pub fn line_of(&self, offset: usize) -> usize {
        self.starts.partition_point(|&start| start <= offset).saturating_sub(1)
    }
}

const SINGLE_LETTER_ESCAPES: &str = "tbnrf\"'\\";

fn hex(digits: &[char]) -> Option<u32> {
    if !digits.iter().all(char::is_ascii_hexdigit) {
        return None;
    }
    let text: String = digits.iter().collect();
    u32::from_str_radix(&text, 16).ok()
}

/// Whether `code_point` is a Unicode scalar value.
fn is_character(code_point: u32) -> bool {
    code_point <= 0x10FFFF && !(0xD800..=0xDFFF).contains(&code_point)
}

/// The first escape in the string token `raw` that SPARQL refuses, as its offset in `raw` and a sentence saying why
/// (`RdfEscapes.Refusal`).
pub fn escape_refusal(raw: &[char]) -> Option<(usize, String)> {
    let quote = raw[0];
    let is_long = raw.len() >= 6 && raw[1] == quote && raw[2] == quote;
    let end = raw.len() - if is_long { 3 } else { 1 };
    let mut i = if is_long { 3 } else { 1 };
    while i < end {
        if raw[i] != '\\' {
            i += 1;
            continue;
        }
        let letter = raw[i + 1];
        if SINGLE_LETTER_ESCAPES.contains(letter) {
            i += 2;
            continue;
        }
        if letter != 'u' && letter != 'U' {
            return Some((i, format!("'\\{letter}' is not a string escape; a backslash is written '\\\\'.")));
        }
        let digits = if letter == 'u' { 4 } else { 8 };
        let written: String = raw[i..end.min(i + 2 + digits)].iter().collect();
        let code_point = if i + 2 + digits > end {
            None
        } else {
            hex(&raw[i + 2..i + 2 + digits])
        };
        let Some(code_point) = code_point else {
            return Some((
                i,
                format!("'{written}' is not an escape: '\\{letter}' takes {digits} hex digits."),
            ));
        };
        if !is_character(code_point) {
            return Some((i, format!("'{written}' does not name a character.")));
        }
        i += 2 + digits;
    }
    None
}

/// A string token's value: its quotes, one or three, stripped, and its escapes decoded (`RdfEscapes.String`).
pub fn decode_string(raw: &[char]) -> String {
    let quote = raw[0];
    let is_long = raw.len() >= 6 && raw[1] == quote && raw[2] == quote;
    let inner = if is_long {
        &raw[3..raw.len() - 3]
    } else {
        &raw[1..raw.len() - 1]
    };
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
            letter @ ('u' | 'U') => {
                let digits = if letter == 'u' { 4 } else { 8 };
                let decoded = (i + digits < inner.len())
                    .then(|| hex(&inner[i + 1..i + 1 + digits]))
                    .flatten()
                    .filter(|c| is_character(*c))
                    .and_then(char::from_u32);
                match decoded {
                    Some(character) => {
                        builder.push(character);
                        i += digits;
                    }
                    None => builder.push(inner[i]),
                }
            }
            other => builder.push(other),
        }
        i += 1;
    }
    builder
}

/// A prefixed name's local part with its reserved-character escapes dropped (`RdfEscapes.Local`).
pub fn decode_local(raw: &str) -> String {
    let characters: Vec<char> = raw.chars().collect();
    let mut builder = String::with_capacity(raw.len());
    let mut i = 0;
    while i < characters.len() {
        if characters[i] == '\\' && i + 1 < characters.len() {
            i += 1;
        }
        builder.push(characters[i]);
        i += 1;
    }
    builder
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(text: &str) -> Vec<char> {
        text.chars().collect()
    }

    #[test]
    fn a_line_is_found_by_its_offset() {
        let index = LineIndex::new(&chars("a\nb\r\nc"));
        assert_eq!(index.line_of(0), 0);
        assert_eq!(index.line_of(1), 0);
        assert_eq!(index.line_of(2), 1);
        assert_eq!(index.line_of(5), 2);
        assert_eq!(index.line_of(99), 2);
    }

    #[test]
    fn escapes_are_decoded_and_refused_as_the_grammar_says() {
        assert_eq!(decode_string(&chars("\"a\\nb\\u0041\"")), "a\nbA");
        assert_eq!(decode_string(&chars("'''x'''")), "x");
        assert_eq!(escape_refusal(&chars("\"a\\qb\"")).unwrap().0, 2);
        assert!(escape_refusal(&chars("\"\\u12zz\"")).unwrap().1.starts_with("'\\u12zz' is not an escape"));
        assert_eq!(escape_refusal(&chars("\"\\t\"")), None);
        assert_eq!(decode_local("a\\.b"), "a.b");
    }

    #[test]
    fn a_number_ends_where_its_grammar_does() {
        assert_eq!(number(&chars("5e x"), 0), 2);
        assert_eq!(number(&chars("5. x"), 0), 1);
        assert_eq!(number(&chars("3.14"), 0), 4);
        assert_eq!(number_datatype("5e"), XSD_DOUBLE);
    }
}
