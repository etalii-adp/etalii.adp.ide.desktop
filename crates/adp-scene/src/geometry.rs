//! Points, rectangles and paths in diagram coordinates: what every primitive of a scene is made of.
//!
//! A [`Path`] keeps its commands rather than SVG text, so the shell can tessellate it without parsing; its
//! [`Path::to_svg`] writes the text the standalone host's canvas library writes for the same geometry (at 8b6ccfc:
//! `canvas/connectors.ts`, `canvas/library/disl/bowedArc.ts`, `canvas/library/shapes/segments.ts`), which is what
//! the ported tests compare. Numbers are written as JavaScript writes them: the shortest form that reads back, and
//! never `-0`.

/// A point in diagram coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const fn new(x: f64, y: f64) -> Self {
        Point { x, y }
    }

    /// The distance to `other`.
    pub fn distance(self, other: Point) -> f64 {
        (other.x - self.x).hypot(other.y - self.y)
    }
}

/// A rectangle by its top-left corner and its size, in diagram coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Rect { x, y, width, height }
    }

    /// The rectangle of `width` and `height` around `centre`.
    pub fn centred(centre: Point, width: f64, height: f64) -> Self {
        Rect::new(centre.x - width / 2.0, centre.y - height / 2.0, width, height)
    }

    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    pub fn centre(&self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Whether `point` lies inside or on the edge.
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x && point.x <= self.right() && point.y >= self.y && point.y <= self.bottom()
    }

    /// Whether the two rectangles share any point, edges included.
    pub fn intersects(&self, other: &Rect) -> bool {
        self.x <= other.right() && other.x <= self.right() && self.y <= other.bottom() && other.y <= self.bottom()
    }

    /// The smallest rectangle holding both.
    pub fn union(&self, other: &Rect) -> Rect {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        Rect::new(
            x,
            y,
            self.right().max(other.right()) - x,
            self.bottom().max(other.bottom()) - y,
        )
    }

    /// The rectangle grown by `by` on every side.
    pub fn inflate(&self, by: f64) -> Rect {
        Rect::new(self.x - by, self.y - by, self.width + 2.0 * by, self.height + 2.0 * by)
    }

    /// The smallest rectangle holding every point; none for no points.
    pub fn around(points: impl IntoIterator<Item = Point>) -> Option<Rect> {
        let mut points = points.into_iter();
        let first = points.next()?;
        let (mut left, mut top, mut right, mut bottom) = (first.x, first.y, first.x, first.y);
        for point in points {
            left = left.min(point.x);
            top = top.min(point.y);
            right = right.max(point.x);
            bottom = bottom.max(point.y);
        }
        Some(Rect::new(left, top, right - left, bottom - top))
    }
}

/// One command of a path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathCommand {
    MoveTo(Point),
    LineTo(Point),
    /// A quadratic curve through one control point.
    QuadTo {
        control: Point,
        to: Point,
    },
    /// A cubic curve through two control points.
    CubicTo {
        first: Point,
        second: Point,
        to: Point,
    },
    /// An elliptical arc, as SVG's `A` states it.
    ArcTo {
        rx: f64,
        ry: f64,
        rotation: f64,
        large_arc: bool,
        sweep: bool,
        to: Point,
    },
    Close,
}

/// A path: one or more subpaths of lines, curves and arcs.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Path {
    pub commands: Vec<PathCommand>,
}

impl Path {
    pub fn new() -> Self {
        Path::default()
    }

    pub fn move_to(mut self, to: Point) -> Self {
        self.commands.push(PathCommand::MoveTo(to));
        self
    }

    pub fn line_to(mut self, to: Point) -> Self {
        self.commands.push(PathCommand::LineTo(to));
        self
    }

    pub fn quad_to(mut self, control: Point, to: Point) -> Self {
        self.commands.push(PathCommand::QuadTo { control, to });
        self
    }

    pub fn cubic_to(mut self, first: Point, second: Point, to: Point) -> Self {
        self.commands.push(PathCommand::CubicTo { first, second, to });
        self
    }

    pub fn arc_to(mut self, rx: f64, ry: f64, rotation: f64, large_arc: bool, sweep: bool, to: Point) -> Self {
        self.commands.push(PathCommand::ArcTo {
            rx,
            ry,
            rotation,
            large_arc,
            sweep,
            to,
        });
        self
    }

    pub fn close(mut self) -> Self {
        self.commands.push(PathCommand::Close);
        self
    }

