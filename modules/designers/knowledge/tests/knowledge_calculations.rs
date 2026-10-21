//! The three things a view does beside filtering, sorting and grouping, as Notion's tables do them (the standalone
//! host's `KnowledgeCalculations.Tests.cs` at 8b6ccfc): the line under a column that calculates over the rows in
//! sight, the columns frozen up to one, and the search - which is the connection's own and is written nowhere.

mod common;

use std::collections::HashSet;

use adp_designer_knowledge::{KnowledgeTable, relations, store, validator};
use adp_desl::table::model::{TableColumn, TableGesture};
use adp_desl::table::values::parse_date;
use common::{EXTENSIONS, EditingTable, RELATED, copy_plain};

/// The example in its second view, which shows every row.
fn open(extension: &str, folder: &std::path::Path) -> EditingTable {
    EditingTable::new(copy_plain(&format!("cities{extension}"), folder))
}

fn calculate(column: &str, calculation: &str) -> TableGesture {
    TableGesture::new("setColumnCalculation")
        .column(column)
        .setting("calculate", calculation)
}

fn column(table: &EditingTable, id: &str) -> TableColumn {
    table
        .baseline()
        .columns
        .into_iter()
        .find(|c| c.id == id)
        .expect("the column")
}

fn shown(table: &EditingTable, id: &str) -> String {
    column(table, id)
        .settings
        .get("calculation")
        .cloned()
        .unwrap_or_else(|| "(none)".into())
}

fn second_view(table: &KnowledgeTable) -> adp_desl::table::content::ContentView {
    table.view("v2").expect("v2").clone()
}

/// The numbers the rows hold for a property, read from the file.
fn numbers(table: &EditingTable, property: &str) -> Vec<f64> {
    let mut numbers: Vec<f64> = table
        .on_disk()
        .rows
        .iter()
        .flat_map(|r| r.cells.iter())
        .filter(|c| c.column_id == property)
        .flat_map(|c| c.values.iter())
        .map(|v| v.parse().expect("a number"))
        .collect();
    numbers.sort_by(|a, b| a.partial_cmp(b).expect("numbers"));
    numbers
}

