//! Pushes boxes that overlap apart (etalii.adp spec 030, FR-018), ported from the standalone host's
//! `EtAlii.Adp.Layout/OverlapResolver.cs` at `8b6ccfc` (code-structure FR-045), for the layouts whose first pass
//! places boxes by a rule that does not promise to keep them clear of each other.

/// Pushes overlapping boxes apart in stable order, and reports how many pairs still overlap when the rounds are spent.
///
/// `centres` holds each box's centre and is moved in place; `sizes` holds each box's width and height in the same
/// order. `separation` is the clear space two boxes keep between them, `rounds` the most passes over every pair before
/// giving up, and `nudge` what each box of a pair is pushed beyond half the overlap, so a pair just cleared does not
/// touch. The result is how many pairs overlapped in the last round; zero when every box is clear.
///
/// Deterministic: pairs are visited in the order given, each round reads the centres the previous round left, and a
/// push is a function of the overlap alone. A pair is pushed along the axis that needs the least movement relative to
/// what it needs, so two boxes nearly side by side are not moved apart the long way, which would undo the arrangement
/// they came from.
pub fn separate(centres: &mut [(f64, f64)], sizes: &[(f64, f64)], separation: f64, rounds: usize, nudge: f64) -> usize {
    let mut overlaps = 0;
    for _ in 0..rounds {
        overlaps = 0;
        for first in 0..centres.len() {
            for second in first + 1..centres.len() {
                // The half-extents two boxes need between their centres to clear each other, plus the separation kept
                // between neighbours.
                let needed_x = ((sizes[first].0 + sizes[second].0) / 2.0) + separation;
                let needed_y = ((sizes[first].1 + sizes[second].1) / 2.0) + separation;

                let dx = centres[second].0 - centres[first].0;
                let dy = centres[second].1 - centres[first].1;

                let overlap_x = needed_x - dx.abs();
                let overlap_y = needed_y - dy.abs();
                if overlap_x <= 0.0 || overlap_y <= 0.0 {
                    continue;
                }

                overlaps += 1;
                if overlap_x / needed_x < overlap_y / needed_y {
                    let push = (overlap_x / 2.0) + nudge;
                    let direction = if dx < 0.0 { -1.0 } else { 1.0 };
                    centres[first].0 -= push * direction;
                    centres[second].0 += push * direction;
                } else {
                    let push = (overlap_y / 2.0) + nudge;
                    let direction = if dy < 0.0 { -1.0 } else { 1.0 };
                    centres[first].1 -= push * direction;
                    centres[second].1 += push * direction;
                }
            }
        }

        if overlaps == 0 {
            return 0;
        }
    }

    overlaps
}
