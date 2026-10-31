//! What FBL 0.10 asks of one binding with several readings that the fixtures do not pin down, on a small binding of
//! groups with members and files: `origins`, `resource.byOrigin`, the items of a flow sequence, `refusals`, `secondary`
//! and a level written on an add.

use std::rc::Rc;

use adp_cel::{Map, Value};
use serde_json::{Value as Json, json};

use crate::documents::loader::{ProblemSeverity, load};
use crate::documents::types::FblBinding;
use crate::history::{OpenBody, UndoResult};
use crate::model::{FblModel, FblOptions, IdRequest};
use crate::planning::{ModelChange, PlanResult};
use crate::routing;

fn document(binding: Json) -> Json {
    json!({ "fbl": "0.10", "bindings": { "groups": binding } })
}

fn binding_of(family: &str) -> Rc<FblBinding> {
    let text = document(json!({
        "claims": { "extensions": [".yml", ".json"], "origins": ["o/groups", "o/one"] },
        "body": { "kind": "file", "family": family },
        "reader": "declared",
        "registration": { "resource": { "byOrigin": { "o/one": { "capture": "group" }, "o/groups": null } } },
        "elements": [
            {
                "name": "summary",
                "type": "Summary",
                "at": "/groups/{group}",
                "origins": ["o/groups"],
                "attributes": { "key": { "capture": "group" } }
            },
            {
                "name": "group",
                "type": "Group",
                "at": "/groups/{group}",
                "origins": ["o/one"],
                "attributes": {
                    "title": { "key": "title", "read": "text", "empty": "refuse", "refusals": { "empty": "A group needs a title." } },
                    "owner": { "key": "owner", "read": "text", "empty": "remove", "secondary": { "key": "lead" } }
                }
            },
            {
                "name": "member",
                "type": "Member",
                "at": "/groups/{group}/members/*",
                "origins": ["o/one"],
                "attributes": {
                    "name": { "key": "name", "read": "text", "reference": { "to": ["member"], "by": "name" } },
                    "role": { "key": "role", "read": "text", "empty": "remove" }
                },
                "insert": {
                    "place": "after-last",
                    "container": "/groups/{group}/members",
                    "keys": ["name", "role"],
                    "refusals": { "container": "The group has no members to add to." }
                },
                "remove": {}
            },
            {
                "name": "pair",
                "type": "Pair",
                "at": "/groups/{group}/pairs/*",
                "origins": ["o/one"],
                "attributes": { "name": { "key": "name", "read": "text" } },
                "insert": { "place": "start", "container": "/groups/{group}/pairs", "keys": ["name"] },
                "remove": { "container": "remove-when-empty" }
            },
            {
                "name": "job",
                "type": "Job",
                "at": "/groups/{group}/jobs/*",
                "origins": ["o/one"],
                "attributes": { "name": { "key": "name", "read": "text" } },
                "insert": { "place": "end", "container": "/groups/{group}/jobs", "keys": ["name"], "skeleton": "task:\n  path: a\n" }
            },
            {
                "name": "notebook",
                "type": "File",
                "at": "/groups/{group}/files/*",
                "origins": ["o/one"],
                "when": "has(entry.notebook)",
                "attributes": {
                    "kind": { "value": "'notebook'" },
                    "path": { "key": "path", "child": "notebook", "read": "text" }
                },
                "insert": {
                    "place": "after-last",
                    "container": "/groups/{group}/files",
                    "when": "attributes.kind == 'notebook'",
                    "refusals": { "when": "Only a notebook or a script can be added." }
                },
                "remove": {}
            },
            {
                "name": "script",
                "type": "File",
                "at": "/groups/{group}/files/*",
                "origins": ["o/one"],
                "when": "has(entry.script)",
                "attributes": {
                    "kind": { "value": "'script'" },
                    "path": { "key": "path", "child": "script", "read": "text" },
                    "line": { "key": "line", "child": "script/where" }
                },
                "insert": {
                    "place": "after-last",
                    "container": "/groups/{group}/files",
                    "when": "attributes.kind == 'script'"
                },
                "remove": {}
            }
        ]
    }));
    let (problems, document) = load(text.to_string().as_bytes());
    assert!(
        !problems.iter().any(|p| p.severity == ProblemSeverity::Error),
        "{problems:?}"
    );
    Rc::new(document.unwrap().bindings.remove(0).1)
}

fn id_of(request: &IdRequest<'_>) -> Option<String> {
    let text = |name: &str| request.attributes.get(name).and_then(Value::as_str).map(str::to_owned);
    Some(match request.rule {
        "group" => "group".into(),
        "summary" => format!("summary:{}", text("key")?),
        _ => format!("{}:{}", request.rule, text("name").or(text("path"))?),
    })
}

