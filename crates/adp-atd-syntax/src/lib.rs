//! The parser of ATD (etalii.adp spec 035, research R9, R10; ATD §3, §5): the version a file declares, the contextual
//! lexer of §3.6 and the LALR(1) tables built from the grammar at build time, a lossless tree whose tokens and trivia
//! are every byte of the file, recovery from syntax errors at the next construct, and the lowering to the abstract
//! tree of `atd.schema.json#/$defs/Tree`.

pub mod ast;
mod lower;
mod parse;
pub mod tables;
pub mod text;

pub use ast::Span;
pub use lower::{LowerError, decode_string, key_text, lower};
pub use parse::{Child, Node, Parse, SyntaxError, Token, TokenKind, describe as describe_terminal, expected_at, parse};
pub use text::LineIndex;

/// The versions of ATD this reader reads (ATD §5.4, §18).
pub const VERSIONS: &[&str] = &[tables::VERSION];

/// The version a file declares in its header, and its byte range: `atd <major>.<minor>` after a byte-order mark,
/// whitespace and comments. `None` when the file does not start with a header.
pub fn declared_version(text: &str) -> Option<(String, usize, usize)> {
    let bytes = text.as_bytes();
    let mut i = if text.starts_with('\u{feff}') { 3 } else { 0 };
    loop {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if text[i..].starts_with("//") {
            i += text[i..].find('\n').unwrap_or(text.len() - i);
        } else if text[i..].starts_with("/*") {
            i += text[i..].find("*/").map(|p| p + 2).unwrap_or(text.len() - i);
        } else {
            break;
        }
    }
    let after = text[i..].strip_prefix("atd")?;
    if !after.starts_with([' ', '\t']) {
        return None;
    }
    let skipped = after.len() - after.trim_start_matches([' ', '\t']).len();
    let start = i + 3 + skipped;
    let version: String = text[start..]
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    if version.is_empty() {
        return None;
    }
    Some((version.clone(), start, start + version.len()))
}
