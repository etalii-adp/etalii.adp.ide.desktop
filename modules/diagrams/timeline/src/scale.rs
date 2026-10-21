//! The only conversions between a time and a horizontal coordinate, and between a row and a vertical one (the
//! standalone host's `TimelineScale.cs` and `TimelineRows.cs` at `8b6ccfc`; `timeline.dis`, `coordinates.axes`).
//!
//! The timeline's x axis is seconds since the Unix epoch, as a double, and its y axis 60 units per row: a canvas
//! layers its zoom on top, and what a gesture hands over is a time and a row, not a pixel. Nothing else in the module
//! does this arithmetic.

use crate::instants::{self, Precision, TICKS_PER_DAY, TICKS_PER_SECOND};

/// The vertical distance between adjacent rows, in the timeline's own y units (`coordinates.axes.rows.scale`).
pub const ROW_HEIGHT: f64 = 60.0;

/// The x of an instant: seconds since the epoch, fractional where the value has them.
pub fn to_seconds(ticks: i64) -> f64 {
    ticks as f64 / TICKS_PER_SECOND as f64
}

/// The instant of an x, rounded to the whole second, the format's finest unit, as .NET's `Math.Round` rounds (half
/// to even); a date snaps to the start of its day, so a drag of a date-only element lands on a date.
pub fn to_time(seconds: f64, precision: Precision) -> i64 {
    let ticks = seconds.round_ties_even() as i64 * TICKS_PER_SECOND;
    match precision {
        Precision::Date => ticks.div_euclid(TICKS_PER_DAY) * TICKS_PER_DAY,
        Precision::DateTime => ticks,
    }
}

/// An instant as document text, in the shape its precision dictates: `yyyy-MM-dd`, or `yyyy-MM-ddTHH:mm:ss`.
pub fn to_text(ticks: i64, precision: Precision) -> String {
    let (year, month, day) = instants::civil_from_days(ticks.div_euclid(TICKS_PER_DAY));
    let date = format!("{year:04}-{month:02}-{day:02}");
    match precision {
        Precision::Date => date,
        Precision::DateTime => {
            let seconds = ticks.rem_euclid(TICKS_PER_DAY) / TICKS_PER_SECOND;
            format!(
                "{date}T{:02}:{:02}:{:02}",
                seconds / 3600,
                seconds / 60 % 60,
                seconds % 60
            )
        }
    }
}

/// The y of a row.
pub fn to_y(row: i64) -> f64 {
    row as f64 * ROW_HEIGHT
}

/// The row nearest a y, halves away from zero (the standalone host's `RowRounding.ToNearestRow`).
pub fn to_nearest_row(y: f64) -> i64 {
    (y / ROW_HEIGHT).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_date_round_trips_through_its_seconds() {
        let ticks = instants::parse("2026-01-05").unwrap();
        assert_eq!(
            to_text(to_time(to_seconds(ticks), Precision::Date), Precision::Date),
            "2026-01-05"
        );
    }

    #[test]
    fn a_date_snaps_to_the_start_of_its_day() {
        let noon = instants::parse("2026-01-05T12:00:00").unwrap();
        assert_eq!(
            to_text(to_time(to_seconds(noon), Precision::Date), Precision::Date),
            "2026-01-05"
        );
    }

    #[test]
    fn a_date_time_keeps_whole_seconds() {
        let ticks = instants::parse("2026-01-05T14:30:07").unwrap();
        assert_eq!(
            to_text(
                to_time(to_seconds(ticks) + 0.4, Precision::DateTime),
                Precision::DateTime
            ),
            "2026-01-05T14:30:07"
        );
    }

    #[test]
    fn a_row_is_sixty_units_and_halves_round_away_from_zero() {
        assert_eq!(to_y(3), 180.0);
        assert_eq!(to_nearest_row(89.0), 1);
        assert_eq!(to_nearest_row(90.0), 2);
        assert_eq!(to_nearest_row(-90.0), -2);
        assert_eq!(to_nearest_row(-89.0), -1);
    }
}
