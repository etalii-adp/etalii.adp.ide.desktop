//! What FBL 0.14 section 6.2 asks of a new member of a mapping that the fixtures do not pin down, on a small binding of
//! the lanes of a board in either family: the places, a container written as its key alone or in flow form, a key that
//! must be quoted, and the levels a missing container is created with.

use std::rc::Rc;

use adp_cel::{Map, Value};
use serde_json::{Value as Json, json};

use crate::documents::loader::{ProblemSeverity, load};
use crate::documents::types::FblBinding;
use crate::history::{OpenBody, UndoResult};
use crate::model::FblOptions;
use crate::planning::{ModelChange, PlanResult};

fn binding(family: &str, insert: Json) -> Rc<FblBinding> {
    let text = json!({ "fbl": "0.14", "bindings": { "lanes": {
        "claims": { "registrationOnly": true },
        "body": { "kind": "file", "family": family },
        "reader": "declared",
        "elements": [
            {
                "name": "lane",
                "type": "Lane",
                "at": "/board/lanes/{lane}",
                "when": "!has(entry.kind)",
                "id": { "from": { "capture": "lane" } },
                "attributes": {
                    "key": { "capture": "lane", "readOnly": "A lane is renamed in the file." },
                    "title": { "key": "title", "empty": "keep" }
                },
                "insert": insert,
                "remove": {}
            }
        ]
    } } });
    let (problems, document) = load(text.to_string().as_bytes());
    assert!(
        !problems.iter().any(|p| p.severity == ProblemSeverity::Error),
        "{problems:?}"
    );
    Rc::new(document.unwrap().bindings.remove(0).1)
}

fn open(family: &str, insert: Json, body: &str) -> OpenBody {
    let options = FblOptions {
        file_name: format!("board.{}", if family == "json" { "json" } else { "yml" }),
        ..FblOptions::default()
    };
    OpenBody::open(body.as_bytes().to_vec(), binding(family, insert), Rc::new(options))
}

fn add(attributes: &[(&str, &str)]) -> ModelChange {
    let attributes: Map = attributes
        .iter()
        .map(|(k, v)| ((*k).to_owned(), Value::from(*v)))
        .collect();
    ModelChange::Add {
        type_name: "Lane".to_owned(),
        id: None,
        attributes,
        parent_id: None,
        index: None,
        slot: None,
    }
}

/// The body after adding `key` titled T, having checked that the add reads back and that its undo gives the bytes back.
fn added(family: &str, insert: Json, body: &str, key: &str) -> String {
    let mut open = open(family, insert, body);
    let change = add(&[("key", key), ("title", "T")]);
    let planned = open.change(&change);
    assert!(matches!(planned, PlanResult::Planned(_)), "{planned:?}");
    let text = String::from_utf8(open.bytes().to_vec()).unwrap();
    assert!(
        open.model().find(key).is_some(),
        "{key} does not read back from:\n{text}"
    );
    assert!(matches!(open.undo(), UndoResult::Done(_)));
    assert_eq!(String::from_utf8_lossy(open.bytes()), body);
    text
}

fn refused(family: &str, insert: Json, body: &str, attributes: &[(&str, &str)]) -> String {
    let mut open = open(family, insert, body);
    let PlanResult::Refused(reason) = open.change(&add(attributes)) else {
        panic!("the add was planned: {}", String::from_utf8_lossy(open.bytes()));
    };
    assert_eq!(String::from_utf8_lossy(open.bytes()), body);
    reason
}

fn place(place: Json) -> Json {
    json!({ "place": place, "container": "/board/lanes", "keys": ["title"] })
}

const YAML: &str = "board:\n  lanes:\n    a:\n      title: A\n    n:\n      kind: note\n    b:\n      title: B\n    m:\n      kind: note\n  other: 1\n";

