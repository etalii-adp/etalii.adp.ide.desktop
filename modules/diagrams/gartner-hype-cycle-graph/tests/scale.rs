//! The time scale, the rows and the time unit (the standalone host's `GhgPhases.Tests.cs`, the scale part, and
//! `GhgTimeUnit.Tests.cs` at `8b6ccfc`): month to x, row to y, the dates the document writes, and a document's
//! `unit:` setting the step its axis is drawn and snapped in. The dates stay months whatever the unit.

mod common;

use adp_diagram::Length;
use adp_diagram_hype_cycle::placement;
use adp_diagram_hype_cycle::reading::{self, Trend};
use adp_diagram_hype_cycle::scale::{self, TimeUnit};
use common::{canvas, m, open, x};
use serde_json::Value as Json;

fn document(unit: Option<&str>) -> String {
    format!(
        "gartner-hypecycle-graph: 1\n{}trends:\n  - id: writing\n    name: Writing\n    start: -3400-01\n    stop: -0500-01\n    row: 0\n    phases: 4\n  - id: printing\n    name: Printing\n    start: 1440-01\n    stop: 1700-01\n    row: 2\n    phases: 4\ninfluences:\n",
        unit.map(|u| format!("unit: {u}\n")).unwrap_or_default()
    )
}

fn trend_document(start: i64, stop: i64, row: i64) -> String {
    format!(
        "gartner-hypecycle-graph: 1\ntrends:\n  - id: t\n    name: T\n    start: {}\n    stop: {}\n    row: {row}\n    phases: 4\ninfluences: []\n",
        scale::format_month(start),
        scale::format_month(stop)
    )
}

/// Where a trend of `text` is drawn: its left edge and top, and its width, in canvas units.
fn drawn(text: &str) -> (f64, f64, f64) {
    let graph = open(text);
    let (x, y) = graph.position("t");
    let element = graph.diagram().element_by_id("t").unwrap();
    let Some(Length::Canvas(width)) = placement::extent(&element).unwrap().width else {
        panic!("a trend is as wide as it lasts");
    };
    (x, y, width)
}

/// The scale, against the one checked-in fixture the standalone client asserts too: a trend's left and right edges
/// are its start and stop, and a row's top and middle are where the design puts them.
#[test]
fn the_scale_agrees_with_the_shared_fixture() {
    let fixture: Json =
        serde_json::from_str(&std::fs::read_to_string(common::fixtures().join("scale-fixture.json")).unwrap()).unwrap();
    assert_eq!(fixture["unitsPerMonth"], 4);
    assert_eq!(scale::UNITS_PER_STEP, 4.0);
    assert_eq!(fixture["origin"], "1900-01");
    assert_eq!(scale::format_month(scale::ORIGIN_MONTH), "1900-01");
    assert_eq!(fixture["trendHeight"].as_f64(), Some(scale::TREND_HEIGHT));
    assert_eq!(fixture["rowStep"].as_f64(), Some(scale::ROW_STEP));
    for month in fixture["months"].as_array().unwrap() {
        let date = month["date"].as_str().unwrap();
        let at = month["x"].as_f64().unwrap();
        let index = scale::parse_month(date).unwrap();
        assert_eq!(scale::x_of(index, TimeUnit::Month), at, "{date}");
        assert_eq!(scale::format_month(scale::nearest_month_at(at, TimeUnit::Month)), date);
        assert_eq!(
            scale::format_month(scale::month_containing(at + 1.5, TimeUnit::Month)),
            date
        );
        if (m(-9999, 1)..m(9999, 1)).contains(&index) {
            let (starting_x, _, _) = drawn(&trend_document(index, index + 12, 0));
            let (stopping_x, _, stopping_width) = drawn(&trend_document(index - 12, index, 0));
            assert_eq!(starting_x, at, "a trend starting at {date} has its left edge there");
            assert_eq!(
                stopping_x + stopping_width,
                at,
                "a trend stopping at {date} has its right edge there"
            );
        }
    }
    let units: Vec<(String, i64)> = fixture["units"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, months)| (name.clone(), months.as_i64().unwrap()))
        .collect();
    for (name, months) in units {
        assert_eq!(TimeUnit::named(&name).months(), months, "{name}");
    }
    for dated in fixture["unitDates"].as_array().unwrap() {
        let unit = TimeUnit::named(dated["unit"].as_str().unwrap());
        let date = dated["date"].as_str().unwrap();
        let at = dated["x"].as_f64().unwrap();
        assert_eq!(scale::x_of(scale::parse_month(date).unwrap(), unit), at, "{date}");
        assert_eq!(scale::format_month(scale::nearest_month_at(at, unit)), date);
        assert_eq!(scale::format_month(scale::month_containing(at + 1.5, unit)), date);
    }
    for row in fixture["rows"].as_array().unwrap() {
        let number = row["row"].as_i64().unwrap();
        let top = row["top"].as_f64().unwrap();
        let middle = row["middle"].as_f64().unwrap();
        assert_eq!(scale::top_of(number), top);
        assert_eq!(scale::row_at_top(top), number);
        assert_eq!(scale::row_at_middle(middle), number);
        let (_, drawn_top, _) = drawn(&trend_document(m(1900, 1), m(1901, 1), number));
        assert_eq!(drawn_top + scale::TREND_HEIGHT / 2.0, middle);
    }
}

