//! The CEL functions of the timeline specification's `net.etalii.adp.generic.timelineTimes` plugin (DISL section
//! 13.1.1; the standalone host's `TimelineTimes.cs` at `8b6ccfc`), implemented with the reading the module reads and
//! writes times with, so a rule, a form check and an operation judge a time exactly as a gesture does.
//!
//! DISL's CEL has no timestamps, and the timeline keeps every time as the text it was written with; the
//! specification's fallbacks only approximate the reading, so the module always supplies these.

use std::time::{SystemTime, UNIX_EPOCH};

use adp_cel::Value;
use adp_disl::{DislElement, DislPluginFunctions};

use crate::instants::{self, Precision, TICKS_PER_DAY, TICKS_PER_SECOND};
use crate::scale;

/// The implementations, for loading the specification.
pub fn plugin_functions() -> DislPluginFunctions {
    DislPluginFunctions::none()
        .add("readableTime", |arguments| {
            Ok(Value::Bool(instants::parse(text(argument(arguments, 0)?)?).is_some()))
        })
        .add("compareTimes", |arguments| {
            let a = readable(argument(arguments, 0)?)?;
            let b = readable(argument(arguments, 1)?)?;
            Ok(Value::Int(a.cmp(&b) as i64))
        })
        .add("dateAfter", |arguments| {
            let time = readable(argument(arguments, 0)?)?;
            let Value::Int(days) = argument(arguments, 1)? else {
                return Err("dateAfter() takes a whole number of days.".to_owned());
            };
            let after = days
                .checked_mul(TICKS_PER_DAY)
                .and_then(|d| time.checked_add(d))
                .ok_or("dateAfter() goes beyond the calendar.")?;
            Ok(Value::String(scale::to_text(after, Precision::Date)))
        })
        .add("today", |_| Ok(Value::String(today())))
        .add("writtenEnd", |arguments| {
            let end = text(argument(arguments, 1)?)?;
            match DislElement::from_value(argument(arguments, 0)?) {
                Some(relation) if relation.is_relation() => Ok(Value::String(written_end(&relation, end))),
                _ => Err("writtenEnd() takes a relation of this timeline and 'source' or 'target'.".to_owned()),
            }
        })
}

/// Today's date in UTC, as `yyyy-MM-dd`.
pub fn today() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default();
    scale::to_text(seconds * TICKS_PER_SECOND, Precision::Date)
}

/// The id a relation's `source` or `target` end is written with: the end as it was read, else the element it
/// reaches; the empty text for none.
pub fn written_end(relation: &DislElement, end: &str) -> String {
    let (written, reached) = if end == "source" {
        (relation.source_id(), relation.source())
    } else {
        (relation.target_id(), relation.target())
    };
    written.or_else(|| reached.map(|e| e.id())).unwrap_or_default()
}

fn argument(arguments: &[Value], index: usize) -> Result<&Value, String> {
    arguments
        .get(index)
        .ok_or_else(|| "A timeline time function was called with too few arguments.".to_owned())
}

fn text(value: &Value) -> Result<&str, String> {
    value
        .as_str()
        .ok_or_else(|| "A timeline time function takes text.".to_owned())
}

/// The instant `value` names, or the sentence that says it does not read.
fn readable(value: &Value) -> Result<i64, String> {
    let text = text(value)?;
    instants::parse(text).ok_or_else(|| format!("'{text}' is not a time this timeline can read."))
}
