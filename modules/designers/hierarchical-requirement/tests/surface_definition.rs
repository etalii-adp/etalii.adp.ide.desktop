//! The surface definition held to the vendored `hierarchical-requirement.des`, as the standalone host's
//! `client/register.test.ts` at a7fdf03 holds its own: the levels and entries, the labels, the icons of the assessment
//! kinds, and the operations its gestures send with their labels and params.

mod common;

use std::fs;

use adp_designer_hierarchical_requirement::surface;
use serde_json::Value as Json;

fn specification() -> Json {
    let bytes = fs::read(common::definition_folder().join("hierarchical-requirement.des")).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    serde_json::from_str(text.trim_start_matches('\u{feff}')).unwrap()
}

#[test]
fn it_draws_the_levels_and_entries_its_definition_names() {
    let des = specification();
    let defined = surface::definition();
    assert_eq!(des["surface"]["kind"], "hierarchy");
    assert_eq!(defined.level_type, des["surface"]["levels"]["type"].as_str().unwrap());
    assert_eq!(defined.entry_type, des["surface"]["entries"]["type"].as_str().unwrap());
}

#[test]
fn it_declares_the_labels_its_definition_has_with_their_prefixes_and_the_types_that_carry_them() {
    let des = specification();
    let declared = des["surface"]["labels"].as_object().unwrap();
    assert_eq!(declared.len(), 3, "the definition was read at all");
    let defined = surface::definition();
    let mut names: Vec<&str> = defined.labels.iter().map(|l| l.label.as_str()).collect();
    names.sort_unstable();
    let mut expected: Vec<&str> = declared.keys().map(String::as_str).collect();
    expected.sort_unstable();
    assert_eq!(names, expected);
    for label in &defined.labels {
        let wanted = &declared[&label.label];
        let on: Vec<&str> = wanted["on"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_str().unwrap())
            .collect();
        assert_eq!(
            (
                label.prefix.as_str(),
                label.on.iter().map(String::as_str).collect::<Vec<_>>()
            ),
            (wanted["prefix"].as_str().unwrap(), on),
            "{}",
            label.label
        );
    }
}

#[test]
fn it_shows_each_assessment_kind_by_the_icon_its_definition_gives_it_keyed_by_the_stored_form() {
    let des = specification();
    let kinds = des["metamodel"]["enums"]["AssessmentKind"]["values"]
        .as_object()
        .unwrap();
    assert_eq!(kinds.len(), 4);
    let defined = surface::definition();
    let mut icons: Vec<(String, String)> = defined.note_icons.clone();
    icons.sort();
    let mut expected: Vec<(String, String)> = kinds
        .values()
        .map(|k| {
            (
                k["value"].as_str().unwrap().to_owned(),
                k["icon"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    expected.sort();
    assert_eq!(icons, expected);
    assert_eq!(defined.several_notes_icon, "mdi-comment-multiple-outline");
}

#[test]
fn it_sends_only_operations_its_definition_has_labelled_and_with_the_params_it_names() {
    let des = specification();
    let operations = des["operations"].as_object().unwrap();
    let defined = surface::definition();
    let o = &defined.operations;
    let widths = defined.widths.as_ref().unwrap();
    for operation in [
        &o.toggle_collapsed,
        &o.set_title,
        &o.set_description,
        &o.add_top_level.operation,
        &o.add_entry.operation,
        &o.add_level.operation,
        &o.move_level,
        &o.move_entry,
        &o.delete,
        &widths.add,
        &widths.set,
        &widths.reset,
    ] {
        assert!(
            operations.contains_key(operation),
            "{operation} is an operation of the definition"
        );
    }
    for add in [&o.add_top_level, &o.add_entry, &o.add_level] {
        assert_eq!(
            operations[&add.operation]["label"],
            add.label.as_str(),
            "{}",
            add.operation
        );
    }
    for moved in [&o.move_level, &o.move_entry] {
        let mut params: Vec<&str> = operations[moved]["params"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        params.sort_unstable();
        let mut expected = vec![defined.move_params.after.as_str(), defined.move_params.into.as_str()];
        expected.sort_unstable();
        assert_eq!(params, expected, "{moved}");
    }
    // The notes' list: its gestures, their params and the words its definition labels them with.
    let notes = &defined.notes;
    let (n, p, w) = (&notes.operations, &notes.params, &notes.words);
    for (operation, params, label) in [
        (&n.add, vec![&p.kind, &p.text], &w.add),
        (&n.change, vec![&p.kind, &p.text, &p.detail], &w.change),
        (&n.remove, vec![], &w.remove),
    ] {
        let declared = &operations[operation];
        assert_eq!(declared["label"], label.as_str(), "{operation}");
        for param in params {
            assert!(declared["params"].get(param).is_some(), "{operation} takes {param}");
        }
    }
    assert_eq!(
        operations[&n.resolve]["label"]["cel"],
        format!("self.resolved ? '{}' : '{}'", w.reopen, w.resolve).as_str()
    );
    let column = &des["metamodel"]["types"]["Column"]["attributes"]["width"];
    assert_eq!(
        (widths.min, widths.max),
        (
            column["min"].as_u64().unwrap() as u32,
            column["max"].as_u64().unwrap() as u32
        )
    );
}

#[test]
fn it_asks_before_a_delete_as_the_confirm_of_its_delete_says() {
    let des = specification();
    let defined = surface::definition();
    let confirm = &des["operations"][defined.operations.delete.as_str()]["confirm"];
    assert_eq!(confirm["threshold"], defined.deletion.threshold as u64);
    // The CEL's message with `self.id` and `count` filled in.
    let cel = confirm["message"]["cel"].as_str().unwrap();
    let written = cel
        .replace("'Delete ' + self.id + ' with", "'Delete REQ-0004 with")
        .replace("' + string(count) + '", "3")
        .trim_matches('\'')
        .to_owned();
    assert_eq!(defined.deletion.question("REQ-0004", 3), written);
}
