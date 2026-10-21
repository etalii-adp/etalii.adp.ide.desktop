//! Ten thousand rows through the designer (the standalone host's `LargeTable.Tests.cs` at 8b6ccfc; knowledge-designer
//! Requirement 9.6): opened, looked at through a window, edited in one cell, sorted, filtered and grouped, with what
//! each took written to the test's output. Two things are asserted rather than timed: an edit of one cell changes the
//! file where that cell is and nowhere else, and nothing the session gives carries more lines than the window asked
//! for.

mod common;

use std::time::Instant;

use adp_desl::table::model::{TableChange, TableGesture};
use common::EditingTable;

const ROWS: usize = 10_000;
const WINDOW: usize = 60;

/// A table of `rows` rows by rule: a name, a number, one of three sizes and a tick on every other row.
fn table(rows: usize) -> String {
    let mut text = String::from("ded: \"0.1\"\r\ndesigner: etalii/knowledge\r\nname: Places\r\nproperties:\r\n");
    text += "  - id: p1\r\n    name: Name\r\n    type: text\r\n    title: true\r\n";
    text += "  - id: p2\r\n    name: People\r\n    type: number\r\n";
    text += "  - id: p3\r\n    name: Size\r\n    type: selection\r\n    options:\r\n      - id: o1\r\n        name: Small\r\n      - id: o2\r\n        name: Medium\r\n      - id: o3\r\n        name: Large\r\n";
    text += "  - id: p4\r\n    name: Seen\r\n    type: checkbox\r\n";
    text += "views:\r\n  - id: v1\r\n    name: All\r\nrows:\r\n";
    for index in 0..rows {
        text += &format!("  - id: r{index}\r\n    cells:\r\n");
        text += &format!("      - property: p1\r\n        text: Place {index}\r\n");
        text += &format!(
            "      - property: p2\r\n        number: {}\r\n",
            (index * 7919) % 100_003
        );
        text += &format!("      - property: p3\r\n        option: o{}\r\n", 1 + (index % 3));
        if index % 2 == 0 {
            text += "      - property: p4\r\n        checked: true\r\n";
        }
    }
    text
}

#[test]
fn ten_thousand_rows_are_opened_through_a_window_and_edited_where_the_edit_is() {
    let folder = common::folder();
    let path = folder.path().join("places.yaml");
    std::fs::write(&path, table(ROWS)).expect("written");
    let before = std::fs::read(&path).expect("read");
    let mut watch = Instant::now();
    let mut took = |what: &str| {
        println!("{what}: {} ms", watch.elapsed().as_millis());
        watch = Instant::now();
    };

    // Opened.
    let mut table = EditingTable::new(path.clone());
    let baseline = table.baseline();
    took(&format!("opened {} KiB", before.len() / 1024));

    // Every row counted, with the line a row is added at, and none of them sent yet.
    assert_eq!(ROWS + 1, baseline.row_count);
    assert!(baseline.findings.is_empty(), "{:?}", baseline.findings);

    // A window in the middle.
    table.window(5000, WINDOW);
    took("a window of the plain view");
    let (_, rows, _) = table.last_rows();
    assert_eq!(WINDOW, rows.len());
    assert_eq!("r5000", rows[0].id);

    // One cell, in the middle of the file.
    table.edit(
        TableGesture::new("setCell")
            .row("r5000")
            .column("p1")
            .values(["Somewhere else"]),
    );
    took("one cell edited and written");

    // The file is the file it was but for that cell - everything before it and everything after it, byte for byte.
    let after = std::fs::read(&path).expect("read");
    let same = before.iter().zip(&after).take_while(|(a, b)| a == b).count();
    let tail = before[same..]
        .iter()
        .rev()
        .zip(after[same..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    assert!((1..="Place 5000".len()).contains(&(before.len() - same - tail)));
    assert!((1..="Somewhere else".len()).contains(&(after.len() - same - tail)));
    assert!(String::from_utf8_lossy(&after).contains("text: Somewhere else"));

    // The view sorted, filtered and grouped - each moves or hides thousands of rows.
    table.edit(
        TableGesture::new("addSort")
            .column("p2")
            .view("v1")
            .setting("direction", "descending"),
    );
    took("sorted by number, descending");
    let sorted = table.last_rows();
    table.edit(TableGesture::new("addFilter").column("p4").view("v1"));
    took("filtered to the ticked rows");
    let filtered = table.last_rows();
    table.edit(TableGesture::new("groupBy").column("p3").view("v1"));
    took("grouped by size");
    let grouped = table.last_rows();
    table.window(0, WINDOW);
    took("a window of the grouped view");

    // The window follows the view - sorted by number as a number, then half the rows, then under headings.
    let numbers: Vec<f64> = sorted
        .1
        .iter()
        .map(|r| r.cell("p2").expect("a number").values[0].parse().expect("a number"))
        .collect();
    let mut descending = numbers.clone();
    descending.sort_by(|a, b| b.partial_cmp(a).expect("numbers"));
    assert_eq!(descending, numbers);
    assert_eq!(ROWS + 1, sorted.2);
    assert_eq!(ROWS / 2 + 1, filtered.2);
    assert!(
        filtered
            .1
            .iter()
            .filter(|r| !r.is_new_row)
            .all(|r| r.cell("p4").is_some())
    );
    assert_eq!(ROWS / 2 + 4 + 4, grouped.2);
    let first = table.last_rows();
    assert!(first.1[0].is_group);
    assert_eq!("Small", first.1[0].label);

    // And nothing that was given, at any point, carried more lines than the window asked for.
    let windows: Vec<usize> = table
        .given
        .iter()
        .filter_map(|c| match c {
            TableChange::Rows { rows, .. } => Some(rows.len()),
            _ => None,
        })
        .collect();
    assert!(windows.len() >= 6, "Only {} windows were given.", windows.len());
    assert!(windows.iter().all(|n| (1..=WINDOW).contains(n)), "{windows:?}");
}
