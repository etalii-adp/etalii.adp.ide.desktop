//! Where a line between two boxes starts and ends, and the curve it takes on the way (the standalone host's
//! `canvas/connectors.ts` at 8b6ccfc): geometry rather than notation, shared by every route a relation is drawn
//! with.
//!
//! A [`ConnectorBox`] is positioned by its centre, as the standalone's connector geometry has it; the scene's own
//! rectangles are corner-based and are turned into one where a route asks.

use crate::geometry::{Path, Point, Rect};

/// A box positioned by its centre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConnectorBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl ConnectorBox {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        ConnectorBox { x, y, width, height }
    }

    /// The box of a corner-based rectangle.
    pub fn of(rect: Rect) -> Self {
        let centre = rect.centre();
        ConnectorBox::new(centre.x, centre.y, rect.width, rect.height)
    }
}

/// Which vertical side of a box a line leaves from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalSide {
    Left,
    Right,
}

/// One side of a box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

/// The point on a box's edge in the direction `(dx, dy)` from its centre; the centre for no direction.
pub fn edge_point_of(box_: ConnectorBox, dx: f64, dy: f64) -> Point {
    if dx == 0.0 && dy == 0.0 {
        return Point::new(box_.x, box_.y);
    }
    let scale = {
        let horizontal = if dx == 0.0 {
            f64::INFINITY
        } else {
            box_.width / 2.0 / dx.abs()
        };
        let vertical = if dy == 0.0 {
            f64::INFINITY
        } else {
            box_.height / 2.0 / dy.abs()
        };
        horizontal.min(vertical)
    };
    Point::new(box_.x + dx * scale, box_.y + dy * scale)
}

/// Where a straight line between two boxes starts and ends: on each edge, along the line between their centres.
pub fn anchors_between(source: ConnectorBox, destination: ConnectorBox) -> (Point, Point) {
    let dx = destination.x - source.x;
    let dy = destination.y - source.y;
    (edge_point_of(source, dx, dy), edge_point_of(destination, -dx, -dy))
}

/// The middle of a box's left or right side.
pub fn side_anchor_of(box_: ConnectorBox, side: HorizontalSide) -> Point {
    let x = match side {
        HorizontalSide::Right => box_.x + box_.width / 2.0,
        HorizontalSide::Left => box_.x - box_.width / 2.0,
    };
    Point::new(x, box_.y)
}

/// Where a line between a parent and a child in a column beside it starts and ends: the facing vertical sides.
pub fn branch_anchors_between(parent: ConnectorBox, child: ConnectorBox) -> (Point, Point) {
    let child_on_right = child.x >= parent.x;
    let (out, back) = if child_on_right {
        (HorizontalSide::Right, HorizontalSide::Left)
    } else {
        (HorizontalSide::Left, HorizontalSide::Right)
    };
    (side_anchor_of(parent, out), side_anchor_of(child, back))
}

/// Where a line between two boxes side by side starts and ends: the vertical sides that face each other.
pub fn facing_anchors_between(from: ConnectorBox, to: ConnectorBox) -> (Point, Point) {
    branch_anchors_between(from, to)
}

/// A horizontal cubic bezier: both control points at the horizontal midpoint.
pub fn horizontal_bezier_path(from: Point, to: Point) -> Path {
    let mid = (from.x + to.x) / 2.0;
    Path::new()
        .move_to(from)
        .cubic_to(Point::new(mid, from.y), Point::new(mid, to.y), to)
}

fn outward(edge: Edge) -> Point {
    match edge {
        Edge::Top => Point::new(0.0, -1.0),
        Edge::Bottom => Point::new(0.0, 1.0),
        Edge::Left => Point::new(-1.0, 0.0),
        Edge::Right => Point::new(1.0, 0.0),
    }
}