fn open_as(family: &str, body: &str, origin: Option<&str>, resource: Option<&str>) -> OpenBody {
    let options = FblOptions {
        file_name: format!("groups.{}", if family == "json" { "json" } else { "yml" }),
        origin: origin.map(str::to_owned),
        resource: resource.map(str::to_owned),
        derive_id: Some(Rc::new(id_of)),
        ..FblOptions::default()
    };
    OpenBody::open(body.as_bytes().to_vec(), binding_of(family), Rc::new(options))
}

fn open(body: &str) -> OpenBody {
    open_as("yaml", body, Some("o/one"), None)
}

fn text_of(body: &OpenBody) -> String {
    String::from_utf8(body.bytes().to_vec()).unwrap()
}

fn values(pairs: &[(&str, &str)]) -> Map {
    pairs.iter().map(|(k, v)| ((*k).to_owned(), Value::from(*v))).collect()
}

fn add(type_name: &str, attributes: &[(&str, &str)]) -> ModelChange {
    ModelChange::Add {
        type_name: type_name.to_owned(),
        id: None,
        attributes: values(attributes),
        parent_id: None,
        index: None,
        slot: None,
    }
}

fn set(id: &str, attributes: &[(&str, &str)]) -> ModelChange {
    ModelChange::Set {
        id: id.to_owned(),
        attributes: values(attributes),
    }
}

fn remove(id: &str) -> ModelChange {
    ModelChange::Remove { id: id.to_owned() }
}

/// Plans and applies `change`, which must be planned, and undoes it to check it is its exact inverse.
fn changed(body: &str, change: &ModelChange) -> String {
    let mut open = open(body);
    match open.change(change) {
        PlanResult::Planned(_) => {}
        other => panic!("{change:?} was not planned: {other:?}"),
    }
    let written = text_of(&open);
    assert!(matches!(open.undo(), UndoResult::Done(_)));
    assert_eq!(text_of(&open), body, "the undo of {change:?}");
    written
}

fn refused(body: &str, change: &ModelChange) -> String {
    let mut open = open(body);
    match open.change(change) {
        PlanResult::Refused(reason) => {
            assert_eq!(text_of(&open), body);
            reason
        }
        other => panic!("{change:?} was not refused: {other:?}"),
    }
}

fn ids(model: &FblModel) -> Vec<&str> {
    model.elements.iter().map(|e| e.id.as_str()).collect()
}

// ---- origins and resources (FBL 0.10 sections 5.1, 8.2) ----

const TWO_GROUPS: &str =
    "groups:\n  a:\n    title: A\n    members:\n      - name: x\n  b:\n    title: B\n    members:\n      - name: y\n";

#[test]
fn a_rule_is_read_only_for_the_readings_its_origins_list() {
    let many = open_as("yaml", TWO_GROUPS, Some("o/groups"), None);
    assert_eq!(ids(&many.model()), ["summary:a", "summary:b"]);
    let one = open_as("yaml", TWO_GROUPS, Some("o/one"), None);
    assert_eq!(ids(&one.model()), ["group", "member:x"]);
}

#[test]
fn a_rule_of_another_reading_plans_no_add() {
    let mut body = open_as("yaml", TWO_GROUPS, Some("o/groups"), None);
    assert!(matches!(
        body.change(&add("Member", &[("name", "z")])),
        PlanResult::Refused(_)
    ));
}

#[test]
fn a_reading_that_selects_no_resource_keeps_its_header_and_selects_nothing() {
    let body = open_as("yaml", TWO_GROUPS, Some("o/groups"), Some("b"));
    assert_eq!(ids(&body.model()), ["summary:a", "summary:b"]);
    assert_eq!(body.reading().missing_resource(), None);
}

#[test]
fn a_reading_selects_its_resource_by_the_capture_of_its_origin() {
    let body = open_as("yaml", TWO_GROUPS, Some("o/one"), Some("b"));
    assert_eq!(ids(&body.model()), ["group", "member:y"]);
    assert_eq!(body.model().resources, ["a", "b"]);
    assert_eq!(body.reading().missing_resource(), None);
}

#[test]
fn a_resource_the_body_does_not_hold_empties_the_reading_and_adds_nowhere_else() {
    let mut body = open_as("yaml", TWO_GROUPS, Some("o/one"), Some("c"));
    assert!(body.model().elements.is_empty());
    assert_eq!(body.reading().missing_resource(), Some("c"));
    assert_eq!(
        body.change(&add("Member", &[("name", "z")])),
        PlanResult::Refused("The group has no members to add to.".into())
    );
}

