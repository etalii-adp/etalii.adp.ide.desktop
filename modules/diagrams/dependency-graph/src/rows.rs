//! The one conversion between a row and a vertical coordinate, and the nearest-row inverse a drag snaps through (the
//! standalone host's `DependencyGraphRows.cs` at `8b6ccfc`, and `RowRounding.ToNearestRow`, which it shares with
//! every row canvas).
//!
//! A row is an integer the author owns: negative is as valid as positive, and gaps are the author's business. The
//! height is a rendering constant, not data: it appears in a node's y only as `row × HEIGHT`.

/// The vertical distance between adjacent rows, in canvas units (`DependencyGraphRows.Height`).
pub const HEIGHT: f64 = 60.0;

/// The y of `row`'s top edge.
pub fn to_y(row: i64) -> f64 {
    row as f64 * HEIGHT
}

/// The row nearest `y`, which a vertical drag snaps to on release: halves round away from zero, so the midpoint
/// between two rows resolves the same way on every row and on both sides of the origin.
pub fn to_nearest_row(y: f64) -> i64 {
    (y / HEIGHT).round() as i64
}

#[cfg(test)]
mod tests {
    //! `DependencyGraphRows.Tests.cs` at `8b6ccfc`: the row-to-y pair, tested hardest at the midpoint between two rows.

    use super::*;

    #[test]
    fn a_row_round_trips_through_y() {
        for row in [-12, -1, 0, 1, 3, 12] {
            assert_eq!(to_nearest_row(to_y(row)), row);
        }
    }

    #[test]
    fn the_exact_midpoint_resolves_the_same_way_between_every_pair_of_rows() {
        for lower in [0, 1, 2, -1, -2] {
            let midpoint = to_y(lower) + HEIGHT / 2.0;
            let expected = if lower >= 0 { lower + 1 } else { lower };
            assert_eq!(to_nearest_row(midpoint), expected, "between {lower} and {}", lower + 1);
        }
    }

    #[test]
    fn just_past_the_midpoint_snaps_to_the_far_row_and_just_short_snaps_back() {
        assert_eq!(to_nearest_row(HEIGHT / 2.0 + 0.01), 1);
        assert_eq!(to_nearest_row(HEIGHT / 2.0 - 0.01), 0);
    }

    #[test]
    fn anywhere_within_a_rows_own_half_snaps_to_that_row() {
        assert_eq!(to_nearest_row(to_y(3) - HEIGHT * 0.49), 3);
        assert_eq!(to_nearest_row(to_y(3) + HEIGHT * 0.49), 3);
    }

    #[test]
    fn a_negative_y_snaps_to_a_negative_row() {
        assert_eq!(to_nearest_row(-HEIGHT), -1);
        assert_eq!(to_nearest_row(-HEIGHT * 0.6), -1);
    }
}
