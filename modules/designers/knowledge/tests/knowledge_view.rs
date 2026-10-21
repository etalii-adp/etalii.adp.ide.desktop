//! A view's lines (the standalone host's `KnowledgeView.Tests.cs` at 8b6ccfc; knowledge-designer Requirements 6.4 to
//! 6.7): which rows its filter lets through, in which order its sorts put them, and under which headings its grouping
//! shows them - through the DESL runtime's view engine, with the Knowledge designer's value types.
//!
//! **Properties of the result rather than examples alone.** A filter is checked against every row of a table of some
//! hundreds, both ways, by a reading of the comparison written out here rather than by the engine's own.

use std::collections::{HashMap, HashSet};

use adp_designer_knowledge::model::property;
use adp_designer_knowledge::{KnowledgeProperty, KnowledgeTable, vocabulary};
use adp_desl::table::content::{
    Condition, ContentCell, ContentOption, ContentRow, ContentView, FilterGroup, FilterItem, ViewSort,
};
use adp_desl::table::engine::{self, LineOptions, NO_VALUE};
use adp_desl::table::model::TableRow;
use adp_desl::table::values::day_number;

fn sizes() -> Vec<ContentOption> {
    vec![
        ContentOption::new("small", "Small", "default"),
        ContentOption::new("medium", "Medium", "default"),
        ContentOption::new("large", "Large", "default"),
    ]
}

fn tags() -> Vec<ContentOption> {
    vec![
        ContentOption::new("port", "Port", "default"),
        ContentOption::new("capital", "Capital", "default"),
        ContentOption::new("old", "Old", "default"),
    ]
}

fn prop(id: &str, value_type: &str, options: Vec<ContentOption>, parent: bool) -> KnowledgeProperty {
    property(
        id,
        id,
        value_type,
        id == "name",
        options,
        if parent { "." } else { "" },
        if parent { "one" } else { "none" },
        "",
        false,
        parent,
    )
}

fn properties() -> Vec<KnowledgeProperty> {
    vec![
        prop("name", "text", vec![], false),
        prop("people", "number", vec![], false),
        prop("seen", "checkbox", vec![], false),
        prop("founded", "date", vec![], false),
        prop("size", "selection", sizes(), false),
        prop("tags", "multipleSelection", tags(), false),
        prop("under", "relation", vec![], true),
    ]
}

/// A date of the proleptic calendar from its day number, as `yyyy-MM-dd`.
fn date(year: i64, month: i64, day: i64) -> String {
    assert!(day_number(year, month, day) > 0);
    format!("{year:04}-{month:02}-{day:02}")
}

/// A table of rows made by rule, not by chance: the same every run, and with every kind of hole - a row without a
/// number, without a size, without tags - so that an empty value is met by every comparison.
fn table_of(count: usize) -> KnowledgeTable {
    let (sizes, tags) = (sizes(), tags());
    let mut rows = Vec::new();
    for index in 0..count {
        let i = index as i64;
        let mut cells = vec![ContentCell::new(
            "name",
            [format!(
                "{}{} city {index}",
                &"abcdefgh"[index % 8..index % 8 + 1],
                (i * 7) % 13
            )],
            "text",
        )];
        if index % 5 != 0 {
            cells.push(ContentCell::new(
                "people",
                [((i * 37) % 101 * if index % 3 == 0 { 1 } else { 10 }).to_string()],
                "number",
            ));
        }
        if index % 2 == 0 {
            cells.push(ContentCell::new("seen", ["true"], "checked"));
        }
        if index % 7 != 0 {
            cells.push(ContentCell::new(
                "founded",
                [date(1200 + ((i * 53) % 800), 1 + (i % 12), 1 + (i % 28))],
                "date",
            ));
        }
        if index % 4 != 0 {
            cells.push(ContentCell::new("size", [sizes[index % 3].id.clone()], "option"));
        }
        if index % 3 != 0 {
            let values = if index % 6 == 1 {
                vec![tags[0].id.clone(), tags[2].id.clone()]
            } else {
                vec![tags[index % 3].id.clone()]
            };
            cells.push(ContentCell::new("tags", values, "options"));
        }
        // A value of a type the property had before: text where a number is expected.
        if index % 41 == 0 {
            cells.retain(|c| c.column_id != "people");
            cells.push(ContentCell::new("people", ["a great many"], "text"));
        }
        rows.push(ContentRow::new(format!("r{index}"), cells));
    }
    KnowledgeTable {
        name: "Cities".into(),
        active_view_id: "v".into(),
        columns: properties(),
        views: Vec::new(),
        rows,
    }
}

