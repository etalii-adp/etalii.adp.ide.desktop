//! Changing a property's type (the standalone host's `KnowledgeTypeChange.Tests.cs` at 8b6ccfc; knowledge-designer
//! Requirements 3.3 and 3.4): every value that has a meaning in the new type is converted in the same step, and every
//! other value stays in the file under the key it had. No value is discarded. The cases are the conversion table of
//! the vendored `knowledge.md`, read from it.

mod common;

use adp_designer_knowledge::edits::{RELATION_NEEDS_TARGET, TITLE_IS_TEXT};
use adp_designer_knowledge::vocabulary;
use adp_desl::table::content::ContentCell;
use adp_desl::table::model::TableGesture;
use common::{EXTENSIONS, EditingTable, copy_example};

/// A cell as text: the key it is held under and what it holds.
fn show(cell: &ContentCell) -> String {
    format!("{}: {}", cell.key, cell.values.join(" | "))
}

/// The table's headings, and the value types each stands for.
fn heading(name: &str) -> Vec<&'static str> {
    match name {
        "Text" => vec!["text"],
        "Number" => vec!["number"],
        "Checkbox" => vec!["checkbox"],
        "Date, date and time, time" => vec!["date", "dateTime", "time"],
        "Selection" => vec!["selection"],
        "Multiple selection" => vec!["multipleSelection"],
        "Relation" => vec!["relation"],
        other => panic!("No heading '{other}'."),
    }
}

/// The conversion table: for each type and each other type, whether the value is kept.
fn conversion_table() -> Vec<(&'static str, &'static str, bool)> {
    let text = std::fs::read_to_string(common::vendor().join("definitions/designers/knowledge.md")).expect("vendored");
    let lines: Vec<&str> = text.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.starts_with("| From"))
        .expect("knowledge.md has a conversion table");
    let cells = |line: &str| -> Vec<String> {
        line.trim()
            .trim_matches('|')
            .split('|')
            .map(|c| c.trim().to_owned())
            .collect()
    };
    let columns = cells(lines[start])[1..].to_vec();
    let mut table = Vec::new();
    for line in lines.iter().skip(start + 2).take_while(|l| l.starts_with('|')) {
        let row = cells(line);
        for (column, to_heading) in columns.iter().enumerate() {
            let cell = &row[column + 1];
            if cell.is_empty() {
                continue;
            }
            for from in heading(&row[0]) {
                for to in heading(to_heading).into_iter().filter(|t| *t != from) {
                    table.push((from, to, cell.starts_with("kept")));
                }
            }
        }
    }
    table
}

#[test]
fn the_conversion_table_is_read_and_covers_every_pair_of_types() {
    let table = conversion_table();
    // Nine types, each to each of the eight others, each said once.
    assert_eq!(72, table.len());
    let mut pairs: Vec<(&str, &str)> = table.iter().map(|(f, t, _)| (*f, *t)).collect();
    pairs.sort_unstable();
    pairs.dedup();
    assert_eq!(72, pairs.len());
    let mut from: Vec<&str> = table.iter().map(|(f, _, _)| *f).collect();
    from.sort_unstable();
    from.dedup();
    let mut types = vocabulary::VALUE_TYPES.to_vec();
    types.sort_unstable();
    assert_eq!(types, from);
    // And it says both things.
    assert!(table.iter().any(|(_, _, kept)| *kept));
    assert!(table.iter().any(|(_, _, kept)| !*kept));
}

/// The example's property of each type, and the row to look at.
fn sample(from: &str, to: &str) -> (&'static str, &'static str) {
    match from {
        "number" => ("p3", "r1"),
        "checkbox" => ("p6", "r1"),
        "date" => ("p7", "r1"),
        "dateTime" => ("p8", "r1"),
        "time" => ("p9", "r1"),
        "selection" => ("p2", "r1"),
        // One tag in the second row, two in the first: one is what converts to a selection.
        "multipleSelection" => ("p4", if to == "selection" { "r2" } else { "r1" }),
        _ => ("", "r1"),
    }
}

/// A text that has a meaning in the type it is to become.
fn text_for(to: &str) -> &'static str {
    match to {
        "number" => "42",
        "checkbox" => "true",
        "date" => "2026-01-02",
        "dateTime" => "2026-01-02T03:04:05+00:00",
        "time" => "03:04",
        _ => "Port",
    }
}

fn cell(table: &EditingTable, row: &str, property: &str) -> ContentCell {
    table
        .on_disk()
        .row(row)
        .expect("the row")
        .cell(property)
        .expect("the cell")
        .clone()
}

