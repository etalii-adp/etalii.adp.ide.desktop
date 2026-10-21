//! Where a timeline element is drawn, and what a gesture that puts it somewhere writes (the standalone host's
//! `TimelineElementMapper.cs`, `Placement` and `Element`, and `Commands/SetTimelinePlacementCommandHandler.cs` at
//! `8b6ccfc`).
//!
//! An element is drawn at its begin across, in seconds since the epoch (at the epoch when its begin does not read, so
//! it stays selectable), and at its row down. A gesture's point is a begin in the element's own precision and the row
//! nearest its y; a period keeps its duration, and one whose times do not read keeps its end as written. The end is
//! written only for a period, and only when it moves: a placement never gives an element an end it did not have. A
//! value that already reads as asked is not written, so a drag that keeps a row leaves it as the author wrote it.

use std::collections::HashMap;

use adp_cel::{Map, Value};
use adp_diagram::ModulePlacement;
use adp_disl::{DislChange, DislDiagram};

use crate::reading::TimelineElement;
use crate::scale;

/// What a placement that would put an element's end before its begin answers.
pub const INVERTED: &str = "An element cannot end before it begins.";

/// Where `element` is drawn: its begin's seconds, else the epoch, and its row's y.
pub fn position(element: &TimelineElement) -> (f64, f64) {
    let x = element.begin_instant().map(scale::to_seconds).unwrap_or(0.0);
    (x, scale::to_y(element.row))
}

/// The begin, the end of a period, and the row a gesture to (`x`, `y`) gives `element`.
pub fn placed(element: &TimelineElement, x: f64, y: f64) -> (String, Option<String>, i64) {
    let precision = element.begin_precision();
    let begin = scale::to_time(x, precision);
    let end = element
        .end
        .as_ref()
        .map(|written| match (element.begin_instant(), element.end_instant()) {
            (Some(was), Some(end)) => scale::to_text(begin + (end - was), crate::instants::Precision::of(written)),
            _ => written.clone(),
        });
    (scale::to_text(begin, precision), end, scale::to_nearest_row(y))
}

/// What a gesture to (`x`, `y`) writes of `element`, with the bounds its placement constraints judge, or why not.
pub fn placement(element: &TimelineElement, x: f64, y: f64) -> Result<ModulePlacement, String> {
    let (begin, end, row) = placed(element, x, y);
    let begin_instant = crate::instants::parse(&begin);
    let end_instant = end.as_deref().and_then(crate::instants::parse);
    if let (Some(begin), Some(end)) = (begin_instant, end_instant)
        && end < begin
    {
        return Err(INVERTED.to_owned());
    }
    let mut attributes = Map::default();
    if begin != element.begin {
        attributes.insert("begin", Value::String(begin.clone()));
    }
    if let (Some(end), Some(was)) = (&end, &element.end)
        && end != was
    {
        attributes.insert("end", Value::String(end.clone()));
    }
    if row != element.row {
        attributes.insert("row", Value::Int(row));
    }
    let bounds = |x: Option<i64>, x2: Option<i64>, row: i64| -> Map {
        let x = x.map(scale::to_seconds).unwrap_or(0.0);
        let x2 = x2.map(scale::to_seconds).unwrap_or(x);
        [
            ("x", Value::Double(x)),
            ("y", Value::Double(scale::to_y(row))),
            ("x2", Value::Double(x2)),
        ]
        .into_iter()
        .collect()
    };
    Ok(ModulePlacement {
        attributes,
        old_bounds: bounds(element.begin_instant(), element.end_instant(), element.row),
        new_bounds: bounds(begin_instant, end_instant.or(begin_instant), row),
    })
}

/// Refuses `changes` when one of them gives an element a begin or an end that puts its end before its begin, both
/// read: judged from what the change writes over what `diagram` holds, so a change that does not touch the times of
/// an element a hand edit inverted still goes through (the standalone host's placement, add and end commands).
pub fn refuse_inversions(diagram: &DislDiagram, changes: &[DislChange]) -> Result<(), String> {
    let mut times: HashMap<String, (String, Option<String>)> = HashMap::new();
    let text = |value: &Value| value.as_str().map(str::to_owned);
    for change in changes {
        let (id, attributes, created) = match change {
            DislChange::Create { id, attributes, .. } => (id, attributes, true),
            DislChange::Set {
                element_id, attributes, ..
            } => (element_id, attributes, false),
            _ => continue,
        };
        if !attributes.contains_key("begin") && !attributes.contains_key("end") {
            continue;
        }
        let (mut begin, mut end) = match times.get(id) {
            Some(known) => known.clone(),
            None if created => (String::new(), None),
            None => match diagram.element_by_id(id) {
                Some(element) => {
                    let read = TimelineElement::of(&element);
                    (read.begin, read.end)
                }
                None => continue,
            },
        };
        if let Some(value) = attributes.get("begin") {
            begin = text(value).unwrap_or_default();
        }
        if let Some(value) = attributes.get("end") {
            end = text(value);
        }
        if let (Some(b), Some(e)) = (
            crate::instants::parse(&begin),
            end.as_deref().and_then(crate::instants::parse),
        ) && e < b
        {
            return Err(INVERTED.to_owned());
        }
        times.insert(id.clone(), (begin, end));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element(begin: &str, end: Option<&str>, row: i64) -> TimelineElement {
        TimelineElement {
            id: "a".into(),
            label: "A".into(),
            begin: begin.into(),
            end: end.map(str::to_owned),
            row,
        }
    }

    #[test]
    fn a_period_moved_keeps_its_duration_and_its_precision() {
        let build = element("2025-09-12", Some("2025-10-27"), 10);
        let (x, y) = position(&build);
        assert_eq!(
            placed(&build, x + 604_800.0, y),
            ("2025-09-19".into(), Some("2025-11-03".into()), 10)
        );
    }

    #[test]
    fn a_moment_is_given_no_end() {
        let launch = element("2026-01-06", None, 11);
        assert_eq!(placed(&launch, 1_768_262_400.0, 720.0), ("2026-01-13".into(), None, 12));
    }

    #[test]
    fn an_unreadable_end_stays_as_written() {
        let odd = element("2026-01-06", Some("later"), 0);
        assert_eq!(placed(&odd, 0.0, 0.0).1.as_deref(), Some("later"));
    }

    #[test]
    fn what_already_reads_as_asked_is_not_written() {
        let build = element("2025-09-12", Some("2025-10-27"), 10);
        let (x, y) = position(&build);
        let moved = placement(&build, x + 604_800.0, y).unwrap();
        assert!(moved.attributes.contains_key("begin"));
        assert!(moved.attributes.contains_key("end"));
        assert!(!moved.attributes.contains_key("row"));
        assert!(placement(&build, x, y).unwrap().attributes.is_empty());
    }

    #[test]
    fn a_date_time_keeps_its_time_of_day() {
        let meeting = element("2026-01-05T14:30:00", Some("2026-01-05T15:00:00"), 0);
        let (x, y) = position(&meeting);
        assert_eq!(
            placed(&meeting, x + 3600.0, y),
            ("2026-01-05T15:30:00".into(), Some("2026-01-05T16:00:00".into()), 0)
        );
    }
}
