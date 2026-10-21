//! Arrange diagram on a hype cycle graph (the standalone host's `GhgArrangement.Tests.cs` and
//! `GhgArrangeBatch.Tests.cs` at `8b6ccfc`): only rows change, no two elements share a row where they meet, and the
//! rows are written in one step that one undo takes back.

mod common;

use adp_diagram_hype_cycle::arrangement;
use adp_diagram_hype_cycle::reading::{self, Trend, Trigger};
use adp_diagram_hype_cycle::scale;
use adp_layout::text_metric;
use common::{EXAMPLES, canvas, element, example, open};

#[test]
fn every_example_arranges_without_overlap_in_no_more_rows_than_it_had() {
    for name in EXAMPLES {
        let graph = open(&example(name));
        let diagram = graph.diagram();
        let unit = reading::unit_of(&diagram);
        let trends: Vec<Trend> = diagram
            .nodes_of_type("Trend")
            .iter()
            .map(Trend::of)
            .filter(|t| t.span().is_some())
            .collect();
        let rows = arrangement::rows_of(&diagram);
        for a in &trends {
            for b in trends.iter().filter(|b| a.id < b.id && rows[&a.id] == rows[&b.id]) {
                let (first, second) = if a.start <= b.start { (a, b) } else { (b, a) };
                let clear = scale::x_of(second.start.unwrap(), unit) - scale::x_of(first.stop.unwrap(), unit);
                assert!(
                    clear >= text_metric::width_of(&second.name, 12.0),
                    "{name}: {} and {} meet on row {}.",
                    first.name,
                    second.name,
                    rows[&a.id]
                );
            }
        }
        let before = diagram
            .nodes_of_type("Trend")
            .iter()
            .map(|t| Trend::of(t).row)
            .chain(diagram.nodes_of_type("Trigger").iter().map(|t| Trigger::of(t).row))
            .max()
            .unwrap()
            + 1;
        let after = rows.values().max().unwrap() + 1;
        assert!(after <= before, "{name}: {after} rows, where it had {before}.");
    }
}

#[test]
fn influenced_trends_are_arranged_on_rows_close_together() {
    // A far-off trend that influences one at the bottom of a stack of three.
    let graph = open(
        "gartner-hypecycle-graph: 1\ntrends:\n  - id: a\n    name: A\n    start: 2000-01\n    stop: 2004-01\n    row: 0\n  - id: b\n    name: B\n    start: 2000-01\n    stop: 2004-01\n    row: 1\n  - id: c\n    name: C\n    start: 2000-01\n    stop: 2004-01\n    row: 2\n  - id: later\n    name: Later\n    start: 2001-01\n    stop: 2006-01\n    row: 7\ninfluences:\n  - id: a--later\n    from: a\n    to: later\n",
    );
    let rows = arrangement::rows_of(&graph.diagram());
    assert_eq!((rows["a"] - rows["later"]).abs(), 1);
}

/// Arrange writes every changed row as one command, and one undo gives back the bytes, for every example; arranged
/// once, a graph is already arranged.
#[test]
fn arrange_writes_every_row_in_one_step_that_one_undo_takes_back() {
    for name in EXAMPLES {
        let mut graph = open(&example(name));
        let original = graph.bytes();
        graph
            .run(&canvas(0.0, 0.0), "ghg.arrange", &[])
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let rows = arrangement::rows_of(&graph.diagram());
        let diagram = graph.diagram();
        for trend in diagram.nodes_of_type("Trend").iter().map(Trend::of) {
            if let Some(row) = rows.get(&trend.id) {
                assert_eq!(trend.row, *row, "{name}: {}", trend.id);
            }
        }
        assert_eq!(
            graph.run(&element(&diagram.nodes_of_type("Trend")[0].id()), "ghg.arrange", &[]),
            Err(arrangement::ALREADY_ARRANGED.to_owned()),
            "{name}"
        );
        graph.undo();
        assert_eq!(graph.bytes(), original, "{name}");
    }
}

#[test]
fn a_graph_without_anything_to_arrange_says_so() {
    let mut graph = open("gartner-hypecycle-graph: 1\ntrends: []\ninfluences: []\n");
    assert_eq!(
        graph.run(&canvas(0.0, 0.0), "ghg.arrange", &[]),
        Err(arrangement::NOTHING_TO_ARRANGE.to_owned())
    );
}
