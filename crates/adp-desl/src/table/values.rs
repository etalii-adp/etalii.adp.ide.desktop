//! How a value in its written form is read as what its type says it is: a number as .NET's invariant
//! `double.TryParse` reads one with `NumberStyles.Float`, a date, a date and time or a time as a point to compare,
//! and text compared ignoring case as `StringComparison.OrdinalIgnoreCase` compares it - the readings the standalone
//! host's `KnowledgeViewEngine` and `KnowledgeCalculations` make, so the two hosts order and filter alike.

use std::cmp::Ordering;

use crate::loading::specification::Editor;

/// Ticks (100 nanoseconds) in a second, as .NET counts them.
const TICKS_PER_SECOND: i64 = 10_000_000;

/// A number in its written form, as `double.TryParse(text, NumberStyles.Float, CultureInfo.InvariantCulture)` reads
/// it: white space around it, a sign, digits with one decimal point, an exponent, or the invariant culture's `NaN`,
/// `Infinity` and `-Infinity`. None for anything else, thousands separators included.
pub fn parse_number(text: &str) -> Option<f64> {
    let trimmed = text.trim();
    match trimmed {
        "NaN" => return Some(f64::NAN),
        "Infinity" | "+Infinity" => return Some(f64::INFINITY),
        "-Infinity" => return Some(f64::NEG_INFINITY),
        _ => {}
    }
    let body = trimmed.strip_prefix(['+', '-']).unwrap_or(trimmed);
    let (mantissa, exponent) = match body.find(['e', 'E']) {
        Some(at) => (&body[..at], Some(&body[at + 1..])),
        None => (body, None),
    };
    let digits = mantissa.chars().filter(char::is_ascii_digit).count();
    let points = mantissa.chars().filter(|c| *c == '.').count();
    if digits == 0 || points > 1 || mantissa.chars().any(|c| !c.is_ascii_digit() && c != '.') {
        return None;
    }
    if let Some(exponent) = exponent {
        let exponent = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
        if exponent.is_empty() || !exponent.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
    }
    trimmed.parse::<f64>().ok()
}

/// .NET's `double.CompareTo`: NaN before every number and equal to itself.
pub fn compare_numbers(left: f64, right: f64) -> Ordering {
    match (left.is_nan(), right.is_nan()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        _ => left.partial_cmp(&right).unwrap_or(Ordering::Equal),
    }
}

fn folded(text: &str) -> impl Iterator<Item = char> + '_ {
    text.chars().flat_map(char::to_uppercase)
}

/// `string.Compare(left, right, StringComparison.OrdinalIgnoreCase)`: by code point once both are upper-cased.
pub fn compare_ignoring_case(left: &str, right: &str) -> Ordering {
    folded(left).cmp(folded(right))
}

pub fn equals_ignoring_case(left: &str, right: &str) -> bool {
    compare_ignoring_case(left, right) == Ordering::Equal
}

fn upper(text: &str) -> String {
    folded(text).collect()
}

pub fn contains_ignoring_case(text: &str, sought: &str) -> bool {
    upper(text).contains(&upper(sought))
}

pub fn starts_with_ignoring_case(text: &str, sought: &str) -> bool {
    upper(text).starts_with(&upper(sought))
}

pub fn ends_with_ignoring_case(text: &str, sought: &str) -> bool {
    upper(text).ends_with(&upper(sought))
}

/// The day number of a date of the proleptic Gregorian calendar: days since 0001-01-01, as `DateOnly.DayNumber`.
pub fn day_number(year: i64, month: i64, day: i64) -> i64 {
    // Howard Hinnant's days_from_civil, moved from 1970-01-01 to 0001-01-01.
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468 + 719_162
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        _ => 28,
    }
}

fn digits(text: &str, count: usize) -> Option<i64> {
    (text.len() == count && text.chars().all(|c| c.is_ascii_digit()))
        .then(|| text.parse().ok())
        .flatten()
}

