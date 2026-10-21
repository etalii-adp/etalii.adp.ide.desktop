//! The time ruler under the canvas (`gartner-hype-cycle-graph.dis`, `coordinates.axes.time.ruler`; the standalone
//! host's library at `8b6ccfc`, `definition/chrome.ts`, `resolveTicks` and `monthTicks`, as `compileNotation` compiled
//! the ruler): pinned to the bottom of the view, its ticks on the boundaries of the coarsest level whose labels still
//! stand 64 pixels apart, on a scale uniform in months.
//!
//! The levels are the specification's: a month and a quarter labelled `MMM uuuu`, a year, a decade, a century and a
//! millennium labelled `uuuu`; a level finer than one step of the diagram's unit is never shown (`minUnit`).

use crate::scale::{self, TimeUnit};

/// One labelled boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct Tick {
    /// Where it falls, in canvas units.
    pub x: f64,
    pub label: String,
}

/// The pixels a labelled tick needs (`minSpacingPx`).
pub const MIN_SPACING: f64 = 64.0;

/// The levels, finest first: how many months one spans, and whether its label names the month.
const LEVELS: [(i64, bool); 6] = [
    (1, true),
    (3, true),
    (12, false),
    (120, false),
    (1200, false),
    (12000, false),
];

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// A month as a tick labels it: `Jan 1950` where its level names the month, the year alone otherwise.
fn label(month_index: i64, named: bool) -> String {
    let year = month_index.div_euclid(12);
    let year_text = if year < 0 {
        format!("-{:04}", -year)
    } else {
        format!("{year:04}")
    };
    if named {
        format!("{} {year_text}", MONTHS[(month_index - year * 12) as usize])
    } else {
        year_text
    }
}

/// The labelled boundaries between the canvas x `from` and `to`, across `width` pixels, in a diagram drawn in
/// `unit`.
pub fn ticks(from: f64, to: f64, width: f64, unit: TimeUnit) -> Vec<Tick> {
    if to <= from || width <= 0.0 || !from.is_finite() || !to.is_finite() {
        return Vec::new();
    }
    let per_month = scale::UNITS_PER_STEP / unit.months() as f64;
    let origin = scale::ORIGIN_MONTH as f64;
    let (first, last) = (origin + from / per_month, origin + to / per_month);
    let most = (width / MIN_SPACING).floor().max(1.0);
    let coarsest = (last - first) / most;
    let levels: Vec<(i64, bool)> = LEVELS
        .into_iter()
        .filter(|(months, _)| *months >= unit.months())
        .collect();
    let Some(&(months, named)) = levels
        .iter()
        .find(|(months, _)| *months as f64 >= coarsest)
        .or(levels.last())
    else {
        return Vec::new();
    };
    let mut ticks = Vec::new();
    let mut at = (first / months as f64).ceil() as i64 * months;
    while (at as f64) <= last && ticks.len() < 4096 {
        ticks.push(Tick {
            x: (at as f64 - origin) * per_month,
            label: label(at, named),
        });
        at += months;
    }
    ticks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wide_view_of_a_few_years_is_labelled_by_month() {
        // 1950-01 to 1951-01 over 1280 pixels: 48 units, 12 months, 20 labels fit.
        let ticks = ticks(2400.0, 2448.0, 1280.0, TimeUnit::Month);
        assert_eq!(ticks.len(), 13);
        assert_eq!(
            ticks[0],
            Tick {
                x: 2400.0,
                label: "Jan 1950".into()
            }
        );
        assert_eq!(ticks[12].label, "Jan 1951");
    }

    #[test]
    fn a_narrow_view_takes_the_coarsest_level_that_still_fits() {
        // Two centuries over 640 pixels: ten labels fit, so not the twenty decades but the centuries.
        let ticks = ticks(0.0, 200.0 * 48.0, 640.0, TimeUnit::Month);
        assert_eq!(
            ticks.iter().map(|t| t.label.as_str()).collect::<Vec<_>>(),
            ["1900", "2000", "2100"]
        );
        // Over 1280 pixels twenty fit, so decades.
        assert_eq!(
            super::ticks(0.0, 200.0 * 48.0, 1280.0, TimeUnit::Month)[1].label,
            "1910"
        );
    }

    #[test]
    fn a_level_finer_than_the_unit_is_never_shown() {
        let ticks = ticks(0.0, 40.0, 1280.0, TimeUnit::Decade);
        assert_eq!(
            ticks.iter().map(|t| t.label.as_str()).collect::<Vec<_>>(),
            [
                "1900", "1910", "1920", "1930", "1940", "1950", "1960", "1970", "1980", "1990", "2000"
            ]
        );
        assert_eq!(ticks[1].x, 4.0);
    }

    #[test]
    fn a_year_before_one_is_signed() {
        assert_eq!(label(scale::month_index(-3200, 1), false), "-3200");
        assert_eq!(label(scale::month_index(5, 3), true), "Mar 0005");
    }
}
