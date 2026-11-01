//! Semantic tokens (contracts/language-server.md): each token by its class, and each name by what it names.

use adp_atd_syntax::TokenKind;

use crate::index::Index;
use crate::positions::Encoding;

pub const TYPES: &[&str] = &[
    "keyword",
    "type",
    "property",
    "function",
    "variable",
    "parameter",
    "string",
    "number",
    "comment",
    "operator",
    "namespace",
    "enumMember",
    "macro",
];
pub const MODIFIERS: &[&str] = &["declaration", "readonly", "documentation"];

fn index_of(kind: &str) -> u32 {
    TYPES.iter().position(|t| *t == kind).unwrap_or(0) as u32
}

/// The tokens of a file as the protocol's relative integers, within byte `bounds` when given.
pub fn encode(text: &str, file: &str, index: &Index, encoding: Encoding, bounds: Option<(usize, usize)>) -> Vec<u32> {
    let parsed = adp_atd_syntax::parse(text);
    let mut data = Vec::new();
    let (mut last_line, mut last_start) = (0u32, 0u32);
    for token in &parsed.tokens {
        if let Some((from, to)) = bounds
            && (token.end <= from || token.start >= to)
        {
            continue;
        }
        let token_text = token.text(text);
        let occurrence = index
            .at(file, token.start)
            .filter(|o| o.start == token.start || o.start == token.start + 1);
        let (kind, modifiers) = if token.kind == TokenKind::Trivia {
            if token_text.starts_with("///") {
                ("comment", 4)
            } else if token_text.starts_with("//") || token_text.starts_with("/*") {
                ("comment", 0)
            } else {
                continue;
            }
        } else if let Some(o) = occurrence {
            let kind = match o.symbol.split(':').next().unwrap_or("") {
                "type" => "type",
                "attr" => "property",
                "fn" | "op" => "function",
                "local" => "variable",
                "capability" => "macro",
                "rule" | "binding" | "style" | "form" | "reason" | "algorithm" => "variable",
                _ => "variable",
            };
            (kind, u32::from(o.declaration))
        } else {
            match token.name() {
                _ if token.is_keyword() => ("keyword", 0),
                "NAME" | "DASHED_WORD" => ("variable", 0),
                "KEY" => ("property", 0),
                "STRING" | "GRAMMAR_STRING" | "BYTES" => ("string", 0),
                "INT" | "UINT" | "DOUBLE" | "LENGTH" | "VERSION" => ("number", 0),
                "ATTR_REF" => ("property", 0),
                "TOKEN_REF" => ("variable", 2),
                "PARAM_REF" => ("parameter", 0),
                "ORIGIN" => ("namespace", 0),
                "CAPABILITY_RANGE" => ("macro", 0),
                "ASSIGN" => ("variable", 0),
                _ => ("operator", 0),
            }
        };
        // One protocol token per line of a token that spans lines.
        let mut piece_start = token.start;
        for piece in token_text.split('\n') {
            let trimmed = piece.trim_end_matches('\r');
            let position = crate::positions::position(text, piece_start, encoding);
            let line = position["line"].as_u64().unwrap_or(0) as u32;
            let start = position["character"].as_u64().unwrap_or(0) as u32;
            let length: usize = trimmed
                .chars()
                .map(|c| match encoding {
                    Encoding::Utf8 => c.len_utf8(),
                    Encoding::Utf16 => c.len_utf16(),
                    Encoding::Utf32 => 1,
                })
                .sum();
            if length > 0 {
                let delta_line = line - last_line;
                let delta_start = if delta_line == 0 { start - last_start } else { start };
                data.extend([delta_line, delta_start, length as u32, index_of(kind), modifiers]);
                last_line = line;
                last_start = start;
            }
            piece_start += piece.len() + 1;
        }
    }
    data
}
