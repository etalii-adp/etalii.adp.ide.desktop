//! Where a trend, a trigger and a note are drawn, and what a gesture that moves or resizes one writes (the standalone
//! host's `GhgElementMapper.cs`, `GhgSession.cs`, `Commands/SetGhgPlacementCommandHandler.cs`,
//! `Commands/SetGhgSpanCommandHandler.cs` and `Commands/SetGhgNoteSizeCommandHandler.cs` at `8b6ccfc`).
//!
//! Positions are canvas units, as the standalone host's canvas drew them: a trend's and a note's top-left corner (its
//! start, or its `at`, across, and its row's top down), a trigger's centre (its date across and its row's middle down).
//! A trend is as wide as it lasts, a note as wide and as tall as it keeps. A move snaps a trend's or a note's left edge
//! to the nearest step and its top to the nearest row, a trigger's centre to the nearest step and the row whose middle
//! is nearest; a trend keeps its length and its dragged boundaries move with it. A drag of a trend's edge writes its
//! start or its stop, its dragged boundaries scaled with the span; a drag of a note's edge writes its size, and its
//! left edge's month and its top's row when the left or the top border moved.

use adp_cel::{Map, Value};
use adp_diagram::{Extent, Length, ModulePlacement, Resize, ResizedEdge};
use adp_disl::DislElement;

use crate::phases;
use crate::reading::{self, Note, Trend, Trigger};
use crate::scale::{self, TimeUnit};

/// What a gesture on a trend whose dates do not read answers (`GhgRefusals.NoSpan`).
pub const NO_SPAN: &str =
    "This trend's dates cannot be read, so it cannot be changed until they are fixed in the file.";

/// What a resize of a trigger, or of a trend's top or bottom, answers.
pub const NOT_RESIZED: &str = "Only a trend's start and stop, and a note's borders, can be dragged.";

fn unit_of(element: &DislElement) -> TimeUnit {
    reading::unit_of(&element.diagram())
}

/// Where `element` is drawn, in canvas units; none for an influence. A node without a readable place is drawn at the
/// origin, so it stays selectable.
pub fn position(element: &DislElement) -> Option<(f64, f64)> {
    if element.is_relation() || element.is_a("Influence") {
        return None;
    }
    let unit = unit_of(element);
    if element.is_a("Trend") {
        let trend = Trend::of(element);
        return Some((
            trend.start.map_or(0.0, |start| scale::x_of(start, unit)),
            scale::top_of(trend.row),
        ));
    }
    if element.is_a("Trigger") {
        let trigger = Trigger::of(element);
        return Some((
            trigger.date.map_or(0.0, |date| scale::x_of(date, unit)),
            scale::top_of(trigger.row) + scale::TREND_HEIGHT / 2.0,
        ));
    }
    if element.is_a("Note") {
        let note = Note::of(element);
        return Some((note.at.map_or(0.0, |at| scale::x_of(at, unit)), scale::top_of(note.row)));
    }
    None
}

/// The size a trend or a note is drawn at, in canvas units; none for a trigger, whose size is the notation's.
pub fn extent(element: &DislElement) -> Option<Extent> {
    if element.is_a("Trend") {
        let trend = Trend::of(element);
        let months = trend.months();
        return (months > 0).then(|| Extent {
            width: Some(Length::Canvas(scale::width_of(months, unit_of(element)))),
            height: None,
        });
    }
    if element.is_a("Note") {
        let note = Note::of(element);
        return Some(Extent {
            width: note.width.filter(|w| *w > 0.0).map(Length::Canvas),
            height: note.height.filter(|h| *h > 0.0).map(Length::Canvas),
        });
    }
    None
}

fn bounds(x: f64, y: f64, x2: Option<f64>) -> Map {
    let mut map: Map = [("x", Value::Double(x)), ("y", Value::Double(y))].into_iter().collect();
    if let Some(x2) = x2 {
        map.insert("x2", Value::Double(x2));
    }
    map
}

