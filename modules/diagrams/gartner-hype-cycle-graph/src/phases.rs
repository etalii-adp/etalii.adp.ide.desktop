//! The four phases, and the one place their boundaries are computed (the standalone host's `GhgPhases.cs` at
//! `8b6ccfc`; `gartner-hype-cycle-graph.dis`, `phaseBoundaries` and the Trend type's `bounds.neighbour`).
//!
//! Months, not fractions, are what is stored and computed: every boundary returned here is a month, because a drawn
//! boundary is a date a reader reads off the axis. A trend draws its first `phases` phases; the last visible one ends
//! at `stop`, and each inner boundary before it is its dragged month when one is stored, otherwise it is spread
//! evenly between its nearest dragged neighbours or the trend's ends. A dragged boundary at or beyond the last visible
//! phase is kept in the document, neither drawn nor a neighbour.

use crate::reading::Trend;

/// How many phases a trend can have.
pub const COUNT: i64 = 4;

/// The phases as the document names them, in order.
pub const NAMES: [&str; 4] = ["peak", "trough", "slope", "plateau"];

/// The phases as a reader names them, in order.
pub const TITLES: [&str; 4] = ["Peak", "Trough", "Slope", "Plateau"];

/// The phase's index, or none when `name` names none.
pub fn index_of(name: &str) -> Option<usize> {
    NAMES.iter().position(|n| *n == name)
}

/// The drawn inner boundaries of `trend`, as month indices: one fewer than its visible phases; none for a trend that
/// cannot be drawn (`BoundariesOf`).
pub fn boundaries_of(trend: &Trend) -> Vec<i64> {
    let Some((start, stop)) = trend.span() else {
        return Vec::new();
    };
    let inner = (trend.visible_phases() - 1) as usize;
    let mut result = vec![0; inner];
    let mut anchor_index: i64 = -1;
    let mut anchor_value = start;
    for index in 0..=inner {
        let stored = if index < inner { trend.dragged[index] } else { None };
        if index < inner && stored.is_none() {
            continue;
        }
        let value = if index == inner {
            stop
        } else {
            stored.unwrap_or(stop).clamp(anchor_value, stop.max(anchor_value))
        };
        let first = (anchor_index + 1) as usize;
        for (between, boundary) in result.iter_mut().enumerate().take(index).skip(first) {
            let exact = anchor_value as f64
                + (value - anchor_value) as f64 * (between as i64 - anchor_index) as f64
                    / (index as i64 - anchor_index) as f64;
            *boundary = exact.round() as i64;
        }
        if index < inner {
            result[index] = value;
        }
        anchor_index = index as i64;
        anchor_value = value;
    }
    result
}

/// The drawn boundaries of `trend` as fractions of its width (`FractionsOf`).
pub fn fractions_of(trend: &Trend) -> Vec<f64> {
    let Some((start, _)) = trend.span() else {
        return Vec::new();
    };
    let months = trend.months() as f64;
    boundaries_of(trend)
        .into_iter()
        .map(|boundary| (boundary - start) as f64 / months)
        .collect()
}

/// The fractions a trend draws its phases at: [`fractions_of`] when they are strictly increasing inside (0, 1), an
/// even spread otherwise (`phaseFractions`, the standalone client's `segments.ts`).
pub fn drawn_fractions(trend: &Trend) -> Vec<f64> {
    let phases = trend.visible_phases() as usize;
    let fractions = fractions_of(trend);
    let ordered = fractions.len() == phases - 1
        && fractions
            .iter()
            .enumerate()
            .all(|(i, f)| *f > 0.0 && *f < 1.0 && (i == 0 || *f > fractions[i - 1]));
    if ordered {
        fractions
    } else {
        (1..phases).map(|i| i as f64 / phases as f64).collect()
    }
}

/// The stored boundaries after a move by `months`: shifted exactly, as `start` and `stop` are (`Moved`).
pub fn moved(dragged: &[Option<i64>; 3], months: i64) -> [Option<i64>; 3] {
    dragged.map(|boundary| boundary.map(|b| b + months))
}

/// The stored boundaries after a resize from one span to another: each offset from the start scaled by the new span
/// over the old, to a month, halves away from zero (the definition's `rescaleBoundaries` hook, `Rescaled`), then
/// every visible phase kept at least a month long.
pub fn rescaled(dragged: &[Option<i64>; 3], old: (i64, i64), new: (i64, i64), phases: i64) -> [Option<i64>; 3] {
    let old_span = (old.1 - old.0).max(1);
    let new_span = new.1 - new.0;
    let scaled = dragged.map(|boundary| {
        boundary.map(|value| new.0 + ((value - old.0) as f64 * new_span as f64 / old_span as f64).round() as i64)
    });
    kept_a_month_apart(&scaled, new.0, new.1, phases, None)
}

/// The stored boundaries with `index` set to `month`, clamped so that no phase, including those spread evenly around
/// it, becomes shorter than a month (`WithBoundary`).
pub fn with_boundary(
    dragged: &[Option<i64>; 3],
    index: usize,
    month: i64,
    start: i64,
    stop: i64,
    phases: i64,
) -> [Option<i64>; 3] {
    let mut slots = *dragged;
    slots[index] = Some(month);
    kept_a_month_apart(&slots, start, stop, phases, Some(index))
}

/// Clamps each stored boundary so every phase is at least a month long: a visible boundary between its neighbouring
/// anchors with room for the phases between, one beyond the last visible phase only inside the span; the `pinned`
/// one first, so the others make room for it (`KeptAMonthApart`, the Trend type's `bounds.neighbour`).
pub fn kept_a_month_apart(
    stored: &[Option<i64>; 3],
    start: i64,
    stop: i64,
    phases: i64,
    pinned: Option<usize>,
) -> [Option<i64>; 3] {
    let mut slots = *stored;
    let inner = (phases.clamp(1, COUNT) - 1) as usize;
    if let Some(pinned) = pinned
        && pinned < inner
        && let Some(moved) = slots[pinned]
    {
        let (low_index, low) = (0..pinned)
            .rev()
            .find_map(|i| slots[i].map(|v| (i as i64, v)))
            .unwrap_or((-1, start));
        let (high_index, high) = (pinned + 1..inner)
            .find_map(|i| slots[i].map(|v| (i as i64, v)))
            .unwrap_or((inner as i64, stop));
        let lowest = low + (pinned as i64 - low_index);
        slots[pinned] = Some(moved.clamp(lowest, lowest.max(high - (high_index - pinned as i64))));
    }
    let mut previous_index: i64 = -1;
    let mut previous = start;
    for (index, slot) in slots.iter_mut().enumerate() {
        let Some(value) = *slot else {
            continue;
        };
        let clamped = if index < inner {
            let lower = previous + (index as i64 - previous_index);
            let upper = stop - (inner - index) as i64;
            value.clamp(lower, lower.max(upper))
        } else {
            let highest = start.max(stop - 1);
            value.clamp((previous + 1).min(highest), highest)
        };
        *slot = Some(clamped);
        previous_index = index as i64;
        previous = clamped;
    }
    slots
}

/// The refusal for a span too short for the phases it shows: every phase is at least a month (`GhgRefusals.TooShort`).
pub fn too_short(months: i64, phases: i64) -> Option<String> {
    if months >= phases.max(1) {
        return None;
    }
    Some(if phases <= 1 {
        "A trend must be at least one month long.".to_owned()
    } else {
        format!("A trend showing {phases} phases must be at least {phases} months long, one per phase.")
    })
}
