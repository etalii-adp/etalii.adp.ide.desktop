//! A timeline element as the module's code reads it from the DISL model: the id it is written with, its label, its
//! begin and, for a period, its end as written, and its row (the standalone host's `TimelineReading.cs` and the
//! `TimelineElement` of its `_Model` at `8b6ccfc`).
//!
//! Every time is the text it was written with, read at face value when it is needed; a row that does not read as a
//! whole number is row 0, as .NET's integer parse left it.

use adp_cel::Value;
use adp_disl::DislElement;

use crate::instants::{self, Precision};

/// The host attribute holding the id an element or relation is written with (`timeline.fbl`, `storedId`).
pub const STORED_ID: &str = "storedId";

/// One element of a timeline, as read.
#[derive(Debug, Clone, PartialEq)]
pub struct TimelineElement {
    /// The id it is written with; empty for none.
    pub id: String,
    pub label: String,
    pub begin: String,
    /// The end as written, for a period; none for a moment.
    pub end: Option<String>,
    pub row: i64,
}

impl TimelineElement {
    /// `element` as read.
    pub fn of(element: &DislElement) -> TimelineElement {
        TimelineElement {
            id: written_id(element),
            label: text_of(&element.value_of("label")),
            begin: text_of(&element.value_of("begin")),
            end: element.is_a("Period").then(|| text_of(&element.value_of("end"))),
            row: row_of(&element.value_of("row")),
        }
    }

    /// The begin, read; none when it does not read.
    pub fn begin_instant(&self) -> Option<i64> {
        instants::parse(&self.begin)
    }

    /// The end, read; none for a moment or an end that does not read.
    pub fn end_instant(&self) -> Option<i64> {
        self.end.as_deref().and_then(instants::parse)
    }

    pub fn begin_precision(&self) -> Precision {
        Precision::of(&self.begin)
    }

    /// What the canvas writes on it: its label, or its id when the label is empty (`shownLabel`).
    pub fn shown_label(&self) -> &str {
        if self.label.is_empty() { &self.id } else { &self.label }
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

/// A value as the text it was written with.
pub fn text_of(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Int(i) => i.to_string(),
        Value::Double(d) => d.to_string(),
        Value::Bool(b) => if *b { "True" } else { "False" }.to_owned(),
        _ => String::new(),
    }
}

/// A row: a whole number, else 0.
pub fn row_of(value: &Value) -> i64 {
    match value {
        Value::Int(i) => *i,
        Value::String(text) => text.trim().parse::<i32>().map(i64::from).unwrap_or(0),
        _ => 0,
    }
}
