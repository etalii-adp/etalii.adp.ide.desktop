//! Editing a knowledge file (the standalone host's `KnowledgeEditing.Tests.cs` at 8b6ccfc; knowledge-designer
//! Requirements 3, 4 and 6): every gesture of the table, made through a session as the host makes it, over each of
//! the three formats - what the file holds afterwards, that an undo gives its bytes back exactly, and what is refused.
//!
//! Not ported: `AnExternalReReadOlderThanAWrite_DoesNotPutTheOldTableBack`, a race between the standalone host's
//! watcher thread and its write task, which a session without threads of its own cannot have.

mod common;

use std::cell::Cell;
use std::rc::Rc;

use adp_designer_knowledge::edits::{
    DATE_EXPECTED, DATE_TIME_EXPECTED, LAST_VIEW, NAME_NEEDED, NUMBER_EXPECTED, OPTION_FIRST, RELATION_NEEDS_TARGET,
    TIME_EXPECTED, TITLE_STAYS,
};
use adp_designer_knowledge::{KnowledgeTable, store, values};
use adp_desl::table::content::{Condition, FilterItem, ViewColumn, ViewSort};
use adp_desl::table::model::TableGesture;
use common::{EXTENSIONS, EditingTable, copy_plain, describe};

/// The example, left in its plain view: every row, in the file's order.
fn open(extension: &str, folder: &std::path::Path) -> EditingTable {
    EditingTable::new(copy_plain(&format!("cities{extension}"), folder))
}

fn cell(table: &KnowledgeTable, row: &str, property: &str) -> Vec<String> {
    table
        .row(row)
        .expect("the row")
        .cell(property)
        .map(|c| c.values.clone())
        .unwrap_or_default()
}

fn set_cell<const N: usize>(row: &str, column: &str, values: [&str; N]) -> TableGesture {
    TableGesture::new("setCell").row(row).column(column).values(values)
}

fn findings_on_disk(table: &EditingTable) -> usize {
    store::read(&table.path).body.expect("it reads").model().findings.len()
}

// ---- cells ----

#[test]
fn a_cell_is_set_written_and_undone_to_the_bytes_it_had() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        let before = table.bytes();

        let settled = table.edit(set_cell("r2", "p1", ["Antwerpen"]));

        // Written, and nothing else in the file moved.
        assert!(settled.written);
        assert_eq!(vec!["Antwerpen"], cell(&table.on_disk(), "r2", "p1"));
        assert_eq!(
            String::from_utf8(before.clone())
                .expect("UTF-8")
                .replace("Antwerp", "Antwerpen"),
            String::from_utf8(table.bytes()).expect("UTF-8")
        );

        // The application's undo, and its redo.
        table.session.undo().expect("undone");
        assert_eq!(before, table.bytes(), "{extension}");
        table.session.redo().expect("redone");
        assert_eq!(vec!["Antwerpen"], cell(&table.on_disk(), "r2", "p1"));
    }
}

#[test]
fn a_cell_of_each_type_is_written_in_its_own_form() {
    for extension in EXTENSIONS {
        // r2 has a name, a country and one tag, and nothing else.
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        table.edit(set_cell("r2", "p3", ["529247"]));
        table.edit(set_cell("r2", "p6", ["true"]));
        table.edit(set_cell("r2", "p7", ["1291-02-21"]));
        table.edit(set_cell("r2", "p8", ["2026-10-09T08:15:00+02:00"]));
        table.edit(set_cell("r2", "p9", ["07:30"]));
        table.edit(set_cell("r2", "p2", ["o1"]));
        table.edit(set_cell("r2", "p4", ["o3", "o4"]));

        let disk = table.on_disk();
        assert_eq!(vec!["529247"], cell(&disk, "r2", "p3"), "{extension}");
        assert_eq!(vec!["true"], cell(&disk, "r2", "p6"));
        assert_eq!(vec!["1291-02-21"], cell(&disk, "r2", "p7"));
        assert_eq!(vec!["2026-10-09T08:15:00+02:00"], cell(&disk, "r2", "p8"));
        assert_eq!(vec!["07:30"], cell(&disk, "r2", "p9"));
        assert_eq!(vec!["o1"], cell(&disk, "r2", "p2"));
        assert_eq!(vec!["o3", "o4"], cell(&disk, "r2", "p4"));
        assert_eq!(0, findings_on_disk(&table));
    }
}

