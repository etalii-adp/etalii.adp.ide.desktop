//! The time-to-x and row-to-y conversions, the module's most error-prone surface (the standalone host's
//! `TimelineScaleTests` and `TimelineRowsTests` at `8b6ccfc`).
//!
//! The round trips run over a spread of instants rather than one convenient sample, because the mistakes this pair
//! can make (an offset applied twice, a truncation instead of a rounding, a date promoted to a date-time) each survive
//! a single well-chosen example and fail somewhere else in the range. The rows are tested hardest at the midpoint
//! between two rows, where an off-by-one is the mistake to expect.

use adp_diagram_timeline::instants::{self, Precision};
use adp_diagram_timeline::scale::{self, ROW_HEIGHT};

const ACROSS_THE_RANGE: [(&str, Precision); 7] = [
    ("1970-01-01T00:00:00", Precision::DateTime),
    ("1969-07-20T20:17:40", Precision::DateTime),
    ("2026-01-05", Precision::Date),
    ("2026-02-16T14:00:00", Precision::DateTime),
    ("2026-12-31T23:59:59", Precision::DateTime),
    ("2100-06-15", Precision::Date),
    ("1900-03-01", Precision::Date),
];

#[test]
fn an_instant_round_trips_through_seconds() {
    for (text, precision) in ACROSS_THE_RANGE {
        let value = instants::parse(text).unwrap();

        let back = scale::to_time(scale::to_seconds(value), precision);

        assert_eq!(value, back, "{text}");
    }
}

#[test]
fn an_instant_round_trips_through_its_text() {
    for (text, precision) in ACROSS_THE_RANGE {
        let value = instants::parse(text).unwrap();

        assert_eq!(text, scale::to_text(value, precision));
    }
}

#[test]
fn the_epoch_is_zero() {
    assert_eq!(0.0, scale::to_seconds(instants::parse("1970-01-01T00:00:00Z").unwrap()));
}

#[test]
fn a_date_only_value_comes_back_date_only_after_a_drag() {
    let begin = instants::parse("2026-01-05").unwrap();
    let dragged = scale::to_seconds(begin) + 3.7 * 60.0 * 60.0;

    let landed = scale::to_time(dragged, Precision::Date);

    assert_eq!(0, landed % instants::TICKS_PER_DAY);
    assert_eq!("2026-01-05", scale::to_text(landed, Precision::Date));
}

#[test]
fn a_fractional_second_rounds_to_the_formats_finest_unit() {
    let value = instants::parse("2026-05-01T12:00:00").unwrap();

    let landed = scale::to_time(scale::to_seconds(value) + 0.4, Precision::DateTime);

    assert_eq!(value, landed);
}

#[test]
fn no_timezone_ever_appears() {
    let value = scale::to_time(
        scale::to_seconds(instants::parse("2026-06-15T12:00:00").unwrap()),
        Precision::DateTime,
    );

    let text = scale::to_text(value, Precision::DateTime);

    assert!(!text.contains('+') && !text.contains('Z'), "{text}");
}

#[test]
fn a_row_round_trips_through_y() {
    for row in [-12, -1, 0, 1, 3, 12] {
        assert_eq!(row, scale::to_nearest_row(scale::to_y(row)));
    }
}

#[test]
fn the_exact_midpoint_resolves_the_same_way_between_every_pair_of_rows() {
    for lower in [0, 1, 2, -1, -2] {
        // Banker's rounding would send the 0/1 midpoint to 0 but the 1/2 midpoint to 2.
        let midpoint = scale::to_y(lower) + ROW_HEIGHT / 2.0;

        let resolved = scale::to_nearest_row(midpoint);

        let expected = if lower >= 0 { lower + 1 } else { lower };
        assert_eq!(expected, resolved, "between {lower} and {}", lower + 1);
    }
}

#[test]
fn just_past_the_midpoint_snaps_to_the_far_row_and_just_short_snaps_back() {
    let midpoint = ROW_HEIGHT / 2.0;

    assert_eq!(1, scale::to_nearest_row(midpoint + 0.01));
    assert_eq!(0, scale::to_nearest_row(midpoint - 0.01));
}

#[test]
fn anywhere_within_a_rows_own_half_snaps_to_that_row() {
    assert_eq!(3, scale::to_nearest_row(scale::to_y(3) - ROW_HEIGHT * 0.49));
    assert_eq!(3, scale::to_nearest_row(scale::to_y(3) + ROW_HEIGHT * 0.49));
}

#[test]
fn a_negative_y_snaps_to_a_negative_row() {
    assert_eq!(-1, scale::to_nearest_row(-ROW_HEIGHT));
    assert_eq!(-1, scale::to_nearest_row(-ROW_HEIGHT * 0.6));
}
