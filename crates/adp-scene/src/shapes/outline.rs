//! One outline per shape, and the two things computed from it (the standalone host's
//! `canvas/library/shapes/outline.ts` at 8b6ccfc): the drawing, the text region and the edge point all read the same
//! outline, so they cannot disagree. A shape with no polygon outline here (`box`, `pill`, `ellipse` and the rest)
//! answers with none, which means "use the rectangle".

use super::BuiltInShape;
use crate::geometry::{Point, Rect};

/// The inner padding between a text region and the outline, on each side.
pub const TEXT_REGION_PADDING: f64 = 6.0;

const SUPERELLIPSE_EXPONENT: f64 = 4.0;
const SUPERELLIPSE_SAMPLES: usize = 64;
const DIODE_ARC_SAMPLES: usize = 24;

/// The corner fractions of the straight-sided shapes.
fn corner_fractions(shape: BuiltInShape) -> Option<&'static [(f64, f64)]> {
    match shape {
        BuiltInShape::Diamond => Some(&[(0.5, 0.0), (1.0, 0.5), (0.5, 1.0), (0.0, 0.5)]),
        BuiltInShape::Hexagon => Some(&[
            (0.25, 0.0),
            (0.75, 0.0),
            (1.0, 0.5),
            (0.75, 1.0),
            (0.25, 1.0),
            (0.0, 0.5),
        ]),
        BuiltInShape::Parallelogram => Some(&[(0.2, 0.0), (1.0, 0.0), (0.8, 1.0), (0.0, 1.0)]),
        BuiltInShape::Trapezoid => Some(&[(0.0, 0.0), (1.0, 0.0), (0.85, 1.0), (0.15, 1.0)]),
        _ => None,
    }
}

/// The shapes this module has an outline for.
pub const OUTLINED_SHAPES: [BuiltInShape; 7] = [
    BuiltInShape::Diamond,
    BuiltInShape::Hexagon,
    BuiltInShape::Parallelogram,
    BuiltInShape::Trapezoid,
    BuiltInShape::Superellipse,
    BuiltInShape::Diode,
    BuiltInShape::ArrowBanner,
];

/// The closed outline of a shape, walked in one direction; empty for a shape with no polygon outline.
pub fn outline_of(shape: BuiltInShape, bounds: Rect) -> Vec<Point> {
    if let Some(corners) = corner_fractions(shape) {
        return corners
            .iter()
            .map(|(fx, fy)| Point::new(bounds.x + fx * bounds.width, bounds.y + fy * bounds.height))
            .collect();
    }
    match shape {
        BuiltInShape::Superellipse => {
            let (a, b) = (bounds.width / 2.0, bounds.height / 2.0);
            let (cx, cy) = (bounds.x + a, bounds.y + b);
            let power = 2.0 / SUPERELLIPSE_EXPONENT;
            (0..SUPERELLIPSE_SAMPLES)
                .map(|index| {
                    let angle = 2.0 * std::f64::consts::PI * index as f64 / SUPERELLIPSE_SAMPLES as f64;
                    let (sin, cos) = angle.sin_cos();
                    Point::new(
                        cx + js_sign(cos) * cos.abs().powf(power) * a,
                        cy + js_sign(sin) * sin.abs().powf(power) * b,
                    )
                })
                .collect()
        }
        BuiltInShape::ArrowBanner => {
            let depth = arrow_banner_point_depth(bounds);
            vec![
                Point::new(bounds.x, bounds.y),
                Point::new(bounds.right() - depth, bounds.y),
                Point::new(bounds.right(), bounds.y + bounds.height / 2.0),
                Point::new(bounds.right() - depth, bounds.bottom()),
                Point::new(bounds.x, bounds.bottom()),
            ]
        }
        BuiltInShape::Diode => {
            let radius = (bounds.height / 2.0).min(bounds.width);
            let straight_to = bounds.right() - radius;
            let centre_y = bounds.y + bounds.height / 2.0;
            let mut points = vec![Point::new(bounds.x, bounds.y), Point::new(straight_to, bounds.y)];
            points.extend((0..=DIODE_ARC_SAMPLES).map(|index| {
                let angle =
                    -std::f64::consts::FRAC_PI_2 + std::f64::consts::PI * index as f64 / DIODE_ARC_SAMPLES as f64;
                Point::new(straight_to + angle.cos() * radius, centre_y + angle.sin() * radius)
            }));
            points.push(Point::new(straight_to, bounds.bottom()));
            points.push(Point::new(bounds.x, bounds.bottom()));
            points
        }
        _ => Vec::new(),
    }
}

