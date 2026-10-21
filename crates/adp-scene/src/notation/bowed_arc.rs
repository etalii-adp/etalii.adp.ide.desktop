//! A bowed arc: one quadratic curve between two boxes, bowed off its chord (the standalone host's
//! `canvas/library/disl/bowedArc.ts`, and the geometry of `bowedArcLine.tsx`, at 8b6ccfc). It is what DISL calls a
//! `curved` routing (section 6.10), with its `curvature` and its `selfLoop`.
//!
//! The control point sits at the chord's midpoint, offset perpendicular to the chord by a fraction of its length, to
//! the left of the direction of travel for a positive bow; so `A -> B` and `B -> A` bow to opposite sides and a
//! two-element cycle draws as an ellipse. Each end is clipped to its box towards the control point, so the curve
//! leaves and arrives along its own tangent. A line from a box to itself is a loop above the box.
//!
//! What rides along the arc is drawn against the same geometry: bars across it (`midMarkers` of type `bar`) and text
//! marks beside it (edge labels placed at a fraction).

use crate::geometry::{Path, Point, Rect, round2};

/// How far an arc bows from its chord when nothing says otherwise, as a fraction of the chord's length. DISL leaves
/// a curve's default bow to the host.
pub const ARC_BOW: f64 = 0.2;

/// The radius of the loop a line from a box to itself is drawn as, when nothing says otherwise.
pub const SELF_LOOP_RADIUS: f64 = 26.0;

/// A bowed arc's drawn geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct BowedArc {
    /// One quadratic segment, or a pair of them for a line from a box to itself, its coordinates rounded to two
    /// decimals as the standalone writes them.
    pub path: Path,
    /// Where the arc leaves the source box.
    pub start: Point,
    /// Where the arc meets the target box, which is where the arrowhead lands.
    pub finish: Point,
    /// The single control point.
    pub control: Point,
    /// The point halfway along the arc.
    pub apex: Point,
    /// The unit normal at the apex.
    pub apex_normal: Point,
}

fn rounded(point: Point) -> Point {
    Point::new(round2(point.x), round2(point.y))
}

/// The arc for a line from `from` to `to`, bowed by `bow` (positive bows to the left of travel), drawn as a loop of
/// `self_loop_radius` when both ends are one box.
pub fn arc_between(from: Rect, to: Rect, bow: f64, self_loop_radius: f64) -> BowedArc {
    let source = from.centre();
    let target = to.centre();
    let (dx, dy) = (target.x - source.x, target.y - source.y);
    let span = dx.hypot(dy);
    if span < 1.0 {
        return self_arc(from, self_loop_radius);
    }
    let normal = Point::new(dy / span, -dx / span);
    let control = Point::new(
        (source.x + target.x) / 2.0 + normal.x * span * bow,
        (source.y + target.y) / 2.0 + normal.y * span * bow,
    );
    let start = boundary_toward(from, control);
    let finish = boundary_toward(to, control);
    BowedArc {
        path: Path::new()
            .move_to(rounded(start))
            .quad_to(rounded(control), rounded(finish)),
        start,
        finish,
        control,
        apex: quadratic_at(start, control, finish, 0.5),
        apex_normal: quadratic_normal_at(start, control, finish, 0.5),
    }
}

/// A point on the arc, as a fraction along it.
pub fn point_along(arc: &BowedArc, t: f64) -> Point {
    quadratic_at(arc.start, arc.control, arc.finish, t)
}

/// The unit normal to the arc at a fraction along it.
pub fn normal_along(arc: &BowedArc, t: f64) -> Point {
    quadratic_normal_at(arc.start, arc.control, arc.finish, t)
}

/// Where a ray from the box's centre toward `target` leaves the box: a rectangle clip, not a circle.
fn boundary_toward(box_: Rect, target: Point) -> Point {
    let centre = box_.centre();
    let (dx, dy) = (target.x - centre.x, target.y - centre.y);
    if dx.abs() < 1e-9 && dy.abs() < 1e-9 {
        return centre;
    }
    let scale_x = if dx.abs() < 1e-9 {
        f64::INFINITY
    } else {
        box_.width / 2.0 / dx.abs()
    };
    let scale_y = if dy.abs() < 1e-9 {
        f64::INFINITY
    } else {
        box_.height / 2.0 / dy.abs()
    };
    let scale = scale_x.min(scale_y);
    Point::new(centre.x + dx * scale, centre.y + dy * scale)
}