fn figure(value: f64) -> String {
    let text = format!("{value:.10}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn count_unique_in(text: &str) -> usize {
    text.matches("count-unique").count()
}

#[test]
fn a_calculation_leaves_its_column_where_it_was() {
    for extension in EXTENSIONS {
        // A view that lists none of its columns shows them in the file's order.
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        table.edit(
            TableGesture::new("resizeColumn")
                .column("p3")
                .view("v2")
                .setting("width", "120"),
        );
        let before: Vec<String> = table.baseline().columns.into_iter().map(|c| c.id).collect();

        // A calculation and a width for columns the view does not list yet, further along.
        table.edit(
            TableGesture::new("setColumnCalculation")
                .column("p6")
                .view("v2")
                .setting("calculate", "countAll"),
        );
        table.edit(
            TableGesture::new("resizeColumn")
                .column("p8")
                .view("v2")
                .setting("width", "200"),
        );

        // The columns are shown in the order they were, and the figure is the column's own.
        let after: Vec<String> = table.baseline().columns.into_iter().map(|c| c.id).collect();
        assert_eq!(before, after, "{extension}");
        assert_ne!("(none)", shown(&table, "p6"));
    }
}

#[test]
fn a_column_shows_under_it_what_its_calculation_comes_to_and_the_file_names_it() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        // Another column may already count different values: the file is judged by how often it says so.
        let counts_unique = count_unique_in(&std::fs::read_to_string(&table.path).expect("read"));
        table.edit(TableGesture::new("setCell").row("r2").column("p3").values(["1000.5"]));
        table.edit(
            TableGesture::new("setCell")
                .row("a8q7ddqb6zyubcyf5vuky6iwb")
                .column("p3")
                .values(["345345"]),
        );
        let rows = table.on_disk().rows.len();
        let numbers = numbers(&table, "p3");
        let distinct: HashSet<String> = numbers.iter().map(|n| n.to_string()).collect();
        assert!(numbers.len() >= 3 && numbers.len() < rows && distinct.len() < numbers.len());
        assert_eq!("(none)", shown(&table, "p3"));
        assert!(
            column(&table, "p3").settings["calculations"]
                .split(',')
                .any(|c| c == "median")
        );
        assert!(
            !column(&table, "p1").settings["calculations"]
                .split(',')
                .any(|c| c == "median")
        );

        // Each figure against the numbers read from the file.
        let count = numbers.len();
        let sum: f64 = numbers.iter().sum();
        let expected: Vec<(&str, String)> = vec![
            ("countAll", rows.to_string()),
            ("countValues", count.to_string()),
            ("countUnique", distinct.len().to_string()),
            ("countEmpty", (rows - count).to_string()),
            ("countNotEmpty", count.to_string()),
            (
                "percentEmpty",
                format!("{}%", figure(round1(100.0 * (rows - count) as f64 / rows as f64))),
            ),
            (
                "percentNotEmpty",
                format!("{}%", figure(round1(100.0 * count as f64 / rows as f64))),
            ),
            ("sum", figure(sum)),
            ("average", figure(sum / count as f64)),
            (
                "median",
                figure(if count % 2 == 1 {
                    numbers[count / 2]
                } else {
                    (numbers[count / 2 - 1] + numbers[count / 2]) / 2.0
                }),
            ),
            ("min", figure(numbers[0])),
            ("max", figure(numbers[count - 1])),
            ("range", figure(numbers[count - 1] - numbers[0])),
        ];
        for (calculation, figure) in &expected {
            table.edit(calculate("p3", calculation));
            assert_eq!(
                (*calculation, figure.as_str()),
                (*calculation, shown(&table, "p3").as_str()),
                "{extension}"
            );
            let view = second_view(&table.on_disk());
            assert_eq!(
                *calculation,
                view.columns.iter().find(|c| c.column_id == "p3").expect("p3").calculate
            );
        }

        // The file says it in its own words: the name with a hyphen, as the definition writes it.
        assert!(std::fs::read_to_string(&table.path).expect("read").contains("range"));
        table.edit(calculate("p3", "countUnique"));
        assert_eq!(
            counts_unique + 1,
            count_unique_in(&std::fs::read_to_string(&table.path).expect("read"))
        );

        // Taken away again: the line shows nothing and the file says nothing.
        table.edit(calculate("p3", ""));
        assert_eq!("(none)", shown(&table, "p3"));
        assert_eq!(
            counts_unique,
            count_unique_in(&std::fs::read_to_string(&table.path).expect("read"))
        );
    }
}

#[test]
fn dates_show_their_ends_a_box_counts_as_filled_when_ticked_and_a_calculation_of_another_type_is_refused() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        table.edit(
            TableGesture::new("setCell")
                .row("r2")
                .column("p7")
                .values(["1291-02-21"]),
        );
        table.edit(
            TableGesture::new("setCell")
                .row("a8q7ddqb6zyubcyf5vuky6iwb")
                .column("p7")
                .values(["1343-09-01"]),
        );
        let mut founded: Vec<String> = table
            .on_disk()
            .rows
            .iter()
            .flat_map(|r| r.cells.iter())
            .filter(|c| c.column_id == "p7")
            .flat_map(|c| c.values.clone())
            .collect();
        founded.sort();
        assert!(founded.len() >= 2);
        let ticked = table
            .on_disk()
            .rows
            .iter()
            .filter(|r| {
                r.cells
                    .iter()
                    .any(|c| c.column_id == "p6" && c.values.iter().any(|v| v == "true"))
            })
            .count();

        // The ends as their author wrote them, and the days between.
        table.edit(calculate("p7", "earliest"));
        assert_eq!(founded[0], shown(&table, "p7"));
        table.edit(calculate("p7", "latest"));
        assert_eq!(*founded.last().expect("a date"), shown(&table, "p7"));
        table.edit(calculate("p7", "dateRange"));
        let days =
            parse_date(founded.last().expect("a date")).expect("a date") - parse_date(&founded[0]).expect("a date");
        assert_eq!(
            format!("{days} day{}", if days == 1 { "" } else { "s" }),
            shown(&table, "p7")
        );

        // A box that is not ticked is empty, whether the file says false or says nothing.
        table.edit(calculate("p6", "countNotEmpty"));
        assert_eq!(ticked.to_string(), shown(&table, "p6"));

        // A sum of names, and a calculation nobody knows: both refused, saying why, and nothing written.
        let bytes = table.bytes();
        let (_, sum) = table.begin(calculate("p1", "sum"));
        let (_, unknown) = table.begin(calculate("p1", "guess"));
        assert!(sum.contains("Name"), "{sum}");
        assert_eq!("That is not a calculation.", unknown);
        assert_eq!(bytes, table.bytes());
    }
}

