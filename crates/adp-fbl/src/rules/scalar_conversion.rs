//! The conversion of a value to the type its specification gives the attribute (FBL 0.5 sections 5.2 and 7.4; the
//! standalone host's `Rules/ScalarConversion`). The type names are the specification languages' scalar types; a
//! type this module does not know, an element type among them, is not checked and the value is kept as it is.

use adp_cel::Value;

use crate::families::yaml::scalars;

/// The value a slot reads as `type_name`, or none when it does not convert. `style` is the style name of a written
/// scalar of a yaml or json body (FBL 0.5 section 2.4), and none for a value that is no such scalar: text from the
/// xml, lines and blocks families, which converts from that text as before FBL 0.5 and is kept as it is, or a
/// collection. A quoted scalar converts to a number or a bool only when `quoted` asks for it, and then from its
/// `written` text by the core schema.
pub(crate) fn convert(
    type_name: &str,
    typed: &Value,
    written: &str,
    style: Option<&str>,
    quoted: bool,
) -> Option<Value> {
    if typed.is_null() {
        return Some(Value::Null);
    }
    let Some(style) = style else {
        return reads(type_name, typed).then(|| typed.clone());
    };
    let mut typed = typed.clone();
    if matches!(style, "single" | "double") && matches!(type_name, "int" | "number" | "bool") {
        if !quoted {
            return None;
        }
        typed = scalars::typed(written);
    }
    match type_name {
        "int" => match typed {
            Value::Int(i) => Some(Value::Int(i)),
            Value::Double(d) if d.fract() == 0.0 && d >= i64::MIN as f64 && d < i64::MAX as f64 => {
                Some(Value::Int(d as i64))
            }
            _ => None,
        },
        "number" => matches!(typed, Value::Int(_) | Value::Double(_)).then_some(typed),
        "bool" => matches!(typed, Value::Bool(_)).then_some(typed),
        _ => reads(type_name, &typed).then_some(typed),
    }
}

/// A text of the `grammar` family read as the type the specification gives its attribute (FBL 0.9 section 5.2), which
/// [`convert`] has said it converts to: a value of any other type is kept as it is.
pub(crate) fn from_text(type_name: &str, value: Value) -> Value {
    let Some(text) = value.as_str().map(str::trim) else {
        return value;
    };
    let typed = match type_name {
        "int" => text.parse().ok().map(Value::Int),
        "number" => text.parse().ok().map(Value::Double),
        "bool" => text.parse().ok().map(Value::Bool),
        _ => None,
    };
    typed.unwrap_or(value)
}

/// What a value of each type is called in a finding: "'wide' is not a whole number".
pub(crate) fn describe(type_name: &str) -> String {
    match type_name {
        "int" => "a whole number".into(),
        "number" => "a number".into(),
        "bool" => "true or false".into(),
        "date" => "a date".into(),
        "datetime" => "a date and time".into(),
        "time" => "a time".into(),
        other => format!("a {other}"),
    }
}

/// Whether a value converts to `type_name` as it is: typed from a tree, text from the other families.
fn reads(type_name: &str, value: &Value) -> bool {
    match type_name {
        "number" => match value {
            Value::Int(_) | Value::Double(_) => true,
            Value::String(text) => is_number(text),
            _ => false,
        },
        "bool" => matches!(value, Value::Bool(_)) || matches!(value.as_str(), Some("true" | "false")),
        "date" => value.as_str().is_some_and(is_date),
        "datetime" => value.as_str().is_some_and(is_date_time),
        "time" => value.as_str().is_some_and(is_time),
        _ => true,
    }
}

/// .NET's `double.TryParse` with `NumberStyles.Float`, and finite.
fn is_number(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.is_empty()
        && !trimmed.contains(['_', ','])
        && !trimmed.starts_with("0x")
        && trimmed.parse::<f64>().is_ok_and(f64::is_finite)
        && !trimmed.eq_ignore_ascii_case("inf")
        && !trimmed.eq_ignore_ascii_case("infinity")
        && !trimmed.eq_ignore_ascii_case("nan")
}

/// `yyyy-MM-dd`.
pub(crate) fn is_date(text: &str) -> bool {
    let b = text.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let digits = |r: std::ops::Range<usize>| b[r].iter().all(u8::is_ascii_digit);
    if !digits(0..4) || !digits(5..7) || !digits(8..10) {
        return false;
    }
    let year: i32 = text[0..4].parse().unwrap_or(0);
    let month: u32 = text[5..7].parse().unwrap_or(0);
    let day: u32 = text[8..10].parse().unwrap_or(0);
    year >= 1 && (1..=12).contains(&month) && day >= 1 && day <= days_in_month(year, month)
}

pub(crate) fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        _ => 28,
    }
}

/// A date and a time, as .NET's round-trip parse reads one: `yyyy-MM-dd`, then `T` or a space, the time, and an
/// optional offset.
fn is_date_time(text: &str) -> bool {
    if text.len() == 10 {
        return is_date(text);
    }
    if text.len() < 16 || !is_date(&text[..10]) || !matches!(text.as_bytes()[10], b'T' | b't' | b' ') {
        return false;
    }
    let rest = &text[11..];
    let (time, offset) = match rest
        .find(['Z', 'z', '+'])
        .or_else(|| rest[1..].find('-').map(|i| i + 1))
    {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    is_time(time) && (offset.is_empty() || matches!(offset, "Z" | "z") || is_offset(offset))
}

fn is_offset(offset: &str) -> bool {
    let b = offset.as_bytes();
    b.len() == 6
        && matches!(b[0], b'+' | b'-')
        && b[3] == b':'
        && b[1..3].iter().chain(&b[4..6]).all(u8::is_ascii_digit)
}

/// `HH:mm`, `HH:mm:ss` or `HH:mm:ss.fffffff`.
fn is_time(text: &str) -> bool {
    let parts: Vec<&str> = text.split(':').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return false;
    }
    let number = |p: &str, max: u32| {
        p.len() <= 2
            && !p.is_empty()
            && p.bytes().all(|b| b.is_ascii_digit())
            && p.parse::<u32>().is_ok_and(|n| n <= max)
    };
    if !number(parts[0], 23) || !number(parts[1], 59) {
        return false;
    }
    if let Some(seconds) = parts.get(2) {
        let (whole, fraction) = seconds.split_once('.').unwrap_or((seconds, "0"));
        return number(whole, 59) && !fraction.is_empty() && fraction.bytes().all(|b| b.is_ascii_digit());
    }
    true
}