/// A cubic bezier whose ends leave and arrive square to the edges they are attached along; an end with no edge keeps
/// the horizontal bezier's control point. The reach is half the ends' distance along the normal's axis, at least 24.
pub fn edge_bezier_path(from: Point, to: Point, from_edge: Option<Edge>, to_edge: Option<Edge>) -> Path {
    let mid = (from.x + to.x) / 2.0;
    let control = |end: Point, edge: Option<Edge>| match edge {
        None => Point::new(mid, end.y),
        Some(edge) => {
            let out = outward(edge);
            let along = if out.x != 0.0 { to.x - from.x } else { to.y - from.y };
            let reach = (along.abs() / 2.0).max(24.0);
            Point::new(end.x + out.x * reach, end.y + out.y * reach)
        }
    };
    Path::new()
        .move_to(from)
        .cubic_to(control(from, from_edge), control(to, to_edge), to)
}

/// A cubic bezier that always leaves `from` rightward and arrives at `to` from its left, looping round when `to`
/// sits behind `from`.
pub fn forward_bezier_path(from: Point, to: Point) -> Path {
    let span = to.x - from.x;
    let reach = (span / 2.0).max(40.0).max(-span * 0.6);
    Path::new()
        .move_to(from)
        .cubic_to(Point::new(from.x + reach, from.y), Point::new(to.x - reach, to.y), to)
}

/// A straight line.
pub fn straight_path(from: Point, to: Point) -> Path {
    Path::new().move_to(from).line_to(to)
}

/// A polyline through the waypoints; a straight line without any.
pub fn polyline_path(from: Point, to: Point, waypoints: &[Point]) -> Path {
    let mut path = Path::new().move_to(from);
    for point in waypoints.iter().chain(std::iter::once(&to)) {
        path = path.line_to(*point);
    }
    path
}

/// Which way an orthogonal route leaves and arrives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

/// An axis-aligned route with two elbows, rounded by `corner_radius` (sharp at zero). `Vertical` is the horizontal
/// route with x and y exchanged, which turns every arc the other way.
pub fn orthogonal_path(from: Point, to: Point, corner_radius: f64, axis: Axis) -> Path {
    match axis {
        Axis::Vertical => {
            let swap = |p: Point| Point::new(p.y, p.x);
            horizontal_orthogonal(swap(from), swap(to), corner_radius, true)
        }
        Axis::Horizontal => horizontal_orthogonal(from, to, corner_radius, false),
    }
}

fn horizontal_orthogonal(from: Point, to: Point, corner_radius: f64, transposed: bool) -> Path {
    let at = |x: f64, y: f64| if transposed { Point::new(y, x) } else { Point::new(x, y) };
    let mid = (from.x + to.x) / 2.0;
    if corner_radius <= 0.0 || from.y == to.y {
        return Path::new()
            .move_to(at(from.x, from.y))
            .line_to(at(mid, from.y))
            .line_to(at(mid, to.y))
            .line_to(at(to.x, to.y));
    }
    let r = corner_radius
        .min((mid - from.x).abs())
        .min((to.y - from.y).abs() / 2.0)
        .min((to.x - mid).abs());
    let x_dir = sign_or_one(mid - from.x);
    let y_dir = sign_or_one(to.y - from.y);
    let clockwise = x_dir * y_dir > 0.0;
    let sweep1 = clockwise != transposed;
    let sweep2 = !sweep1;
    Path::new()
        .move_to(at(from.x, from.y))
        .line_to(at(mid - x_dir * r, from.y))
        .arc_to(r, r, 0.0, false, sweep1, at(mid, from.y + y_dir * r))
        .line_to(at(mid, to.y - y_dir * r))
        .arc_to(r, r, 0.0, false, sweep2, at(mid + x_dir * r, to.y))
        .line_to(at(to.x, to.y))
}

fn sign_or_one(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        1.0
    }
}

/// A single-control-point curve bowed perpendicular to the chord, to the same side of travel.
pub fn arc_path(from: Point, to: Point, bow: f64) -> Path {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let control = Point::new((from.x + to.x) / 2.0 - dy * bow, (from.y + to.y) / 2.0 + dx * bow);
    Path::new().move_to(from).quad_to(control, to)
}

/// How far a soft curve bows, as a fraction of its chord.
const SOFT_BOW: f64 = 0.15;

/// The declared `quadratic-bezier` route: [`arc_path`] with a gentler bow.
pub fn quadratic_bezier_path(from: Point, to: Point) -> Path {
    arc_path(from, to, SOFT_BOW)
}

