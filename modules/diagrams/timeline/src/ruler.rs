//! The time ruler's ladder: which time boundaries deserve a label for a span in view, and where they fall (the
//! standalone host's `client/timelineTicks.ts` at `8b6ccfc`).
//!
//! A pure function of the range in view and its width in pixels: labels are at least 80 pixels apart, on round
//! boundaries of the finest step that keeps them so, in UTC, as the module reads every time. Up to a week the steps
//! are fixed durations; beyond it the calendar takes over, the first of a month, of a quarter, or of a year.

use crate::instants::{civil_from_days, days_from_civil};

/// One labelled boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct Tick {
    /// Where it falls, in seconds since the epoch.
    pub seconds: f64,
    pub label: String,
}

const MINUTE: f64 = 60.0;
const HOUR: f64 = 3_600.0;
const DAY: f64 = 86_400.0;
const WEEK: f64 = 7.0 * DAY;

/// The pixels a labelled tick needs, or the labels collide (`MIN_LABEL_SPACING_PX`).
const MIN_LABEL_SPACING: f64 = 80.0;

/// The fixed-duration rungs (`FIXED_STEPS`).
const FIXED_STEPS: [f64; 9] = [
    1.0,
    15.0,
    MINUTE,
    5.0 * MINUTE,
    15.0 * MINUTE,
    HOUR,
    6.0 * HOUR,
    DAY,
    WEEK,
];

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// The labelled boundaries between `start` and `end`, seconds since the epoch, across `width` pixels (`ticksFor`).
pub fn ticks(start: f64, end: f64, width: f64) -> Vec<Tick> {
    if end <= start || width <= 0.0 || !start.is_finite() || !end.is_finite() {
        return Vec::new();
    }
    let span = end - start;
    let most = (width / MIN_LABEL_SPACING).floor().max(1.0);
    let coarsest = span / most;
    if let Some(step) = FIXED_STEPS.into_iter().find(|step| *step >= coarsest) {
        return fixed(start, end, step);
    }
    if coarsest <= 31.0 * DAY {
        return months(start, end, 1);
    }
    if coarsest <= 93.0 * DAY {
        return months(start, end, 3);
    }
    let years = (coarsest / (366.0 * DAY)).ceil().max(1.0) as i64;
    years_of(start, end, years)
}

/// Year, month (0 to 11), day, hours, minutes and seconds of a time, in UTC.
fn civil(seconds: f64) -> (i64, u32, u32, i64, i64, i64) {
    let whole = seconds.floor() as i64;
    let days = whole.div_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let of_day = whole.rem_euclid(86_400);
    (year, month - 1, day, of_day / 3_600, of_day / 60 % 60, of_day % 60)
}

fn utc(year: i64, month: u32) -> f64 {
    days_from_civil(year, month + 1, 1) as f64 * DAY
}

fn fixed(start: f64, end: f64, step: f64) -> Vec<Tick> {
    let mut ticks = Vec::new();
    let mut at = (start / step).ceil() * step;
    while at <= end {
        ticks.push(Tick {
            seconds: at,
            label: label_for(at, step),
        });
        at += step;
    }
    ticks
}

fn months(start: f64, end: f64, per_tick: u32) -> Vec<Tick> {
    let mut ticks = Vec::new();
    let (mut year, first, ..) = civil(start);
    let mut month = first.div_ceil(per_tick) * per_tick;
    if month >= 12 {
        month -= 12;
        year += 1;
    }
    loop {
        let at = utc(year, month);
        if at > end {
            return ticks;
        }
        if at >= start {
            ticks.push(Tick {
                seconds: at,
                label: if month == 0 {
                    year.to_string()
                } else {
                    format!("{} {year}", MONTHS[month as usize])
                },
            });
        }
        month += per_tick;
        if month >= 12 {
            month -= 12;
            year += 1;
        }
    }
}

fn years_of(start: f64, end: f64, per_tick: i64) -> Vec<Tick> {
    let mut ticks = Vec::new();
    let (first, ..) = civil(start);
    let mut year = (first as f64 / per_tick as f64).ceil() as i64 * per_tick;
    loop {
        let at = utc(year, 0);
        if at > end {
            return ticks;
        }
        if at >= start {
            ticks.push(Tick {
                seconds: at,
                label: year.to_string(),
            });
        }
        year += per_tick;
    }
}

fn label_for(seconds: f64, step: f64) -> String {
    let (year, month, day, hours, minutes, secs) = civil(seconds);
    if step >= DAY {
        return if day == 1 {
            format!("{} {year}", MONTHS[month as usize])
        } else {
            format!("{} {day}", MONTHS[month as usize])
        };
    }
    if step >= MINUTE {
        return format!("{hours:02}:{minutes:02}");
    }
    format!("{hours:02}:{minutes:02}:{secs:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> f64 {
        crate::scale::to_seconds(crate::instants::parse(text).unwrap())
    }

    #[test]
    fn two_years_across_a_wide_ruler_are_labelled_by_quarter_on_the_first_of_the_month() {
        let ticks = ticks(at("2024-11-21"), at("2026-12-17"), 1200.0);
        let labels: Vec<&str> = ticks.iter().map(|t| t.label.as_str()).collect();
        assert_eq!(labels.first(), Some(&"2025"));
        assert!(labels.contains(&"Apr 2025") && labels.contains(&"2026"), "{labels:?}");
        assert_eq!(ticks[0].seconds, at("2025-01-01"));
    }

    #[test]
    fn a_few_days_are_labelled_by_the_day_and_a_few_hours_by_the_hour() {
        let days: Vec<String> = ticks(at("2026-01-05"), at("2026-01-09"), 800.0)
            .into_iter()
            .map(|t| t.label)
            .collect();
        assert_eq!(days, ["Jan 5", "Jan 6", "Jan 7", "Jan 8", "Jan 9"]);
        let hours = ticks(at("2026-01-05T09:00:00"), at("2026-01-05T12:00:00"), 400.0);
        assert_eq!(hours[0].label, "09:00");
    }

    #[test]
    fn an_empty_range_or_ruler_has_no_ticks() {
        assert!(ticks(10.0, 10.0, 800.0).is_empty());
        assert!(ticks(0.0, 10.0, 0.0).is_empty());
    }
}