#[test]
fn a_new_yaml_member_goes_where_its_place_puts_it() {
    let new = "    c:\n      title: T\n";
    let at = |before: &str| YAML.replacen(before, &format!("{new}{before}"), 1);
    assert_eq!(
        added("yaml", place(json!("after-last")), YAML, "c"),
        at("    m:\n"),
        "after the rule's last member, before a member of no rule"
    );
    assert_eq!(added("yaml", place(json!("end")), YAML, "c"), at("  other: 1\n"));
    assert_eq!(added("yaml", place(json!("start")), YAML, "c"), at("    a:\n"));
    assert_eq!(
        added("yaml", place(json!({ "before": "b" })), YAML, "c"),
        at("    b:\n")
    );
    assert_eq!(
        added("yaml", place(json!({ "before": "absent" })), YAML, "c"),
        at("  other: 1\n"),
        "a member the file does not have: at the end"
    );
}

#[test]
fn a_yaml_key_is_written_as_a_new_string_value() {
    let body = "board:\n  lanes:\n    a:\n      title: A\n";
    let quoted = |key: &str, written: &str| {
        assert_eq!(
            added("yaml", place(json!("end")), body, key),
            format!("{body}    {written}:\n      title: T\n")
        );
    };
    quoted("to do", "to do");
    quoted("yes", "\"yes\"");
    quoted("1", "\"1\"");
    quoted("a: b", "\"a: b\"");
}

#[test]
fn a_key_a_mapping_holds_is_refused_however_it_is_written() {
    let body = "board:\n  lanes:\n    \"a\":\n      title: A\n";
    let reason = refused("yaml", place(json!("end")), body, &[("key", "a"), ("title", "T")]);
    assert_eq!(reason, "A Lane with the key 'a' is already there.");
    assert_eq!(
        refused("yaml", place(json!("end")), body, &[("title", "T")]),
        "A Lane needs a key."
    );
    assert_eq!(
        refused("yaml", place(json!("end")), body, &[("key", ""), ("title", "T")]),
        "A Lane needs a key."
    );
}

#[test]
fn a_yaml_container_written_as_its_key_alone_takes_the_first_member_below_it() {
    assert_eq!(
        added(
            "yaml",
            place(json!("end")),
            "board:\n  lanes: # none yet\n  other: 1\n",
            "c"
        ),
        "board:\n  lanes: # none yet\n    c:\n      title: T\n  other: 1\n"
    );
}

#[test]
fn a_yaml_container_in_flow_form_or_of_another_kind_takes_no_member() {
    let refuses = |body: &str| refused("yaml", place(json!("end")), body, &[("key", "c")]);
    assert!(refuses("board:\n  lanes: {}\n").contains("flow mapping"));
    assert!(refuses("board:\n  lanes: {a: {title: A}}\n").contains("flow mapping"));
    assert!(refuses("board:\n  lanes:\n    - a\n").contains("not a mapping"));
    assert!(refuses("x: &a\n  b:\n    title: B\nboard:\n  lanes: *a\n").contains("alias"));
}

#[test]
fn a_missing_yaml_container_is_refused_without_create_and_created_with_it() {
    assert_eq!(
        refused(
            "yaml",
            place(json!("end")),
            "board:\n  other: 1\n",
            &[("key", "c"), ("title", "T")]
        ),
        "The file has no board/lanes to add to."
    );
    let create = json!({ "place": "end", "container": "/board/lanes", "keys": ["title"], "create": { "at": "end" } });
    let create_in_flow = create.clone();
    assert_eq!(
        added("yaml", create, "board:\n  other: 1\n", "c"),
        "board:\n  other: 1\n  lanes:\n    c:\n      title: T\n"
    );
    assert!(refused("yaml", create_in_flow, "board: {other: 1}\n", &[("key", "c")]).contains("no place"));
}

#[test]
fn a_new_yaml_member_keeps_a_body_without_a_final_line_ending() {
    assert_eq!(
        added(
            "yaml",
            place(json!("end")),
            "board:\n  lanes:\n    a:\n      title: A",
            "c"
        ),
        "board:\n  lanes:\n    a:\n      title: A\n    c:\n      title: T"
    );
}

