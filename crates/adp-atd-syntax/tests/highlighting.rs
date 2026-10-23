//! The generated TextMate grammar against the lexer (etalii.adp spec 035, T097, FR-052): tokenizing every vendored
//! example with `highlighting/atd.sublime-syntax`, as `syntect` and the desktop host's editor do, gives each token
//! the scope of the class ATD §3.3 gives the token the ATD lexer reads there.
//!
//! One difference is allowed, because a TextMate grammar has no parser: a run of words and dots followed by `:` (and
//! not `::`) may take the property-name scope, as a setting's key does, since the lexer reads it as a key only where
//! the grammar admits a setting.

use std::path::PathBuf;

use adp_atd_syntax::TokenKind;
use syntect::parsing::{ParseState, ScopeStack, ScopeStackOp, SyntaxDefinition, SyntaxSetBuilder};

const PROPERTY: &str = "support.type.property-name.";

fn spec_dir() -> PathBuf {
    match std::env::var_os("ATD_SPEC_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/specifications/atd"),
    }
}

/// The scopes a token of this terminal may start with: those of its class (ATD §3.3).
fn scopes_of(terminal: &str, text: &str) -> &'static [&'static str] {
    match terminal {
        "COMMENT" | "BLOCK_COMMENT" => &["comment."],
        "STRING" | "BYTES" => &["string."],
        "INT" | "UINT" | "DOUBLE" | "VERSION" | "LENGTH" => &["constant.numeric."],
        "CAPABILITY_RANGE" => &["constant.other.capability-range."],
        "ORIGIN" => &["entity.name.namespace.origin."],
        "ATTR_REF" => &["variable.other.attribute-reference."],
        "TOKEN_REF" => &["variable.other.token-reference."],
        "PARAM_REF" => &["variable.parameter.param-reference."],
        "KEY" => &[PROPERTY],
        "DASHED_WORD" => &["variable.other.word."],
        // CEL's literals, which the grammar reads as names.
        "NAME" | "ASSIGN" if matches!(text, "true" | "false" | "null") => &["constant.language."],
        "NAME" | "ASSIGN" => &["variable.other.name."],
        keyword if keyword.starts_with('"') && keyword[1..].starts_with(|c: char| c.is_ascii_alphabetic()) => {
            if matches!(text, "true" | "false" | "null") {
                &["constant.language."]
            } else {
                &["keyword.control.", "keyword.other."]
            }
        }
        // Every other string terminal: an operator or punctuation.
        _ => &["keyword.operator.", "punctuation."],
    }
}

/// The innermost scope at every byte of `text`.
fn scopes(text: &str) -> Vec<String> {
    let grammar = std::fs::read_to_string(spec_dir().join("highlighting/atd.sublime-syntax")).unwrap();
    let mut builder = SyntaxSetBuilder::new();
    builder.add(SyntaxDefinition::load_from_str(&grammar, true, None).unwrap());
    let set = builder.build();
    let syntax = set.find_syntax_by_extension("atd").unwrap();
    let mut scopes = vec![String::new(); text.len()];
    let mut state = ParseState::new(syntax);
    let mut stack = ScopeStack::new();
    let mut at = 0;
    for line in text.split_inclusive('\n') {
        let ops = state.parse_line(line, &set).unwrap();
        let mut last = 0;
        for (i, op) in ops.iter().chain(std::iter::once(&(line.len(), ScopeStackOp::Noop))) {
            let top = stack.as_slice().last().map(|s| s.build_string()).unwrap_or_default();
            for scope in &mut scopes[at + last..at + *i] {
                scope.clone_from(&top);
            }
            last = *i;
            stack.apply(op).unwrap();
        }
        at += line.len();
    }
    scopes
}

#[test]
fn every_examples_tokens_take_the_scope_of_their_class() {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(spec_dir())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "atd"))
        .collect();
    paths.sort();
    let mut tokens = 0;
    let mut failures = Vec::new();
    for path in &paths {
        let text = std::fs::read_to_string(path).unwrap();
        let scopes = scopes(&text);
        let parsed = adp_atd_syntax::parse(&text);
        assert!(parsed.errors.is_empty(), "{} parses", path.display());
        for token in &parsed.tokens {
            let token_text = token.text(&text);
            if token.start == token.end || token_text.trim().is_empty() {
                continue;
            }
            tokens += 1;
            let terminal = match token.kind {
                TokenKind::Trivia if token_text.starts_with("/*") => "BLOCK_COMMENT",
                TokenKind::Trivia => "COMMENT",
                _ => token.name(),
            };
            let scope = &scopes[token.start];
            let allowed = scopes_of(terminal, token_text);
            // Within a run the grammar scopes as a key: the run ends before spaces and `:`.
            let in_key = scope.starts_with(PROPERTY) && {
                let end = (token.start..text.len())
                    .find(|&i| !scopes[i].starts_with(PROPERTY))
                    .unwrap_or(text.len());
                let rest = text[end..].trim_start_matches([' ', '\t']);
                rest.starts_with(':') && !rest.starts_with("::")
            };
            if !in_key && !allowed.iter().any(|a| scope.starts_with(a)) {
                let line = text[..token.start].matches('\n').count() + 1;
                failures.push(format!(
                    "{}:{line}: {terminal} {token_text:?} has {scope:?}, expected one of {allowed:?}",
                    path.file_name().unwrap().to_string_lossy()
                ));
            }
        }
    }
    assert!(
        paths.len() >= 8 && tokens > 1000,
        "{} examples, {tokens} tokens",
        paths.len()
    );
    assert!(
        failures.is_empty(),
        "{} tokens:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