/// A row is found through the one shared rounding rule, so a top exactly between two rows rounds away from zero.
#[test]
fn a_top_exactly_between_rows_rounds_away_from_zero() {
    for (rows, expected) in [(0.5, 1), (-0.5, -1), (2.5, 3), (-2.5, -3)] {
        assert_eq!(scale::row_at_top(rows * scale::ROW_STEP), expected);
        assert_eq!(
            scale::row_at_middle(rows * scale::ROW_STEP + scale::TREND_HEIGHT / 2.0),
            expected
        );
    }
}

#[test]
fn a_month_that_is_not_yyyy_mm_does_not_parse() {
    for text in [
        "1900-13",
        "1900-00",
        "1900",
        "19000-01",
        "1900-1",
        "-900-01",
        "+1900-01",
        "-1234567-01",
    ] {
        assert_eq!(scale::parse_month(text), None, "{text}");
    }
}

#[test]
fn a_named_unit_is_read() {
    for (name, months) in [("month", 1), ("year", 12), ("decade", 120), ("century", 1200)] {
        let graph = open(&document(Some(name)));
        assert_eq!(reading::unit_of(&graph.diagram()).months(), months, "{name}");
        assert!(
            graph.session.problems(graph.workspace.documents()).is_empty(),
            "{name}: {:?}",
            graph.session.problems(graph.workspace.documents())
        );
    }
}

#[test]
fn a_document_that_names_no_unit_is_drawn_in_months() {
    let graph = open(&document(None));
    assert_eq!(reading::unit_of(&graph.diagram()), TimeUnit::Month);
}

#[test]
fn an_unknown_unit_is_drawn_in_months() {
    for name in ["millennium", "Year", "[year]"] {
        let graph = open(&document(Some(name)));
        assert_eq!(reading::unit_of(&graph.diagram()), TimeUnit::Month, "{name}");
        assert_eq!(graph.diagram().nodes_of_type("Trend").len(), 2);
    }
}

/// A step of any unit is four canvas units wide, measured from 1900-01.
#[test]
fn the_x_of_a_month_is_four_units_a_step() {
    for (name, year, months, at) in [
        ("month", 1901, 0, 48.0),
        ("year", 1901, 0, 4.0),
        ("year", 1800, 0, -400.0),
        ("decade", 2000, 0, 40.0),
        ("decade", -3400, 0, -2120.0),
        ("century", 1500, 0, -16.0),
        ("year", 1900, 6, 2.0),
    ] {
        assert_eq!(
            scale::x_of(m(year, 1) + months, TimeUnit::named(name)),
            at,
            "{name} {year}"
        );
    }
}