/// `yyyy-MM-dd`, exactly, as `DateOnly.TryParseExact` reads it: the day number.
pub fn parse_date(text: &str) -> Option<i64> {
    let mut parts = text.split('-');
    let (year, month, day) = (
        digits(parts.next()?, 4)?,
        digits(parts.next()?, 2)?,
        digits(parts.next()?, 2)?,
    );
    if parts.next().is_some() || year < 1 || !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
        return None;
    }
    Some(day_number(year, month, day))
}

/// A time of day, `H:mm`, `H:mm:ss` or `H:mm:ss.fffffff`, as ticks since midnight (`TimeOnly.Ticks`).
pub fn parse_time(text: &str) -> Option<i64> {
    let text = text.trim();
    let mut parts = text.split(':');
    let hours = parts.next()?;
    let minutes = parts.next()?;
    let seconds = parts.next();
    if parts.next().is_some() || hours.is_empty() || hours.len() > 2 || !hours.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let hours: i64 = hours.parse().ok()?;
    let minutes = digits(minutes, 2)?;
    let (seconds, fraction) = match seconds {
        None => (0, 0),
        Some(s) => {
            let (whole, fraction) = s.split_once('.').unwrap_or((s, ""));
            let whole = digits(whole, 2)?;
            if fraction.len() > 7 || !fraction.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
            let fraction = if fraction.is_empty() {
                0
            } else {
                format!("{fraction:0<7}").parse::<i64>().ok()?
            };
            (whole, fraction)
        }
    };
    if hours > 23 || minutes > 59 || seconds > 59 {
        return None;
    }
    Some((hours * 3600 + minutes * 60 + seconds) * TICKS_PER_SECOND + fraction)
}

/// A date and time of ISO 8601, as the instant it is, in ticks since 0001-01-01 in UTC: `yyyy-MM-dd`, a `T` or a
/// space, a time, and `Z` or an offset. A value without an offset is read as UTC: the standalone host's
/// `DateTimeOffset.TryParse` assumes the machine's own zone there, which would make two machines order one file
/// differently.
pub fn parse_date_time(text: &str) -> Option<i64> {
    let text = text.trim();
    let (date, rest) = match text.find(['T', 't', ' ']) {
        Some(at) => (&text[..at], Some(&text[at + 1..])),
        None => (text, None),
    };
    let days = parse_date(date)?;
    let Some(rest) = rest else {
        return Some(days * 86_400 * TICKS_PER_SECOND);
    };
    let (time, offset) = if let Some(time) = rest.strip_suffix(['Z', 'z']) {
        (time, 0)
    } else if let Some(at) = rest.rfind(['+', '-']) {
        let (sign, offset) = (if &rest[at..at + 1] == "-" { -1 } else { 1 }, &rest[at + 1..]);
        let (hours, minutes) = offset.split_once(':').unwrap_or((offset, "00"));
        let (hours, minutes) = (digits(hours, 2)?, digits(minutes, 2)?);
        (&rest[..at], sign * (hours * 3600 + minutes * 60) * TICKS_PER_SECOND)
    } else {
        (rest, 0)
    };
    let time = parse_time(time)?;
    Some(days * 86_400 * TICKS_PER_SECOND + time - offset)
}

/// A date, a date and time or a time as a point to compare: the day number of a date, the ticks of the instant in
/// UTC, or the ticks of the time of day (`KnowledgeViewEngine.Moment`). None for what is not one.
pub fn moment(editor: Editor, value: &str) -> Option<i64> {
    match editor {
        Editor::Date => parse_date(value),
        Editor::DateTime => parse_date_time(value),
        Editor::Time => parse_time(value),
        _ => None,
    }
}

/// Ticks in a day: what a date's day numbers are multiplied by to compare with ticks.
pub const TICKS_PER_DAY: i64 = 86_400 * TICKS_PER_SECOND;
