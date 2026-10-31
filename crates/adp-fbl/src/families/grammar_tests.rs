//! What the `grammar` family does of FBL 0.11 that the fixtures do not pin down, on a small grammar of named items
//! with blocks: `item NAME [title [note]] [{ ... }]`, one per line, with line and block comments.

use std::rc::Rc;

use adp_cel::{Map, Value};
use serde_json::{Value as Json, json};

use crate::documents::loader::load;
use crate::documents::types::{FblBinding, SlotEscape};
use crate::history::{OpenBody, UndoResult};
use crate::model::{FblModel, FblOptions, codes};
use crate::planning::{ModelChange, PlanResult};

use super::split_items;
use crate::text::Span;

const GRAMMAR: &str = r#"start: unit*
?unit: item
     | pair
     | _NL
pair: "=" NAME
item: NAME [title [note]] [_block]
_block: "{" _block_item* "}"
_block_item: item _NL
           | _NL
title: STRING
note: STRING
NAME: /[a-z]+/
STRING: /"([^"\\\r\n]|\\[^\r\n])*"/
_NL: /(\r?\n[ \t]*)+/
WHITESPACE: /[ \t]+/
COMMENT: /\/\/[^\r\n]*/
BLOCK: /\/\*([^*]|\*+[^*\/])*(\*+\/)?/
%ignore WHITESPACE
%ignore COMMENT
%ignore BLOCK
"#;

const MATCH: &str = r#""?(?<value>(?:[^"\\]|\\.)*)"?"#;

fn binding_with(rule: Json) -> Rc<FblBinding> {
    binding_of(GRAMMAR, "{", rule)
}

fn binding_of(grammar: &str, open: &str, rule: Json) -> Rc<FblBinding> {
    let mut item = json!({
        "name": "item",
        "type": "Item",
        "at": "/**/item",
        "id": { "from": { "node": "NAME" } },
        "parent": { "rules": ["item"] },
        "opens": { "open": " {", "close": "}" },
        "attributes": {
            "title": {
                "node": "title",
                "match": MATCH,
                "escape": { "with": "\\", "quote": "\"" },
                "create": { "emit": " \"{value}\"", "after": ["name"] }
            },
            "note": {
                "node": "note",
                "match": MATCH,
                "escape": { "with": "\\", "quote": "\"" },
                "create": { "emit": " \"{value}\"", "after": ["title", "name"], "pad": true }
            },
            "name": { "node": "NAME", "readOnly": true }
        },
        "insert": { "place": "last-child", "emit": "{id} \"{title}\"" },
        "remove": {}
    });
    for (key, value) in rule.as_object().unwrap() {
        item[key] = value.clone();
    }
    let document = json!({
        "fbl": "0.11",
        "bindings": {
            "items": {
                "claims": { "extensions": [".items"], "origins": ["example/items"] },
                "body": {
                    "kind": "file",
                    "family": "grammar",
                    "grammar": grammar,
                    "recover": {
                        "unit": "unit",
                        "sync": ["_NL"],
                        "nested": { "open": open, "close": "}" },
                        "unclosed": { "BLOCK": "*/" }
                    }
                },
                "reader": "declared",
                "text": { "indent": 2 },
                "elements": [item]
            }
        }
    });
    let (problems, document) = load(document.to_string().as_bytes());
    let document = document.unwrap_or_else(|| panic!("{problems:?}"));
    Rc::new(document.bindings.into_iter().next().unwrap().1)
}

fn open(text: &str) -> OpenBody {
    OpenBody::open(
        text.as_bytes().to_vec(),
        binding_with(json!({})),
        Rc::new(FblOptions::default()),
    )
}

fn codes_of(model: &FblModel) -> Vec<&str> {
    model.findings.iter().map(|f| f.code.as_str()).collect()
}

fn set(id: &str, attribute: &str, value: &str) -> ModelChange {
    let mut attributes = Map::default();
    attributes.insert(attribute, Value::from(value));
    ModelChange::Set {
        id: id.into(),
        attributes,
    }
}

fn add(id: &str, parent: Option<&str>, title: &str) -> ModelChange {
    let mut attributes = Map::default();
    attributes.insert("title", Value::from(title));
    ModelChange::Add {
        type_name: "Item".into(),
        id: Some(id.into()),
        attributes,
        parent_id: parent.map(str::to_owned),
        index: None,
        slot: None,
    }
}

fn text_of(body: &OpenBody) -> &str {
    std::str::from_utf8(body.bytes()).unwrap()
}

