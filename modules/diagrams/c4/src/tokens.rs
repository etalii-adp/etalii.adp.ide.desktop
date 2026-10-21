//! Splitting a line of Structurizr DSL into its words, and rewriting one argument of it (etalii.adp spec 030, task
//! T061).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4/_Model/C4Tokens.cs`
//! and the core's `EtAlii.Adp.Documents/LineLexer.cs`, `CommentPolicy.cs`, `EscapePolicy.cs`, `LineToken.cs` and
//! `QuotedString.cs`, as the C4 module configured them: `#` and `//` comments outside quoted strings, `\"` for a quote
//! inside one, and braces as tokens of their own. The DSL's arguments are separated by whitespace, except that a
//! quoted string is one argument however much whitespace it holds. An offset is a byte offset into the line, where the
//! standalone host counted UTF-16 code units; both name the same character.

/// One token of a line and where it sits in that line, its quotes included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineToken {
    /// The token's text, unquoted and unescaped.
    pub value: String,
    /// The byte offset of its first character in the line: the opening quote, when it has one.
    pub start: usize,
    /// How many bytes it occupies, its quotes included.
    pub length: usize,
    /// Whether it was written in quotes.
    pub quoted: bool,
}

const COMMENT_MARKERS: [&str; 2] = ["#", "//"];
const PUNCTUATION: [char; 2] = ['{', '}'];

/// `text` without its comment: everything from the first `#` or `//` that is not inside a quoted string.
pub fn strip_comment(text: &str) -> &str {
    let bytes = text.as_bytes();
    let mut in_quotes = false;
    for (index, character) in text.char_indices() {
        if character == '"' && !(index > 0 && bytes[index - 1] == b'\\') {
            in_quotes = !in_quotes;
            continue;
        }
        if in_quotes {
            continue;
        }
        if COMMENT_MARKERS.iter().any(|marker| text[index..].starts_with(marker)) {
            return &text[..index];
        }
    }
    text
}

/// The tokens of `line` with their positions, its comment left out, so an edit can replace one argument and leave
/// every other character of the line exactly as it was.
pub fn split_with_spans(line: &str) -> Vec<LineToken> {
    let code = strip_comment(line);
    let characters: Vec<(usize, char)> = code.char_indices().collect();
    let end_of = |position: usize| characters.get(position).map_or(code.len(), |(offset, _)| *offset);
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        let (start, character) = characters[index];
        if character.is_whitespace() {
            index += 1;
            continue;
        }
        if character == '"' {
            index += 1;
            let mut value = String::new();
            while index < characters.len() {
                let (_, current) = characters[index];
                if current == '\\' && characters.get(index + 1).is_some_and(|(_, next)| *next == '"') {
                    value.push('"');
                    index += 2;
                    continue;
                }
                if current == '"' {
                    index += 1;
                    break;
                }
                value.push(current);
                index += 1;
            }
            tokens.push(LineToken {
                value,
                start,
                length: end_of(index) - start,
                quoted: true,
            });
            continue;
        }
        if PUNCTUATION.contains(&character) {
            tokens.push(LineToken {
                value: character.to_string(),
                start,
                length: character.len_utf8(),
                quoted: false,
            });
            index += 1;
            continue;
        }
        while index < characters.len() {
            let (_, current) = characters[index];
            if current.is_whitespace() || current == '"' || PUNCTUATION.contains(&current) {
                break;
            }
            index += 1;
        }
        let end = end_of(index);
        tokens.push(LineToken {
            value: code[start..end].to_owned(),
            start,
            length: end - start,
            quoted: false,
        });
    }
    tokens
}

/// `value` as a DSL string: in quotes, a quote inside it escaped.
pub fn quoted(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\\\""))
}

/// `line` with the argument at `argument_index` (counted after the token at `keyword_index`) set to `value`.
/// Arguments the line does not have yet are appended, with empty ones in between, so setting a technology on a
/// container that has only a name gives it an empty description rather than shifting the technology into the
/// description's place.
pub fn replace_argument(line: &str, keyword_index: usize, argument_index: usize, value: &str) -> String {
    let tokens = split_with_spans(line);
    let target = keyword_index + 1 + argument_index;
    let quoted = quoted(value);

    // The arguments stop at a brace: `container "A" "b" {` has two, not three.
    let mut end = tokens.len();
    for (index, token) in tokens.iter().enumerate().skip(keyword_index + 1) {
        if !token.quoted && (token.value == "{" || token.value == "}") {
            end = index;
            break;
        }
    }

    if target < end {
        let token = &tokens[target];
        return format!(
            "{}{}{}",
            &line[..token.start],
            quoted,
            &line[token.start + token.length..]
        );
    }

    // Append, padding any gap with empty strings so positions keep their meaning.
    let insert_at = if end < tokens.len() {
        tokens[end].start
    } else {
        line.len()
    };
    let padding = " \"\"".repeat(target.saturating_sub(end));
    let tail = &line[insert_at..];
    let head = line[..insert_at].trim_end();
    let tail = if tail.is_empty() {
        String::new()
    } else {
        format!(" {}", tail.trim_start())
    };
    format!("{head}{padding} {quoted}{tail}")
}

/// `code` as tokens, quotes removed from quoted ones and `\"` unescaped inside them, braces tokens of their own. An
/// unterminated quote yields what there is rather than failing: the parser reports a malformed line better than a
/// tokenizer can.
pub fn split(code: &str) -> Vec<String> {
    let characters: Vec<char> = code.chars().collect();
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut started = false;
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        if in_quotes {
            if character == '\\' && characters.get(index + 1) == Some(&'"') {
                current.push('"');
                index += 2;
                continue;
            }
            if character == '"' {
                in_quotes = false;
                index += 1;
                continue;
            }
            current.push(character);
            index += 1;
            continue;
        }
        if character == '"' {
            in_quotes = true;
            started = true;
            index += 1;
            continue;
        }
        if character.is_whitespace() {
            if started {
                tokens.push(std::mem::take(&mut current));
                started = false;
            }
            index += 1;
            continue;
        }
        // Braces are their own tokens, so "model {" and "model{" read the same.
        if character == '{' || character == '}' {
            if started {
                tokens.push(std::mem::take(&mut current));
                started = false;
            }
            tokens.push(character.to_string());
            index += 1;
            continue;
        }
        current.push(character);
        started = true;
        index += 1;
    }
    if started {
        tokens.push(current);
    }
    tokens
}