#[test]
fn a_cell_is_cleared_and_one_of_several_values_is_taken_out() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());

        // A number cleared, a tick taken away, and one of two tags removed.
        table.edit(set_cell("r1", "p3", []));
        table.edit(set_cell("r1", "p6", ["false"]));
        table.edit(set_cell("r1", "p4", ["o4"]));

        let disk = table.on_disk();
        assert!(cell(&disk, "r1", "p3").is_empty(), "{extension}");
        assert!(cell(&disk, "r1", "p6").is_empty());
        assert_eq!(vec!["o4"], cell(&disk, "r1", "p4"));
        assert_eq!(vec!["Amsterdam"], cell(&disk, "r1", "p1"));
        assert_eq!(0, findings_on_disk(&table));
    }
}

#[test]
fn a_date_and_time_given_without_an_offset_is_written_with_one() {
    // What the browser's own date and time input gives. The standalone host writes this machine's offset; this
    // port reads no time zone and writes UTC's, so the file still says which instant it is.
    let folder = common::folder();
    let mut table = open(".yaml", folder.path());
    table.edit(set_cell("r2", "p8", ["2026-03-01T12:00"]));
    let written = cell(&table.on_disk(), "r2", "p8");
    assert_eq!(vec!["2026-03-01T12:00:00+00:00"], written);
}

#[test]
fn a_value_that_is_not_of_its_type_is_refused_and_nothing_is_written() {
    for (property, value, refusal) in [
        ("p3", "many", NUMBER_EXPECTED),
        ("p7", "27 October", DATE_EXPECTED),
        ("p8", "yesterday", DATE_TIME_EXPECTED),
        ("p9", "noon", TIME_EXPECTED),
        (
            "p2",
            "nothing-like-an-option",
            "That option is no longer one of this property's.",
        ),
    ] {
        let folder = common::folder();
        let mut table = open(".yaml", folder.path());
        let before = table.bytes();
        let (_, answer) = table.begin(set_cell("r1", property, [value]));
        assert_eq!(refusal, answer);
        assert_eq!(before, table.bytes());
        assert!(!table.session.can_undo());
    }
}

#[test]
fn an_option_made_while_filling_in_a_cell_is_made_and_chosen_in_one_step() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        let before = table.bytes();

        table.edit(
            TableGesture::new("setCell")
                .row("r2")
                .column("p2")
                .setting("newOption", "Luxembourg"),
        );

        // The option exists once, with an id of its own, and the cell names it.
        let disk = table.on_disk();
        let made: Vec<_> = disk
            .column("p2")
            .expect("p2")
            .options
            .iter()
            .filter(|o| o.name == "Luxembourg")
            .collect();
        assert_eq!(1, made.len(), "{extension}");
        assert!(!["o1", "o2", "o3", "o4"].contains(&made[0].id.as_str()));
        assert_eq!(vec![made[0].id.clone()], cell(&disk, "r2", "p2"));

        // One step: one undo takes back both.
        table.session.undo().expect("undone");
        assert_eq!(before, table.bytes());
    }
}

// ---- rows ----

#[test]
fn a_row_is_added_at_the_end_after_another_and_in_a_group_with_its_value() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        table.edit(TableGesture::new("addRow"));
        table.edit(TableGesture::new("addRow").target("r1"));
        table.edit(TableGesture::new("addRow").view("v1").setting("group", "o2"));

        // Three new rows beside the example's eleven, each with an id never used; one after r1, the others at the end.
        let rows = table.on_disk().rows;
        assert_eq!(14, rows.len(), "{extension}");
        assert_eq!("r1", rows[0].id);
        assert_eq!("r2", rows[2].id);
        let mut ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(14, ids.len());

        // The row added in the group of Belgium is a row of Belgium.
        assert_eq!(vec!["o2"], cell(&table.on_disk(), &rows[13].id, "p2"));
        assert!(rows[1].cells.is_empty());
        assert!(rows[12].cells.is_empty());
    }
}