#[test]
fn a_calculation_is_over_the_rows_in_sight_and_one_left_from_another_type_shows_nothing_and_is_kept() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        table.edit(TableGesture::new("setCell").row("r2").column("p3").values(["530504"]));
        table.edit(calculate("p3", "sum"));
        let all = shown(&table, "p3");

        // The view is filtered to the cities of one country.
        table.edit(TableGesture::new("addFilter").column("p2"));
        table.edit(
            TableGesture::new("setFilter")
                .target("0")
                .values(["o1"])
                .setting("comparison", "is"),
        );

        // The sum is of those rows alone.
        let dutch: f64 = table
            .on_disk()
            .rows
            .iter()
            .filter(|r| {
                r.cells
                    .iter()
                    .any(|c| c.column_id == "p2" && c.values.iter().any(|v| v == "o1"))
            })
            .flat_map(|r| {
                r.cells
                    .iter()
                    .filter(|c| c.column_id == "p3")
                    .flat_map(|c| c.values.clone())
            })
            .map(|v| v.parse::<f64>().expect("a number"))
            .sum();
        assert_eq!(figure(dutch), shown(&table, "p3"), "{extension}");
        assert_ne!(all, shown(&table, "p3"));

        // The population becomes text: a sum of text shows nothing, and the view still says what it was asked for.
        table.edit(
            TableGesture::new("setColumnType")
                .column("p3")
                .setting("type", "text")
                .setting("confirmed", "true"),
        );
        assert_eq!("", shown(&table, "p3"));
        let view = second_view(&table.on_disk());
        assert_eq!(
            "sum",
            view.columns.iter().find(|c| c.column_id == "p3").expect("p3").calculate
        );
    }
}

#[test]
fn a_view_is_frozen_up_to_a_column_a_copy_of_it_is_too_and_deleting_that_column_unfreezes_both() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        assert!(!table.baseline().settings.settings.contains_key("freeze"));

        // Frozen up to the country, with a calculation under the population.
        table.edit(TableGesture::new("freezeColumns").column("p2"));
        table.edit(calculate("p3", "max"));

        // The view says so, to the file and to the screen; saying it again changes nothing.
        assert_eq!("p2", second_view(&table.on_disk()).freeze);
        assert_eq!("p2", table.baseline().settings.settings["freeze"]);
        let bytes = table.bytes();
        let (_, again) = table.begin(TableGesture::new("freezeColumns").column("p2"));
        assert_eq!("", again);
        assert_eq!(bytes, table.bytes());

        // The view is copied: the copy is frozen as far, and calculates the same.
        table.edit(TableGesture::new("duplicateView").view("v2"));
        let on_disk = table.on_disk();
        let copy = on_disk
            .views
            .iter()
            .find(|v| v.id != "v1" && v.id != "v2")
            .expect("the copy");
        assert_eq!("p2", copy.freeze);
        assert_eq!(
            "max",
            copy.columns.iter().find(|c| c.column_id == "p3").expect("p3").calculate
        );

        // The country goes: neither view is frozen up to a column that is not there, and the table reports nothing.
        table.edit(TableGesture::new("deleteColumn").column("p2"));
        assert!(table.on_disk().views.iter().all(|v| v.freeze.is_empty()));
        assert!(
            !table
                .baseline()
                .findings
                .iter()
                .any(|f| f.code == validator::VIEW_NAMES_MISSING_PROPERTY)
        );

        // Frozen up to the population, then not at all.
        table.edit(TableGesture::new("freezeColumns").view("v2").column("p3"));
        table.edit(TableGesture::new("freezeColumns").view("v2"));
        assert_eq!("", second_view(&table.on_disk()).freeze);
        let (_, gone) = table.begin(TableGesture::new("freezeColumns").view("v2").column("p2"));
        assert_ne!("", gone);
    }
}

