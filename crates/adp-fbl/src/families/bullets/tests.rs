//! The `bullets` family (FBL 0.7 sections 4.8 and 6.3) on the cases the conformance fixtures do not hold: the kinds of
//! line that are not items, columns counted with tabs, sections, and the line ending and indentation an edit writes
//! (the standalone host's `Bullets/BulletsFamily.Tests.cs` at `a7fdf03`).

use std::rc::Rc;

use adp_cel::{Map, Value};

use crate::documents::{self, ProblemSeverity};
use crate::history::OpenBody;
use crate::model::FblOptions;
use crate::planning::{ModelChange, PlanResult};

const TOPICS: &str = r#"{ "fbl": "0.7", "bindings": { "topics": {
  "claims": { "registrationOnly": true }, "body": { "kind": "file", "family": "bullets" }, "reader": "declared",
  "sections": [ { "name": "topics", "heading": "^Topics$" } ],
  "elements": [ {
    "name": "topic", "type": "Topic", "within": [ "topics" ], "line": "^\\[(?<id>[a-z0-9]+)\\] (?<title>.+)$",
    "id": { "from": { "group": "id" } }, "parent": { "rules": [ "topic" ], "slot": "children" },
    "attributes": { "title": { "group": "title" }, "notes": { "continuation": true } },
    "insert": { "place": "after-last", "container": "topics", "emit": "\\[{id}\\] {title}" }, "remove": {}
  } ]
} } }"#;

fn open(text: &str) -> OpenBody {
    let (problems, document) = documents::load(TOPICS.as_bytes());
    assert!(
        !problems.iter().any(|p| p.severity == ProblemSeverity::Error),
        "{problems:?}"
    );
    let binding = Rc::new(document.unwrap().binding("topics").unwrap().clone());
    OpenBody::open(
        text.as_bytes().to_vec(),
        binding,
        Rc::new(FblOptions {
            file_name: "plan.md".into(),
            ..FblOptions::default()
        }),
    )
}

fn text(body: &OpenBody) -> String {
    String::from_utf8(body.bytes().to_vec()).unwrap()
}

fn ids(body: &OpenBody) -> Vec<String> {
    body.model().elements.iter().map(|e| e.id.clone()).collect()
}

fn parent_of(body: &OpenBody, id: &str) -> Option<String> {
    body.model().find(id).unwrap().parent_id.clone()
}

fn move_topic(id: &str, parent: &str, index: i64) -> ModelChange {
    ModelChange::Move {
        id: id.into(),
        new_parent_id: Some(parent.into()),
        index,
    }
}

#[test]
fn items_nest_by_their_markers_column_a_tab_counting_four_columns_with_every_marker() {
    // b's tab puts its marker at column 4, so c's at column 2 closes b and stands in a.
    let body = open("## Topics\n\n- [a] A\n\t* [b] B\n  + [c] C\n");

    assert_eq!(ids(&body), ["a", "b", "c"]);
    assert_eq!(parent_of(&body, "a"), None);
    assert_eq!(parent_of(&body, "b").as_deref(), Some("a"));
    assert_eq!(parent_of(&body, "c").as_deref(), Some("a"));
}

#[test]
fn an_item_line_inside_a_fence_is_not_an_item() {
    let body = open("## Topics\n\n```\n- [x] X\n```\n- [a] A\n");

    assert_eq!(ids(&body), ["a"]);
}

#[test]
fn a_line_underlined_with_equals_is_no_heading_so_the_section_is_missing() {
    let body = open("Topics\n======\n\n- [a] A\n");

    assert!(ids(&body).is_empty());
}

#[test]
fn a_section_runs_to_the_next_heading_of_its_level_or_higher_past_deeper_ones() {
    let body = open("## Topics\n- [a] A\n### Detail\n- [b] B\n## Other\n- [c] C\n");

    assert_eq!(ids(&body), ["a", "b"]);
}

#[test]
fn an_item_nested_in_an_item_no_rule_reads_is_not_looked_into() {
    let body = open("## Topics\n- not a topic\n  - [b] B\n- [c] C\n");

    assert_eq!(ids(&body), ["c"]);
}

#[test]
fn the_continuation_loses_the_columns_before_the_content_column_a_tab_it_cuts_into_leaving_its_rest() {
    // The content column is 2; the tab spans columns 0 to 4, so two of its columns remain as spaces.
    let body = open("## Topics\n- [a] A\n\t  indented\n  plain\n");

    let model = body.model();
    assert_eq!(
        model.find("a").unwrap().attributes.get("notes").and_then(Value::as_str),
        Some("    indented\nplain")
    );
}

#[test]
fn a_new_item_takes_the_line_ending_of_the_line_it_lands_on() {
    let mut body = open("## Topics\r\n\r\n- [a] A\r\n");
    let mut attributes = Map::default();
    attributes.insert("title", Value::String("B".into()));

    let result = body.change(&ModelChange::Add {
        type_name: "Topic".into(),
        id: Some("b".into()),
        attributes,
        parent_id: None,
        index: None,
        slot: None,
    });

    assert!(matches!(result, PlanResult::Planned(_)), "{result:?}");
    assert_eq!(text(&body), "## Topics\r\n\r\n- [a] A\r\n- [b] B\r\n");
}

#[test]
fn a_move_into_another_item_shifts_every_line_that_is_not_blank_by_the_difference_in_depth() {
    let mut body = open("## Topics\n- [a] A\n- [b] B\n  note\n\n  more\n");

    // b becomes a's first child, its marker at a's content column 2.
    let result = body.change(&move_topic("b", "a", 0));

    // The blank line stays blank.
    assert!(matches!(result, PlanResult::Planned(_)), "{result:?}");
    assert_eq!(text(&body), "## Topics\n- [a] A\n  - [b] B\n    note\n\n    more\n");
    assert_eq!(parent_of(&body, "b").as_deref(), Some("a"));
}

#[test]
fn a_move_into_its_own_descendant_is_refused_and_writes_nothing() {
    let input = "## Topics\n- [a] A\n  - [b] B\n";
    let mut body = open(input);

    let result = body.change(&move_topic("a", "b", 0));

    assert!(matches!(result, PlanResult::Refused(_)), "{result:?}");
    assert_eq!(text(&body), input);
}
