//! A selected row in the property grid (the standalone host's `KnowledgeRowContext.Tests.cs` at 8b6ccfc;
//! knowledge-designer Requirement 4.6): the row is found under its table's file and named by its title, its values
//! are the grid's rows, each edited as its type is, and an edit made there is a step that is undone as it is in the
//! table.
//!
//! The standalone host's test follows a tracked selection through the workspace's document store; the desktop host's
//! shell reads the row again from the table's session, so following it is [`row_context::describe_in`] on the table as
//! it is after each change.

mod common;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use adp_context::{PropertyEditor, PropertyRow};
use adp_designer_knowledge::row_context::{self, RELATION_STAYS};
use adp_designer_knowledge::{edits, store};
use adp_fbl::ModelChange;

/// The example with its registration, in a fresh folder.
fn registered(folder: &Path) -> (PathBuf, PathBuf) {
    let cities = common::copy_example("cities.yaml", folder);
    let registration = folder.join("cities.adp");
    std::fs::write(&registration, "etalii/knowledge\r\nbody: cities.yaml\r\n").expect("written");
    (cities, registration)
}

fn described(cities: &Path, row_id: &str) -> HashMap<String, PropertyRow> {
    row_context::properties_of_file(cities, row_id)
        .into_iter()
        .map(|r| (r.id.clone(), r))
        .collect()
}

#[test]
fn a_row_is_selected_under_its_tables_file_by_its_id_and_named_by_its_title() {
    let folder = common::folder();
    let (cities, registration) = registered(folder.path());

    let (under_registration, row) = row_context::describe(&registration, "r1").expect("the row is there");
    let (under_body, _) = row_context::describe(&cities, "r1").expect("the row is there");
    let missing = row_context::describe(&registration, "r999");

    assert!(adp_designer_knowledge::relations::same_path(
        &under_registration,
        &cities
    ));
    assert!(adp_designer_knowledge::relations::same_path(&under_body, &cities));
    assert_eq!(row.title, "Amsterdam");
    assert_eq!(missing.unwrap_err(), row_context::ROW_GONE);

    // A file that is no knowledge table has no rows to select.
    let notes = folder.path().join("notes.yaml");
    std::fs::write(&notes, "just: notes\r\n").expect("written");
    assert_eq!(
        row_context::describe(&notes, "r1").unwrap_err(),
        row_context::NOT_A_TABLE
    );
}

#[test]
fn a_rows_values_are_the_grids_rows_each_as_its_type_is_edited() {
    let folder = common::folder();
    let (cities, _) = registered(folder.path());

    let rows = described(&cities, "r1");

    let shown = |id: &str| (rows[id].label.as_str(), rows[id].value.as_str(), rows[id].editor);
    assert_eq!(shown("p1"), ("Name", "Amsterdam", PropertyEditor::Line));
    assert_eq!(
        (rows["p2"].value.as_str(), rows["p2"].editor),
        ("Netherlands", PropertyEditor::Choice)
    );
    assert_eq!(rows["p2"].candidates[..3], ["", "Netherlands", "Belgium"]);
    assert_eq!(
        (rows["p3"].value.as_str(), rows["p3"].editor),
        ("931298", PropertyEditor::Line)
    );
    assert_eq!(
        (rows["p4"].value.as_str(), rows["p4"].editor),
        ("Port, Capital", PropertyEditor::Tags)
    );
    assert_eq!(
        (rows["p6"].value.as_str(), rows["p6"].editor),
        ("true", PropertyEditor::Toggle)
    );
    assert_eq!(rows["p7"].value, "1275-10-27");
    for id in ["p1", "p2", "p3", "p4", "p6", "p7"] {
        assert!(!rows[id].is_read_only(), "{id}");
    }

    // A relation is shown and not edited here.
    assert_eq!(rows["p5"].read_only_reason, RELATION_STAYS);
}

#[test]
fn an_edit_in_the_grid_is_a_step_that_is_undone_and_what_is_not_a_value_is_refused() {
    let folder = common::folder();
    let (cities, _) = registered(folder.path());
    let before = std::fs::read(&cities).expect("read");
    let mut steps = Vec::new();
    let set = |property: &str, value: &str| row_context::set(&cities, "r1", property, value);

    // A text, an option by its name, a tag by its name, and a checkbox.
    for (property, value) in [
        ("p1", "Amsterdam (NH)"),
        ("p2", "Belgium"),
        ("p4", "Capital"),
        ("p6", "false"),
    ] {
        steps.push(set(property, value).expect("set").expect("a byte changed"));
    }

    // In the file as ids, and in the grid as names.
    let table = store::read(&cities).body.expect("read").table().clone();
    let row = table.row("r1").expect("the row");
    let cell = |id: &str| row.cell(id).map(|c| c.values.clone());
    assert_eq!(cell("p1"), Some(vec!["Amsterdam (NH)".to_owned()]));
    assert_eq!(cell("p2"), Some(vec!["o2".to_owned()]));
    assert_eq!(cell("p4"), Some(vec!["o4".to_owned()]));
    assert_eq!(cell("p6"), None);
    let rows = described(&cities, "r1");
    assert_eq!(
        (
            rows["p1"].value.as_str(),
            rows["p2"].value.as_str(),
            rows["p4"].value.as_str(),
            rows["p6"].value.as_str()
        ),
        ("Amsterdam (NH)", "Belgium", "Capital", "false")
    );

    // What is no value of its property, and a relation: each refused with its reason, and nothing written.
    let bytes = std::fs::read(&cities).expect("read");
    let set = |property: &str, value: &str| row_context::set(&cities, "r1", property, value);
    assert_eq!(set("p3", "a great many").unwrap_err(), edits::NUMBER_EXPECTED);
    assert_eq!(
        set("p2", "Atlantis").unwrap_err(),
        "'Atlantis' is not one of the options of Country."
    );
    assert_eq!(set("p5", "zh").unwrap_err(), RELATION_STAYS);
    assert_eq!(std::fs::read(&cities).expect("read"), bytes);

    // Four undos: the file as it was.
    for step in steps.iter().rev() {
        step.undo().expect("undone");
    }
    assert_eq!(std::fs::read(&cities).expect("read"), before);
}

#[test]
fn a_selected_row_is_followed_while_its_table_changes_and_let_go_when_it_is_deleted() {
    let folder = common::folder();
    let (cities, _) = registered(folder.path());

    // Renamed through the grid: read again, it has its new title.
    row_context::set(&cities, "r1", "p1", "Mokum").expect("set");
    let table = store::read(&cities).body.expect("read").table().clone();
    assert_eq!(
        row_context::describe_in(&table, "r1").map(|d| d.title).as_deref(),
        Some("Mokum")
    );

    // Deleted: there is nothing to follow.
    let body = store::read(&cities).body.expect("read");
    let without = body
        .change(&[ModelChange::Remove { id: "r1".into() }])
        .expect("removed");
    std::fs::write(&cities, without).expect("written");
    let table = store::read(&cities).body.expect("read").table().clone();
    assert_eq!(row_context::describe_in(&table, "r1"), None);
}