#[test]
fn a_skeleton_stands_below_the_keys_of_a_new_yaml_member_with_its_placeholders() {
    let insert = json!({ "place": "end", "container": "/board/lanes", "keys": ["title"], "skeleton": "cards:\n  - name: {key}\n" });
    assert_eq!(
        added("yaml", insert, "board:\n  lanes:\n    a:\n      title: A\n", "c"),
        "board:\n  lanes:\n    a:\n      title: A\n    c:\n      title: T\n      cards:\n        - name: c\n"
    );
}

const JSON: &str = "{\n  \"board\": {\n    \"lanes\": {\n      \"a\": {\n        \"title\": \"A\"\n      },\n      \"b\": {\n        \"title\": \"B\"\n      }\n    }\n  }\n}\n";

#[test]
fn a_new_json_member_goes_where_its_place_puts_it() {
    let new = "\"c\": {\n        \"title\": \"T\"\n      }";
    assert_eq!(
        added("json", place(json!("start")), JSON, "c"),
        JSON.replacen("\"a\": {", &format!("{new},\n      \"a\": {{"), 1)
    );
    assert_eq!(
        added("json", place(json!({ "before": "b" })), JSON, "c"),
        JSON.replacen("\"b\": {", &format!("{new},\n      \"b\": {{"), 1)
    );
    assert_eq!(
        added("json", place(json!("end")), JSON, "c"),
        JSON.replacen("\"B\"\n      }", &format!("\"B\"\n      }},\n      {new}"), 1)
    );
}

#[test]
fn a_new_json_member_is_written_on_one_line_among_siblings_that_are() {
    assert_eq!(
        added(
            "json",
            place(json!("end")),
            "{ \"board\": { \"lanes\": { \"a\": { \"title\": \"A\" } } } }\n",
            "c"
        ),
        "{ \"board\": { \"lanes\": { \"a\": { \"title\": \"A\" }, \"c\": { \"title\": \"T\" } } } }\n"
    );
}

#[test]
fn an_empty_json_object_takes_the_first_member_one_step_deeper_than_its_key() {
    assert_eq!(
        added(
            "json",
            place(json!("end")),
            "{\n  \"board\": {\n    \"lanes\": {}\n  }\n}\n",
            "c"
        ),
        "{\n  \"board\": {\n    \"lanes\": {\n      \"c\": {\n        \"title\": \"T\"\n      }\n    }\n  }\n}\n"
    );
}

#[test]
fn a_missing_json_container_is_created_with_its_levels_around_the_member() {
    let create = json!({ "place": "end", "container": "/board/lanes", "keys": ["title"], "create": { "at": "end" } });
    assert_eq!(
        added(
            "json",
            create.clone(),
            "{\n  \"board\": {\n    \"other\": 1\n  }\n}\n",
            "c"
        ),
        "{\n  \"board\": {\n    \"other\": 1,\n    \"lanes\": {\n      \"c\": {\n        \"title\": \"T\"\n      }\n    }\n  }\n}\n"
    );
    assert_eq!(
        added("json", create.clone(), "{}\n", "c"),
        "{\n  \"board\": {\n    \"lanes\": {\n      \"c\": {\n        \"title\": \"T\"\n      }\n    }\n  }\n}\n"
    );
    assert_eq!(
        added("json", create, "{ \"other\": 1 }\n", "c"),
        "{ \"other\": 1, \"board\": {\"lanes\": {\"c\": { \"title\": \"T\" }}} }\n"
    );
    assert_eq!(
        refused(
            "json",
            place(json!("end")),
            "{\n  \"other\": 1\n}\n",
            &[("key", "c"), ("title", "T")]
        ),
        "The lanes file has no /board/lanes to add the Lane to."
    );
}

#[test]
fn a_skeleton_has_no_json_form_so_a_member_with_one_is_refused() {
    let insert = json!({ "place": "end", "container": "/board/lanes", "keys": ["title"], "skeleton": "cards: []\n" });
    assert!(refused("json", insert, JSON, &[("key", "c")]).contains("skeleton"));
}