    /// A closed polygon through `points`; empty for fewer than two.
    pub fn polygon(points: &[Point]) -> Self {
        if points.len() < 2 {
            return Path::new();
        }
        let mut path = Path::new().move_to(points[0]);
        for point in &points[1..] {
            path = path.line_to(*point);
        }
        path.close()
    }

    /// An open polyline through `points`.
    pub fn polyline(points: &[Point]) -> Self {
        let mut path = Path::new();
        for (index, point) in points.iter().enumerate() {
            path = if index == 0 {
                path.move_to(*point)
            } else {
                path.line_to(*point)
            };
        }
        path
    }

    /// An ellipse inscribed in `bounds`, as two arcs.
    pub fn ellipse(bounds: Rect) -> Self {
        let (rx, ry) = (bounds.width / 2.0, bounds.height / 2.0);
        let centre = bounds.centre();
        Path::new()
            .move_to(Point::new(bounds.x, centre.y))
            .arc_to(rx, ry, 0.0, false, true, Point::new(bounds.right(), centre.y))
            .arc_to(rx, ry, 0.0, false, true, Point::new(bounds.x, centre.y))
            .close()
    }

    /// A rectangle with corners rounded as SVG's `rect` rounds them for `rx`: each radius at most half its side.
    pub fn rounded_rect(bounds: Rect, radius: f64) -> Self {
        let rx = radius.max(0.0).min(bounds.width / 2.0);
        let ry = radius.max(0.0).min(bounds.height / 2.0);
        if rx <= 0.0 || ry <= 0.0 {
            return Path::polygon(&[
                Point::new(bounds.x, bounds.y),
                Point::new(bounds.right(), bounds.y),
                Point::new(bounds.right(), bounds.bottom()),
                Point::new(bounds.x, bounds.bottom()),
            ]);
        }
        let (left, top, right, bottom) = (bounds.x, bounds.y, bounds.right(), bounds.bottom());
        Path::new()
            .move_to(Point::new(left + rx, top))
            .line_to(Point::new(right - rx, top))
            .arc_to(rx, ry, 0.0, false, true, Point::new(right, top + ry))
            .line_to(Point::new(right, bottom - ry))
            .arc_to(rx, ry, 0.0, false, true, Point::new(right - rx, bottom))
            .line_to(Point::new(left + rx, bottom))
            .arc_to(rx, ry, 0.0, false, true, Point::new(left, bottom - ry))
            .line_to(Point::new(left, top + ry))
            .arc_to(rx, ry, 0.0, false, true, Point::new(left + rx, top))
            .close()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// The path moved by `(dx, dy)`.
    pub fn translated(&self, dx: f64, dy: f64) -> Path {
        let moved = |p: Point| Point::new(p.x + dx, p.y + dy);
        Path {
            commands: self
                .commands
                .iter()
                .map(|command| match *command {
                    PathCommand::MoveTo(p) => PathCommand::MoveTo(moved(p)),
                    PathCommand::LineTo(p) => PathCommand::LineTo(moved(p)),
                    PathCommand::QuadTo { control, to } => PathCommand::QuadTo {
                        control: moved(control),
                        to: moved(to),
                    },
                    PathCommand::CubicTo { first, second, to } => PathCommand::CubicTo {
                        first: moved(first),
                        second: moved(second),
                        to: moved(to),
                    },
                    PathCommand::ArcTo {
                        rx,
                        ry,
                        rotation,
                        large_arc,
                        sweep,
                        to,
                    } => PathCommand::ArcTo {
                        rx,
                        ry,
                        rotation,
                        large_arc,
                        sweep,
                        to: moved(to),
                    },
                    PathCommand::Close => PathCommand::Close,
                })
                .collect(),
        }
    }

    /// The first point the path draws from.
    pub fn start(&self) -> Option<Point> {
        self.commands.iter().find_map(|c| match c {
            PathCommand::MoveTo(p) => Some(*p),
            _ => None,
        })
    }

    /// The point the path ends on.
    pub fn end(&self) -> Option<Point> {
        self.commands.iter().rev().find_map(end_of)
    }

    /// The direction the path leaves its start in, as a vector; none for a path of no length.
    pub fn start_direction(&self) -> Option<Point> {
        let start = self.start()?;
        let next = self
            .commands
            .iter()
            .skip_while(|c| !matches!(c, PathCommand::MoveTo(_)))
            .nth(1)?;
        let towards = match *next {
            PathCommand::LineTo(p) => p,
            PathCommand::QuadTo { control, to } => first_distinct(start, &[control, to])?,
            PathCommand::CubicTo { first, second, to } => first_distinct(start, &[first, second, to])?,
            PathCommand::ArcTo { to, .. } => to,
            _ => return None,
        };
        Some(Point::new(towards.x - start.x, towards.y - start.y))
    }

    /// The direction the path arrives at its end in, as a vector; none for a path of no length.
    pub fn end_direction(&self) -> Option<Point> {
        let last = self.commands.iter().rposition(|c| end_of(c).is_some())?;
        let end = end_of(&self.commands[last])?;
        let previous = self.commands[..last].iter().rev().find_map(end_of)?;
        let from = match self.commands[last] {
            PathCommand::LineTo(_) | PathCommand::ArcTo { .. } => previous,
            PathCommand::QuadTo { control, .. } => first_distinct(end, &[control, previous])?,
            PathCommand::CubicTo { first, second, .. } => first_distinct(end, &[second, first, previous])?,
            _ => return None,
        };
        Some(Point::new(end.x - from.x, end.y - from.y))
    }

    /// A rectangle holding everything the path draws: its points and control points, and an arc's ellipse.
    pub fn bounds(&self) -> Option<Rect> {
        let mut points = Vec::new();
        let mut current = Point::default();
        for command in &self.commands {
            match *command {
                PathCommand::MoveTo(p) | PathCommand::LineTo(p) => points.push(p),
                PathCommand::QuadTo { control, to } => points.extend([control, to]),
                PathCommand::CubicTo { first, second, to } => points.extend([first, second, to]),
                PathCommand::ArcTo {
                    rx,
                    ry,
                    rotation,
                    large_arc,
                    sweep,
                    to,
                } => {
                    points.extend(arc_points(current, rx, ry, rotation, large_arc, sweep, to));
                    points.push(to);
                }
                PathCommand::Close => {}
            }
            if let Some(p) = end_of(command) {
                current = p;
            }
        }
        Rect::around(points)
    }

    /// The path as SVG path data, in the standalone canvas library's spelling.
    pub fn to_svg(&self) -> String {
        self.commands
            .iter()
            .map(|command| match *command {
                PathCommand::MoveTo(p) => format!("M {} {}", num(p.x), num(p.y)),
                PathCommand::LineTo(p) => format!("L {} {}", num(p.x), num(p.y)),
                PathCommand::QuadTo { control, to } => {
                    format!("Q {} {} {} {}", num(control.x), num(control.y), num(to.x), num(to.y))
                }
                PathCommand::CubicTo { first, second, to } => format!(
                    "C {} {}, {} {}, {} {}",
                    num(first.x),
                    num(first.y),
                    num(second.x),
                    num(second.y),
                    num(to.x),
                    num(to.y)
                ),
                PathCommand::ArcTo {
                    rx,
                    ry,
                    rotation,
                    large_arc,
                    sweep,
                    to,
                } => format!(
                    "A {} {} {} {} {} {} {}",
                    num(rx),
                    num(ry),
                    num(rotation),
                    u8::from(large_arc),
                    u8::from(sweep),
                    num(to.x),
                    num(to.y)
                ),
                PathCommand::Close => "Z".to_owned(),
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn end_of(command: &PathCommand) -> Option<Point> {
    match *command {
        PathCommand::MoveTo(p) | PathCommand::LineTo(p) => Some(p),
        PathCommand::QuadTo { to, .. } | PathCommand::CubicTo { to, .. } | PathCommand::ArcTo { to, .. } => Some(to),
        PathCommand::Close => None,
    }
}

/// Points along an SVG arc from `from` to `to`, its extremes among them to within a sample, from its centre
/// parameterisation (SVG 1.1, appendix F.6.5); the chord's ends for a degenerate arc.
fn arc_points(from: Point, rx: f64, ry: f64, rotation: f64, large_arc: bool, sweep: bool, to: Point) -> Vec<Point> {
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if rx == 0.0 || ry == 0.0 || from.distance(to) < 1e-12 {
        return vec![from, to];
    }
    let (sin, cos) = rotation.to_radians().sin_cos();
    let (dx, dy) = ((from.x - to.x) / 2.0, (from.y - to.y) / 2.0);
    let (x1, y1) = (cos * dx + sin * dy, -sin * dx + cos * dy);
    let lambda = (x1 * x1) / (rx * rx) + (y1 * y1) / (ry * ry);
    if lambda > 1.0 {
        rx *= lambda.sqrt();
        ry *= lambda.sqrt();
    }
    let numerator = (rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1).max(0.0);
    let denominator = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let mut factor = (numerator / denominator).sqrt();
    if large_arc == sweep {
        factor = -factor;
    }
    let (cx1, cy1) = (factor * rx * y1 / ry, -factor * ry * x1 / rx);
    let centre = Point::new(
        cos * cx1 - sin * cy1 + (from.x + to.x) / 2.0,
        sin * cx1 + cos * cy1 + (from.y + to.y) / 2.0,
    );
    let angle = |ux: f64, uy: f64| uy.atan2(ux);
    let start = angle((x1 - cx1) / rx, (y1 - cy1) / ry);
    let mut delta = angle((-x1 - cx1) / rx, (-y1 - cy1) / ry) - start;
    if sweep && delta < 0.0 {
        delta += 2.0 * std::f64::consts::PI;
    } else if !sweep && delta > 0.0 {
        delta -= 2.0 * std::f64::consts::PI;
    }
    const SAMPLES: usize = 32;
    (0..=SAMPLES)
        .map(|i| {
            let theta = start + delta * i as f64 / SAMPLES as f64;
            let (x, y) = (rx * theta.cos(), ry * theta.sin());
            Point::new(cos * x - sin * y + centre.x, sin * x + cos * y + centre.y)
        })
        .collect()
}

fn first_distinct(from: Point, candidates: &[Point]) -> Option<Point> {
    candidates.iter().copied().find(|p| p.distance(from) > 1e-9)
}

/// A number as JavaScript writes it in a template: the shortest text that reads back, and `0` for negative zero.
pub fn num(value: f64) -> String {
    if value == 0.0 {
        return "0".to_owned();
    }
    format!("{value}")
}

/// `value` rounded to two decimals, as the canvas library rounds path coordinates.
pub fn round2(value: f64) -> f64 {
    // Math.round(x) is floor(x + 0.5): halves go up, where Rust's round takes them away from zero.
    ((value * 100.0) + 0.5).floor() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_written_as_javascript_writes_them() {
        assert_eq!("80", num(80.0));
        assert_eq!("12.5", num(12.5));
        assert_eq!("0", num(-0.0));
        assert_eq!("-3", num(-3.0));
    }

    #[test]
    fn rounding_takes_halves_up_as_math_round_does() {
        assert_eq!(1.23, round2(1.234));
        assert_eq!(1.24, round2(1.235_000_1));
        assert_eq!(-1.23, round2(-1.234));
    }

    #[test]
    fn a_path_is_written_in_the_canvas_librarys_spelling() {
        let path = Path::new()
            .move_to(Point::new(0.0, 0.0))
            .cubic_to(Point::new(5.0, 0.0), Point::new(5.0, 10.0), Point::new(10.0, 10.0))
            .quad_to(Point::new(1.0, 2.0), Point::new(3.0, 4.0))
            .arc_to(2.0, 2.0, 0.0, false, true, Point::new(5.0, 6.0))
            .close();
        assert_eq!("M 0 0 C 5 0, 5 10, 10 10 Q 1 2 3 4 A 2 2 0 0 1 5 6 Z", path.to_svg());
    }

    #[test]
    fn a_path_knows_the_directions_it_leaves_and_arrives_in() {
        let path = Path::new().move_to(Point::new(0.0, 0.0)).cubic_to(
            Point::new(5.0, 0.0),
            Point::new(5.0, 10.0),
            Point::new(10.0, 10.0),
        );
        assert_eq!(Some(Point::new(5.0, 0.0)), path.start_direction());
        assert_eq!(Some(Point::new(5.0, 0.0)), path.end_direction());
    }

    #[test]
    fn a_paths_bounds_follow_its_arcs_rather_than_their_radii() {
        let bounds = Rect::new(10.0, 20.0, 100.0, 40.0);
        let close = |a: Rect, b: Rect| {
            (a.x - b.x).abs() < 1e-6
                && (a.y - b.y).abs() < 1e-6
                && (a.width - b.width).abs() < 1e-6
                && (a.height - b.height).abs() < 1e-6
        };
        assert!(close(bounds, Path::ellipse(bounds).bounds().unwrap()));
        assert!(close(bounds, Path::rounded_rect(bounds, 8.0).bounds().unwrap()));
    }

    #[test]
    fn rectangles_intersect_union_and_hold_points() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(10.0, 5.0, 5.0, 5.0);
        assert!(a.intersects(&b));
        assert!(!a.intersects(&Rect::new(11.0, 0.0, 1.0, 1.0)));
        assert_eq!(Rect::new(0.0, 0.0, 15.0, 10.0), a.union(&b));
        assert!(a.contains(Point::new(10.0, 10.0)));
    }
}
