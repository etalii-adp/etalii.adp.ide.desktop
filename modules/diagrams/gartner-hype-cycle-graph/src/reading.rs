//! A trend, a trigger, a note or an influence as the module's code reads it from the DISL model (the standalone
//! host's `GhgParser.cs` and the records of its `_Model` at `8b6ccfc`).
//!
//! Every value is read from what is stored, never from a default, so a date that is missing or does not read is
//! none, as the standalone host's parser left it; a row or a phase count that does not read is its fallback.

use adp_cel::Value;
use adp_disl::{DislDiagram, DislElement};

use crate::phases::COUNT;
use crate::scale::TimeUnit;

/// The host attribute holding the id an entry is written with (`gartner-hype-cycle-graph.fbl`, `storedId`).
pub const STORED_ID: &str = "storedId";

/// The attributes of a trend holding its stored inner boundaries, in boundary order.
pub const BOUNDARY_ATTRIBUTES: [&str; 3] = ["peakEnd", "troughEnd", "slopeEnd"];

/// One trend, as read.
#[derive(Debug, Clone, PartialEq)]
pub struct Trend {
    pub id: String,
    pub name: String,
    pub start: Option<i64>,
    pub stop: Option<i64>,
    pub row: i64,
    /// How many phases are visible, as written.
    pub phases: i64,
    /// The stored inner boundaries, one slot per boundary; none where it was never dragged.
    pub dragged: [Option<i64>; 3],
    /// The line it is written on, for the order of the arrangement's writes.
    pub line: i32,
}

impl Trend {
    pub fn of(element: &DislElement) -> Trend {
        Trend {
            id: written_id(element),
            name: text(element, "name").unwrap_or_default(),
            start: month(element, "start"),
            stop: month(element, "stop"),
            row: integer(element, "row", 0),
            phases: integer(element, "phases", COUNT),
            dragged: BOUNDARY_ATTRIBUTES.map(|attribute| month(element, attribute)),
            line: element.line().unwrap_or(i32::MAX),
        }
    }

    /// Both dates, when both read and the stop is after the start: whether it can be drawn at all.
    pub fn span(&self) -> Option<(i64, i64)> {
        match (self.start, self.stop) {
            (Some(start), Some(stop)) if stop > start => Some((start, stop)),
            _ => None,
        }
    }

    /// The span in months; zero without one.
    pub fn months(&self) -> i64 {
        self.span().map_or(0, |(start, stop)| stop - start)
    }

    /// The phase count clamped into 1 to 4, which is what is drawn.
    pub fn visible_phases(&self) -> i64 {
        self.phases.clamp(1, COUNT)
    }
}

/// One trigger, as read.
#[derive(Debug, Clone, PartialEq)]
pub struct Trigger {
    pub id: String,
    pub name: String,
    pub date: Option<i64>,
    pub row: i64,
    pub line: i32,
}

impl Trigger {
    pub fn of(element: &DislElement) -> Trigger {
        Trigger {
            id: written_id(element),
            name: text(element, "name").unwrap_or_default(),
            date: month(element, "date"),
            row: integer(element, "row", 0),
            line: element.line().unwrap_or(i32::MAX),
        }
    }
}

/// One note, as read.
#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub id: String,
    pub at: Option<i64>,
    pub row: i64,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub line: i32,
}

impl Note {
    pub fn of(element: &DislElement) -> Note {
        Note {
            id: written_id(element),
            at: month(element, "at"),
            row: integer(element, "row", 0),
            width: number(element, "width"),
            height: number(element, "height"),
            line: element.line().unwrap_or(i32::MAX),
        }
    }

    /// Its position and size, when it has a readable position and a positive size: whether it can be drawn.
    pub fn placed(&self) -> Option<(i64, f64, f64)> {
        match (self.at, self.width, self.height) {
            (Some(at), Some(width), Some(height)) if width > 0.0 && height > 0.0 => Some((at, width, height)),
            _ => None,
        }
    }
}

/// The diagram's time unit: its top-level `unit:`, the month for none or one it does not know.
pub fn unit_of(diagram: &DislDiagram) -> TimeUnit {
    match diagram.attributes().get("unit") {
        Some(Value::String(name)) => TimeUnit::named(name),
        _ => TimeUnit::Month,
    }
}

/// The id `element` is written with: the binding's `storedId`, else its model id.
pub fn written_id(element: &DislElement) -> String {
    match element.host_attributes().get(STORED_ID) {
        Some(Value::String(id)) => id.clone(),
        Some(_) => String::new(),
        None => element.id(),
    }
}

/// A stored scalar as its text; none when nothing is stored.
pub fn text(element: &DislElement, name: &str) -> Option<String> {
    match element.attributes().get(name)? {
        Value::String(text) => Some(text.clone()),
        Value::Int(i) => Some(i.to_string()),
        Value::Double(d) => Some(d.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Null => Some(String::new()),
        _ => None,
    }
}

/// A stored month; none when nothing is stored or it does not read.
pub fn month(element: &DislElement, name: &str) -> Option<i64> {
    match element.attributes().get(name)? {
        Value::Int(month) => Some(*month),
        Value::String(text) => crate::scale::parse_month(text),
        _ => None,
    }
}

/// A stored whole number, else `fallback`.
pub fn integer(element: &DislElement, name: &str, fallback: i64) -> i64 {
    match element.attributes().get(name) {
        Some(Value::Int(i)) => *i,
        Some(Value::String(text)) => text.trim().parse().unwrap_or(fallback),
        _ => fallback,
    }
}

/// A stored number; none when nothing is stored or it does not read.
pub fn number(element: &DislElement, name: &str) -> Option<f64> {
    match element.attributes().get(name)? {
        Value::Double(d) => Some(*d),
        Value::Int(i) => Some(*i as f64),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    }
}
