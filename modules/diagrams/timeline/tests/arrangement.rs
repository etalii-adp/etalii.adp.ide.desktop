//! "Arrange diagram" on a timeline: only rows change, nothing overlaps, and one undo puts it back (the standalone
//! host's `TimelineArrangementTests` at `8b6ccfc`), through the session's `timeline.arrange` entry.

mod common;

use std::collections::HashMap;

use adp_diagram::{ActionOutcome, ActionTarget};
use adp_diagram_timeline::arrangement;
use adp_diagram_timeline::reading::TimelineElement;
use common::{Timeline, open, roadmap};

const CANVAS: ActionTarget = ActionTarget::Canvas { x: 0.0, y: 0.0 };

fn read(timeline: &Timeline) -> (Vec<TimelineElement>, Vec<(String, String)>) {
    let document = timeline.workspace.documents().get(&timeline.body).unwrap();
    arrangement::read(&document.model().diagram)
}

fn rows_of(timeline: &Timeline) -> HashMap<String, i64> {
    let (elements, links) = read(timeline);
    arrangement::rows_of(&elements, &links)
}

#[test]
fn arranging_the_roadmap_uses_fewer_rows_overlaps_nothing_and_keeps_every_time() {
    let mut timeline = open(&roadmap());
    let (before, _) = read(&timeline);

    let outcome = timeline.execute(&CANVAS, "timeline.arrange", &[]);
    let (after, _) = read(&timeline);

    assert_eq!(ActionOutcome::Done, outcome);
    let rows = |elements: &[TimelineElement]| {
        let mut rows: Vec<i64> = elements.iter().map(|e| e.row).collect();
        rows.sort_unstable();
        rows.dedup();
        rows.len()
    };
    assert!(rows(&after) < rows(&before));
    let times = |elements: &[TimelineElement]| {
        elements
            .iter()
            .map(|e| (e.id.clone(), e.begin.clone(), e.end.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(times(&before), times(&after));
    assert_eq!(Some(0), after.iter().map(|e| e.row).min());
    let periods: Vec<&TimelineElement> = after.iter().filter(|e| e.end.is_some()).collect();
    for a in &periods {
        for b in periods.iter().filter(|b| b.id != a.id && b.row == a.row) {
            assert!(
                a.end_instant() < b.begin_instant() || b.end_instant() < a.begin_instant(),
                "{} and {} overlap on row {}.",
                a.label,
                b.label,
                a.row
            );
        }
    }
}

#[test]
fn undoing_an_arrangement_is_byte_identical_and_redo_arranges_again() {
    let mut timeline = open(&roadmap());
    let original = timeline.text();
    assert_eq!(ActionOutcome::Done, timeline.execute(&CANVAS, "timeline.arrange", &[]));
    let arranged = timeline.text();

    timeline.undo();
    let undone = timeline.text();
    timeline.redo();

    assert_eq!(original, undone);
    assert_eq!(arranged, timeline.text());
}

#[test]
fn arranging_twice_is_refused_the_second_time_because_nothing_would_move() {
    let mut timeline = open(&roadmap());
    assert_eq!(ActionOutcome::Done, timeline.execute(&CANVAS, "timeline.arrange", &[]));

    let again = timeline.execute(&CANVAS, "timeline.arrange", &[]);

    assert!(
        matches!(&again, ActionOutcome::Refused(reason) if reason.contains("already arranged")),
        "{again:?}"
    );
}

#[test]
fn an_empty_timeline_has_nothing_to_arrange() {
    let mut timeline = open("timeline: 1\r\nelements: []\r\n");

    let refused = timeline.execute(&CANVAS, "timeline.arrange", &[]);

    assert_eq!(
        ActionOutcome::Refused("There is nothing to arrange until this timeline has an element.".into()),
        refused
    );
}

#[test]
fn a_related_chain_is_arranged_on_one_row() {
    let timeline = open(concat!(
        "timeline: 1\n",
        "elements:\n",
        "  - id: other\n    label: Other\n    begin: 2026-01-01\n    end: 2026-03-01\n    row: 0\n",
        "  - id: a\n    label: A\n    begin: 2026-01-01\n    end: 2026-03-01\n    row: 5\n",
        "  - id: b\n    label: B\n    begin: 2026-04-01\n    end: 2026-06-01\n    row: 0\n",
        "  - id: c\n    label: C\n    begin: 2026-07-01\n    end: 2026-09-01\n    row: 9\n",
        "connections:\n",
        "  - id: ab\n    from: a\n    to: b\n",
        "  - id: bc\n    from: b\n    to: c\n",
    ));

    let rows = rows_of(&timeline);

    assert_eq!(rows["a"], rows["b"]);
    assert_eq!(rows["b"], rows["c"]);
    assert_ne!(rows["a"], rows["other"]);
}

#[test]
fn a_moment_and_the_later_element_it_leads_to_are_never_on_one_row_so_the_link_misses_its_label() {
    let timeline = open(&roadmap());
    let (elements, links) = read(&timeline);
    let by_id: HashMap<&str, &TimelineElement> = elements.iter().map(|e| (e.id.as_str(), e)).collect();

    let rows = rows_of(&timeline);

    let leaving: Vec<&(String, String)> = links
        .iter()
        .filter(|(from, to)| {
            by_id[from.as_str()].end_instant().is_none()
                && by_id[to.as_str()].begin_instant() > by_id[from.as_str()].begin_instant()
        })
        .collect();
    assert!(!leaving.is_empty());
    for (from, to) in leaving {
        assert_ne!(rows[from], rows[to], "{from} -> {to}");
    }
}
