//! What an author does to single groups of a grouped view (the standalone host's `KnowledgeGroups.Tests.cs` at
//! 8b6ccfc; knowledge-designer Requirement 6.6): hide one, show the hidden ones again, and move one up or down - each
//! stored in the view, in every format, and each what the table shows from then on.

mod common;

use std::path::Path;

use adp_designer_knowledge::edits::SORTED_ROWS_STAY;
use adp_desl::table::engine::NO_VALUE;
use adp_desl::table::model::TableGesture;
use common::{EXTENSIONS, EditingTable, copy_grouped, copy_plain};

/// The example: its first view is grouped by the country its cities are in.
fn grouped(extension: &str, folder: &Path) -> EditingTable {
    let table = EditingTable::new(copy_grouped(&format!("cities{extension}"), folder));
    assert_eq!("p2", first_view(&table).group_by);
    table
}

fn first_view(table: &EditingTable) -> adp_desl::table::content::ContentView {
    table.on_disk().view("v1").expect("v1").clone()
}

fn headings(table: &mut EditingTable) -> Vec<String> {
    table.lines().into_iter().filter(|l| l.is_group).map(|l| l.id).collect()
}

fn moved(key: &str, direction: &str) -> TableGesture {
    TableGesture::new("moveGroup")
        .view("v1")
        .target(key)
        .setting("direction", direction)
}

/// The groups the view says it has, as the group menu reads them: key, name and whether it is hidden.
fn listed(table: &EditingTable) -> Vec<(String, String, bool)> {
    let settings = table.baseline().settings.settings;
    let groups: serde_json::Value = serde_json::from_str(&settings["groups"]).expect("JSON");
    groups
        .as_array()
        .expect("a list")
        .iter()
        .map(|g| {
            (
                g["key"].as_str().expect("a key").to_owned(),
                g["label"].as_str().expect("a label").to_owned(),
                g["hidden"].as_bool().expect("hidden or not"),
            )
        })
        .collect()
}

#[test]
fn a_group_is_moved_one_place_among_those_shown_and_the_view_names_every_group_from_then_on() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = grouped(extension, folder.path());
        let before = headings(&mut table);
        assert!(before.len() >= 3, "{before:?}");
        assert!(first_view(&table).group_order.is_empty());

        // The second group goes up: it changed places with the first, on the screen and in the file.
        table.edit(moved(&before[1], "up"));
        let mut expected = vec![before[1].clone(), before[0].clone()];
        expected.extend(before.iter().skip(2).cloned());
        assert_eq!(expected, headings(&mut table), "{extension}");
        assert_eq!(expected, first_view(&table).group_order);

        // And down again; then the last one down, which has nowhere to go.
        table.edit(moved(&before[1], "down"));
        let bytes = table.bytes();
        let (_, answer) = table.begin(moved(before.last().expect("a group"), "down"));
        assert_eq!(before, headings(&mut table));
        assert_eq!("", answer);
        assert_eq!(bytes, table.bytes());
    }
}

#[test]
fn a_group_is_hidden_said_to_be_hidden_stepped_over_by_a_move_and_shown_again() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = grouped(extension, folder.path());
        let before = headings(&mut table);
        assert!(!table.baseline().settings.settings.contains_key("hiddenGroups"));

        // The second group is hidden: gone from the screen, in the file, and counted for the control that shows it.
        table.edit(TableGesture::new("hideGroup").view("v1").target(before[1].clone()));
        let mut expected = vec![before[0].clone()];
        expected.extend(before.iter().skip(2).cloned());
        assert_eq!(expected, headings(&mut table));
        assert_eq!(vec![before[1].clone()], first_view(&table).hidden_groups);
        assert_eq!("1", table.baseline().settings.settings["hiddenGroups"]);

        // The third group goes up: over the hidden one, to the place before the first.
        table.edit(moved(&before[2], "up"));
        let mut expected = vec![before[2].clone(), before[0].clone()];
        expected.extend(before.iter().skip(3).cloned());
        assert_eq!(expected, headings(&mut table));

        // Every hidden group is shown again: none is hidden, and the one that was stands where the order has it.
        table.edit(TableGesture::new("showGroups").view("v1"));
        assert!(first_view(&table).hidden_groups.is_empty());
        assert!(!table.baseline().settings.settings.contains_key("hiddenGroups"));
        let mut now = headings(&mut table);
        assert_eq!(before[2], now[0]);
        now.sort();
        let mut all = before.clone();
        all.sort();
        assert_eq!(all, now);
    }
}