/// A Catmull-Rom spline through the waypoints, as cubic segments; a straight line without any.
pub fn spline_path(from: Point, to: Point, waypoints: &[Point]) -> Path {
    let mut points = vec![from];
    points.extend_from_slice(waypoints);
    points.push(to);
    if points.len() == 2 {
        return straight_path(from, to);
    }
    let mut path = Path::new().move_to(from);
    for i in 0..points.len() - 1 {
        let p0 = points[i.saturating_sub(1)];
        let p1 = points[i];
        let p2 = points[i + 1];
        let p3 = points[(i + 2).min(points.len() - 1)];
        let c1 = Point::new(p1.x + (p2.x - p0.x) / 6.0, p1.y + (p2.y - p0.y) / 6.0);
        let c2 = Point::new(p2.x - (p3.x - p1.x) / 6.0, p2.y - (p3.y - p1.y) / 6.0);
        path = path.cubic_to(c1, c2, p2);
    }
    path
}

/// The midpoint of two points.
pub fn midpoint_of(from: Point, to: Point) -> Point {
    Point::new((from.x + to.x) / 2.0, (from.y + to.y) / 2.0)
}

#[cfg(test)]
mod tests {
    //! The standalone host's `connectors.test.ts`.
    use super::*;
    use crate::geometry::PathCommand;

    fn box_(x: f64, y: f64) -> ConnectorBox {
        ConnectorBox::new(x, y, 100.0, 40.0)
    }

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn an_edge_point_lands_on_the_side_or_the_top_a_direction_reaches_first() {
        assert_eq!(p(50.0, 0.0), edge_point_of(box_(0.0, 0.0), 1.0, 0.0));
        assert_eq!(p(-50.0, 0.0), edge_point_of(box_(0.0, 0.0), -1.0, 0.0));
        assert_eq!(p(0.0, 20.0), edge_point_of(box_(0.0, 0.0), 0.0, 1.0));
        assert_eq!(p(0.0, -20.0), edge_point_of(box_(0.0, 0.0), 0.0, -1.0));
        assert_eq!(
            p(20.0, 20.0),
            edge_point_of(ConnectorBox::new(0.0, 0.0, 200.0, 40.0), 1.0, 1.0)
        );
        assert_eq!(
            p(50.0, 25.0),
            edge_point_of(ConnectorBox::new(0.0, 0.0, 100.0, 100.0), 2.0, 1.0)
        );
        assert_eq!(p(7.0, 9.0), edge_point_of(box_(7.0, 9.0), 0.0, 0.0));
    }

    #[test]
    fn anchors_between_boxes_face_each_other_and_reverse_with_the_ends() {
        let (from, to) = anchors_between(
            ConnectorBox::new(0.0, 0.0, 100.0, 80.0),
            ConnectorBox::new(0.0, 200.0, 100.0, 80.0),
        );
        assert_eq!((p(0.0, 40.0), p(0.0, 160.0)), (from, to));
        let forward = anchors_between(box_(0.0, 0.0), box_(300.0, 120.0));
        let backward = anchors_between(box_(300.0, 120.0), box_(0.0, 0.0));
        assert_eq!(forward.1, backward.0);
        assert_eq!(forward.0, backward.1);
    }

    #[test]
    fn a_branch_leaves_the_side_facing_its_child() {
        assert_eq!(
            (p(50.0, 0.0), p(250.0, 80.0)),
            branch_anchors_between(box_(0.0, 0.0), box_(300.0, 80.0))
        );
        assert_eq!(
            (p(-50.0, 0.0), p(-250.0, 80.0)),
            branch_anchors_between(box_(0.0, 0.0), box_(-300.0, 80.0))
        );
        assert_eq!(50.0, branch_anchors_between(box_(0.0, 0.0), box_(0.0, 200.0)).0.x);
        assert_eq!(p(60.0, 20.0), side_anchor_of(box_(10.0, 20.0), HorizontalSide::Right));
        assert_eq!(p(-40.0, 20.0), side_anchor_of(box_(10.0, 20.0), HorizontalSide::Left));
    }