fn table() -> KnowledgeTable {
    table_of(240)
}

#[derive(Default)]
struct ViewOf {
    filter: Option<FilterGroup>,
    sorts: Vec<ViewSort>,
    group_by: &'static str,
    hide_empty: bool,
    order: Vec<&'static str>,
    hidden: Vec<&'static str>,
    collapsed: Vec<&'static str>,
}

fn view(of: ViewOf) -> ContentView {
    let strings = |v: Vec<&str>| v.into_iter().map(str::to_owned).collect();
    ContentView {
        filter: of.filter.unwrap_or_default(),
        sorts: of.sorts,
        group_by: of.group_by.into(),
        hide_empty_groups: of.hide_empty,
        group_order: strings(of.order),
        hidden_groups: strings(of.hidden),
        collapsed: strings(of.collapsed),
        ..ContentView::new("v", "View")
    }
}

fn filtered(conditions: Vec<FilterItem>, any: bool) -> ContentView {
    view(ViewOf {
        filter: Some(FilterGroup { any, items: conditions }),
        ..ViewOf::default()
    })
}

fn condition(column: &str, operator: &str, value: &str) -> FilterItem {
    FilterItem::Condition(Condition::new(column, operator, value))
}

fn lines_with(
    table: &KnowledgeTable,
    view: &ContentView,
    editable: bool,
    kept: Option<&HashSet<String>>,
) -> Vec<TableRow> {
    let none = HashSet::new();
    engine::lines(
        table,
        Some(view),
        &LineOptions {
            kept: Some(kept.unwrap_or(&none)),
            editable,
            ..LineOptions::default()
        },
    )
}

fn lines(table: &KnowledgeTable, view: &ContentView) -> Vec<TableRow> {
    lines_with(table, view, false, None)
}

fn one<'r>(row: &'r ContentRow, property: &str, key: &str) -> Option<&'r str> {
    row.cells
        .iter()
        .find(|c| c.column_id == property && c.key == key)
        .and_then(|c| c.values.first())
        .map(String::as_str)
}

fn number(row: &ContentRow) -> Option<f64> {
    one(row, "people", "number").map(|t| t.parse().expect("a number"))
}

/// What a comparison means, by the row's own values. Not the engine's code: the check on it.
fn means(row: &ContentRow, property: &str, comparison: &str, given: &str) -> bool {
    let name = one(row, "name", "text").unwrap_or_default();
    let people = number(row);
    let founded = one(row, "founded", "date");
    let size = one(row, "size", "option");
    let tags: Vec<&str> = row
        .cells
        .iter()
        .find(|c| c.column_id == "tags")
        .map(|c| c.values.iter().map(String::as_str).collect())
        .unwrap_or_default();
    let has = match property {
        "name" => !name.is_empty(),
        "people" => people.is_some(),
        "seen" => one(row, "seen", "checked").is_some(),
        "founded" => founded.is_some(),
        "size" => size.is_some(),
        _ => !tags.is_empty(),
    };
    let lower = |s: &str| s.to_lowercase();
    let n = || given.parse::<f64>().expect("a number");
    match (property, comparison) {
        (_, "is-empty") => !has,
        (_, "is-not-empty") => has,
        ("name", "is") => lower(name) == lower(given),
        ("name", "is-not") => lower(name) != lower(given),
        ("name", "contains") => lower(name).contains(&lower(given)),
        ("name", "does-not-contain") => !lower(name).contains(&lower(given)),
        ("name", "starts-with") => lower(name).starts_with(&lower(given)),
        ("name", "ends-with") => lower(name).ends_with(&lower(given)),
        ("people", "equals") => people == Some(n()),
        ("people", "does-not-equal") => people != Some(n()),
        ("people", "greater-than") => people.is_some_and(|p| p > n()),
        ("people", "less-than") => people.is_some_and(|p| p < n()),
        ("people", "at-least") => people.is_some_and(|p| p >= n()),
        ("people", "at-most") => people.is_some_and(|p| p <= n()),
        ("seen", "is-checked") => has,
        ("seen", "is-not-checked") => !has,
        ("founded", "is") => founded == Some(given),
        ("founded", "is-before") => founded.is_some_and(|f| f < given),
        ("founded", "is-after") => founded.is_some_and(|f| f > given),
        ("founded", "is-on-or-before") => founded.is_some_and(|f| f <= given),
        ("founded", "is-on-or-after") => founded.is_some_and(|f| f >= given),
        ("size", "is") => size == Some(given),
        ("size", "is-not") => size != Some(given),
        ("tags", "contains") => tags.contains(&given),
        ("tags", "does-not-contain") => !tags.contains(&given),
        _ => panic!("No meaning written for {property} {comparison}."),
    }
}

