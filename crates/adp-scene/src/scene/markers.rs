//! The library's arrowheads in diagram coordinates (the `<marker>` definitions of the standalone host's
//! `DiagramCanvas.tsx` at 8b6ccfc): each drawn in a 10 by 10 box whose point (9, 5) sits on the line's end, scaled by
//! its marker width over 10 times the line's width, as SVG's `markerUnits="strokeWidth"` scales it, and turned with
//! the line (`orient="auto-start-reverse"`: a start marker points back along the line).

use super::MarkerShape;
use crate::geometry::{Path, Point};
use crate::notation::catalog::MarkerKind;

/// The marker drawn at `tip`, pointing at `angle`, for a line of `stroke_width`; none for no marker.
pub fn marker_shape(kind: MarkerKind, tip: Point, angle: f64, stroke_width: f64) -> Option<MarkerShape> {
    let (width, filled): (f64, bool) = match kind {
        MarkerKind::None => return None,
        MarkerKind::Arrow => (6.0, true),
        MarkerKind::OpenArrow => (7.0, false),
        MarkerKind::Diamond => (8.0, true),
        MarkerKind::Circle => (7.0, true),
    };
    let scale = width / 10.0 * stroke_width.max(f64::MIN_POSITIVE);
    let (sin, cos) = angle.sin_cos();
    let at = |x: f64, y: f64| {
        let (lx, ly) = ((x - 9.0) * scale, (y - 5.0) * scale);
        Point::new(tip.x + lx * cos - ly * sin, tip.y + lx * sin + ly * cos)
    };
    let path = match kind {
        MarkerKind::Arrow => Path::polygon(&[at(0.0, 0.0), at(10.0, 5.0), at(0.0, 10.0)]),
        MarkerKind::OpenArrow => Path::polyline(&[at(1.0, 1.0), at(9.0, 5.0), at(1.0, 9.0)]),
        MarkerKind::Diamond => Path::polygon(&[at(1.0, 5.0), at(5.0, 1.0), at(9.0, 5.0), at(5.0, 9.0)]),
        MarkerKind::Circle => {
            let radius = 4.0 * scale;
            let centre = at(5.0, 5.0);
            Path::new()
                .move_to(Point::new(centre.x - radius, centre.y))
                .arc_to(
                    radius,
                    radius,
                    0.0,
                    false,
                    true,
                    Point::new(centre.x + radius, centre.y),
                )
                .arc_to(
                    radius,
                    radius,
                    0.0,
                    false,
                    true,
                    Point::new(centre.x - radius, centre.y),
                )
                .close()
        }
        MarkerKind::None => return None,
    };
    Some(MarkerShape {
        kind,
        tip,
        angle,
        path,
        filled,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_arrowhead_points_along_the_line_with_its_tip_just_past_the_end() {
        let marker = marker_shape(MarkerKind::Arrow, Point::new(100.0, 0.0), 0.0, 1.0).unwrap();
        let points: Vec<Point> = marker
            .path
            .commands
            .iter()
            .filter_map(|c| match c {
                crate::geometry::PathCommand::MoveTo(p) | crate::geometry::PathCommand::LineTo(p) => Some(*p),
                _ => None,
            })
            .collect();
        let close = |a: Point, b: Point| a.distance(b) < 1e-9;
        assert!(close(Point::new(94.6, -3.0), points[0]));
        assert!(close(Point::new(100.6, 0.0), points[1]));
        assert!(close(Point::new(94.6, 3.0), points[2]));
        assert!(marker.filled);
        assert!(marker_shape(MarkerKind::None, Point::new(0.0, 0.0), 0.0, 1.0).is_none());
    }

    #[test]
    fn a_marker_turns_with_its_line() {
        let marker = marker_shape(
            MarkerKind::Arrow,
            Point::new(0.0, 0.0),
            std::f64::consts::FRAC_PI_2,
            1.0,
        )
        .unwrap();
        let bounds = marker.path.bounds().unwrap();
        // Pointing down: the head lies above the tip, across x.
        assert!(bounds.y < 0.0 && bounds.x < 0.0 && bounds.right() > 0.0);
    }
}