#[test]
fn an_origin_a_binding_does_not_claim_is_refused_where_a_rule_or_by_origin_names_it() {
    let refused_with = |text: Json| {
        let (problems, _) = load(text.to_string().as_bytes());
        assert!(problems.iter().any(|p| p.message.contains("'o/other'")), "{problems:?}");
    };
    let rule = json!({ "name": "a", "type": "A", "at": "/a", "attributes": { "n": { "key": "n" } } });
    let base = json!({
        "claims": { "extensions": [".yml"], "origins": ["o/one"] },
        "body": { "kind": "file", "family": "yaml" },
        "reader": "declared",
        "elements": [rule]
    });
    let mut listed = base.clone();
    listed["elements"][0]["origins"] = json!(["o/other"]);
    refused_with(document(listed));
    let mut by_origin = base.clone();
    by_origin["registration"] = json!({ "resource": { "byOrigin": { "o/other": null } } });
    refused_with(document(by_origin));
}

// ---- a template's key (FBL 0.10 section 13) ----

#[test]
fn a_template_writes_its_key_in_lower_case() {
    let written = routing::replace("{key:lower} {key} {base} {key:upper}", "dir/My_Job.yml", &|_| {
        String::new()
    });
    assert_eq!(written, "my_job My_Job My_Job {key:upper}");
}

// ---- reading a flow sequence (FBL 0.10 section 4.3) ----

#[test]
fn the_items_of_a_flow_sequence_are_entries_however_they_are_written() {
    let body = open(
        "groups:\n  a:\n    title: A\n    members: [\n      {name: x, role: \"lead, chief\"}, # first\n      {name: 'y''s', role: 'r'},\n      { name: z }\n    ]\n    files: [{notebook: {path: n/one}}, {script: {path: s/two, where: {line: 3}}}]\n",
    );
    let model = body.model();
    assert_eq!(
        ids(&model),
        [
            "group",
            "member:x",
            "member:y's",
            "member:z",
            "notebook:n/one",
            "script:s/two"
        ]
    );
    let x = model.find("member:x").unwrap();
    assert_eq!(x.attributes.get("role"), Some(&Value::from("lead, chief")));
    assert_eq!(
        model.find("script:s/two").unwrap().attributes.get("line"),
        Some(&Value::Int(3))
    );
    assert!(model.findings.is_empty(), "{:?}", model.findings);
}

#[test]
fn a_flow_collection_written_badly_is_read_without_hanging() {
    for flow in ["[: a]", "[, ,]", "[{: a}]", "[{a: }]", "[{a}, [b}]"] {
        let body = open(&format!("groups:\n  a:\n    title: A\n    members: {flow}\n"));
        assert!(!body.model().unreadable, "{flow}");
    }
}

#[test]
fn a_value_inside_a_flow_item_is_read_only_and_a_key_goes_nowhere_inside_it() {
    let body = "groups:\n  a:\n    title: A\n    members: [{name: x, role: lead}]\n";
    assert!(refused(body, &set("member:x", &[("role", "chief")])).contains("flow collection"));
    let unset = "groups:\n  a:\n    title: A\n    members: [{name: x}]\n";
    assert!(refused(unset, &set("member:x", &[("role", "chief")])).contains("flow mapping"));
}

#[test]
fn an_item_of_a_flow_sequence_is_not_moved() {
    let body = "groups:\n  a:\n    title: A\n    members: [{name: x}, {name: y}]\n";
    let mut open = open(body);
    let change = ModelChange::Move {
        id: "member:x".into(),
        new_parent_id: None,
        index: -1,
    };
    assert!(matches!(open.change(&change), PlanResult::Refused(_)));
    assert_eq!(text_of(&open), body);
}