/// A snap lands on the start of the nearest step: a January in a diagram of years, a year ending in 0 in one of
/// decades.
#[test]
fn a_snap_lands_on_the_start_of_the_nearest_step() {
    for (name, at, year) in [
        ("year", 401.0, 2000),
        ("year", -397.0, 1801),
        ("decade", 41.0, 2000),
        ("decade", -2119.0, -3400),
        ("century", -17.0, 1500),
    ] {
        assert_eq!(
            scale::nearest_month_at(at, TimeUnit::named(name)),
            m(year, 1),
            "{name} {at}"
        );
    }
}

#[test]
fn a_drop_is_the_start_of_the_step_it_falls_in() {
    for (name, at, year) in [("year", 7.9, 1901), ("year", -0.1, 1899), ("decade", 43.9, 2000)] {
        assert_eq!(
            scale::month_containing(at, TimeUnit::named(name)),
            m(year, 1),
            "{name} {at}"
        );
    }
}

/// The canvas is handed each trend at the unit's scale.
#[test]
fn every_trend_is_drawn_at_its_documents_scale() {
    let graph = open(&document(Some("decade")));
    let writing = graph.diagram().element_by_id("writing").unwrap();
    assert_eq!(
        placement::extent(&writing).unwrap().width,
        Some(Length::Canvas((2900.0 * 12.0) * 4.0 / 120.0))
    );
    assert_eq!(graph.position("writing").0, x(-3400, TimeUnit::Decade));
    assert_eq!(Trend::of(&writing).months(), 2900 * 12);
}

/// A trend moved in a diagram of decades starts on a decade, and keeps its length.
#[test]
fn a_move_in_a_diagram_of_decades_lands_on_a_decade() {
    let mut graph = open(&document(Some("decade")));
    graph
        .move_to("printing", scale::x_of(m(1453, 7), TimeUnit::Decade), scale::top_of(4))
        .unwrap();
    let printing = Trend::of(&graph.diagram().element_by_id("printing").unwrap());
    assert_eq!(
        (printing.start, printing.stop, printing.row),
        (Some(m(1450, 1)), Some(m(1710, 1)), 4)
    );
}

/// A trend dropped into a diagram of centuries is twelve centuries long, from the century it landed in.
#[test]
fn a_drop_in_a_diagram_of_centuries_is_twelve_centuries_long() {
    let mut graph = open(&document(Some("century")));
    graph
        .run(
            &canvas(x(1250, TimeUnit::Century), scale::top_of(6) + 10.0),
            "ghg.add.trend",
            &["new"],
        )
        .unwrap();
    let added = Trend::of(&graph.diagram().element_by_id("new").unwrap());
    assert_eq!(added.name, "New trend");
    assert_eq!((added.start, added.stop), (Some(m(1200, 1)), Some(m(2400, 1))));
}

/// A trend dropped from the toolbox lands where the canvas menu's Add trend here puts it: from the start of the step
/// the drop fell in, twelve steps long, on the row its middle is nearest.
#[test]
fn a_toolbox_drop_reads_the_point_in_the_diagrams_unit() {
    let mut graph = open(&document(Some("decade")));
    graph
        .session
        .drop_tool(
            &mut graph.workspace,
            "ghg.toolbox.trend",
            x(1955, TimeUnit::Decade) + 1.0,
            scale::top_of(6) + 10.0,
            &adp_disl::behavior::ids::fixed(&["dropped"]),
        )
        .unwrap();
    let dropped = Trend::of(&graph.diagram().element_by_id("dropped").unwrap());
    assert_eq!(
        (dropped.start, dropped.stop, dropped.row),
        (Some(m(1950, 1)), Some(m(2070, 1)), 6)
    );
}
