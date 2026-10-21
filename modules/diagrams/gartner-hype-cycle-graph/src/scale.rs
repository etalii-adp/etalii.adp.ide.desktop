//! The time scale and the rows, stated once: month to x, row to y, and the month dates the document writes (the
//! standalone host's `GhgScale.cs` and `_Model/GhgTimeUnit.cs` at `8b6ccfc`; `gartner-hype-cycle-graph.dis`,
//! `coordinates.axes.time` and `coordinates.axes.rows`).
//!
//! The horizontal axis is uniform in steps of the diagram's time unit, [`UNITS_PER_STEP`] canvas units each, from a
//! fixed origin of 1900-01, so x is a pure function of a date; dates before the origin are negative. A month is one
//! integer, `year × 12 + (month − 1)`, as DISL's `yearMonth` holds it, written back as `YYYY-MM`. A document may draw
//! in a coarser step, a year, a decade or a century, so that thousands of years fit on a canvas; a snap lands on the
//! start of a step. Rows are [`ROW_STEP`] units apart: a trend's height and a gutter for influences.

/// Canvas units per step of a diagram's time unit, whichever unit it is.
pub const UNITS_PER_STEP: f64 = 4.0;

/// Every trend's height, in canvas units.
pub const TREND_HEIGHT: f64 = 32.0;

/// A trigger's diameter: half a trend's height.
pub const TRIGGER_SIZE: f64 = TREND_HEIGHT / 2.0;

/// The distance between two rows: a trend's height plus a 24-unit gutter for influences.
pub const ROW_STEP: f64 = 56.0;

/// The month index of 1900-01, the date at x = 0.
pub const ORIGIN_MONTH: i64 = 1900 * 12;

/// The step a diagram's time axis is drawn and snapped in, which its document names in a top-level `unit:` key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeUnit {
    Month,
    Year,
    Decade,
    Century,
}

impl TimeUnit {
    /// The unit a document names, or the month for none it knows: an unknown value draws in months.
    pub fn named(name: &str) -> TimeUnit {
        match name.trim() {
            "year" => TimeUnit::Year,
            "decade" => TimeUnit::Decade,
            "century" => TimeUnit::Century,
            _ => TimeUnit::Month,
        }
    }

    /// How many months one step spans.
    pub fn months(self) -> i64 {
        match self {
            TimeUnit::Month => 1,
            TimeUnit::Year => 12,
            TimeUnit::Decade => 120,
            TimeUnit::Century => 1200,
        }
    }
}

/// The month index of `year` and `month` (1 to 12).
pub fn month_index(year: i64, month: i64) -> i64 {
    year * 12 + (month - 1)
}

/// A `YYYY-MM` date as a month index, or none when it is not one: a year before 1 is signed and astronomical, with
/// four to six digits after the sign (`ParseMonth`).
pub fn parse_month(text: &str) -> Option<i64> {
    let text = text.trim();
    let (sign, rest) = match text.strip_prefix('-') {
        Some(rest) => (-1, rest),
        None => (1, text),
    };
    let (year, month) = rest.split_once('-')?;
    let year_digits = if sign < 0 { 4..=6 } else { 4..=4 };
    if !year_digits.contains(&year.len())
        || month.len() != 2
        || !year.bytes().all(|b| b.is_ascii_digit())
        || !month.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let year: i64 = year.parse().ok()?;
    let month: i64 = month.parse().ok()?;
    (1..=12).contains(&month).then(|| month_index(sign * year, month))
}

/// A month index as the document writes it: `YYYY-MM`, or `-YYYY-MM` before year 0 (`FormatMonth`).
pub fn format_month(month_index: i64) -> String {
    let year = month_index.div_euclid(12);
    let month = month_index - year * 12;
    let sign = if year < 0 { "-" } else { "" };
    format!("{sign}{:04}-{:02}", year.abs(), month + 1)
}

/// The canvas x of the start of a month, in a diagram drawn in `unit`.
pub fn x_of(month_index: i64, unit: TimeUnit) -> f64 {
    (month_index - ORIGIN_MONTH) as f64 * UNITS_PER_STEP / unit.months() as f64
}

/// The start of the step nearest `x`, halves away from zero: what a move or a resize snaps to (`NearestMonthAt`).
pub fn nearest_month_at(x: f64, unit: TimeUnit) -> i64 {
    ORIGIN_MONTH + (x / UNITS_PER_STEP).round() as i64 * unit.months()
}