#[test]
fn rows_are_deleted_in_one_step() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        let before = table.bytes();
        let all: Vec<String> = table.on_disk().rows.into_iter().map(|r| r.id).collect();
        table.edit(TableGesture::new("deleteRows").values(all));
        assert!(table.on_disk().rows.is_empty(), "{extension}");
        table.session.undo().expect("undone");
        assert_eq!(before, table.bytes());
    }
}

// ---- properties ----

#[test]
fn a_property_is_added_at_the_end_and_beside_another() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        table.edit(TableGesture::new("addColumn").setting("type", "number"));
        table.edit(
            TableGesture::new("addColumn")
                .view("v1")
                .target("p3")
                .setting("type", "text")
                .setting("side", "left"),
        );
        table.edit(
            TableGesture::new("addColumn")
                .view("v1")
                .target("p3")
                .setting("type", "checkbox")
                .setting("side", "right"),
        );

        // Named for their types, each with an id never used, standing where they were put.
        let disk = table.on_disk();
        assert_eq!(
            vec![
                "Name",
                "Country",
                "Text",
                "Population",
                "Checkbox",
                "Tags",
                "Province",
                "Visited",
                "Founded",
                "Updated",
                "Opens",
                "Number"
            ],
            disk.columns.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            "{extension}"
        );
        let id_of = |name: &str| {
            disk.columns
                .iter()
                .find(|p| p.name == name)
                .expect("the property")
                .clone()
        };
        assert_eq!(
            vec!["text", "number", "checkbox"],
            ["Text", "Number", "Checkbox"]
                .iter()
                .map(|n| id_of(n).value_type)
                .collect::<Vec<_>>()
        );

        // The view that had an order of its own has them beside the column they were put beside.
        let (text, checkbox) = (id_of("Text").id, id_of("Checkbox").id);
        let listed: Vec<String> = disk
            .view("v1")
            .expect("v1")
            .columns
            .iter()
            .map(|c| c.column_id.clone())
            .collect();
        assert_eq!(
            vec![text, "p3".into(), checkbox, "p2".into(), "p6".into(), "p7".into()],
            listed
        );
    }
}

#[test]
fn a_relation_is_not_added_from_the_list_of_types() {
    let folder = common::folder();
    let mut table = open(".yaml", folder.path());
    let (_, answer) = table.begin(TableGesture::new("addColumn").setting("type", "relation"));

    // A property is made under the name it is given, and not under one another property has.
    table.edit(
        TableGesture::new("addColumn")
            .values([" Mayor "])
            .setting("type", "text"),
    );
    let (_, taken) = table.begin(
        TableGesture::new("addColumn")
            .values(["mayor"])
            .setting("type", "number"),
    );
    assert!(
        table
            .on_disk()
            .columns
            .iter()
            .any(|p| p.name == "Mayor" && p.value_type == "text")
    );
    assert_eq!("Another property is already called 'mayor'.", taken);
    assert_eq!(RELATION_NEEDS_TARGET, answer);
}

#[test]
fn a_property_is_renamed_but_not_to_a_name_another_has_nor_to_nothing() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        table.edit(TableGesture::new("renameColumn").column("p3").values(["Inhabitants"]));
        let before = table.bytes();
        let (_, taken) = table.begin(TableGesture::new("renameColumn").column("p3").values(["country"]));
        let (_, empty) = table.begin(TableGesture::new("renameColumn").column("p3").values(["  "]));
        assert_eq!("Inhabitants", table.on_disk().column("p3").expect("p3").name);
        assert_eq!("Another property is already called 'country'.", taken);
        assert_eq!(NAME_NEEDED, empty);
        assert_eq!(before, table.bytes());
    }
}

#[test]
fn a_deleted_property_takes_its_cells_and_its_place_in_every_view_with_it() {
    for extension in EXTENSIONS {
        // p2 has cells in both rows, and v1 filters on it and is grouped by it.
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        let before = table.bytes();

        let settled = table.edit(TableGesture::new("deleteColumn").column("p2"));

        // Nothing in the file names it any more, and nothing is reported.
        assert!(settled.written);
        let body = store::read(&table.path).body.expect("it reads");
        assert!(body.table().column("p2").is_none());
        assert!(body.table().rows.iter().all(|r| r.cell("p2").is_none()));
        let view = body.table().view("v1").expect("v1");
        assert_eq!("", view.group_by);
        assert!(view.filter.items.is_empty());
        assert!(body.model().findings.is_empty());
        assert!(
            !body
                .model()
                .elements
                .iter()
                .any(|e| e.attributes.values().any(|v| values::text(Some(v)) == "p2"))
        );

        // One step.
        table.session.undo().expect("undone");
        assert_eq!(before, table.bytes(), "{extension}");
    }
}