#[test]
fn the_view_lists_its_groups_hidden_ones_too_and_one_of_them_is_shown_again_by_itself() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = grouped(extension, folder.path());
        let before = headings(&mut table);
        assert_eq!(before, listed(&table).into_iter().map(|g| g.0).collect::<Vec<_>>());
        assert_eq!(("o1".to_owned(), "Netherlands".to_owned(), false), listed(&table)[0]);
        let last = listed(&table).pop().expect("a group");
        assert_eq!((NO_VALUE, "No Country"), (last.0.as_str(), last.1.as_str()));
        table.edit(TableGesture::new("hideGroup").view("v1").target(before[0].clone()));
        table.edit(TableGesture::new("hideGroup").view("v1").target(before[1].clone()));

        // Both are still listed, as hidden, where they stand.
        assert_eq!(before, listed(&table).into_iter().map(|g| g.0).collect::<Vec<_>>());
        assert_eq!(
            vec![true, true],
            listed(&table).into_iter().take(2).map(|g| g.2).collect::<Vec<_>>()
        );

        // The second one is shown again: it alone.
        table.edit(TableGesture::new("showGroup").view("v1").target(before[1].clone()));
        assert_eq!(vec![before[0].clone()], first_view(&table).hidden_groups);
        assert_eq!(before[1..].to_vec(), headings(&mut table));
    }
}

#[test]
fn groups_are_put_in_a_place_among_all_of_them_and_in_the_order_of_their_names_with_the_rows_without_a_value_last() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = grouped(extension, folder.path());
        let before = headings(&mut table);
        let names: std::collections::HashMap<String, String> = listed(&table).into_iter().map(|g| (g.0, g.1)).collect();

        // The first group is dropped at the third place.
        table.edit(
            TableGesture::new("moveGroup")
                .view("v1")
                .target(before[0].clone())
                .index(2),
        );
        let mut moved = vec![before[1].clone(), before[2].clone(), before[0].clone()];
        moved.extend(before.iter().skip(3).cloned());
        assert_eq!(moved, headings(&mut table));
        assert_eq!(moved, first_view(&table).group_order);

        // By name, and by name backwards.
        table.edit(TableGesture::new("sortGroups").view("v1").setting("order", "ascending"));
        let ascending = headings(&mut table);
        table.edit(
            TableGesture::new("sortGroups")
                .view("v1")
                .setting("order", "descending"),
        );
        let descending = headings(&mut table);

        // The named groups in the order of their names, and the rows without a country last both times.
        let mut named: Vec<String> = before.iter().filter(|k| *k != NO_VALUE).cloned().collect();
        named.sort_by_key(|k| names[k].to_lowercase());
        let mut expected = named.clone();
        expected.push(NO_VALUE.into());
        assert_eq!(expected, ascending);
        let mut backwards: Vec<String> = named.into_iter().rev().collect();
        backwards.push(NO_VALUE.into());
        assert_eq!(backwards, descending);
        assert_eq!(descending, first_view(&table).group_order);
    }
}

#[test]
fn rows_are_moved_together_beside_another_row_in_a_view_without_a_sort_and_not_in_one_with_a_sort() {
    for extension in EXTENSIONS {
        // The example's second view has no sort; two rows are added so that there is something to move.
        let folder = common::folder();
        let mut table = EditingTable::new(copy_plain(&format!("cities{extension}"), folder.path()));
        assert!(table.on_disk().view("v2").expect("v2").sorts.is_empty());
        for _ in 0..2 {
            table.edit(TableGesture::new("addRow").view("v2"));
        }
        let before: Vec<String> = table.on_disk().rows.into_iter().map(|r| r.id).collect();
        assert!(before.len() >= 4);
        let last = before.last().expect("a row").clone();

        // The first and the third row go after the last, together: in the file's order, which is the order they were.
        table.edit(
            TableGesture::new("moveRows")
                .view("v2")
                .values([before[2].clone(), before[0].clone()])
                .target(last.clone())
                .setting("place", "after"),
        );
        let mut expected: Vec<String> = before
            .iter()
            .filter(|id| **id != before[0] && **id != before[2])
            .cloned()
            .collect();
        expected.push(before[0].clone());
        expected.push(before[2].clone());
        assert_eq!(
            expected,
            table.on_disk().rows.into_iter().map(|r| r.id).collect::<Vec<_>>(),
            "{extension}"
        );

        // One of them back before the first.
        table.edit(
            TableGesture::new("moveRows")
                .view("v2")
                .row(before[0].clone())
                .target(expected[0].clone()),
        );
        let mut back = vec![before[0].clone()];
        back.extend(expected.iter().filter(|id| **id != before[0]).cloned());
        assert_eq!(back, table.on_disk().rows.into_iter().map(|r| r.id).collect::<Vec<_>>());

        // In the view that sorts, nothing moves.
        let bytes = table.bytes();
        let (_, answer) = table.begin(
            TableGesture::new("moveRows")
                .view("v1")
                .row(before[0].clone())
                .target(last),
        );
        assert_eq!(SORTED_ROWS_STAY, answer);
        assert_eq!(bytes, table.bytes());
    }
}