fn ids(lines: &[TableRow]) -> HashSet<String> {
    lines.iter().map(|l| l.id.clone()).collect()
}

// ---- the filter ----

const COMPARISONS: [(&str, &str, &str); 30] = [
    ("name", "is", "A0 CITY 0"),
    ("name", "is-not", "a0 city 0"),
    ("name", "contains", "CITY 1"),
    ("name", "does-not-contain", "city 1"),
    ("name", "starts-with", "B"),
    ("name", "ends-with", "7"),
    ("name", "is-empty", ""),
    ("name", "is-not-empty", ""),
    ("people", "equals", "370"),
    ("people", "does-not-equal", "370"),
    ("people", "greater-than", "99"),
    ("people", "less-than", "100"),
    ("people", "at-least", "100"),
    ("people", "at-most", "99"),
    ("people", "is-empty", ""),
    ("people", "is-not-empty", ""),
    ("seen", "is-checked", ""),
    ("seen", "is-not-checked", ""),
    ("founded", "is", "1253-02-02"),
    ("founded", "is-before", "1500-01-01"),
    ("founded", "is-after", "1500-01-01"),
    ("founded", "is-on-or-before", "1253-02-02"),
    ("founded", "is-on-or-after", "1253-02-02"),
    ("founded", "is-empty", ""),
    ("size", "is", "medium"),
    ("size", "is-not", "medium"),
    ("size", "is-empty", ""),
    ("tags", "contains", "old"),
    ("tags", "does-not-contain", "old"),
    ("tags", "is-not-empty", ""),
];

#[test]
fn every_row_shown_passes_the_condition_and_every_row_hidden_does_not() {
    let table = table();
    for (property, comparison, given) in COMPARISONS {
        let shown = ids(&lines(
            &table,
            &filtered(vec![condition(property, comparison, given)], false),
        ));
        // Both ways, and the case is not empty on either side - a filter that shows all or nothing proves little.
        for row in &table.rows {
            assert_eq!(
                means(row, property, comparison, given),
                shown.contains(&row.id),
                "{property} {comparison} '{given}' on {}",
                row.id
            );
        }
        if property != "name" || !matches!(comparison, "is-empty" | "is-not-empty" | "is-not") {
            assert!(
                (1..table.rows.len()).contains(&shown.len()),
                "{property} {comparison}: {} shown",
                shown.len()
            );
        }
    }
}

#[test]
fn every_comparison_the_definition_has_is_one_the_engine_knows() {
    let table = table();
    let samples = [
        ("text", "name", "a0 city 0"),
        ("number", "people", "100"),
        ("checkbox", "seen", ""),
        ("date", "founded", "1500-01-01"),
        ("selection", "size", "medium"),
        ("multipleSelection", "tags", "old"),
    ];
    for (value_type, property, given) in samples {
        let comparisons = vocabulary::comparisons(value_type).expect("comparisons");
        assert!(!comparisons.is_empty());
        for comparison in comparisons {
            let shown = lines(&table, &filtered(vec![condition(property, comparison, given)], false));
            // A comparison the engine did not know would let every row through.
            assert!(
                shown.len() < table.rows.len(),
                "{value_type} {comparison} filtered nothing."
            );
        }
    }
}