#[test]
fn a_freeze_naming_a_property_the_table_does_not_have_is_reported_and_the_view_opens_without_it() {
    let folder = common::folder();
    let path = copy_plain("cities.yaml", folder.path());
    let text = std::fs::read_to_string(&path).expect("read");
    let frozen = text.replace("    name: Map", "    name: Map\n    freeze: p99");
    assert_ne!(text, frozen);
    std::fs::write(&path, frozen).expect("written");

    let table = EditingTable::new(path);
    let baseline = table.baseline();
    assert!(
        baseline
            .findings
            .iter()
            .any(|f| f.code == validator::VIEW_NAMES_MISSING_PROPERTY && f.message.contains("p99"))
    );
    assert!(!baseline.settings.settings.contains_key("freeze"));
    assert_eq!("p99", second_view(&table.on_disk()).freeze);
}

#[test]
fn a_search_shows_the_rows_that_mention_it_by_name_option_or_related_row_and_writes_nothing() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        let found = |table: &mut EditingTable| -> Vec<String> {
            table
                .lines()
                .into_iter()
                .filter(|l| !l.id.is_empty())
                .map(|l| l.id)
                .collect()
        };
        let all = found(&mut table);
        let on_disk = table.on_disk();
        let first = &on_disk.rows[0];
        let name = first.cell("p1").expect("a name").values[0].clone();
        let part = &name[1..];
        table.edit(calculate("p1", "countAll"));
        let bytes = table.bytes();

        // By a part of a name, whatever its case.
        table.edit(TableGesture::new("search").values([part.to_uppercase()]));
        let named: Vec<String> = on_disk
            .rows
            .iter()
            .filter(|r| {
                r.cells.iter().any(|c| {
                    c.column_id != "p6"
                        && c.key == "text"
                        && c.values.iter().any(|v| v.to_lowercase().contains(&part.to_lowercase()))
                })
            })
            .map(|r| r.id.clone())
            .collect();
        let now = found(&mut table);
        assert!(now.contains(&first.id));
        assert_eq!(named, now);
        assert!(named.len() < all.len(), "A search that finds every row tells nothing.");
        assert_eq!(named.len().to_string(), shown(&table, "p1"));
        assert_eq!(part.to_uppercase(), table.baseline().settings.settings["search"]);

        // By the name of an option, which the rows hold by its id.
        table.edit(TableGesture::new("search").values(["Belgium"]));
        let belgian: Vec<String> = on_disk
            .rows
            .iter()
            .filter(|r| {
                r.cells
                    .iter()
                    .any(|c| c.column_id == "p2" && c.values.iter().any(|v| v == "o2"))
            })
            .map(|r| r.id.clone())
            .collect();
        assert!(!belgian.is_empty());
        let now = found(&mut table);
        assert_eq!(
            belgian,
            now.iter()
                .filter(|id| belgian.contains(id))
                .cloned()
                .collect::<Vec<_>>()
        );
        assert!(!now.iter().any(|id| id == "o2"));

        // By the title of a related row, which the rows hold by its id too.
        let provinces = store::read(&folder.path().join(RELATED))
            .body
            .expect("it reads")
            .table()
            .clone();
        let related = on_disk
            .rows
            .iter()
            .flat_map(|r| r.cells.iter())
            .find(|c| c.column_id == "p5" && !c.values.is_empty())
            .expect("a relation")
            .values[0]
            .clone();
        let title = relations::title_of(&provinces, provinces.row(&related).expect("the related row"));
        table.edit(TableGesture::new("search").values([title]));
        let in_province: Vec<String> = on_disk
            .rows
            .iter()
            .filter(|r| {
                r.cells
                    .iter()
                    .any(|c| c.column_id == "p5" && c.values.contains(&related))
            })
            .map(|r| r.id.clone())
            .collect();
        let now = found(&mut table);
        assert_eq!(
            in_province,
            now.iter()
                .filter(|id| in_province.contains(id))
                .cloned()
                .collect::<Vec<_>>()
        );

        // For what no row mentions, then for nothing: every row again, and the file was never written.
        table.edit(TableGesture::new("search").values(["no city is called this"]));
        assert!(found(&mut table).is_empty());
        table.edit(TableGesture::new("search").values([""]));
        assert_eq!(all, found(&mut table));
        assert!(!table.baseline().settings.settings.contains_key("search"));
        assert_eq!(bytes, table.bytes());
    }
}