#[test]
fn a_rename_rewrites_a_reference_inside_a_flow_item_as_one_scalar() {
    let binding = document(json!({
        "claims": { "extensions": [".yml"], "origins": ["o/one"] },
        "body": { "kind": "file", "family": "yaml" },
        "reader": "declared",
        "elements": [{
            "name": "task",
            "type": "Task",
            "at": "/tasks/*",
            "attributes": { "key": { "key": "key", "reference": { "to": ["task"], "by": "key" } } }
        }],
        "relations": [{
            "name": "wait",
            "type": "Wait",
            "at": "/tasks/*/after/*",
            "source": { "key": "key" },
            "target": { "parent": "key" }
        }]
    }));
    let (problems, document) = load(binding.to_string().as_bytes());
    assert!(
        !problems.iter().any(|p| p.severity == ProblemSeverity::Error),
        "{problems:?}"
    );
    let options = FblOptions {
        derive_id: Some(Rc::new(|r: &IdRequest<'_>| {
            r.attributes.get("key").and_then(Value::as_str).map(str::to_owned)
        })),
        file_name: "tasks.yml".into(),
        ..FblOptions::default()
    };
    let mut body = OpenBody::open(
        b"tasks:\n  - key: a\n  - key: b\n    after: [{key: a}]\n".to_vec(),
        Rc::new(document.unwrap().bindings.remove(0).1),
        Rc::new(options),
    );
    assert!(matches!(
        body.change(&set("a", &[("key", "c,d")])),
        PlanResult::Planned(_)
    ));
    assert_eq!(
        text_of(&body),
        "tasks:\n  - key: c,d\n  - key: b\n    after: [{key: \"c,d\"}]\n"
    );
}

// ---- inserting into a flow sequence (FBL 0.10 section 6.2) ----

const MEMBERS: &str = "groups:\n  a:\n    title: A\n    members: ";

fn members(flow: &str) -> String {
    format!("{MEMBERS}{flow}\n")
}

#[test]
fn a_one_line_flow_sequence_takes_a_new_item_before_its_close() {
    assert_eq!(
        changed(
            &members("[{name: x}]"),
            &add("Member", &[("role", "lead"), ("name", "w")])
        ),
        members("[{name: x}, {name: w, role: lead}]")
    );
}

#[test]
fn a_value_a_flow_collection_would_read_as_its_own_is_quoted() {
    assert_eq!(
        changed(
            &members("[{name: x}]"),
            &add("Member", &[("name", "a, b"), ("role", "{c}")])
        ),
        members("[{name: x}, {name: \"a, b\", role: \"{c}\"}]")
    );
    assert_eq!(
        changed(&members("[{name: x}]"), &add("Member", &[("name", "yes")])),
        members("[{name: x}, {name: \"yes\"}]")
    );
}

#[test]
fn a_one_line_flow_sequence_with_a_trailing_comma_takes_no_second_one() {
    assert_eq!(
        changed(&members("[{name: x},]"), &add("Member", &[("name", "w")])),
        members("[{name: x}, {name: w}]")
    );
}

#[test]
fn place_start_writes_the_new_item_after_the_open() {
    let body = "groups:\n  a:\n    title: A\n    pairs: [{name: x}]\n";
    assert_eq!(
        changed(body, &add("Pair", &[("name", "w")])),
        "groups:\n  a:\n    title: A\n    pairs: [{name: w}, {name: x}]\n"
    );
}

#[test]
fn a_level_of_a_new_flow_item_is_a_flow_mapping() {
    let body = "groups:\n  a:\n    title: A\n    files: [{notebook: {path: one}}]\n";
    assert_eq!(
        changed(
            body,
            &add("File", &[("kind", "script"), ("path", "two"), ("line", "3")])
        ),
        "groups:\n  a:\n    title: A\n    files: [{notebook: {path: one}}, {script: {path: two, where: {line: \"3\"}}}]\n"
    );
}

#[test]
fn an_empty_flow_sequence_is_opened_whatever_is_between_its_brackets_but_a_comment() {
    let block = format!("{MEMBERS}\n      - name: w\n");
    let block = block.replacen("members: \n", "members:\n", 1);
    assert_eq!(changed(&members("[ ]"), &add("Member", &[("name", "w")])), block);
    assert_eq!(
        changed(&format!("{MEMBERS}[\n    ]\n"), &add("Member", &[("name", "w")])),
        block
    );
    assert!(
        refused(&format!("{MEMBERS}[ # none\n    ]\n"), &add("Member", &[("name", "w")])).contains("several lines")
    );
}

#[test]
fn a_flow_mapping_and_a_new_item_with_a_skeleton_take_no_insert() {
    assert!(refused(&members("{}"), &add("Member", &[("name", "w")])).contains("flow mapping"));
    let jobs = "groups:\n  a:\n    title: A\n    jobs: [{name: x}]\n";
    assert!(refused(jobs, &add("Job", &[("name", "w")])).contains("skeleton"));
}

// ---- removing from a flow sequence (FBL 0.10 section 6.2) ----

#[test]
fn removing_an_item_takes_one_separator() {
    let body = members("[{name: x}, {name: y},  {name: z}]");
    assert_eq!(changed(&body, &remove("member:x")), members("[{name: y},  {name: z}]"));
    assert_eq!(changed(&body, &remove("member:y")), members("[{name: x}, {name: z}]"));
    assert_eq!(changed(&body, &remove("member:z")), members("[{name: x}, {name: y}]"));
}

