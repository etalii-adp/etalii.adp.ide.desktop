//! A value as a body's family read it, in its written form (the standalone host's `KnowledgeValues` at 8b6ccfc).
//! YAML and JSON give numbers and booleans as such and XML gives text for everything, so everything that compares
//! or shows a value goes through here and the three formats cannot differ in what a table shows.

use adp_cel::Value;
use adp_fbl::planning::dotnet_double;

/// The value as text: a boolean as `true` or `false`, a number in its invariant form, nothing as empty, a list as
/// its first item (a reference read as a list of one, as some families give it).
pub fn text(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(text)) => text.clone(),
        Some(Value::Bool(flag)) => if *flag { "true" } else { "false" }.into(),
        Some(Value::Int(whole)) => whole.to_string(),
        Some(Value::Double(real)) => dotnet_double(*real),
        Some(Value::List(items)) => text(items.first()),
        Some(other) => other.to_string(),
    }
}

/// Whether the value is the boolean true, however its family wrote it.
pub fn is_true(value: Option<&Value>) -> bool {
    matches!(value, Some(Value::Bool(true))) || matches!(value, Some(Value::String(s)) if s == "true")
}

/// The value as a whole number, or none when it is absent or is not one.
pub fn whole(value: Option<&Value>) -> Option<i64> {
    match value? {
        Value::Int(whole) => i32::try_from(*whole).ok().map(i64::from),
        Value::Double(real) if real.fract() == 0.0 && *real >= f64::from(i32::MIN) && *real <= f64::from(i32::MAX) => {
            Some(*real as i64)
        }
        Value::String(text) => text.trim().parse::<i32>().ok().map(i64::from),
        _ => None,
    }
}
