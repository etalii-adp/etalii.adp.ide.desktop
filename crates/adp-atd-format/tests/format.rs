//! The formatter against ATD §16 (spec 035, T096): formatting every example twice changes nothing the second time,
//! keeps every comment in its place among the tokens, keeps the tokens, and leaves a file that does not parse as it
//! is; a splice leaves every byte outside it unchanged.

use std::path::PathBuf;

use adp_atd_format::{Splice, apply, format, splice_between};
use adp_atd_syntax::TokenKind;

fn spec_dir() -> PathBuf {
    match std::env::var_os("ATD_SPEC_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/specifications/atd"),
    }
}

fn examples() -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = std::fs::read_dir(spec_dir())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "atd"))
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).unwrap(),
            )
        })
        .collect();
    found.sort();
    found
}

/// The tokens and comments of a text, without whitespace: what formatting must keep, in order.
fn significant(text: &str) -> Vec<String> {
    let parsed = adp_atd_syntax::parse(text);
    parsed
        .tokens
        .iter()
        .filter(|t| {
            t.kind != TokenKind::Trivia || t.text(text).trim_start().starts_with("//") || t.text(text).starts_with("/*")
        })
        .map(|t| t.text(text).trim_end().to_owned())
        .collect()
}

#[test]
fn formatting_is_idempotent_and_keeps_every_token_and_comment() {
    for (name, text) in examples() {
        let once = format(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
        let twice = format(&once).unwrap_or_else(|e| panic!("{name}, formatted: {e}"));
        assert_eq!(once, twice, "{name}: the second formatting changed the file");
        assert_eq!(
            significant(&text),
            significant(&once),
            "{name}: formatting changed a token or moved a comment"
        );
    }
}

#[test]
fn comments_stay_with_their_constructs_and_layout_is_canonical() {
    let text = "atd 0.1\n// before the tool\ndiagram a/b version \"1\" {   label   \"x\"   // on the label's line\n\n\n  model { element A { attribute n:   string  required\n   /* before contains */ contains A ordered } }\n  // last in the tool\n}\n";
    let formatted = format(text).unwrap();
    let expected = "atd 0.1\n// before the tool\ndiagram a/b version \"1\" {\n  label \"x\"  // on the label's line\n\n  model {\n    element A {\n      attribute n: string required\n      /* before contains */\n      contains A ordered\n    }\n  }\n  // last in the tool\n}\n";
    assert_eq!(formatted, expected);
    assert_eq!(format(&formatted).unwrap(), formatted);
}

#[test]
fn a_block_of_settings_stays_on_one_line_when_it_fits() {
    let text = "atd 0.1\ndiagram a/b version \"1\" {\n  model {\n    element A {\n      attribute n: string {\n        label: \"N\"\n        order: 1\n      }\n    }\n  }\n}\n";
    let formatted = format(text).unwrap();
    assert!(
        formatted.contains("attribute n: string { label: \"N\" order: 1 }"),
        "{formatted}"
    );
}

#[test]
fn a_file_that_does_not_parse_is_not_formatted() {
    let error = format("atd 0.1\ndiagram a/b version \"1\" {\n  model { element }\n").unwrap_err();
    assert!(error.contains("does not parse"), "{error}");
}

#[test]
fn a_splice_leaves_every_byte_outside_it_unchanged() {
    let (_, text) = examples().into_iter().find(|(n, _)| n == "mindmap.atd").unwrap();
    let at = text.find("\"Mind map\"").unwrap();
    let splice = Splice {
        start: at,
        end: at + "\"Mind map\"".len(),
        text: "\"Mind maps\"".into(),
    };
    let edited = apply(&text, std::slice::from_ref(&splice));
    assert_eq!(&edited[..at], &text[..at]);
    assert_eq!(&edited[at + splice.text.len()..], &text[splice.end..]);
    assert_eq!(
        splice_between(&text, &edited),
        Some(Splice {
            start: at + 9,
            end: at + 9,
            text: "s".into()
        })
    );
}

/// A binding's grammar is ATD grammar notation, not ATD: its lines are kept as written, and only their indentation
/// follows the block they stand in (ATD §11.9, §16).
#[test]
fn a_bindings_grammar_keeps_its_lines() {
    let written = std::fs::read_to_string(spec_dir().join("binding-grammar.atd")).unwrap();
    let grammar = |text: &str| -> Vec<String> {
        let open = text.find("grammar {\n").expect("an inline grammar") + "grammar {\n".len();
        let close = open + text[open..].find("\n    }\n").expect("its closing brace");
        text[open..close].lines().map(str::to_owned).collect()
    };

    let formatted = format(&written).unwrap();

    assert_eq!(grammar(&formatted), grammar(&written));
    // Indented one level deeper, the grammar is written one level deeper.
    let deeper = written.replace("\n      ", "\n        ");
    let reformatted = format(&deeper).unwrap();
    assert_eq!(grammar(&reformatted), grammar(&written));
}