/// What a move of `element` to (`x`, `y`) writes, or why it may not; none for an influence.
pub fn placement(element: &DislElement, x: f64, y: f64) -> Option<Result<ModulePlacement, String>> {
    let unit = unit_of(element);
    let (old_x, old_y) = position(element)?;
    let mut attributes = Map::default();
    if element.is_a("Trigger") {
        let trigger = Trigger::of(element);
        let date = scale::nearest_month_at(x, unit);
        attributes.insert("date", Value::Int(date));
        let row = scale::row_at_middle(y);
        if row != trigger.row {
            attributes.insert("row", Value::Int(row));
        }
    } else if element.is_a("Note") {
        attributes.insert("at", Value::Int(scale::nearest_month_at(x, unit)));
        attributes.insert("row", Value::Int(scale::row_at_top(y)));
    } else {
        let trend = Trend::of(element);
        let Some((start, stop)) = trend.span() else {
            return Some(Err(NO_SPAN.to_owned()));
        };
        let new_start = scale::nearest_month_at(x, unit);
        let shift = new_start - start;
        attributes.insert("start", Value::Int(new_start));
        attributes.insert("stop", Value::Int(stop + shift));
        let row = scale::row_at_top(y);
        if row != trend.row {
            attributes.insert("row", Value::Int(row));
        }
        boundaries_into(&mut attributes, &phases::moved(&trend.dragged, shift));
    }
    Some(Ok(ModulePlacement {
        attributes,
        old_bounds: bounds(old_x, old_y, None),
        new_bounds: bounds(x, y, None),
    }))
}

/// The stored boundaries as the attributes that hold them: a dragged one written, one never dragged removed.
pub fn boundaries_into(attributes: &mut Map, dragged: &[Option<i64>; 3]) {
    for (attribute, boundary) in reading::BOUNDARY_ATTRIBUTES.iter().zip(dragged) {
        attributes.insert(*attribute, boundary.map_or(Value::Null, Value::Int));
    }
}

/// What a trend's span set to `start` and `stop` writes, its dragged boundaries scaled with it and kept a month
/// apart, or why it may not (`SetGhgSpanCommandHandler`).
pub fn span(trend: &Trend, start: i64, stop: i64) -> Result<Map, String> {
    let Some(old) = trend.span() else {
        return Err(NO_SPAN.to_owned());
    };
    if stop <= start {
        return Err("A trend must stop after it starts, at least one month later.".to_owned());
    }
    if let Some(refusal) = phases::too_short(stop - start, trend.visible_phases()) {
        return Err(refusal);
    }
    let mut attributes: Map = [("start", Value::Int(start)), ("stop", Value::Int(stop))]
        .into_iter()
        .collect();
    boundaries_into(
        &mut attributes,
        &phases::rescaled(&trend.dragged, old, (start, stop), trend.visible_phases()),
    );
    Ok(attributes)
}

/// What a drag of one of `element`'s edges writes, or why it may not.
pub fn resize(element: &DislElement, resize: &Resize) -> Option<Result<ModulePlacement, String>> {
    let unit = unit_of(element);
    let (old_x, old_y) = position(element)?;
    if element.is_a("Note") {
        let note = Note::of(element);
        let at = scale::nearest_month_at(resize.x, unit);
        let row = scale::row_at_top(resize.y);
        let mut attributes = Map::default();
        if note.at != Some(at) {
            attributes.insert("at", Value::Int(at));
        }
        if note.row != row {
            attributes.insert("row", Value::Int(row));
        }
        attributes.insert("width", Value::Double(resize.width));
        attributes.insert("height", Value::Double(resize.height));
        return Some(Ok(ModulePlacement {
            attributes,
            old_bounds: bounds(old_x, old_y, None),
            new_bounds: bounds(resize.x, resize.y, None),
        }));
    }
    if !element.is_a("Trend") || !resize.edge.is_horizontal() {
        return Some(Err(NOT_RESIZED.to_owned()));
    }
    let trend = Trend::of(element);
    let Some((start, stop)) = trend.span() else {
        return Some(Err(NO_SPAN.to_owned()));
    };
    let (new_start, new_stop) = match resize.edge {
        ResizedEdge::Left => (scale::nearest_month_at(resize.x, unit), stop),
        _ => (start, scale::nearest_month_at(resize.x + resize.width, unit)),
    };
    Some(span(&trend, new_start, new_stop).map(|attributes| ModulePlacement {
        attributes,
        old_bounds: bounds(old_x, old_y, Some(old_x + scale::width_of(stop - start, unit))),
        new_bounds: bounds(scale::x_of(new_start, unit), old_y, Some(scale::x_of(new_stop, unit))),
    }))
}

/// A point on the canvas as an addition there reads it, in a diagram drawn in `unit`: the month index and the row,
/// both fractional (`GhgDefinition.Position`).
pub fn position_at(x: f64, y: f64, unit: TimeUnit) -> Map {
    let (month, row) = scale::position(x, y, unit);
    [("x", Value::Double(month)), ("y", Value::Double(row))]
        .into_iter()
        .collect()
}