    #[test]
    fn the_path_builders_write_what_the_canvas_library_writes() {
        assert_eq!("M 1 2 L 3 4", straight_path(p(1.0, 2.0), p(3.0, 4.0)).to_svg());
        assert_eq!(
            "M 0 0 C 50 0, 50 60, 100 60",
            horizontal_bezier_path(p(0.0, 0.0), p(100.0, 60.0)).to_svg()
        );
        assert_eq!(
            "M 200 0 C 150 0, 150 60, 100 60",
            horizontal_bezier_path(p(200.0, 0.0), p(100.0, 60.0)).to_svg()
        );
        assert_eq!(p(5.0, 15.0), midpoint_of(p(0.0, 0.0), p(10.0, 30.0)));
    }

    fn quad(path: &Path) -> (Point, Point, Point) {
        match (path.commands[0], path.commands[1]) {
            (PathCommand::MoveTo(from), PathCommand::QuadTo { control, to }) => (from, control, to),
            _ => panic!("not a single quadratic segment: {}", path.to_svg()),
        }
    }

    fn off_chord(path: &Path) -> f64 {
        let (from, control, to) = quad(path);
        let (dx, dy) = (to.x - from.x, to.y - from.y);
        ((control.x - from.x) * dy - (control.y - from.y) * dx) / dx.hypot(dy)
    }

    #[test]
    fn the_quadratic_route_bends_in_proportion_to_its_length_to_the_same_side_of_travel() {
        let (from, _, to) = quad(&quadratic_bezier_path(p(10.0, 20.0), p(210.0, 120.0)));
        assert_eq!((p(10.0, 20.0), p(210.0, 120.0)), (from, to));
        for (a, b) in [
            ((0.0, 0.0), (200.0, 0.0)),
            ((0.0, 0.0), (0.0, 200.0)),
            ((30.0, 40.0), (170.0, 190.0)),
        ] {
            assert!(off_chord(&quadratic_bezier_path(p(a.0, a.1), p(b.0, b.1))).abs() > 1.0);
        }
        let short = off_chord(&quadratic_bezier_path(p(0.0, 0.0), p(100.0, 0.0))).abs();
        let long = off_chord(&quadratic_bezier_path(p(0.0, 0.0), p(400.0, 0.0))).abs();
        assert!((long / short - 4.0).abs() < 1e-5);
        assert!(off_chord(&quadratic_bezier_path(p(0.0, 0.0), p(200.0, 0.0))).abs() < 50.0);
        let there = quadratic_bezier_path(p(0.0, 0.0), p(200.0, 0.0));
        let back = quadratic_bezier_path(p(200.0, 0.0), p(0.0, 0.0));
        assert_eq!(off_chord(&there).signum(), off_chord(&back).signum());
        assert_eq!(quad(&there).1.y.signum(), -quad(&back).1.y.signum());
    }

    #[test]
    fn an_orthogonal_route_runs_sideways_or_vertically_and_rounds_its_elbows() {
        assert_eq!(
            "M 0 0 L 50 0 L 50 60 L 100 60",
            orthogonal_path(p(0.0, 0.0), p(100.0, 60.0), 0.0, Axis::Horizontal).to_svg()
        );
        assert_eq!(
            "M 0 0 L 0 30 L 100 30 L 100 60",
            orthogonal_path(p(0.0, 0.0), p(100.0, 60.0), 0.0, Axis::Vertical).to_svg()
        );
        assert_eq!(
            "M 0 0 L 0 20 A 10 10 0 0 0 10 30 L 90 30 A 10 10 0 0 1 100 40 L 100 60",
            orthogonal_path(p(0.0, 0.0), p(100.0, 60.0), 10.0, Axis::Vertical).to_svg()
        );
    }

    #[test]
    fn a_forward_bezier_loops_round_when_its_target_sits_behind() {
        assert_eq!(
            "M 0 0 C 50 0, 50 10, 100 10",
            forward_bezier_path(p(0.0, 0.0), p(100.0, 10.0)).to_svg()
        );
        assert_eq!(
            "M 100 0 C 160 0, -60 10, 0 10",
            forward_bezier_path(p(100.0, 0.0), p(0.0, 10.0)).to_svg()
        );
    }
}