#[test]
fn conditions_are_joined_by_all_or_by_any_and_a_group_is_one_item() {
    let table = table();
    let large = ("size", "is", "large");
    let seen = ("seen", "is-checked", "");
    let many = ("people", "at-least", "500");
    let c = |(p, o, v): (&str, &str, &str)| condition(p, o, v);
    let is = |row: &ContentRow, (p, o, v): (&str, &str, &str)| means(row, p, o, v);

    let all = ids(&lines(&table, &filtered(vec![c(large), c(seen)], false)));
    let any = ids(&lines(&table, &filtered(vec![c(large), c(seen)], true)));
    let nested = ids(&lines(
        &table,
        &filtered(
            vec![c(large), FilterItem::Group(FilterGroup::any(vec![c(seen), c(many)]))],
            false,
        ),
    ));

    for row in &table.rows {
        assert_eq!(is(row, large) && is(row, seen), all.contains(&row.id));
        assert_eq!(is(row, large) || is(row, seen), any.contains(&row.id));
        assert_eq!(
            is(row, large) && (is(row, seen) || is(row, many)),
            nested.contains(&row.id)
        );
    }
    assert!(all.len() < nested.len() && nested.len() < any.len());
}

#[test]
fn a_condition_that_is_not_finished_filters_nothing_and_a_row_added_here_stays_in_sight() {
    let table = table_of(30);
    let unfinished = lines(&table, &filtered(vec![condition("people", "greater-than", "")], false));
    let nobody = filtered(vec![condition("name", "is", "nothing is called this")], false);
    let hidden = lines(&table, &nobody);
    let kept_rows: HashSet<String> = HashSet::from(["r7".to_owned()]);
    let kept = lines_with(&table, &nobody, false, Some(&kept_rows));

    assert_eq!(30, unfinished.len());
    assert!(hidden.is_empty());
    assert_eq!(vec!["r7"], kept.iter().map(|l| l.id.as_str()).collect::<Vec<_>>());
}

// ---- the sorts ----

#[test]
fn a_sort_by_number_is_by_number_with_the_rows_without_one_last_and_is_stable() {
    for descending in [false, true] {
        let table = table();
        let place: HashMap<&str, usize> = table.rows.iter().enumerate().map(|(i, r)| (r.id.as_str(), i)).collect();
        let lines = lines(
            &table,
            &view(ViewOf {
                sorts: vec![ViewSort {
                    column_id: "people".into(),
                    descending,
                }],
                ..ViewOf::default()
            }),
        );
        // Every row once.
        assert_eq!(table.rows.len(), ids(&lines).len());
        let rows: Vec<&ContentRow> = lines.iter().map(|l| &table.rows[place[l.id.as_str()]]).collect();
        for pair in rows.windows(2) {
            let (before, after) = (number(pair[0]), number(pair[1]));
            // A row without a number is after every row with one, whichever way the sort runs.
            assert!(
                !(before.is_none() && after.is_some()),
                "{} has no number and stands before {}",
                pair[0].id,
                pair[1].id
            );
            if let (Some(b), Some(a)) = (before, after) {
                // As numbers: 9 before 10, which as text it would not be.
                assert!(if descending { b >= a } else { b <= a }, "{b} stands before {a}.");
            }
            // Stable: rows the sort does not tell apart keep the file's order.
            if before == after {
                assert!(
                    place[pair[0].id.as_str()] < place[pair[1].id.as_str()],
                    "{} and {} changed places",
                    pair[0].id,
                    pair[1].id
                );
            }
        }
        assert!(rows.iter().any(|r| number(r).is_none()));
    }
}