#[test]
fn the_title_property_is_not_deleted_and_not_hidden() {
    let folder = common::folder();
    let mut table = open(".yaml", folder.path());
    let before = table.bytes();
    let (_, deleted) = table.begin(TableGesture::new("deleteColumn").column("p1"));
    let (_, hidden) = table.begin(TableGesture::new("hideColumn").column("p1").view("v1"));
    assert_eq!(TITLE_STAYS, deleted);
    assert_eq!(TITLE_STAYS, hidden);
    assert_eq!(before, table.bytes());
}

#[test]
fn a_duplicated_property_has_its_type_and_its_options_under_ids_of_its_own() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        table.edit(TableGesture::new("duplicateColumn").column("p2").view("v2"));

        // Beside the first, of its type, its options copied by name and colour.
        let disk = table.on_disk();
        let copy = &disk.columns[2];
        assert_eq!("Country 2", copy.name, "{extension}");
        assert_eq!("selection", copy.value_type);
        assert_eq!(
            vec![
                "Netherlands:blue",
                "Belgium:yellow",
                "Germany:default",
                "France:default",
                "Finland:default",
                "Italy:default"
            ],
            copy.options
                .iter()
                .map(|o| format!("{}:{}", o.name, o.colour))
                .collect::<Vec<_>>()
        );
        assert!(
            copy.options
                .iter()
                .all(|o| !["o1", "o2", "o3", "o4"].contains(&o.id.as_str()))
        );
        assert_ne!("p2", copy.id);
    }
}

// ---- views ----

#[test]
fn a_view_is_added_renamed_moved_and_deleted_but_never_the_last() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        table.edit(TableGesture::new("addView"));
        let added = table.on_disk().views[2].clone();
        table.edit(TableGesture::new("renameView").view(added.id.clone()).values(["Large"]));
        table.edit(TableGesture::new("moveView").view(added.id.clone()).index(0));
        assert_eq!("View", added.name);
        assert_eq!(
            vec!["Large", "All cities", "Map"],
            table
                .on_disk()
                .views
                .iter()
                .map(|v| v.name.as_str())
                .collect::<Vec<_>>(),
            "{extension}"
        );

        // Delete down to one.
        table.edit(TableGesture::new("deleteView").view("v1"));
        table.edit(TableGesture::new("deleteView").view("v2"));
        let before = table.bytes();
        let (_, last) = table.begin(TableGesture::new("deleteView").view(added.id.clone()));

        // The view the file was left in is gone with its name, and the last view stays.
        let body = store::read(&table.path).body.expect("it reads");
        assert_eq!(
            vec!["Large"],
            body.table().views.iter().map(|v| v.name.as_str()).collect::<Vec<_>>()
        );
        assert_eq!("", body.table().active_view_id);
        assert!(body.model().findings.is_empty());
        assert_eq!(LAST_VIEW, last);
        assert_eq!(before, table.bytes());
    }
}

#[test]
fn a_duplicated_view_has_everything_the_first_has() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        table.edit(TableGesture::new("duplicateView").view("v1"));

        // Beside the first, under a name and an id of its own, and otherwise the same.
        let views = table.on_disk().views;
        assert_eq!(
            vec!["All cities", "All cities 2", "Map"],
            views.iter().map(|v| v.name.as_str()).collect::<Vec<_>>(),
            "{extension}"
        );
        let (first, copy) = (&views[0], &views[1]);
        assert_ne!(first.id, copy.id);
        assert_eq!(first.columns, copy.columns);
        assert_eq!(first.sorts, copy.sorts);
        assert_eq!(first.group_by, copy.group_by);
        assert_eq!(first.filter.any, copy.filter.any);
        assert_eq!(first.filter.items, copy.filter.items);
    }
}

fn column_of(view: &adp_desl::table::content::ContentView, property: &str) -> ViewColumn {
    view.columns
        .iter()
        .find(|c| c.column_id == property)
        .expect("the column")
        .clone()
}

