//! The phase boundaries, computed once (the standalone host's `GhgPhases.Tests.cs` at `8b6ccfc`, task 15): spread
//! evenly, kept where dragged, moved, rescaled and kept a month apart.

mod common;

use adp_diagram_hype_cycle::phases;
use adp_diagram_hype_cycle::reading::Trend;
use common::m;

fn trend(start: i64, stop: i64, phases: i64, dragged: [Option<i64>; 3]) -> Trend {
    Trend {
        id: "t".into(),
        name: "T".into(),
        start: Some(start),
        stop: Some(stop),
        row: 0,
        phases,
        dragged,
        line: 0,
    }
}

const NONE: [Option<i64>; 3] = [None, None, None];

/// With nothing dragged, the visible phases divide the span evenly.
#[test]
fn with_nothing_dragged_the_phases_are_even() {
    assert_eq!(
        phases::boundaries_of(&trend(m(1900, 1), m(1940, 1), 4, NONE)),
        [m(1910, 1), m(1920, 1), m(1930, 1)]
    );
    assert_eq!(
        phases::boundaries_of(&trend(m(1900, 1), m(1940, 1), 2, NONE)),
        [m(1920, 1)]
    );
    assert!(phases::boundaries_of(&trend(m(1900, 1), m(1940, 1), 1, NONE)).is_empty());
}

/// One dragged boundary stays where it was put, and the others spread evenly between it and the trend's ends.
#[test]
fn one_dragged_stays_and_the_others_spread_between_it_and_the_ends() {
    // Trough ends at 1904: the Peak takes half of 1900-1904, the Slope half of 1904-1940.
    let boundaries = phases::boundaries_of(&trend(m(1900, 1), m(1940, 1), 4, [None, Some(m(1904, 1)), None]));
    assert_eq!(boundaries, [m(1902, 1), m(1904, 1), m(1922, 1)]);
}

#[test]
fn two_dragged_stay_and_the_third_spreads_between_the_nearest() {
    let boundaries = phases::boundaries_of(&trend(
        m(1900, 1),
        m(1940, 1),
        4,
        [Some(m(1901, 1)), None, Some(m(1911, 1))],
    ));
    assert_eq!(boundaries, [m(1901, 1), m(1906, 1), m(1911, 1)]);
}

/// A boundary dragged beyond the last visible phase is kept, not drawn, and is no neighbour for the ones that are.
#[test]
fn a_dragged_boundary_beyond_the_last_phase_is_not_drawn_and_not_a_neighbour() {
    let mut two = trend(m(1900, 1), m(1940, 1), 2, [None, Some(m(1904, 1)), None]);
    assert_eq!(phases::boundaries_of(&two), [m(1920, 1)]);
    assert_eq!(phases::fractions_of(&two), [0.5]);
    // And applies again when its phase is shown.
    two.phases = 3;
    assert_eq!(phases::boundaries_of(&two), [m(1902, 1), m(1904, 1)]);
}

/// A move keeps the dragged boundaries exactly.
#[test]
fn a_move_shifts_every_stored_boundary_exactly() {
    assert_eq!(
        phases::moved(&[None, Some(m(1804, 3)), Some(m(1820, 1))], 120),
        [None, Some(m(1814, 3)), Some(m(1830, 1))]
    );
}

/// A resize scales each dragged boundary with the span and snaps it to a month: a resize that kept absolute
/// boundaries would leave one outside.
#[test]
fn a_resize_scales_the_dragged_boundaries_and_keeps_them_inside_the_span() {
    // 1900-1940 with the Trough ending a quarter of the way in, shrunk to 1900-1920.
    let dragged = phases::rescaled(
        &[None, Some(m(1910, 1)), None],
        (m(1900, 1), m(1940, 1)),
        (m(1900, 1), m(1920, 1)),
        4,
    );
    assert_eq!(dragged, [None, Some(m(1905, 1)), None]);
    // A left-edge resize: the offset is from the new start.
    let from_the_left = phases::rescaled(
        &[None, Some(m(1910, 1)), None],
        (m(1900, 1), m(1940, 1)),
        (m(1920, 1), m(1940, 1)),
        4,
    );
    assert_eq!(from_the_left, [None, Some(m(1925, 1)), None]);
}

#[test]
fn a_resize_keeps_every_phase_at_least_a_month() {
    // Four phases squeezed into four months: each boundary must land a month apart.
    let dragged = phases::rescaled(
        &[Some(m(1900, 2)), Some(m(1900, 3)), Some(m(1900, 4))],
        (m(1900, 1), m(1940, 1)),
        (m(1900, 1), m(1900, 5)),
        4,
    );
    let squeezed = trend(m(1900, 1), m(1900, 5), 4, dragged);
    let mut points = vec![m(1900, 1)];
    points.extend(phases::boundaries_of(&squeezed));
    points.push(m(1900, 5));
    for pair in points.windows(2) {
        assert!(pair[1] - pair[0] >= 1, "{} to {}", pair[0], pair[1]);
    }
}

/// A boundary drag is clamped so no phase, even or dragged, is under a month.
#[test]
fn a_boundary_drag_is_clamped_a_month_short_of_its_neighbours() {
    // Dragging the Peak's end past the stop leaves room for the Trough, Slope and Plateau.
    let dragged = phases::with_boundary(&NONE, 0, m(1950, 1), m(1900, 1), m(1940, 1), 4);
    assert_eq!(dragged, [Some(m(1939, 10)), None, None]);
    assert_eq!(
        phases::boundaries_of(&trend(m(1900, 1), m(1940, 1), 4, dragged)),
        [m(1939, 10), m(1939, 11), m(1939, 12)]
    );
}

/// Fractions that are not strictly increasing inside (0, 1) draw as an even spread instead (`phaseFractions`).
#[test]
fn out_of_order_fractions_are_drawn_evenly() {
    let stacked = trend(m(1900, 1), m(1940, 1), 3, [Some(m(1900, 1)), Some(m(1900, 1)), None]);
    assert_eq!(phases::drawn_fractions(&stacked), [1.0 / 3.0, 2.0 / 3.0]);
    let dragged = trend(m(1900, 1), m(1940, 1), 2, [Some(m(1910, 1)), None, None]);
    assert_eq!(phases::drawn_fractions(&dragged), [0.25]);
}

#[test]
fn a_span_too_short_for_its_phases_is_refused_in_its_own_words() {
    assert_eq!(
        phases::too_short(3, 4).as_deref(),
        Some("A trend showing 4 phases must be at least 4 months long, one per phase.")
    );
    assert_eq!(
        phases::too_short(0, 1).as_deref(),
        Some("A trend must be at least one month long.")
    );
    assert_eq!(phases::too_short(4, 4), None);
}