/// A line from a box to itself: two quadratic segments in a loop above it.
fn self_arc(box_: Rect, radius: f64) -> BowedArc {
    let centre = box_.centre();
    let top = box_.y;
    let start = Point::new(centre.x - box_.width * 0.15, top);
    let finish = Point::new(centre.x + box_.width * 0.15, top);
    let control = Point::new(centre.x, top - radius * 2.0);
    let left = Point::new(start.x - radius, top - radius * 1.6);
    let right = Point::new(finish.x + radius, top - radius * 1.6);
    BowedArc {
        path: Path::new()
            .move_to(rounded(start))
            .quad_to(rounded(left), rounded(control))
            .quad_to(rounded(right), rounded(finish)),
        start,
        finish,
        control,
        apex: control,
        apex_normal: Point::new(1.0, 0.0),
    }
}

fn quadratic_at(start: Point, control: Point, finish: Point, t: f64) -> Point {
    let inverse = 1.0 - t;
    Point::new(
        inverse * inverse * start.x + 2.0 * inverse * t * control.x + t * t * finish.x,
        inverse * inverse * start.y + 2.0 * inverse * t * control.y + t * t * finish.y,
    )
}

fn quadratic_normal_at(start: Point, control: Point, finish: Point, t: f64) -> Point {
    let inverse = 1.0 - t;
    let dx = 2.0 * inverse * (control.x - start.x) + 2.0 * t * (finish.x - control.x);
    let dy = 2.0 * inverse * (control.y - start.y) + 2.0 * t * (finish.y - control.y);
    let length = dx.hypot(dy);
    if length < 1e-9 {
        Point::new(0.0, -1.0)
    } else {
        Point::new(dy / length, -dx / length)
    }
}

/// One bar across the arc at a fraction along it, square to the arc at its apex so the bars of a group are
/// parallel (`bowedArcAdornment`).
pub fn bar_across(arc: &BowedArc, t: f64, length: f64) -> (Point, Point) {
    let at = point_along(arc, t);
    let half = length / 2.0;
    (
        Point::new(at.x - arc.apex_normal.x * half, at.y - arc.apex_normal.y * half),
        Point::new(at.x + arc.apex_normal.x * half, at.y + arc.apex_normal.y * half),
    )
}

/// Where a mark beside the arc is anchored: `distance` off the arc at `t`, to the left of travel or the right.
pub fn mark_beside(arc: &BowedArc, t: f64, left_side: bool, distance: f64) -> Point {
    let at = point_along(arc, t);
    let normal = normal_along(arc, t);
    let distance = if left_side { distance } else { -distance };
    Point::new(at.x + normal.x * distance, at.y + normal.y * distance)
}

#[cfg(test)]
mod tests {
    //! The standalone host's `bowedArc.test.ts`.
    use super::*;
    use crate::geometry::PathCommand;

    fn box_at(x: f64, y: f64) -> Rect {
        Rect::new(x - 60.0, y - 20.0, 120.0, 40.0)
    }

    fn arc(from: Rect, to: Rect) -> BowedArc {
        arc_between(from, to, ARC_BOW, SELF_LOOP_RADIUS)
    }

    fn quads(path: &Path) -> usize {
        path.commands
            .iter()
            .filter(|c| matches!(c, PathCommand::QuadTo { .. }))
            .count()
    }

    fn cubics(path: &Path) -> usize {
        path.commands
            .iter()
            .filter(|c| matches!(c, PathCommand::CubicTo { .. }))
            .count()
    }

    #[test]
    fn the_shape_is_one_quadratic_bowed_by_the_stated_fraction_of_its_chord() {
        let (left, right) = (box_at(0.0, 0.0), box_at(400.0, 0.0));
        let a = arc(left, right);
        assert_eq!((1, 0), (quads(&a.path), cubics(&a.path)));
        assert!((a.control.x - 200.0).abs() < 1e-6);
        assert!((a.control.y.abs() - 400.0 * ARC_BOW).abs() < 1e-6);
        let near = arc(left, box_at(200.0, 0.0));
        let far = arc(left, box_at(800.0, 0.0));
        assert!(far.control.y.abs() > near.control.y.abs() * 2.0);
    }