/// JavaScript's `Math.sign`: zero for zero.
fn js_sign(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// How far an arrow banner's point reaches back from its right edge: `min(h/2, w/2)`.
pub fn arrow_banner_point_depth(bounds: Rect) -> f64 {
    (bounds.height / 2.0).min(bounds.width / 2.0).max(0.0)
}

/// Whether a point lies inside a closed outline, by the even-odd rule; the edge counts as inside.
pub fn is_inside_outline(point: Point, outline: &[Point]) -> bool {
    if outline.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = outline.len() - 1;
    for i in 0..outline.len() {
        let (a, b) = (outline[i], outline[j]);
        if on_segment(point, a, b) {
            return true;
        }
        let crosses = (a.y > point.y) != (b.y > point.y);
        if crosses && point.x < a.x + (point.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn on_segment(point: Point, a: Point, b: Point) -> bool {
    let cross = (b.x - a.x) * (point.y - a.y) - (b.y - a.y) * (point.x - a.x);
    if cross.abs() > 1e-6 {
        return false;
    }
    point.x >= a.x.min(b.x) - 1e-6
        && point.x <= a.x.max(b.x) + 1e-6
        && point.y >= a.y.min(b.y) - 1e-6
        && point.y <= a.y.max(b.y) + 1e-6
}

fn span_at(outline: &[Point], y: f64) -> Option<(f64, f64)> {
    let mut xs = Vec::new();
    let mut j = outline.len() - 1;
    for i in 0..outline.len() {
        let (a, b) = (outline[i], outline[j]);
        if (a.y - y).abs() < 1e-9 {
            xs.push(a.x);
        }
        if (a.y < y && b.y > y) || (b.y < y && a.y > y) {
            xs.push(a.x + (y - a.y) / (b.y - a.y) * (b.x - a.x));
        }
        j = i;
    }
    if xs.is_empty() {
        return None;
    }
    Some((
        xs.iter().copied().fold(f64::INFINITY, f64::min),
        xs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    ))
}

/// The widest rectangle of `band_height`, centred vertically in the shape, whose corners lie inside the outline,
/// less [`TEXT_REGION_PADDING`] on each side: where a label goes. For a shape with no outline it is the bounds less
/// the padding; a band the shape cannot hold collapses to nothing at the centre.
pub fn text_region_of(shape: BuiltInShape, bounds: Rect, band_height: f64) -> Rect {
    let outline = outline_of(shape, bounds);
    if outline.is_empty() {
        return inset(bounds);
    }
    let centre_y = bounds.y + bounds.height / 2.0;
    let half = band_height.min(bounds.height) / 2.0;
    let (top, bottom) = (centre_y - half, centre_y + half);
    let (Some(upper), Some(lower)) = (span_at(&outline, top), span_at(&outline, bottom)) else {
        return Rect::new(bounds.x + bounds.width / 2.0, centre_y, 0.0, 0.0);
    };
    let from = upper.0.max(lower.0);
    let to = upper.1.min(lower.1);
    inset(Rect::new(from, top, (to - from).max(0.0), half * 2.0))
}

fn inset(bounds: Rect) -> Rect {
    let width = (bounds.width - 2.0 * TEXT_REGION_PADDING).max(0.0);
    let height = (bounds.height - 2.0 * TEXT_REGION_PADDING).max(0.0);
    Rect::new(
        bounds.x + (bounds.width - width) / 2.0,
        bounds.y + (bounds.height - height) / 2.0,
        width,
        height,
    )
}

/// Where the ray from the shape's centre in direction `(dx, dy)` meets its outline; none for a shape with no outline
/// here, which keeps the caller's rectangle.
pub fn outline_edge_point(shape: BuiltInShape, bounds: Rect, dx: f64, dy: f64) -> Option<Point> {
    let outline = outline_of(shape, bounds);
    if outline.len() < 3 {
        return None;
    }
    let centre = bounds.centre();
    if dx == 0.0 && dy == 0.0 {
        return Some(centre);
    }
    let mut nearest: Option<f64> = None;
    let mut j = outline.len() - 1;
    for i in 0..outline.len() {
        let (a, b) = (outline[i], outline[j]);
        j = i;
        let (ex, ey) = (b.x - a.x, b.y - a.y);
        let denominator = dx * ey - dy * ex;
        if denominator.abs() < 1e-12 {
            continue;
        }
        let (wx, wy) = (a.x - centre.x, a.y - centre.y);
        let along = (wx * ey - wy * ex) / denominator;
        let across = (wx * dy - wy * dx) / denominator;
        if along >= 0.0 && (-1e-9..=1.0 + 1e-9).contains(&across) && nearest.is_none_or(|n| along < n) {
            nearest = Some(along);
        }
    }
    nearest.map(|along| Point::new(centre.x + dx * along, centre.y + dy * along))
}

/// The four corners of a rectangle.
pub fn corners_of(bounds: Rect) -> [Point; 4] {
    [
        Point::new(bounds.x, bounds.y),
        Point::new(bounds.right(), bounds.y),
        Point::new(bounds.right(), bounds.bottom()),
        Point::new(bounds.x, bounds.bottom()),
    ]
}

#[cfg(test)]
mod tests {
    //! The standalone host's `outline.test.ts`, and the outline half of `shapeCatalog.test.ts`.
    use super::*;

    const SHARED_HEIGHT: f64 = 48.0;
    const BAND_HEIGHT: f64 = 24.0;

    fn bounds_of(width: f64) -> Rect {
        Rect::new(10.0, 20.0, width, SHARED_HEIGHT)
    }

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn the_outlined_shapes_are_built_ins_with_an_outline_and_no_other_shape_has_one() {
        for shape in OUTLINED_SHAPES {
            assert!(BuiltInShape::ALL.contains(&shape));
            assert!(outline_of(shape, bounds_of(160.0)).len() >= 3);
        }
        for shape in [
            BuiltInShape::Box,
            BuiltInShape::Pill,
            BuiltInShape::Ellipse,
            BuiltInShape::Cylinder,
            BuiltInShape::None,
        ] {
            assert!(outline_of(shape, bounds_of(160.0)).is_empty());
        }
    }

    #[test]
    fn the_existing_polygons_keep_their_corners() {
        assert_eq!(
            vec![p(110.0, 20.0), p(210.0, 44.0), p(110.0, 68.0), p(10.0, 44.0)],
            outline_of(BuiltInShape::Diamond, bounds_of(200.0))
        );
        let bounds = Rect::new(0.0, 0.0, 200.0, 60.0);
        assert_eq!(
            vec![
                p(50.0, 0.0),
                p(150.0, 0.0),
                p(200.0, 30.0),
                p(150.0, 60.0),
                p(50.0, 60.0),
                p(0.0, 30.0)
            ],
            outline_of(BuiltInShape::Hexagon, bounds)
        );
        assert_eq!(
            vec![p(40.0, 0.0), p(200.0, 0.0), p(160.0, 60.0), p(0.0, 60.0)],
            outline_of(BuiltInShape::Parallelogram, bounds)
        );
    }

    #[test]
    fn the_text_region_fits_inside_the_outline() {
        for shape in [BuiltInShape::Superellipse, BuiltInShape::Trapezoid, BuiltInShape::Diode] {
            for width in [80.0, 160.0, 400.0] {
                let bounds = bounds_of(width);
                let outline = outline_of(shape, bounds);
                let region = text_region_of(shape, bounds, BAND_HEIGHT);
                assert!(region.width > 0.0 && region.height > 0.0, "{shape:?} at {width}");
                for corner in corners_of(region) {
                    assert!(
                        is_inside_outline(corner, &outline),
                        "{corner:?} of {shape:?} at {width}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_shape_without_an_outline_gives_the_bounds_less_the_padding() {
        assert_eq!(
            Rect::new(16.0, 26.0, 148.0, 36.0),
            text_region_of(BuiltInShape::Box, bounds_of(160.0), BAND_HEIGHT)
        );
    }

    #[test]
    fn a_band_taller_than_the_shape_allows_collapses() {
        assert_eq!(
            0.0,
            text_region_of(BuiltInShape::Superellipse, bounds_of(80.0), SHARED_HEIGHT * 4.0).width
        );
    }

    #[test]
    fn the_edge_counts_as_inside_and_a_degenerate_outline_holds_nothing() {
        let outline = outline_of(BuiltInShape::Trapezoid, bounds_of(100.0));
        assert!(is_inside_outline(p(25.0, 68.0), &outline));
        assert!(is_inside_outline(p(60.0, 44.0), &outline));
        assert!(!is_inside_outline(p(12.0, 67.0), &outline));
        assert!(!is_inside_outline(p(0.0, 0.0), &[]));
    }

    /// DISL's superellipse (section 6.7): |2x/w - 1|^n + |2y/h - 1|^n, which is 1 on the curve.
    fn superellipse_level(point: Point, w: f64, h: f64, n: f64) -> f64 {
        (2.0 * point.x / w - 1.0).abs().powf(n) + (2.0 * point.y / h - 1.0).abs().powf(n)
    }

    #[test]
    fn the_superellipse_is_disls_curve_of_exponent_four_with_its_extremes_sampled() {
        for (w, h) in [(200.0, 60.0), (80.0, 48.0), (400.0, 32.0)] {
            let outline = outline_of(BuiltInShape::Superellipse, Rect::new(0.0, 0.0, w, h));
            for point in &outline {
                assert!((superellipse_level(*point, w, h, 4.0) - 1.0).abs() < 1e-9);
            }
            for extreme in [p(w, h / 2.0), p(w / 2.0, h), p(0.0, h / 2.0), p(w / 2.0, 0.0)] {
                assert!(
                    outline.iter().any(|point| point.distance(extreme) < 1e-5),
                    "{extreme:?}"
                );
            }
        }
    }

    #[test]
    fn the_edge_point_is_where_the_ray_meets_the_outline() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 60.0);
        assert_eq!(
            Some(p(200.0, 30.0)),
            outline_edge_point(BuiltInShape::Diamond, bounds, 1.0, 0.0)
        );
        assert_eq!(
            Some(p(100.0, 0.0)),
            outline_edge_point(BuiltInShape::Diamond, bounds, 0.0, -1.0)
        );
        assert_eq!(None, outline_edge_point(BuiltInShape::Box, bounds, 1.0, 0.0));
    }
}
