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

/// The names of the params `operation` of the definition takes, sorted.
fn params_of(des: &Json, operation: &str) -> Vec<String> {
    let mut params: Vec<String> = des["operations"][operation]["params"]
        .as_object()
        .map(|p| p.keys().cloned().collect())
        .unwrap_or_default();
    params.sort_unstable();
    params
}

fn sorted(names: &[&String]) -> Vec<String> {
    let mut names: Vec<String> = names.iter().map(|n| (*n).clone()).collect();
    names.sort_unstable();
    names
}

#[test]
fn it_edits_the_tags_with_the_tag_operations_its_definition_has_and_asks_before_a_delete_as_its_confirm_says() {
    let des = specification();
    let defined = surface::definition();
    let edited: Vec<&str> = defined
        .labels
        .iter()
        .filter(|l| l.edit.is_some())
        .map(|l| l.label.as_str())
        .collect();
    assert_eq!(edited, ["tags"], "only the tags are edited here");
    let edit = defined.labels[0].edit.as_ref().unwrap();
    let (o, p) = (&edit.operations, &edit.params);
    for (operation, params) in [
        (&o.add, vec![&p.name]),
        (&o.remove, vec![&p.name]),
        (&o.rename, vec![&p.name]),
        (&o.recolour, vec![&p.colour]),
        (&o.reorder, vec![&p.after]),
        (&o.delete, vec![]),
    ] {
        assert!(
            des["operations"].get(operation).is_some(),
            "{operation} is an operation of the definition"
        );
        assert_eq!(params_of(&des, operation), sorted(&params), "{operation}");
    }
    assert_eq!(des["operations"][&o.add]["label"], edit.words.add.as_str());
    assert_eq!(des["operations"][&o.remove]["label"], edit.words.remove.as_str());
    let confirm = &des["operations"][&o.delete]["confirm"];
    assert_eq!(confirm["threshold"], edit.deletion.threshold as u64);
    let written = confirm["message"]["cel"]
        .as_str()
        .unwrap()
        .replace(
            "string(count) + ' entries carry #' + self.name + '",
            "'3 entries carry #ux",
        )
        .trim_matches('\'')
        .to_owned();
    assert_eq!(edit.deletion.question("ux", 3), written);
}

#[test]
fn it_picks_the_category_and_the_state_with_the_operations_its_definition_has_and_asks_before_a_delete_as_its_confirm_says()
 {
    let des = specification();
    let defined = surface::definition();
    let picked: Vec<_> = defined.labels.iter().filter(|l| l.pick.is_some()).collect();
    assert_eq!(
        picked.iter().map(|l| l.label.as_str()).collect::<Vec<_>>(),
        ["category", "state"]
    );
    for label in picked {
        let pick = label.pick.as_ref().unwrap();
        let (o, p, w) = (&pick.operations, &pick.params, &pick.words);
        assert_eq!(p.value, label.label);
        for (operation, params) in [
            (&o.set, vec![&p.value]),
            (&o.clear, vec![]),
            (&o.add, vec![&p.list.name, &p.list.colour]),
            (&o.rename, vec![&p.list.name]),
            (&o.recolour, vec![&p.list.colour]),
            (&o.reorder, vec![&p.list.after]),
            (&o.delete, vec![]),
        ] {
            assert!(
                des["operations"].get(operation).is_some(),
                "{operation} is an operation of the definition"
            );
            assert_eq!(params_of(&des, operation), sorted(&params), "{operation}");
        }
        assert_eq!(des["operations"][&o.set]["label"], w.value.as_str());
        assert_eq!(des["operations"][&o.clear]["label"], w.clear.as_str());
        assert_eq!(label.value_word(), w.value);
        let confirm = &des["operations"][&o.delete]["confirm"];
        assert_eq!(confirm["threshold"], pick.deletion.threshold as u64);
        let written = confirm["message"]["cel"]
            .as_str()
            .unwrap()
            .replace(
                "string(count) + ' requirements are ' + self.name + '",
                "'3 requirements are draft",
            )
            .trim_matches('\'')
            .to_owned();
        assert_eq!(pick.deletion.question("draft", 3), written, "{}", label.label);
    }
}

#[test]
fn it_stores_renames_and_removes_filters_with_the_operations_its_definition_has_and_sorts_by_the_keys_of_its_notation()
{
    let des = specification();
    let defined = surface::definition();
    let filters = defined.filters.as_ref().unwrap();
    let (o, p) = (&filters.operations, &filters.params);
    assert_eq!(des["surface"]["filters"]["type"], "Filter");
    assert_eq!(params_of(&des, &o.store), sorted(&[&p.name, &p.query]));
    assert_eq!(params_of(&des, &o.rename), sorted(&[&p.name]));
    assert!(des["operations"].get(&o.remove).is_some());
    assert_eq!(params_of(&des, &o.remove), Vec::<String>::new());
    let keys: Vec<&str> = filters.sort_keys.iter().map(|k| k.key.as_str()).collect();
    assert_eq!(keys, ["number", "title"]);
}