    #[test]
    fn a_two_element_cycle_draws_a_loop_whichever_way_round_the_boxes_sit() {
        let (left, right) = (box_at(0.0, 0.0), box_at(400.0, 0.0));
        let there = arc(left, right);
        let back = arc(right, left);
        assert_ne!(there.control.y.signum(), back.control.y.signum());
        assert!((there.control.y + back.control.y).abs() < 1e-6);
        assert!(there.apex.distance(back.apex) > 60.0);
        assert!((there.apex.y - back.apex.y).abs() > 50.0);
        let mirrored = arc(box_at(400.0, 0.0), box_at(0.0, 0.0));
        assert_eq!(back.control.y.signum(), mirrored.control.y.signum());
        assert_ne!(there.control.y.signum(), mirrored.control.y.signum());
    }

    #[test]
    fn a_ring_bows_every_line_to_the_same_side_of_travel() {
        let ring: Vec<Rect> = (0..4)
            .map(|index| {
                let angle = index as f64 * std::f64::consts::FRAC_PI_2;
                box_at((300.0 * angle.cos()).round(), (300.0 * angle.sin()).round())
            })
            .collect();
        for index in 0..4 {
            let a = arc(ring[index], ring[(index + 1) % 4]);
            let (from, to) = (ring[index].centre(), ring[(index + 1) % 4].centre());
            let midpoint = Point::new((from.x + to.x) / 2.0, (from.y + to.y) / 2.0);
            assert!(a.control.x.hypot(a.control.y) > midpoint.x.hypot(midpoint.y));
        }
    }

    #[test]
    fn the_ends_meet_the_boxes_clipped_to_the_rectangle() {
        let on_boundary = |box_: Rect, point: Point| {
            let centre = box_.centre();
            ((point.x - centre.x).abs() - box_.width / 2.0).abs() < 1e-6
                || ((point.y - centre.y).abs() - box_.height / 2.0).abs() < 1e-6
        };
        let (left, right) = (box_at(0.0, 0.0), box_at(400.0, 0.0));
        let a = arc(left, right);
        assert!(on_boundary(left, a.start) && on_boundary(right, a.finish));
        assert!(a.finish.x < right.centre().x && a.start.x > left.centre().x);
        let up = arc(left, box_at(0.0, -400.0));
        assert!((up.start.y - left.y).abs() < 1e-6);
        assert!(up.start.x.abs() <= left.width / 2.0 + 1e-6);
    }

    #[test]
    fn what_is_drawn_along_the_arc_follows_the_arc() {
        let a = arc(box_at(0.0, 0.0), box_at(400.0, 0.0));
        let chord = Point::new((a.start.x + a.finish.x) / 2.0, (a.start.y + a.finish.y) / 2.0);
        assert!((a.apex.y - chord.y).abs() > 10.0);
        assert!((a.apex.x - (chord.x + a.control.x) / 2.0).abs() < 1e-6);
        assert!((a.apex.y - (chord.y + a.control.y) / 2.0).abs() < 1e-6);
        for t in [0.1, 0.5, 0.9] {
            let normal = normal_along(&a, t);
            assert!((normal.x.hypot(normal.y) - 1.0).abs() < 1e-6);
        }
        assert_eq!(a.start, point_along(&a, 0.0));
        assert_eq!(a.finish, point_along(&a, 1.0));
        assert_eq!(a.apex, point_along(&a, 0.5));
    }

    #[test]
    fn degenerate_placements_give_finite_geometry_and_a_loop_above_the_box() {
        let left = box_at(0.0, 0.0);
        let own = arc(left, left);
        assert_eq!(2, quads(&own.path));
        assert!(own.apex.y < left.y);
        for (x, y) in [
            (0.0, 0.0),
            (400.0, 0.0),
            (-400.0, 0.0),
            (0.0, 400.0),
            (0.0, -400.0),
            (300.0, 300.0),
            (-300.0, -300.0),
            (1.0, 1.0),
        ] {
            let a = arc(left, box_at(x, y));
            assert!(
                !a.path.to_svg().contains("NaN") && !a.path.to_svg().contains("inf"),
                "{x},{y}"
            );
            assert!(a.apex.x.is_finite() && a.apex.y.is_finite());
        }
    }

    #[test]
    fn bars_cross_the_arc_square_to_its_apex_and_marks_sit_beside_it() {
        let a = arc(box_at(0.0, 0.0), box_at(400.0, 0.0));
        let (from, to) = bar_across(&a, 0.5, 10.0);
        assert!((from.distance(to) - 10.0).abs() < 1e-9);
        let mark = mark_beside(&a, 0.86, true, 11.0);
        let expected = Point::new(
            point_along(&a, 0.86).x + normal_along(&a, 0.86).x * 11.0,
            point_along(&a, 0.86).y + normal_along(&a, 0.86).y * 11.0,
        );
        assert_eq!(expected, mark);
    }
}