#[test]
fn a_value_is_converted_or_kept_as_the_table_says() {
    // Every pair of the table but a relation's: a relation is made by choosing its target.
    for (from, to, kept) in conversion_table()
        .into_iter()
        .filter(|(f, t, _)| *f != "relation" && *t != "relation")
    {
        let folder = common::folder();
        let mut table = EditingTable::new(copy_example("cities.yaml", folder.path()));
        let (mut property, row) = sample(from, to);
        let made;
        if from == "text" {
            table.edit(TableGesture::new("addColumn").setting("type", "text"));
            made = table.on_disk().columns.last().expect("a property").id.clone();
            property = &made;
            table.edit(
                TableGesture::new("setCell")
                    .row(row)
                    .column(property)
                    .values([text_for(to)]),
            );
        }
        let before = cell(&table, row, property);
        assert_eq!(vocabulary::key_of(from), before.key, "{from} to {to}");

        table.edit(TableGesture::new("setColumnType").column(property).setting("type", to));

        // The property has the type, and the value is under the new type's key or still under its own.
        let disk = table.on_disk();
        assert_eq!(to, disk.column(property).expect("the property").value_type);
        let after = cell(&table, row, property);
        assert!(!after.values.is_empty(), "{from} to {to}");
        if kept {
            assert_eq!(
                (&before.key, &before.values),
                (&after.key, &after.values),
                "{from} to {to}"
            );
        } else {
            assert_eq!(vocabulary::key_of(to), after.key, "{from} to {to}");
        }

        // One step, whatever it converted.
        table.session.undo().expect("undone");
        assert_eq!(from, table.on_disk().column(property).expect("the property").value_type);
        assert_eq!(show(&before), show(&cell(&table, row, property)), "{from} to {to}");
    }
}

#[test]
fn a_value_without_a_meaning_in_the_new_type_is_still_in_the_file_and_there_again_when_the_type_changes_back() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = EditingTable::new(copy_example(&format!("cities{extension}"), folder.path()));
        table.edit(TableGesture::new("addColumn").setting("type", "text"));
        let id = table.on_disk().columns.last().expect("a property").id.clone();
        table.edit(TableGesture::new("setCell").row("r1").column(id.clone()).values(["42"]));
        table.edit(
            TableGesture::new("setCell")
                .row("r2")
                .column(id.clone())
                .values(["a great many"]),
        );

        table.edit(
            TableGesture::new("setColumnType")
                .column(id.clone())
                .setting("type", "number"),
        );

        // The number is a number, and the word is the word it was, as text.
        assert_eq!("number: 42", show(&cell(&table, "r1", &id)), "{extension}");
        assert_eq!("text: a great many", show(&cell(&table, "r2", &id)));
        assert!(
            std::fs::read_to_string(&table.path)
                .expect("read")
                .contains("a great many")
        );

        // Back to text: both are text again, and neither was lost on the way.
        table.edit(
            TableGesture::new("setColumnType")
                .column(id.clone())
                .setting("type", "text"),
        );
        assert_eq!("text: 42", show(&cell(&table, "r1", &id)));
        assert_eq!("text: a great many", show(&cell(&table, "r2", &id)));
    }
}

#[test]
fn text_becoming_a_selection_gets_the_option_of_that_name_made_once_when_there_is_none() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = EditingTable::new(copy_example(&format!("cities{extension}"), folder.path()));
        table.edit(TableGesture::new("addColumn").setting("type", "text"));
        let id = table.on_disk().columns.last().expect("a property").id.clone();
        table.edit(
            TableGesture::new("setCell")
                .row("r1")
                .column(id.clone())
                .values(["Large"]),
        );
        table.edit(
            TableGesture::new("setCell")
                .row("r2")
                .column(id.clone())
                .values(["large"]),
        );

        table.edit(
            TableGesture::new("setColumnType")
                .column(id.clone())
                .setting("type", "selection"),
        );

        // One option, named as it was first written, and both rows have it.
        let disk = table.on_disk();
        let options = &disk.column(&id).expect("the property").options;
        assert_eq!(1, options.len(), "{extension}");
        assert_eq!("Large", options[0].name);
        for row in ["r1", "r2"] {
            assert_eq!(vec![options[0].id.clone()], cell(&table, row, &id).values);
        }
    }
}

#[test]
fn several_options_becoming_text_are_their_names_and_a_cell_set_afterwards_holds_one_value() {
    for extension in EXTENSIONS {
        // r1 has the tags Port and Capital.
        let folder = common::folder();
        let mut table = EditingTable::new(copy_example(&format!("cities{extension}"), folder.path()));
        table.edit(TableGesture::new("setColumnType").column("p4").setting("type", "text"));
        assert_eq!("text: Port, Capital", show(&cell(&table, "r1", "p4")), "{extension}");

        // A number becomes a checkbox, which keeps it; ticking the box then replaces it.
        table.edit(
            TableGesture::new("setColumnType")
                .column("p3")
                .setting("type", "checkbox"),
        );
        assert_eq!("number: 931298", show(&cell(&table, "r1", "p3")));
        table.edit(TableGesture::new("setCell").row("r1").column("p3").values(["true"]));

        // One value, the new one.
        assert_eq!("checked: true", show(&cell(&table, "r1", "p3")));
        assert!(!std::fs::read_to_string(&table.path).expect("read").contains("931298"));
    }
}

#[test]
fn the_title_propertys_type_does_not_change_and_nothing_becomes_a_relation_from_the_menu() {
    let folder = common::folder();
    let mut table = EditingTable::new(copy_example("cities.yaml", folder.path()));
    let before = table.bytes();
    let (_, title) = table.begin(
        TableGesture::new("setColumnType")
            .column("p1")
            .setting("type", "number"),
    );
    let (_, relation) = table.begin(
        TableGesture::new("setColumnType")
            .column("p3")
            .setting("type", "relation"),
    );
    assert_eq!(TITLE_IS_TEXT, title);
    assert_eq!(RELATION_NEEDS_TARGET, relation);
    assert_eq!(before, table.bytes());
}