/// The start of the step `x` falls inside: what a drop at a mid-step x means (`MonthContaining`).
pub fn month_containing(x: f64, unit: TimeUnit) -> i64 {
    ORIGIN_MONTH + (x / UNITS_PER_STEP).floor() as i64 * unit.months()
}

/// The width of a span of months, in a diagram drawn in `unit`.
pub fn width_of(months: i64, unit: TimeUnit) -> f64 {
    months as f64 * UNITS_PER_STEP / unit.months() as f64
}

/// The top edge of a row.
pub fn top_of(row: i64) -> f64 {
    row as f64 * ROW_STEP
}

/// The row whose top edge is nearest `top`, halves away from zero on both sides of the origin (`RowAtTop`,
/// `RowRounding.ToNearestRow`).
pub fn row_at_top(top: f64) -> i64 {
    (top / ROW_STEP).round() as i64
}

/// The row whose vertical middle is nearest `y` (`RowAtMiddle`).
pub fn row_at_middle(y: f64) -> i64 {
    row_at_top(y - TREND_HEIGHT / 2.0)
}

/// A point on the canvas as the specification's `position` reads it (DISL section 12.3; `GhgDefinition.Position`):
/// `x` the month index on the time axis, fractional between month starts, and `y` the row, fractional between rows.
pub fn position(x: f64, y: f64, unit: TimeUnit) -> (f64, f64) {
    (
        ORIGIN_MONTH as f64 + x / UNITS_PER_STEP * unit.months() as f64,
        y / ROW_STEP,
    )
}

const SHORT_MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// A date as a trigger's label writes it, in the diagram's unit: `Dec 1947` in a diagram of months, the year alone,
/// unpadded, in one of years or coarser (`FormatWhen`).
pub fn format_when(month_index: i64, unit: TimeUnit) -> String {
    let year = month_index.div_euclid(12);
    if unit.months() > 1 {
        return year.to_string();
    }
    let month = (month_index - year * 12) as usize;
    format!("{} {year}", SHORT_MONTHS[month])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_month_round_trips_through_its_index() {
        for text in ["1947-12", "1900-01", "0000-01", "-3200-01", "2045-06"] {
            assert_eq!(format_month(parse_month(text).unwrap()), text);
        }
        assert_eq!(parse_month("1947-13"), None);
        assert_eq!(parse_month("47-12"), None);
        assert_eq!(parse_month("12345-01"), None);
        assert_eq!(format_month(-1), "-0001-12");
    }

    #[test]
    fn a_month_is_four_units_from_1900() {
        assert_eq!(x_of(parse_month("1901-01").unwrap(), TimeUnit::Month), 48.0);
        assert_eq!(x_of(parse_month("1899-12").unwrap(), TimeUnit::Month), -4.0);
        assert_eq!(x_of(parse_month("1910-01").unwrap(), TimeUnit::Decade), 4.0);
        assert_eq!(width_of(12, TimeUnit::Year), 4.0);
    }

    #[test]
    fn a_snap_rounds_halves_away_from_zero_and_a_drop_floors() {
        assert_eq!(format_month(nearest_month_at(1010.0, TimeUnit::Month)), "1921-02");
        assert_eq!(format_month(nearest_month_at(-2.0, TimeUnit::Month)), "1899-12");
        assert_eq!(format_month(month_containing(-2.0, TimeUnit::Month)), "1899-12");
        assert_eq!(format_month(month_containing(7.9, TimeUnit::Month)), "1900-02");
        assert_eq!(format_month(nearest_month_at(6.0, TimeUnit::Year)), "1902-01");
    }

    #[test]
    fn rows_are_fifty_six_units_and_halves_round_away_from_zero() {
        assert_eq!(row_at_top(28.0), 1);
        assert_eq!(row_at_top(-28.0), -1);
        assert_eq!(row_at_top(27.9), 0);
        assert_eq!(row_at_middle(56.0 + 16.0), 1);
    }

    #[test]
    fn a_trigger_date_is_written_in_the_units_precision() {
        assert_eq!(
            format_when(parse_month("1947-12").unwrap(), TimeUnit::Month),
            "Dec 1947"
        );
        assert_eq!(format_when(parse_month("1947-12").unwrap(), TimeUnit::Decade), "1947");
        assert_eq!(format_when(parse_month("-3200-01").unwrap(), TimeUnit::Year), "-3200");
    }
}