#[test]
fn the_only_item_is_removed_alone_and_the_container_goes_when_the_rule_says() {
    assert_eq!(changed(&members("[{name: x}]"), &remove("member:x")), members("[]"));
    let pairs = "groups:\n  a:\n    title: A\n    pairs: [{name: x}] # kept with the line\n    other: 1\n";
    assert_eq!(
        changed(pairs, &remove("pair:x")),
        "groups:\n  a:\n    title: A\n    other: 1\n"
    );
}

// ---- refusals (FBL 0.10 section 6.4) ----

#[test]
fn a_binding_gives_the_sentence_of_a_refused_set_and_a_refused_add() {
    let body = "groups:\n  a:\n    title: A\n    files:\n      - notebook:\n          path: one\n";
    assert_eq!(refused(body, &set("group", &[("title", "")])), "A group needs a title.");
    // Neither rule's `when` holds, so the first rule's sentence is the reason.
    assert_eq!(
        refused(body, &add("File", &[("kind", "other"), ("path", "two")])),
        "Only a notebook or a script can be added."
    );
}

// ---- a secondary slot (FBL 0.10 section 5.2) ----

#[test]
fn emptying_an_attribute_removes_its_secondary_so_that_the_value_does_not_come_back() {
    let lead = "groups:\n  a:\n    title: A\n    lead: l\n";
    assert_eq!(
        changed(lead, &set("group", &[("owner", "")])),
        "groups:\n  a:\n    title: A\n"
    );
    let both = "groups:\n  a:\n    title: A\n    owner: o\n    lead: l\n";
    assert_eq!(
        changed(both, &set("group", &[("owner", "")])),
        "groups:\n  a:\n    title: A\n"
    );
}

#[test]
fn a_binding_with_a_secondary_and_an_override_is_refused() {
    let text = document(json!({
        "claims": { "extensions": [".yml"], "origins": ["o/one"] },
        "body": { "kind": "file", "family": "yaml" },
        "reader": "declared",
        "elements": [{
            "name": "a",
            "type": "A",
            "at": "/a",
            "attributes": { "n": { "key": "n", "secondary": { "key": "m" }, "override": { "key": "o" } } }
        }]
    }));
    let (problems, _) = load(text.to_string().as_bytes());
    assert!(
        problems.iter().any(|p| p.message.contains("'secondary'")),
        "{problems:?}"
    );
}

#[test]
fn a_secondary_is_removed_from_a_json_body_too() {
    let body = "{\n  \"groups\": {\n    \"a\": {\n      \"title\": \"A\",\n      \"lead\": \"l\"\n    }\n  }\n}\n";
    let mut open = open_as("json", body, Some("o/one"), None);
    assert_eq!(
        open.model().find("group").unwrap().attributes.get("owner"),
        Some(&Value::from("l"))
    );
    assert!(matches!(
        open.change(&set("group", &[("owner", "o")])),
        PlanResult::Planned(_)
    ));
    let written = text_of(&open);
    assert!(written.contains("\"owner\": \"o\""), "{written}");
    assert!(!written.contains("lead"), "{written}");
}

// ---- a level written on an add (FBL 0.10 section 6.2) ----

const FILES: &str = "groups:\n  a:\n    title: A\n    files:\n      - notebook:\n          path: one\n";

#[test]
fn an_add_is_planned_by_the_rule_whose_when_holds() {
    assert_eq!(
        changed(FILES, &add("File", &[("kind", "script"), ("path", "two")])),
        format!("{FILES}      - script:\n          path: two\n")
    );
}

#[test]
fn attributes_that_reach_one_level_share_it_and_a_deeper_one_is_one_step_deeper() {
    assert_eq!(
        changed(
            FILES,
            &add("File", &[("kind", "script"), ("path", "two"), ("line", "3")])
        ),
        format!("{FILES}      - script:\n          path: two\n          where:\n            line: \"3\"\n")
    );
}

#[test]
fn a_level_of_a_new_json_entry_is_an_object() {
    let body =
        "{ \"groups\": { \"a\": { \"title\": \"A\", \"files\": [{ \"notebook\": { \"path\": \"one\" } }] } } }\n";
    let mut open = open_as("json", body, Some("o/one"), None);
    assert!(matches!(
        open.change(&add("File", &[("kind", "script"), ("path", "two")])),
        PlanResult::Planned(_)
    ));
    assert!(
        text_of(&open).contains("[{ \"notebook\": { \"path\": \"one\" } }, { \"script\": { \"path\": \"two\" } }]"),
        "{}",
        text_of(&open)
    );
}