#[test]
fn a_comment_opening_inside_a_token_is_the_tokens() {
    let body = open("a \"see http://x//y /* z\" // real\n");

    let model = body.model();
    assert_eq!(
        model.find("a").unwrap().attributes.get("title"),
        Some(&Value::from("see http://x//y /* z"))
    );
    assert_eq!(codes_of(&model), Vec::<&str>::new());
}

#[test]
fn a_comment_that_runs_over_lines_goes_with_its_owner() {
    let mut body = open("a \"A\" /* over\n two lines */\nb \"B\"\n");

    let removed = body.change(&ModelChange::Remove { id: "a".into() });

    assert!(matches!(removed, PlanResult::Planned(_)), "{removed:?}");
    assert_eq!(text_of(&body), "b \"B\"\n");
}

#[test]
fn a_comment_above_at_another_indentation_or_after_a_blank_line_leads_nothing() {
    let mut body = open("// kept\n\n  // other\nb \"B\"\n// leads\nc \"C\"\n");

    let removed = body.change(&ModelChange::Remove { id: "c".into() });
    assert!(matches!(removed, PlanResult::Planned(_)), "{removed:?}");
    assert_eq!(text_of(&body), "// kept\n\n  // other\nb \"B\"\n");
    assert!(matches!(body.undo(), UndoResult::Done(_)));

    let removed = body.change(&ModelChange::Remove { id: "b".into() });
    assert!(matches!(removed, PlanResult::Planned(_)), "{removed:?}");
    assert_eq!(text_of(&body), "// kept\n\n  // other\n// leads\nc \"C\"\n");
}

#[test]
fn a_comment_before_an_entry_on_its_line_is_trivia() {
    let mut body = open("/* a */ b \"B\"\nc\n");

    let removed = body.change(&ModelChange::Remove { id: "b".into() });

    assert!(matches!(removed, PlanResult::Planned(_)), "{removed:?}");
    assert_eq!(text_of(&body), "c\n");
}