#[test]
fn several_sorts_apply_in_turn_the_first_deciding_first() {
    let table = table();
    let sizes = sizes();
    let lines = lines(
        &table,
        &view(ViewOf {
            sorts: vec![
                ViewSort {
                    column_id: "size".into(),
                    descending: false,
                },
                ViewSort {
                    column_id: "people".into(),
                    descending: true,
                },
            ],
            ..ViewOf::default()
        }),
    );
    let rows: Vec<&ContentRow> = lines.iter().map(|l| table.row(&l.id).expect("a row")).collect();
    let size = |row: &ContentRow| {
        one(row, "size", "option").map_or(usize::MAX, |id| {
            sizes.iter().position(|o| o.id == id).expect("an option")
        })
    };
    for pair in rows.windows(2) {
        assert!(size(pair[0]) <= size(pair[1]), "The first sort did not decide first.");
        if size(pair[0]) == size(pair[1])
            && let (Some(b), Some(a)) = (number(pair[0]), number(pair[1]))
        {
            assert!(b >= a, "Within one size the second sort did not decide.");
        }
    }
    // Small, Medium, Large is the options' order; by name it would be Large, Medium, Small.
    let mut seen: Vec<&str> = Vec::new();
    for row in &rows {
        if let Some(s) = one(row, "size", "option")
            && !seen.contains(&s)
        {
            seen.push(s);
        }
    }
    assert_eq!(vec!["small", "medium", "large"], seen);
}

#[test]
fn a_sort_is_of_the_rows_the_filter_lets_through() {
    let table = table();
    let lines = lines(
        &table,
        &view(ViewOf {
            filter: Some(FilterGroup::all(vec![condition("seen", "is-checked", "")])),
            sorts: vec![ViewSort {
                column_id: "name".into(),
                descending: false,
            }],
            ..ViewOf::default()
        }),
    );
    assert_eq!(120, lines.len());
    let names: Vec<String> = lines
        .iter()
        .map(|l| l.cell("name").expect("a name").values[0].clone())
        .collect();
    let mut sorted = names.clone();
    sorted.sort_by_key(|n| n.to_uppercase());
    assert_eq!(sorted, names);
}

// ---- groups ----

/// The ids of the rows beneath each heading, by the heading's key.
fn beneath(lines: &[TableRow]) -> HashMap<String, Vec<String>> {
    let mut beneath: HashMap<String, Vec<String>> = HashMap::new();
    let mut current = String::new();
    for line in lines {
        if line.is_group {
            current = line.id.clone();
            beneath.insert(current.clone(), Vec::new());
        } else if !line.is_new_row {
            beneath
                .get_mut(&current)
                .expect("a heading first")
                .push(line.id.clone());
        }
    }
    beneath
}

fn grouped(by: &'static str) -> ContentView {
    view(ViewOf {
        group_by: by,
        ..ViewOf::default()
    })
}

#[test]
fn a_grouping_shows_every_row_under_its_value_with_a_heading_for_the_rows_without_and_counts_what_is_beneath() {
    let table = table();
    let lines = lines(&table, &grouped("size"));

    // The options' order, then the rows without a size.
    let headings: Vec<&TableRow> = lines.iter().filter(|l| l.is_group).collect();
    assert_eq!(
        vec!["small", "medium", "large", NO_VALUE],
        headings.iter().map(|h| h.id.as_str()).collect::<Vec<_>>()
    );
    assert_eq!(
        vec!["Small", "Medium", "Large", "No size"],
        headings.iter().map(|h| h.label.as_str()).collect::<Vec<_>>()
    );

    // Every row once, under the heading of its value, and each heading counting the rows beneath it.
    let beneath = beneath(&lines);
    assert_eq!(table.rows.len(), beneath.values().map(Vec::len).sum::<usize>());
    for heading in headings {
        assert_eq!(heading.count, beneath[&heading.id].len());
        for id in &beneath[&heading.id] {
            assert_eq!(
                heading.id,
                one(table.row(id).expect("a row"), "size", "option").unwrap_or(NO_VALUE)
            );
        }
    }
    assert!(!beneath[NO_VALUE].is_empty());
}

