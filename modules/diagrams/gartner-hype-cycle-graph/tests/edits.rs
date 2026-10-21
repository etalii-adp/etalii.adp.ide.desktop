//! Every edit of a trend or an influence (the standalone host's `GhgCommands.Tests.cs` at `8b6ccfc`, task 18): every
//! edit edits, every undo gives back the original bytes, and every refusal writes nothing, on the technology-trends
//! example. Each command of the standalone module is reached here the way the desktop host's canvas and grid reach
//! it: an action, a property row or a gesture on the session.

mod common;

use adp_diagram::ResizedEdge;
use adp_diagram_hype_cycle::reading::{BOUNDARY_ATTRIBUTES, Trend};
use adp_diagram_hype_cycle::scale::{self, TimeUnit};
use common::{Graph, canvas, connection, element, example, m, open, x};

fn graph() -> Graph {
    open(&example("technology-trends"))
}

/// The edit `name`, made on `graph`.
fn edit(graph: &mut Graph, name: &str) -> Result<(), String> {
    match name {
        "add a trend" => graph.run(
            &canvas(x(2030, TimeUnit::Month) + 1.0, scale::top_of(90) + 10.0),
            "ghg.add.trend",
            &["new-trend"],
        ),
        "remove a trend" => graph.run(&element("steamboats"), "ghg.remove", &[]),
        "move" => graph.move_to("railways", x(1830, TimeUnit::Month), scale::top_of(40)),
        "resize from the left" => graph.set("steam-engine", "ghg.start", "1780-01"),
        "resize from the right" => graph.set("steam-engine", "ghg.stop", "1950-01"),
        "drag the right edge" => {
            let (left, top) = graph.position("steam-engine");
            let right = x(1950, TimeUnit::Month);
            graph.resize("steam-engine", ResizedEdge::Right, left, top, right - left, 32.0)
        }
        "drag a boundary" => graph.set("railways", "ghg.peak-end", "1840-01"),
        "even the phases" => graph.run(&element("steam-engine"), "ghg.even-phases", &[]),
        "lower the phases" => graph.set("railways", "ghg.phases", "Peak and Trough"),
        "rename" => graph.run_answered(&element("railways"), "ghg.rename", "Railroads"),
        "set tags" => graph.set("railways", "ghg.tags", "transport, industry, society"),
        "describe a trend" => graph.set(
            "iron-smelting",
            "ghg.description",
            "Darby's coke furnace at Coalbrookdale.",
        ),
        "describe an influence" => graph.set("steam-engine--railways", "ghg.description", ""),
        // Railways never influences the steam engine, so the reverse of an existing influence is allowed.
        "influence" => graph.run(
            &connection("railways", "steam-engine"),
            "ghg.connect.influence",
            &["new-influence"],
        ),
        "move an attachment" => graph.set("steam-engine--railways", "ghg.to-attachment", "trough/bottom/0.4"),
        "remove an influence" => graph.run(&element("steam-engine--railways"), "ghg.disconnect", &[]),
        "arrange" => graph.run(&canvas(0.0, 0.0), "ghg.arrange", &[]),
        _ => panic!("no such edit: {name}"),
    }
}

const EVERY_EDIT: [&str; 18] = [
    "add a trend",
    "remove a trend",
    "move",
    "resize from the left",
    "resize from the right",
    "drag the right edge",
    "drag a boundary",
    "even the phases",
    "lower the phases",
    "rename",
    "set tags",
    "describe a trend",
    "describe an influence",
    "influence",
    "move an attachment",
    "remove an influence",
    "arrange",
    "remove a trend",
];

/// Every edit, then its undo, gives the original bytes.
#[test]
fn every_edit_then_its_undo_gives_the_original_bytes() {
    for name in EVERY_EDIT {
        let mut graph = graph();
        let original = graph.bytes();
        edit(&mut graph, name).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_ne!(
            graph.bytes(),
            original,
            "{name} edited nothing, so its undo proves nothing"
        );
        graph.undo();
        assert_eq!(graph.bytes(), original, "{name}");
    }
}

fn refusal(graph: &mut Graph, name: &str) -> Result<(), String> {
    match name {
        "influence a trend that already influences it" => {
            graph.run(&connection("steam-engine", "railways"), "ghg.connect.influence", &["x"])
        }
        "influence itself" => graph.run(&connection("railways", "railways"), "ghg.connect.influence", &["x"]),
        "influence a trend that is not there" => {
            graph.run(&connection("railways", "nothing-here"), "ghg.connect.influence", &["x"])
        }
        "stop before the start" => graph.set("railways", "ghg.stop", "1820-01"),
        "drag a hidden boundary" => graph.set("industry-4", "ghg.slope-end", "2016-01"),
        "drag to a date that is not one" => graph.set("railways", "ghg.peak-end", "soon"),
        "remove a trend that is not there" => graph.run(&element("nothing-here"), "ghg.remove", &[]),
        "remove an influence that is not there" => graph.run(&element("nothing-here"), "ghg.disconnect", &[]),
        _ => panic!("no such refusal: {name}"),
    }
}

/// Every refusal says why and leaves the bytes unchanged.
#[test]
fn every_refusal_writes_nothing_and_says_why() {
    for (name, saying) in [
        (
            "influence a trend that already influences it",
            "already influences that one",
        ),
        ("influence itself", "cannot influence itself"),
        ("influence a trend that is not there", "from one trend to another"),
        ("stop before the start", "at least"),
        ("drag a hidden boundary", "between two visible phases"),
        ("drag to a date that is not one", "is not a date"),
        ("remove a trend that is not there", ""),
        ("remove an influence that is not there", ""),
    ] {
        let mut graph = graph();
        let original = graph.bytes();
        let refused = refusal(&mut graph, name).expect_err(name);
        assert!(refused.contains(saying), "{name}: {refused}");
        assert_eq!(graph.bytes(), original, "{name}");
    }
}

