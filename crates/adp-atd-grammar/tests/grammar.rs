//! `adp-atd-grammar` against contracts/grammar.md and ATD §3.6 and §4 (spec 035, T032): the notation's admitted and
//! refused forms, a grammar with each kind of conflict refused with the conflict named as Lark names it, and the
//! contextual lexer's rule.

use adp_atd_grammar::{BuildError, build_single};

fn conflicts(text: &str) -> Vec<String> {
    match build_single(text, "start") {
        Err(BuildError::Conflicts(found)) => found.iter().map(|c| c.to_string()).collect(),
        Err(other) => panic!("another error: {other}"),
        Ok(_) => Vec::new(),
    }
}

#[test]
fn every_admitted_form_reads() {
    let built = build_single(
        r#"
?start: item* [tail] -> top
item: _part+ "." | "(" start ")" -> group
_part: NAME | KEY
tail: ";" ("," NAME)+
NAME: /[a-z]+/
KEY.2: /[A-Z]+/
_SKIP: /[ ]+/
%ignore _SKIP
"#,
        "start",
    );
    assert!(built.is_ok(), "{built:?}");
}

#[test]
fn a_shift_reduce_conflict_is_refused() {
    let found = conflicts("start: a | a \"x\"\na: \"x\" | \"x\" \"x\"\n");
    assert!(
        found
            .iter()
            .any(|c| c.starts_with("Shift/Reduce conflict for terminal \"x\"")),
        "{found:?}"
    );
}

#[test]
fn a_reduce_reduce_conflict_is_refused() {
    let found = conflicts("start: a | b\na: \"x\"\nb: \"x\"\n");
    assert!(
        found
            .iter()
            .any(|c| c.starts_with("Reduce/Reduce conflict for terminal $END")),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|c| c.contains("<a : \"x\">") && c.contains("<b : \"x\">")),
        "{found:?}"
    );
}

#[test]
fn a_terminal_collision_is_refused() {
    match build_single("start: A | B\nA: /[a-z]+/\nB: /[a-c]+/\n", "start") {
        Err(BuildError::Collisions(found)) => {
            assert_eq!((found[0].first.as_str(), found[0].second.as_str()), ("A", "B"));
            assert!(found[0].to_string().starts_with("Collision between Terminals A and B."));
        }
        other => panic!("expected a collision, got {other:?}"),
    }
}

#[test]
fn terminals_a_state_does_not_accept_together_do_not_collide() {
    assert!(build_single("start: \"a\" A | \"b\" B\nA: /[a-z]+/\nB: /[a-c]+/\n", "start").is_ok());
}

#[test]
fn the_lexer_reads_as_lark_reads() {
    let built = build_single(
        "start: (WORD | \"if\" | \"=\" | \"==\" | NUM | DEC)*\nWORD: /[a-z]+/\nNUM: /[0-9]+/\nDEC.1: /[0-9]+/\n",
        "start",
    );
    // NUM and DEC collide unless their priorities differ: they do, so this builds.
    let built = built.expect("builds");
    let lexer = &built.lexer;
    let all: Vec<usize> = (0..lexer.matchers.len()).collect();
    let name = |text: &str| {
        let lexeme = lexer.next(text, 0, &all).expect("a token");
        (lexer.matchers[lexeme.terminal].name.clone(), lexeme.end)
    };
    assert_eq!(
        name("iffy"),
        ("WORD".to_owned(), 4),
        "a word that starts with a keyword of its priority"
    );
    assert_eq!(
        name("if x"),
        ("\"if\"".to_owned(), 2),
        "a keyword of the pattern that matches it whole"
    );
    assert_eq!(name("== 1"), ("\"==\"".to_owned(), 2), "the longest string");
    assert_eq!(name("42"), ("DEC".to_owned(), 2), "the higher priority");
}

#[test]
fn a_filtered_rule_has_no_alias() {
    assert!(matches!(
        build_single("start: _a\n_a: \"x\" -> y\n", "start"),
        Err(BuildError::Compose(_))
    ));
}

#[test]
fn constructs_outside_the_notation_are_refused() {
    for text in [
        "start: a\na.2: \"x\"\n",
        "start: \"x\"s\n",
        "start: /x/i\n",
        "%declare X\nstart: \"x\"\n",
    ] {
        assert!(
            matches!(build_single(text, "start"), Err(BuildError::Compose(_))),
            "{text}"
        );
    }
}

/// ATD §4.1 as amended on 2026-11-01: a string may be case-insensitive, `"x"i`, and a keyword written so at the
/// priority of the word it stands in for is that word's keyword in any case, as Lark's lexer reads it (§3.6).
#[test]
fn a_case_insensitive_string_is_a_keyword_in_any_case() {
    let built = build_single(
        "start: (_GROUP | WORD)*\n_GROUP: \"group\"i\nWORD: /[A-Za-z]+/\n_S: /[ ]+/\n%ignore _S\n",
        "start",
    )
    .expect("builds");
    let lexer = &built.lexer;
    let all: Vec<usize> = (0..lexer.matchers.len()).collect();
    let name = |text: &str| {
        let lexeme = lexer.next(text, 0, &all).expect("a token");
        (lexer.matchers[lexeme.terminal].name.clone(), lexeme.end)
    };
    assert_eq!(name("group x"), ("_GROUP".to_owned(), 5));
    assert_eq!(name("GROUP x"), ("_GROUP".to_owned(), 5));
    assert_eq!(name("Group x"), ("_GROUP".to_owned(), 5));
    assert_eq!(
        name("grouped x"),
        ("WORD".to_owned(), 7),
        "a word that starts with the keyword"
    );
}

/// ATD §4.7: the normalized tables name a terminal's flags, as `atd-grammar.py --tables` does.
#[test]
fn the_tables_name_a_case_insensitive_string_by_its_flag() {
    let built = build_single("start: _GROUP WORD\n_GROUP: \"group\"i\nWORD: /[a-z]+/\n", "start").expect("builds");
    let tables = adp_atd_grammar::tables::to_json(&built.grammar, &built.tables, "g.lark");
    assert!(
        tables.contains(r#"{"name": "_GROUP", "kind": "string", "value": "group", "priority": 0, "flags": "i"}"#),
        "{tables}"
    );
    assert!(
        tables.contains(r#"{"name": "WORD", "kind": "pattern", "value": "[a-z]+", "priority": 0}"#),
        "{tables}"
    );
}