#[test]
fn a_row_with_several_values_is_under_each_of_them() {
    let table = table();
    let both: Vec<&str> = table
        .rows
        .iter()
        .filter(|r| r.cells.iter().any(|c| c.column_id == "tags" && c.values.len() == 2))
        .map(|r| r.id.as_str())
        .collect();
    let beneath = beneath(&lines(&table, &grouped("tags")));

    // Under Port and under Old, and so counted twice - and the rows without a tag once, under their own heading.
    assert!(!both.is_empty());
    for id in &both {
        assert!(beneath["port"].iter().any(|r| r == id) && beneath["old"].iter().any(|r| r == id));
    }
    assert_eq!(
        table.rows.len() + both.len(),
        beneath.values().map(Vec::len).sum::<usize>()
    );
    assert_eq!(
        table
            .rows
            .iter()
            .filter(|r| r.cells.iter().all(|c| c.column_id != "tags"))
            .count(),
        beneath[NO_VALUE].len()
    );
}

#[test]
fn a_checkbox_groups_as_unticked_and_ticked_with_no_heading_for_no_value() {
    let lines = lines(&table(), &grouped("seen"));
    let headings: Vec<&TableRow> = lines.iter().filter(|l| l.is_group).collect();
    assert_eq!(
        vec!["false", "true"],
        headings.iter().map(|h| h.id.as_str()).collect::<Vec<_>>()
    );
    assert_eq!(vec![120, 120], headings.iter().map(|h| h.count).collect::<Vec<_>>());
}

#[test]
fn a_view_orders_hides_and_folds_its_groups_and_leaves_out_the_empty_ones_when_asked() {
    // No row is large.
    let mut table = table();
    table.rows.retain(|r| one(r, "size", "option") != Some("large"));

    let plain = lines_with(&table, &grouped("size"), true, None);
    let arranged = lines_with(
        &table,
        &view(ViewOf {
            group_by: "size",
            hide_empty: true,
            order: vec![NO_VALUE, "medium"],
            hidden: vec!["small"],
            collapsed: vec!["medium"],
            ..ViewOf::default()
        }),
        true,
        None,
    );

    // An empty group is shown with nothing beneath it but the line to add a row at.
    assert_eq!(
        0,
        plain
            .iter()
            .find(|l| l.is_group && l.id == "large")
            .expect("large")
            .count
    );
    assert!(plain.iter().any(|l| l.is_new_row && l.id == "large"));

    // The view's order first, the hidden group and the empty one gone, the folded one a heading with its count and
    // nothing beneath.
    assert_eq!(
        vec![NO_VALUE, "medium"],
        arranged
            .iter()
            .filter(|l| l.is_group)
            .map(|l| l.id.as_str())
            .collect::<Vec<_>>()
    );
    let medium = arranged
        .iter()
        .find(|l| l.is_group && l.id == "medium")
        .expect("medium");
    assert!(medium.collapsed);
    assert!(medium.count > 0);
    assert_eq!(medium, arranged.last().expect("a line"));
    assert!(!arranged.iter().any(|l| l.is_new_row && l.id == "medium"));
}

#[test]
fn rows_are_sorted_within_their_groups() {
    let lines = lines(
        &table(),
        &view(ViewOf {
            sorts: vec![ViewSort {
                column_id: "people".into(),
                descending: true,
            }],
            group_by: "size",
            ..ViewOf::default()
        }),
    );
    for rows in beneath(&lines).values() {
        let numbers: Vec<f64> = rows
            .iter()
            .filter_map(|id| lines.iter().find(|l| l.id == *id && !l.is_group))
            .filter_map(|l| l.cell("people").and_then(|c| c.values[0].parse::<f64>().ok()))
            .collect();
        let mut sorted = numbers.clone();
        sorted.sort_by(|a, b| b.partial_cmp(a).expect("numbers"));
        assert_eq!(sorted, numbers);
    }
}

// ---- nesting ----

fn family(rows: &[(&str, &str)]) -> KnowledgeTable {
    KnowledgeTable {
        name: "Places".into(),
        active_view_id: "v".into(),
        columns: properties(),
        views: Vec::new(),
        rows: rows
            .iter()
            .map(|(id, under)| {
                let mut cells = vec![ContentCell::new("name", [*id], "text")];
                if !under.is_empty() {
                    cells.push(ContentCell::new("under", [*under], "rows"));
                }
                ContentRow::new(*id, cells)
            })
            .collect(),
    }
}