fn touching(graph: &Graph, id: &str) -> usize {
    graph
        .diagram()
        .relations_of_type("Influence")
        .iter()
        .filter(|i| i.source_id().as_deref() == Some(id) || i.target_id().as_deref() == Some(id))
        .count()
}

/// Deleting a trend with two influences restores all three on one undo.
#[test]
fn deleting_a_trend_with_two_influences_restores_all_three_on_undo() {
    let mut graph = graph();
    let original = graph.bytes();
    // Steamboats: the steam engine made them, and they led to ocean steamships.
    assert_eq!(touching(&graph, "steamboats"), 2);
    graph.run(&element("steamboats"), "ghg.remove", &[]).unwrap();
    assert!(graph.diagram().element_by_id("steamboats").is_none());
    assert_eq!(touching(&graph, "steamboats"), 0);
    graph.undo();
    assert_eq!(graph.bytes(), original);
    assert_eq!(touching(&graph, "steamboats"), 2);
}

/// A second influence in one direction is refused with a sentence by the module's own check, while one the other
/// way is accepted.
#[test]
fn a_scripted_duplicate_is_refused_while_the_opposite_direction_is_accepted() {
    let mut graph = graph();
    let original = graph.bytes();
    assert_eq!(
        graph.run(&connection("steam-engine", "railways"), "ghg.connect.influence", &["x"]),
        Err("This trend already influences that one; a trend influences another once in each direction.".to_owned())
    );
    assert_eq!(graph.bytes(), original);
    graph
        .run(
            &connection("railways", "steam-engine"),
            "ghg.connect.influence",
            &["back"],
        )
        .unwrap();
    let back = graph.diagram().element_by_id("back").unwrap();
    assert_eq!(back.source_id().as_deref(), Some("railways"));
    assert_eq!(back.target_id().as_deref(), Some("steam-engine"));
    // The source's last visible phase, bottom, halfway, to the target's Peak, top, halfway.
    let text = graph.text();
    assert!(
        text.contains("  - id: back\n    from: railways\n    from-phase: plateau\n    from-edge: bottom\n    from-at: 0.5\n    to: steam-engine\n    to-phase: peak\n    to-edge: top\n    to-at: 0.5\n"),
        "{text}"
    );
}

/// A boundary dragged past its neighbour stops a month short of it.
#[test]
fn a_boundary_dragged_past_its_neighbour_is_clamped() {
    let mut graph = graph();
    // The steam engine's Trough ends at 1800-01; drag the Peak's end to 1850.
    graph.set("steam-engine", "ghg.peak-end", "1850-01").unwrap();
    let steam = Trend::of(&graph.diagram().element_by_id("steam-engine").unwrap());
    assert_eq!(steam.dragged, [Some(m(1799, 12)), Some(m(1800, 1)), None]);
}

/// A drag reaches the document through the session and is one undo away, like every other edit.
#[test]
fn a_move_through_the_session_is_written_and_one_undo_away() {
    let mut graph = graph();
    let original = graph.bytes();
    graph
        .move_to("railways", x(1830, TimeUnit::Month), scale::top_of(40))
        .unwrap();
    let railways = Trend::of(&graph.diagram().element_by_id("railways").unwrap());
    assert_eq!((railways.start, railways.row), (Some(m(1830, 1)), 40));
    assert_eq!(railways.stop, Some(m(1830, 1) + (m(1930, 1) - m(1825, 9))));
    graph.undo();
    assert_eq!(graph.bytes(), original);
}

/// A drag the graph cannot place is refused and writes nothing: an influence, which follows its ends.
#[test]
fn a_drag_the_graph_cannot_place_is_refused_and_writes_nothing() {
    let mut graph = graph();
    let original = graph.bytes();
    assert!(
        graph
            .move_to("steam-engine--railways", x(1830, TimeUnit::Month), scale::top_of(40))
            .is_err()
    );
    assert_eq!(graph.bytes(), original);
}

/// A move keeps a trend's dragged boundaries where they were in it, and a resize scales them with the span.
#[test]
fn a_move_shifts_the_dragged_boundaries_and_a_span_scales_them() {
    let mut graph = graph();
    graph
        .move_to("steam-engine", x(1775, TimeUnit::Month), scale::top_of(22))
        .unwrap();
    let steam = Trend::of(&graph.diagram().element_by_id("steam-engine").unwrap());
    assert_eq!((steam.start, steam.stop), (Some(m(1775, 1)), Some(m(1910, 1))));
    assert_eq!(steam.dragged, [None, Some(m(1810, 1)), None]);
    assert!(graph.text().contains("    trough-end: 1810-01\n"));
    graph.set("steam-engine", "ghg.stop", "2045-01").unwrap();
    let steam = Trend::of(&graph.diagram().element_by_id("steam-engine").unwrap());
    // 35 of 135 years in, now of 270 years.
    assert_eq!(steam.dragged[1], Some(m(1845, 1)));
    let _ = BOUNDARY_ATTRIBUTES;
}

/// A phase count the span is too short for is refused.
#[test]
fn a_phase_count_the_span_is_too_short_for_is_refused() {
    let mut graph = open(
        "gartner-hypecycle-graph: 1\ntrends:\n  - id: t\n    name: T\n    start: 2000-01\n    stop: 2000-03\n    row: 0\n    phases: 2\ninfluences: []\n",
    );
    let original = graph.bytes();
    assert_eq!(
        graph.set("t", "ghg.phases", "All four"),
        Err("A trend showing 4 phases must be at least 4 months long, one per phase.".to_owned())
    );
    assert_eq!(graph.bytes(), original);
}