#[test]
fn a_views_columns_are_hidden_resized_wrapped_and_moved_and_no_other_view_changes() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        let other = table.on_disk().view("v1").expect("v1").clone();
        let calculated = column_of(table.on_disk().view("v2").expect("v2"), "p5");

        // All in the second view.
        table.edit(TableGesture::new("hideColumn").column("p4").view("v2"));
        table.edit(
            TableGesture::new("resizeColumn")
                .column("p2")
                .view("v2")
                .setting("width", "240"),
        );
        table.edit(
            TableGesture::new("setColumnWrap")
                .column("p2")
                .view("v2")
                .setting("wrap", "true"),
        );
        table.edit(TableGesture::new("moveColumn").column("p3").view("v2").index(0));

        // p3 first, and an entry only for a column the view has something to say about.
        let disk = table.on_disk();
        let view = disk.view("v2").expect("v2");
        assert_eq!(
            "p3",
            view.columns.iter().find(|c| c.visible).expect("a column").column_id,
            "{extension}"
        );
        let entry = |id: &str, visible: bool, width: i64, wrap: bool| ViewColumn {
            column_id: id.into(),
            visible,
            width,
            wrap,
            calculate: String::new(),
        };
        assert_eq!(entry("p4", false, 0, false), column_of(view, "p4"));
        assert_eq!(entry("p2", true, 240, true), column_of(view, "p2"));
        assert_eq!(calculated, column_of(view, "p5"));
        assert!(
            !view
                .columns
                .iter()
                .any(|c| ["p6", "p7", "p8", "p9"].contains(&c.column_id.as_str()))
        );

        // The other view, the properties and the rows are as they were.
        assert_eq!(other.columns, disk.view("v1").expect("v1").columns);
        assert_eq!(
            vec!["p1", "p2", "p3", "p4", "p5", "p6", "p7", "p8", "p9"],
            disk.columns.iter().map(|p| p.id.as_str()).collect::<Vec<_>>()
        );

        // Shown again, unwrapped.
        table.edit(TableGesture::new("showColumn").column("p4").view("v2"));
        table.edit(
            TableGesture::new("setColumnWrap")
                .column("p2")
                .view("v2")
                .setting("wrap", "false"),
        );
        let disk = table.on_disk();
        let view = disk.view("v2").expect("v2");
        assert!(column_of(view, "p4").visible);
        assert!(!column_of(view, "p2").wrap);
    }
}

#[test]
fn a_views_sorts_are_added_turned_moved_and_removed() {
    for extension in EXTENSIONS {
        // v1 sorts by population, descending.
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        table.edit(
            TableGesture::new("addSort")
                .column("p1")
                .view("v1")
                .setting("direction", "ascending"),
        );
        table.edit(
            TableGesture::new("setSort")
                .column("p3")
                .view("v1")
                .setting("direction", "ascending"),
        );
        table.edit(TableGesture::new("moveSort").column("p1").view("v1").index(0));
        let sort = |id: &str| ViewSort {
            column_id: id.into(),
            descending: false,
        };
        assert_eq!(
            vec![sort("p1"), sort("p3")],
            table.on_disk().view("v1").expect("v1").sorts,
            "{extension}"
        );

        table.edit(TableGesture::new("removeSort").column("p3").view("v1"));
        table.edit(TableGesture::new("removeSort").column("p1").view("v1"));

        // None left, and the file still reads without a word.
        assert!(table.on_disk().view("v1").expect("v1").sorts.is_empty());
        assert_eq!(0, findings_on_disk(&table));
    }
}

fn condition(column: &str, operator: &str, value: &str) -> FilterItem {
    FilterItem::Condition(Condition::new(column, operator, value))
}