#[test]
fn escapes_are_resolved_read_and_written_only_on_a_quote_or_the_escape() {
    let escape = SlotEscape { with: '\\', quote: '"' };
    assert_eq!(escape.read(r#"Jo \"the\" C:\temp \\"#), r#"Jo "the" C:\temp \"#);
    assert_eq!(escape.write(r#"Jo "the" C:\temp"#), r#"Jo \"the\" C:\\temp"#);
    assert_eq!(escape.read(&escape.write(r#"a\"b\\"#)), r#"a\"b\\"#);
}

#[test]
fn a_new_entrys_values_are_escaped_and_read_back() {
    let mut body = open("a {\n  b \"B\"\n}\n");

    let added = body.change(&add("c", Some("a"), "Say \"hi\""));

    assert!(matches!(added, PlanResult::Planned(_)), "{added:?}");
    assert_eq!(text_of(&body), "a {\n  b \"B\"\n  c \"Say \\\"hi\\\"\"\n}\n");
    let refused = body.change(&add("d", Some("a"), "two\nlines"));
    assert!(matches!(refused, PlanResult::Refused(_)), "{refused:?}");
}

#[test]
fn two_absent_parts_set_in_one_edit_are_written_once_each() {
    let mut body = open("a // c\n");
    let mut attributes = Map::default();
    attributes.insert("title", Value::from("T"));
    attributes.insert("note", Value::from("N"));

    let set = body.change(&ModelChange::Set {
        id: "a".into(),
        attributes,
    });

    assert!(matches!(set, PlanResult::Planned(_)), "{set:?}");
    assert_eq!(text_of(&body), "a \"T\" \"N\" // c\n");
}

#[test]
fn an_absent_part_is_padded_and_refused_without_a_place() {
    let mut body = open("a // c\n");

    let padded = body.change(&set("a", "note", "N"));

    assert!(matches!(padded, PlanResult::Planned(_)), "{padded:?}");
    assert_eq!(text_of(&body), "a \"\" \"N\" // c\n");
}

#[test]
fn a_first_child_goes_into_an_empty_block_before_its_closing_line() {
    let mut body = open("a {\n}\n");

    let added = body.change(&add("b", Some("a"), "B"));

    assert!(matches!(added, PlanResult::Planned(_)), "{added:?}");
    assert_eq!(text_of(&body), "a {\n  b \"B\"\n}\n");
}

#[test]
fn a_block_is_written_before_a_trailing_comment_and_undone_whole() {
    let original = "a \"A\" // c\n";
    let mut body = open(original);

    let added = body.change(&add("b", Some("a"), "B"));

    assert!(matches!(added, PlanResult::Planned(_)), "{added:?}");
    assert_eq!(text_of(&body), "a \"A\" { // c\n  b \"B\"\n}\n");
    assert!(matches!(body.undo(), UndoResult::Done(_)));
    assert_eq!(text_of(&body), original);
}

#[test]
fn a_block_written_on_a_last_line_without_an_ending_keeps_the_body_without_one() {
    let mut body = open("a \"A\"");

    let added = body.change(&add("b", Some("a"), "B"));

    assert!(matches!(added, PlanResult::Planned(_)), "{added:?}");
    assert_eq!(text_of(&body), "a \"A\" {\n  b \"B\"\n}");
}

#[test]
fn a_line_feed_in_open_writes_a_line_ending_and_the_entrys_indentation() {
    let grammar = format!(
        "{}_OPEN: /(\\r?\\n[ \\t]*)*\\{{/\n",
        GRAMMAR.replace("_block: \"{\" ", "_block: _OPEN ")
    );
    let binding = binding_of(&grammar, "_OPEN", json!({ "opens": { "open": "\n{", "close": "}" } }));
    let mut body = OpenBody::open(b"p {\n  a\n}\n".to_vec(), binding, Rc::new(FblOptions::default()));

    let added = body.change(&add("b", Some("a"), "B"));

    assert!(matches!(added, PlanResult::Planned(_)), "{added:?}");
    assert_eq!(text_of(&body), "p {\n  a\n  {\n    b \"B\"\n  }\n}\n");
}

#[test]
fn a_statement_that_does_not_parse_in_a_block_is_skipped_from_its_boundary() {
    let body = open("a {\n  b \"B\" \"N\" \"too many\"\n  c \"C\"\n}\nd\n");

    let model = body.model();
    assert!(model.find("b").is_none());
    assert_eq!(model.find("c").unwrap().parent_id.as_deref(), Some("a"));
    assert!(model.find("d").is_some());
    assert_eq!(codes_of(&model), [codes::UNPARSED_TEXT]);
    assert_eq!(model.findings[0].location.as_ref().unwrap().line, 2);
}

#[test]
fn a_skip_ends_before_a_close_at_its_blocks_depth() {
    let body = open("a {\n  b \"B\" \"N\" \"too many\" }\nc\n");

    let model = body.model();
    assert_eq!(codes_of(&model), [codes::UNPARSED_TEXT]);
    assert!(model.find("a").is_some());
    assert_eq!(model.find("c").unwrap().parent_id, None);
}

#[test]
fn a_block_the_body_ends_in_is_closed_with_its_statement_skipped() {
    let body = open("a {\n  b \"B\"\n  c \"C\" \"N\" \"too many");

    let model = body.model();
    assert!(model.find("b").is_some());
    assert!(model.find("c").is_none());
    let found = codes_of(&model);
    assert!(found.contains(&codes::UNCLOSED_BLOCK), "{found:?}");
    assert_eq!(
        found.iter().filter(|c| **c == codes::UNPARSED_TEXT).count(),
        1,
        "{found:?}"
    );
}

#[test]
fn nothing_is_added_to_an_unclosed_block_and_a_closed_one_beside_it_takes_an_add() {
    let mut body = open("a {\n  b {\n    c\n  }\n");

    assert_eq!(
        body.change(&add("d", Some("a"), "D")),
        PlanResult::Refused("The block is not closed, so nothing can be added to it.".into())
    );
    let added = body.change(&add("d", Some("b"), "D"));
    assert!(matches!(added, PlanResult::Planned(_)), "{added:?}");
    assert_eq!(text_of(&body), "a {\n  b {\n    c\n    d \"D\"\n  }\n");
}

#[test]
fn a_body_that_parses_reports_nothing_with_nested_recovery() {
    let body = open("// c\na {\n  b \"B\" // d\n}\n");

    assert_eq!(codes_of(&body.model()), Vec::<&str>::new());
}

#[test]
fn a_close_after_a_part_of_a_top_level_statement_is_skipped_alone() {
    let body = open("= } a\nb\n}\nc\n");

    let model = body.model();
    assert!(model.find("b").is_some() && model.find("c").is_some());
    assert_eq!(codes_of(&model), [codes::UNMATCHED_CLOSE, codes::UNMATCHED_CLOSE]);
    assert_eq!(model.findings[0].location.as_ref().unwrap().line, 1);
}

#[test]
fn items_are_cut_at_the_text_and_trimmed_and_an_empty_piece_is_none() {
    let text = " Extra ,More,, ";
    let items = split_items(text, Span::new(10, 10 + text.len()), ",");

    assert_eq!(items, [("Extra", Span::new(11, 16)), ("More", Span::new(18, 22))]);
    assert_eq!(split_items("", Span::new(0, 0), ","), []);
}

#[test]
fn a_gathered_binding_that_is_not_read_only_is_refused_when_loaded() {
    let document = json!({
        "fbl": "0.11",
        "bindings": {
            "items": {
                "claims": { "extensions": [".items"] },
                "body": { "kind": "file", "family": "grammar", "grammar": GRAMMAR },
                "reader": "declared",
                "elements": [{
                    "name": "item", "type": "Item", "at": "/item",
                    "attributes": { "tags": { "node": "title", "gather": [{ "node": "note" }] } }
                }]
            }
        }
    });

    let (problems, loaded) = load(document.to_string().as_bytes());

    assert!(loaded.is_none());
    assert!(problems.iter().any(|p| p.pointer.ends_with("/gather")), "{problems:?}");
}

#[test]
fn a_split_holding_an_escape_character_is_refused_when_loaded() {
    let document = json!({
        "fbl": "0.13",
        "bindings": {
            "items": {
                "claims": { "extensions": [".items"] },
                "body": { "kind": "file", "family": "grammar", "grammar": GRAMMAR },
                "reader": "declared",
                "elements": [{
                    "name": "item", "type": "Item", "at": "/item",
                    "attributes": { "tags": {
                        "node": "title", "split": "\\", "escape": { "with": "\\", "quote": "\"" }
                    } }
                }]
            }
        }
    });

    let (problems, loaded) = load(document.to_string().as_bytes());

    assert!(loaded.is_none());
    assert!(problems.iter().any(|p| p.pointer.ends_with("/split")), "{problems:?}");
}

/// A binding whose block rules name views: `selectedView` rules read only in the view the reading shows, and a block
/// rule with `origins` is a view for readings of those origins only (FBL 0.11 sections 4.9, 9.3).
fn views(origin: Option<&str>, view: Option<&str>) -> FblModel {
    let document = json!({
        "fbl": "0.11",
        "bindings": {
            "items": {
                "claims": { "extensions": [".items"], "origins": ["o/a", "o/b"] },
                "body": { "kind": "file", "family": "grammar", "grammar": GRAMMAR },
                "reader": "declared",
                "blocks": [
                    { "name": "a", "at": "/item[NAME='a']", "origins": ["o/a"], "view": { "node": "title", "match": MATCH } },
                    { "name": "b", "at": "/item[NAME='b']", "origins": ["o/b"], "view": { "value": "'bee'" } }
                ],
                "elements": [{
                    "name": "inner", "type": "Inner", "at": "/item/item", "selectedView": true, "attributes": {}
                }]
            }
        }
    });
    let (problems, loaded) = load(document.to_string().as_bytes());
    let binding = loaded
        .unwrap_or_else(|| panic!("{problems:?}"))
        .bindings
        .into_iter()
        .next()
        .unwrap()
        .1;
    let options = FblOptions {
        origin: origin.map(str::to_owned),
        view: view.map(str::to_owned),
        ..FblOptions::default()
    };
    OpenBody::open(
        b"a \"First\" {\n  x\n}\nb {\n  y\n  z\n}\n".to_vec(),
        Rc::new(binding),
        Rc::new(options),
    )
    .model()
}

fn lines_of(model: &FblModel) -> Vec<usize> {
    model.elements.iter().map(|e| e.line).collect()
}

#[test]
fn a_reading_shows_its_origins_first_view_or_the_one_its_header_names() {
    let first = views(None, None);
    assert_eq!(
        first.views.iter().map(|v| v.name.as_str()).collect::<Vec<_>>(),
        ["First", "bee"]
    );
    assert_eq!(lines_of(&first), [2]);

    assert_eq!(lines_of(&views(Some("o/b"), None)), [5, 6]);
    assert_eq!(views(Some("o/b"), None).views.len(), 1);
    assert_eq!(lines_of(&views(None, Some("BEE"))), [5, 6]);
    assert_eq!(lines_of(&views(Some("o/b"), Some("first"))), Vec::<usize>::new());
}
