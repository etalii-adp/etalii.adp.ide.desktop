//! What the line under a column shows of the rows in sight: a count, a share, the ends of the dates or a figure
//! over the numbers, as the view's column names it in `calculate` (the standalone host's `KnowledgeCalculations` at
//! 8b6ccfc). A calculation that does not suit its property's type - left from a type the property had before, or
//! written by hand - shows nothing and is kept as written.

use std::collections::HashSet;

use adp_desl::table::content::ContentRow;
use adp_desl::table::engine::values_of;
use adp_desl::table::values::{self, TICKS_PER_DAY};

use crate::model::KnowledgeProperty;

const COUNTS: [&str; 7] = [
    "countAll",
    "countValues",
    "countUnique",
    "countEmpty",
    "countNotEmpty",
    "percentEmpty",
    "percentNotEmpty",
];
const DATES: [&str; 3] = ["earliest", "latest", "dateRange"];
const NUMBERS: [&str; 6] = ["sum", "average", "median", "min", "max", "range"];

/// The calculations a property of a type can have, in the order they are offered.
pub fn for_type(value_type: &str) -> Vec<&'static str> {
    match value_type {
        "number" => COUNTS.iter().chain(NUMBERS.iter()).copied().collect(),
        "date" | "dateTime" => COUNTS.iter().chain(DATES.iter()).copied().collect(),
        _ => COUNTS.to_vec(),
    }
}

/// Whether the name is a calculation at all, of whatever type.
pub fn is_known(calculation: &str) -> bool {
    COUNTS.contains(&calculation) || DATES.contains(&calculation) || NUMBERS.contains(&calculation)
}

/// `Math.Round(value, decimals)`: to the nearest, a tie to the even neighbour.
fn round(value: f64, decimals: i32) -> f64 {
    let scale = 10f64.powi(decimals);
    let scaled = value * scale;
    let floor = scaled.floor();
    let fraction = scaled - floor;
    let rounded = if (fraction - 0.5).abs() < f64::EPSILON * scaled.abs().max(1.0) {
        if floor % 2.0 == 0.0 { floor } else { floor + 1.0 }
    } else {
        scaled.round()
    };
    rounded / scale
}

/// A figure without the noise of its arithmetic: ten decimals at most, and none it does not need.
fn figure(value: f64) -> String {
    let text = format!("{:.10}", round(value, 10));
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" { "0".into() } else { text.to_owned() }
}

fn share(part: usize, all: usize) -> String {
    if all == 0 {
        return String::new();
    }
    format!("{}%", figure(round(100.0 * part as f64 / all as f64, 1)))
}

fn unit(amount: f64, unit: &str) -> String {
    format!(
        "{} {unit}{}",
        figure(amount),
        if (amount - 1.0).abs() < f64::EPSILON { "" } else { "s" }
    )
}

/// How long it is from the earliest to the latest, in the largest unit that says it whole enough: days, else hours,
/// else minutes.
fn span(ticks: i64) -> String {
    let days = ticks as f64 / TICKS_PER_DAY as f64;
    if days >= 1.0 {
        return unit(round(days, 1), "day");
    }
    let hours = days * 24.0;
    if hours >= 1.0 {
        unit(round(hours, 1), "hour")
    } else {
        unit(round(hours * 60.0, 0), "minute")
    }
}

/// What the calculation comes to over the rows, as the line shows it; empty where it does not suit the property.
pub fn of(property: &KnowledgeProperty, calculation: &str, rows: &[&ContentRow]) -> String {
    if !for_type(&property.value_type).contains(&calculation) {
        return String::new();
    }
    // A box that is not ticked holds nothing, as a cell without a value does.
    let held: Vec<Vec<&String>> = rows
        .iter()
        .map(|row| {
            values_of(row, property)
                .iter()
                .filter(|v| property.value_type != "checkbox" || v.as_str() == "true")
                .collect()
        })
        .collect();
    let filled = held.iter().filter(|h| !h.is_empty()).count();
    match calculation {
        "countAll" => return rows.len().to_string(),
        "countValues" => return held.iter().map(Vec::len).sum::<usize>().to_string(),
        "countUnique" => {
            let unique: HashSet<&String> = held.iter().flatten().copied().collect();
            return unique.len().to_string();
        }
        "countEmpty" => return (rows.len() - filled).to_string(),
        "countNotEmpty" => return filled.to_string(),
        "percentEmpty" => return share(rows.len() - filled, rows.len()),
        "percentNotEmpty" => return share(filled, rows.len()),
        _ => {}
    }
    if property.value_type == "number" {
        let mut numbers: Vec<f64> = held
            .iter()
            .flatten()
            .filter_map(|v| values::parse_number(v))
            .filter(|n| n.is_finite())
            .collect();
        if numbers.is_empty() {
            return String::new();
        }
        numbers.sort_by(|a, b| values::compare_numbers(*a, *b));
        let count = numbers.len();
        return figure(match calculation {
            "sum" => numbers.iter().sum(),
            "average" => numbers.iter().sum::<f64>() / count as f64,
            "median" if count % 2 == 1 => numbers[count / 2],
            "median" => (numbers[count / 2 - 1] + numbers[count / 2]) / 2.0,
            "min" => numbers[0],
            "max" => numbers[count - 1],
            _ => numbers[count - 1] - numbers[0],
        });
    }
    // The dates as written, with the point each is: the ends are shown as their author wrote them.
    let mut moments: Vec<(i64, &String)> = held
        .iter()
        .flatten()
        .filter_map(|v| values::moment(property.kind.editor, v).map(|at| (at, *v)))
        .collect();
    if moments.is_empty() {
        return String::new();
    }
    moments.sort_by_key(|(at, _)| *at);
    let (first, last) = (moments[0], moments[moments.len() - 1]);
    match calculation {
        "earliest" => first.1.clone(),
        "latest" => last.1.clone(),
        _ if property.value_type == "date" => span((last.0 - first.0) * TICKS_PER_DAY),
        _ => span(last.0 - first.0),
    }
}