#[test]
fn a_views_filter_is_built_changed_and_taken_apart() {
    for extension in EXTENSIONS {
        // v1 has one condition, Country is Netherlands.
        let folder = common::folder();
        let mut table = open(extension, folder.path());

        // A second condition, a group with a condition of its own, and any instead of all.
        table.edit(
            TableGesture::new("addFilter")
                .column("p3")
                .view("v1")
                .setting("comparison", "greater-than"),
        );
        table.edit(
            TableGesture::new("setFilter")
                .column("p3")
                .view("v1")
                .target("1")
                .values(["500000"])
                .setting("comparison", "at-least"),
        );
        table.edit(TableGesture::new("addFilterGroup").view("v1"));
        table.edit(TableGesture::new("addFilter").column("p1").view("v1").target("2"));
        table.edit(
            TableGesture::new("setFilter")
                .view("v1")
                .target("2/0")
                .values(["Ams"])
                .setting("comparison", "starts-with"),
        );
        table.edit(
            TableGesture::new("setFilterMatch")
                .view("v1")
                .target("2")
                .setting("match", "any"),
        );
        table.edit(TableGesture::new("setFilterMatch").view("v1").setting("match", "any"));

        let filter = table.on_disk().view("v1").expect("v1").filter.clone();
        assert!(filter.any, "{extension}");
        assert_eq!(condition("p2", "is", "o1"), filter.items[0]);
        assert_eq!(condition("p3", "at-least", "500000"), filter.items[1]);
        let FilterItem::Group(group) = &filter.items[2] else {
            panic!("a group")
        };
        assert!(group.any);
        assert_eq!(vec![condition("p1", "starts-with", "Ams")], group.items);

        // A condition for a header's menu names no comparison, and gets its type's first: the last item, after the
        // group, in every format.
        table.edit(TableGesture::new("addFilter").column("p6").view("v1"));
        let filter = table.on_disk().view("v1").expect("v1").filter.clone();
        assert_eq!(condition("p6", "is-checked", ""), filter.items[3], "{extension}");
        assert!(matches!(filter.items[2], FilterItem::Group(_)));

        // Taken apart from the end.
        for at in ["3", "2", "1", "0"] {
            table.edit(TableGesture::new("removeFilter").view("v1").target(at));
        }
        assert!(table.on_disk().view("v1").expect("v1").filter.items.is_empty());
        assert_eq!(0, findings_on_disk(&table));
    }
}

#[test]
fn a_filters_groups_go_two_deep() {
    let folder = common::folder();
    let mut table = open(".yaml", folder.path());
    table.edit(TableGesture::new("addFilterGroup").view("v1"));
    table.edit(TableGesture::new("addFilterGroup").view("v1").target("1"));
    let (_, answer) = table.begin(TableGesture::new("addFilterGroup").view("v1").target("1/0"));
    assert_eq!("A filter's groups go two deep.", answer);
}

#[test]
fn a_views_grouping_is_set_its_groups_folded_and_taken_away() {
    for extension in EXTENSIONS {
        // v1 is grouped by Country.
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        let view = |table: &EditingTable| table.on_disk().view("v1").expect("v1").clone();
        table.edit(
            TableGesture::new("toggleGroup")
                .view("v1")
                .target("o1")
                .setting("collapsed", "true"),
        );
        table.edit(
            TableGesture::new("toggleRow")
                .view("v1")
                .row("r1")
                .setting("collapsed", "true"),
        );
        table.edit(
            TableGesture::new("setHideEmptyGroups")
                .view("v1")
                .setting("hide", "true"),
        );
        assert_eq!(vec!["o1", "r1"], view(&table).collapsed, "{extension}");
        assert!(view(&table).hide_empty_groups);

        // Unfolded, and then grouped by something else - which forgets the old grouping's groups.
        table.edit(
            TableGesture::new("toggleGroup")
                .view("v1")
                .target("o1")
                .setting("collapsed", "false"),
        );
        table.edit(TableGesture::new("groupBy").column("p6").view("v1"));
        assert_eq!("p6", view(&table).group_by);
        assert!(view(&table).collapsed.is_empty());

        let (_, refused) = table.begin(TableGesture::new("groupBy").column("p3").view("v1"));
        table.edit(TableGesture::new("groupBy").column("").view("v1"));
        assert_eq!("A view is grouped by a selection, a checkbox or a relation.", refused);
        assert_eq!("", view(&table).group_by);
        assert_eq!(0, findings_on_disk(&table));
    }
}

// ---- shown at once, written behind ----