fn depths(lines: &[TableRow]) -> Vec<String> {
    lines.iter().map(|l| format!("{}:{}", l.id, l.depth)).collect()
}

#[test]
fn a_table_with_a_parent_relation_nests_its_rows_in_a_view_that_is_not_grouped_and_under_each_heading_of_one_that_is() {
    // A country with two provinces, one of them large.
    let mut table = family(&[("nl", ""), ("south", "nl"), ("north", "nl"), ("be", "")]);
    for row in table.rows.iter_mut().filter(|r| r.id == "nl" || r.id == "south") {
        row.cells.push(ContentCell::new("size", ["large"], "option"));
    }
    let plain = lines(&table, &view(ViewOf::default()));
    let grouped = lines(&table, &grouped("size"));

    // Under their parent in the first; in the second under it where both stand under the same heading, and at the
    // top where they do not.
    assert_eq!(vec!["nl:0", "south:1", "north:1", "be:0"], depths(&plain));
    let last = grouped
        .iter()
        .rev()
        .find(|l| l.is_group && l.count > 0)
        .expect("a heading")
        .id
        .clone();
    let shown: Vec<String> = grouped
        .iter()
        .filter(|l| if l.is_group { l.count > 0 } else { !l.is_new_row })
        .map(|l| {
            if l.is_group {
                l.id.clone()
            } else {
                format!("{}:{}", l.id, l.depth)
            }
        })
        .collect();
    assert_eq!(
        vec![
            "large".to_owned(),
            "nl:0".into(),
            "south:1".into(),
            last,
            "north:0".into(),
            "be:0".into()
        ],
        shown
    );
}

#[test]
fn grouped_by_the_parent_relation_rows_are_nested_under_their_parents_to_any_depth() {
    // A country, its provinces, their cities - listed out of order.
    let table = family(&[
        ("leiden", "south"),
        ("nl", ""),
        ("south", "nl"),
        ("north", "nl"),
        ("be", ""),
        ("haarlem", "north"),
        ("delft", "south"),
    ]);
    let lines = lines(&table, &grouped("under"));
    assert_eq!(
        vec!["nl:0", "south:1", "leiden:2", "delft:2", "north:1", "haarlem:2", "be:0"],
        depths(&lines)
    );
    assert!(!lines.iter().any(|l| l.is_group));
    assert_eq!(
        vec!["nl", "south", "north"],
        lines
            .iter()
            .filter(|l| l.has_children)
            .map(|l| l.id.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn a_folded_parent_hides_everything_under_it_and_a_row_whose_parent_is_not_shown_stands_at_the_top() {
    let table = family(&[
        ("nl", ""),
        ("south", "nl"),
        ("leiden", "south"),
        ("orphan", "nowhere"),
        ("be", ""),
    ]);
    let lines = lines(
        &table,
        &view(ViewOf {
            group_by: "under",
            collapsed: vec!["nl"],
            ..ViewOf::default()
        }),
    );
    assert_eq!(vec!["nl:0", "orphan:0", "be:0"], depths(&lines));
    assert!(lines[0].collapsed);
}

#[test]
fn rows_that_are_each_others_parents_are_shown_once_and_none_is_lost() {
    // A circle, which a file written outside ADP can hold.
    let table = family(&[("a", "b"), ("b", "c"), ("c", "a"), ("self", "self"), ("free", "")]);
    let lines = lines(&table, &grouped("under"));
    let mut shown: Vec<&str> = lines.iter().map(|l| l.id.as_str()).collect();
    shown.sort_unstable();
    assert_eq!(vec!["a", "b", "c", "free", "self"], shown);
}

// ---- the lines a row is added at ----

#[test]
fn an_editable_view_ends_with_the_line_a_row_is_added_at_and_a_view_that_is_not_does_not() {
    let table = table_of(3);
    let editable = lines_with(&table, &view(ViewOf::default()), true, None);
    let shown = lines(&table, &view(ViewOf::default()));
    assert_eq!(4, editable.len());
    assert!(editable[3].is_new_row);
    assert_eq!("", editable[3].id);
    assert_eq!(3, shown.len());
    assert!(!shown.iter().any(|l| l.is_new_row));
}