#[test]
fn an_edit_is_shown_before_it_is_written_and_marked_until_it_is() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        let before = table.bytes();

        // Writes wait: the gesture is made and nothing is written yet.
        let (edit, answer) = table.begin(set_cell("r2", "p1", ["Antwerpen"]));

        // Accepted and shown, marked as not written - and the file not touched yet.
        assert_eq!("", answer);
        let lines = table.lines();
        let shown = lines
            .iter()
            .find(|l| l.id == "r2")
            .expect("r2")
            .cell("p1")
            .expect("p1")
            .clone();
        assert_eq!(vec!["Antwerpen"], shown.values);
        assert!(shown.pending, "{extension}");
        assert_eq!(before, table.bytes());

        // Written, and no longer marked.
        let settled = table.settled(&edit);
        assert!(settled.written);
        let lines = table.lines();
        let shown = lines
            .iter()
            .find(|l| l.id == "r2")
            .expect("r2")
            .cell("p1")
            .expect("p1")
            .clone();
        assert_eq!(vec!["Antwerpen"], shown.values);
        assert!(!shown.pending);
        assert_eq!(vec!["Antwerpen"], cell(&table.on_disk(), "r2", "p1"));
    }
}

#[test]
fn edits_made_while_one_is_waiting_are_written_in_the_order_they_were_made_each_on_the_one_before() {
    let folder = common::folder();
    let mut table = open(".yaml", folder.path());

    // A row, and a cell of that row - which exists only as an edit not yet written.
    let (first, _) = table.begin(TableGesture::new("addRow"));
    let row = table.lines()[2].id.clone();
    let (second, answer) = table.begin(set_cell(&row, "p1", ["Ghent"]));
    let (third, _) = table.begin(set_cell(&row, "p3", ["265086"]));
    let outcomes = [table.settled(&first), table.settled(&second), table.settled(&third)];

    assert_eq!("", answer);
    assert!(outcomes.iter().all(|o| o.written), "{outcomes:?}");
    assert_eq!(vec!["Ghent"], cell(&table.on_disk(), &row, "p1"));
    assert_eq!(vec!["265086"], cell(&table.on_disk(), &row, "p3"));
}

#[test]
fn a_refused_write_takes_the_edit_back_and_every_later_edit_with_it() {
    // The first write is refused, as a file that cannot be written refuses it.
    let folder = common::folder();
    let path = copy_plain("cities.yaml", folder.path());
    let before = std::fs::read(&path).expect("read");
    let writes = Rc::new(Cell::new(0));
    let counted = writes.clone();
    let mut table = EditingTable::with_writer(
        path,
        Box::new(move |command| {
            counted.set(counted.get() + 1);
            if counted.get() == 1 {
                Err("cities.yaml could not be written: the disk is full.".into())
            } else {
                command.execute()
            }
        }),
    );

    let (first, _) = table.begin(set_cell("r2", "p1", ["Antwerpen"]));
    let (second, _) = table.begin(set_cell("r2", "p3", ["529247"]));
    let (third, _) = table.begin(TableGesture::new("addRow"));
    assert_eq!(13, table.lines().len());
    let settled = table.settled(&first);

    // Said once, with its reason and with what went with it; the table shows the file again.
    assert!(!settled.written);
    assert_eq!("cities.yaml could not be written: the disk is full.", settled.error);
    assert_eq!(vec![second, third], settled.taken_back);
    let lines = table.lines();
    assert_eq!(12, lines.len());
    assert_eq!(
        vec!["Antwerp"],
        lines
            .iter()
            .find(|l| l.id == "r2")
            .expect("r2")
            .cell("p1")
            .expect("p1")
            .values
    );

    // And none of the later edits was written after all.
    table.session.write_pending();
    assert_eq!(1, writes.get());
    assert_eq!(before, table.bytes());
}

#[test]
fn closing_a_table_writes_the_edits_still_to_be_written() {
    let folder = common::folder();
    let table = open(".yaml", folder.path());
    let EditingTable { path, mut session, .. } = table;
    adp_desl::table::session::DesignerSession::edit(&mut session, "e", &set_cell("r2", "p1", ["Antwerpen"]));
    assert!(session.has_pending_writes());
    session.close();
    let disk = store::read(&path).body.expect("it reads").table().clone();
    assert_eq!(vec!["Antwerpen"], cell(&disk, "r2", "p1"));
}

#[test]
fn one_property_is_set_in_several_rows_at_once_as_one_step_and_for_none_when_one_refuses_it() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        let before = table.bytes();

        // Both rows get the same country, in one gesture.
        table.edit(
            TableGesture::new("setCells")
                .column("p2")
                .values(["o2"])
                .setting("rows", "r1,r2"),
        );
        assert_eq!(vec!["o2"], cell(&table.on_disk(), "r1", "p2"));
        assert_eq!(vec!["o2"], cell(&table.on_disk(), "r2", "p2"));

        // One undo: both back, in that one step.
        table.session.undo().expect("undone");
        assert_eq!(before, table.bytes(), "{extension}");

        // A value one of the rows cannot hold is given to none, and an option is not made on the way.
        let (_, word) = table.begin(
            TableGesture::new("setCells")
                .column("p3")
                .values(["a great many"])
                .setting("rows", "r1,r2"),
        );
        let (_, made) = table.begin(
            TableGesture::new("setCells")
                .column("p2")
                .setting("rows", "r1,r2")
                .setting("newOption", "Luxembourg"),
        );
        assert_eq!(NUMBER_EXPECTED, word);
        assert_eq!(OPTION_FIRST, made);
        assert_eq!(before, table.bytes());
    }
}

/// What is shown before a write and what the file holds after it are the same table. The two are made by different
/// code - a projection in memory, and the FBL runtime on the bytes - and this is what holds them together.
#[test]
fn what_is_shown_before_a_write_is_what_the_file_holds_after_it() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        let gestures = vec![
            set_cell("r2", "p3", ["529247"]),
            set_cell("r1", "p3", []),
            set_cell("r2", "p4", ["o4"]),
            set_cell("r1", "p4", []),
            set_cell("r1", "p6", ["false"]),
            TableGesture::new("setCell")
                .row("r1")
                .column("p2")
                .setting("newOption", "Luxembourg"),
            TableGesture::new("addRow").target("r1"),
            TableGesture::new("addRow").view("v1").setting("group", "o2"),
            TableGesture::new("addColumn")
                .view("v1")
                .target("p3")
                .setting("type", "date")
                .setting("side", "left"),
            TableGesture::new("renameColumn").column("p3").values(["Inhabitants"]),
            TableGesture::new("duplicateColumn").column("p4").view("v1"),
            TableGesture::new("moveColumn").column("p6").view("v1").index(1),
            TableGesture::new("resizeColumn")
                .column("p1")
                .view("v2")
                .setting("width", "300"),
            TableGesture::new("hideColumn").column("p5").view("v1"),
            TableGesture::new("addSort").column("p1").view("v1"),
            TableGesture::new("moveSort").column("p1").view("v1").index(0),
            TableGesture::new("addFilterGroup").view("v1"),
            TableGesture::new("addFilter").column("p1").view("v1").target("1"),
            TableGesture::new("setFilter")
                .view("v1")
                .target("1/0")
                .values(["A"])
                .setting("comparison", "contains"),
            TableGesture::new("addFilter").column("p3").view("v1"),
            TableGesture::new("addFilterGroup").view("v1").target("1"),
            TableGesture::new("addFilter").column("p6").view("v1").target("1"),
            TableGesture::new("removeFilter").view("v1").target("0"),
            TableGesture::new("setFilterMatch")
                .view("v1")
                .target("0")
                .setting("match", "any"),
            TableGesture::new("toggleGroup")
                .view("v1")
                .target("o1")
                .setting("collapsed", "true"),
            TableGesture::new("duplicateView").view("v1"),
            TableGesture::new("moveView").view("v2").index(0),
            TableGesture::new("deleteColumn").column("p2"),
            TableGesture::new("deleteRows").values(["r2"]),
            TableGesture::new("deleteView").view("v1"),
        ];
        for gesture in gestures {
            // Shown, held; then written.
            let kind = gesture.kind.clone();
            let (edit, answer) = table.begin(gesture);
            assert_eq!("", answer, "{kind}");
            let shown = table.session.shown().clone();
            let settled = table.settled(&edit);
            assert!(settled.written, "{kind}: {}", settled.error);
            let (held, seen) = (describe(&table.on_disk()), describe(&shown));
            assert!(
                held == seen,
                "After {kind} in {extension} the file and what was shown differ.\nFile:\n{held}\nShown:\n{seen}"
            );
        }
    }
}
