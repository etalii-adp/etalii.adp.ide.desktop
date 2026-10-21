//! An arrow banner cut into segments (the standalone host's `canvas/library/shapes/segments.ts` at 8b6ccfc): each
//! segment's polygon, the dividers between them, and the stretch of the top and bottom edges each one owns. One
//! geometry, read by the drawing, an attachment that names a segment and the boundary handles alike.
//!
//! The standalone reads the count and the boundaries through its payload bindings; here they arrive as numbers,
//! which the caller evaluated.

use super::outline::arrow_banner_point_depth;
use crate::connectors::Edge;
use crate::geometry::{Point, Rect};

/// How the boundaries between segments are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Divider {
    #[default]
    Chevron,
    Line,
}

/// One drawn segment: its closed polygon and the stretches of the top and bottom edges it owns.
#[derive(Debug, Clone, PartialEq)]
pub struct SegmentGeometry {
    pub index: usize,
    pub polygon: Vec<Point>,
    pub top: (f64, f64),
    pub bottom: (f64, f64),
}

/// The line between two segments: a chevron's three points, or a straight line's two.
#[derive(Debug, Clone, PartialEq)]
pub struct SegmentDivider {
    pub index: usize,
    pub x: f64,
    pub points: Vec<Point>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SegmentLayout {
    pub count: usize,
    pub segments: Vec<SegmentGeometry>,
    pub dividers: Vec<SegmentDivider>,
}

/// Where on an element a connection end sits, along one edge or one segment's stretch of it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeAttachment {
    pub edge: Edge,
    pub region: Option<usize>,
    pub at: f64,
}

/// How many segments are drawn: the count given, rounded and kept within 1..max; `max` when there is none.
pub fn segment_count_of(count: Option<f64>, max: f64) -> usize {
    let max = max.floor().max(1.0);
    match count {
        Some(raw) if raw.is_finite() => (raw.round().max(1.0)).min(max) as usize,
        _ => max as usize,
    }
}

/// The boundary fractions given, when usable for `count` segments (exactly `count - 1`, strictly increasing, strictly
/// inside 0..1); an even spread otherwise.
pub fn boundary_fractions_of(given: Option<&[f64]>, count: usize) -> Vec<f64> {
    let even: Vec<f64> = (1..count).map(|index| index as f64 / count as f64).collect();
    let Some(given) = given else {
        return even;
    };
    if given.len() + 1 != count {
        return even;
    }
    let usable = given.iter().enumerate().all(|(index, value)| {
        value.is_finite() && *value > 0.0 && *value < 1.0 && (index == 0 || *value > given[index - 1])
    });
    if usable { given.to_vec() } else { even }
}

/// The segments of a banner over `bounds`, split at `fractions` of its width.
pub fn segment_layout_of(bounds: Rect, count: usize, fractions: &[f64], divider: Divider) -> SegmentLayout {
    let depth = arrow_banner_point_depth(bounds);
    let (left, top, bottom) = (bounds.x, bounds.y, bounds.bottom());
    let middle = bounds.y + bounds.height / 2.0;
    let shoulder = bounds.right() - depth;
    let tip = bounds.right();

    let boundaries: Vec<f64> = fractions
        .iter()
        .map(|fraction| left + fraction * bounds.width)
        .collect();
    let tails: Vec<f64> = boundaries
        .iter()
        .enumerate()
        .map(|(index, at)| match divider {
            Divider::Line => at.min(shoulder),
            Divider::Chevron => {
                let previous = if index == 0 { left } else { boundaries[index - 1] };
                (at - depth.min(at - previous)).min(shoulder)
            }
        })
        .collect();

    let segments = (0..count)
        .map(|index| {
            let first = index == 0;
            let last = index + 1 == count;
            let left_tail = if first { left } else { tails[index - 1] };
            let left_tip = if first { left } else { boundaries[index - 1] };
            let right_tail = if last { shoulder } else { tails[index] };
            let right_tip = if last { tip } else { boundaries[index] };
            let mut polygon = vec![
                Point::new(left_tail, top),
                Point::new(right_tail, top),
                Point::new(right_tip, middle),
                Point::new(right_tail, bottom),
                Point::new(left_tail, bottom),
            ];
            if !first && left_tip != left_tail {
                polygon.push(Point::new(left_tip, middle));
            }
            SegmentGeometry {
                index,
                polygon,
                top: (left_tail, right_tail),
                bottom: (left_tail, right_tail),
            }
        })
        .collect();

    let dividers = boundaries
        .iter()
        .enumerate()
        .map(|(index, at)| SegmentDivider {
            index,
            x: *at,
            points: if divider == Divider::Line || tails[index] == *at {
                vec![Point::new(*at, top), Point::new(*at, bottom)]
            } else {
                vec![
                    Point::new(tails[index], top),
                    Point::new(*at, middle),
                    Point::new(tails[index], bottom),
                ]
            },
        })
        .collect();

    SegmentLayout {
        count,
        segments,
        dividers,
    }
}

/// Where a dragged boundary may come to rest: snapped to the step from `origin`, halves away from zero, then kept at
/// least one step from its neighbours.
pub fn boundary_landing(layout: &SegmentLayout, bounds: Rect, index: usize, x: f64, step: f64, origin: f64) -> f64 {
    let unit = if step > 0.0 { step } else { 1.0 };
    let previous = if index == 0 {
        bounds.x
    } else {
        layout.dividers[index - 1].x
    };
    let next = if index + 1 == layout.dividers.len() {
        bounds.right()
    } else {
        layout.dividers[index + 1].x
    };
    let steps = (x - origin) / unit;
    let rounded = if steps >= 0.0 {
        (steps + 0.5).floor()
    } else {
        -(-steps + 0.5).floor()
    };
    let snapped = origin + rounded * unit;
    let (low, high) = (previous + unit, next - unit);
    if low > high {
        (previous + next) / 2.0
    } else {
        snapped.max(low).min(high)
    }
}

/// Where an attachment lies on an element, from its current bounds and segments; a region the element does not draw
/// falls back to the whole edge.
pub fn attachment_point_of(attachment: EdgeAttachment, bounds: Rect, layout: Option<&SegmentLayout>) -> Point {
    let at = attachment.at.clamp(0.0, 1.0);
    let segment = attachment
        .region
        .and_then(|region| layout.and_then(|l| l.segments.get(region)));
    match attachment.edge {
        Edge::Top | Edge::Bottom => {
            let (from, to) = match segment {
                Some(segment) if attachment.edge == Edge::Top => segment.top,
                Some(segment) => segment.bottom,
                None => (bounds.x, bounds.right()),
            };
            let y = if attachment.edge == Edge::Top {
                bounds.y
            } else {
                bounds.bottom()
            };
            Point::new(from + at * (to - from), y)
        }
        Edge::Left => Point::new(bounds.x, bounds.y + at * bounds.height),
        Edge::Right => Point::new(bounds.right(), bounds.y + at * bounds.height),
    }
}

/// A fraction kept within 0..1 and to two decimals, which is what a document stores.
fn rounded_fraction(value: f64) -> f64 {
    (value.clamp(0.0, 1.0) * 100.0).round() / 100.0
}

/// The attachment nearest a point, on the edges an element declares.
pub fn nearest_attachment(
    point: Point,
    bounds: Rect,
    edges: &[Edge],
    layout: Option<&SegmentLayout>,
) -> Option<EdgeAttachment> {
    let mut best: Option<(EdgeAttachment, f64)> = None;
    let mut consider = |attachment: EdgeAttachment| {
        let distance = attachment_point_of(attachment, bounds, layout).distance(point);
        if best.is_none_or(|(_, known)| distance < known) {
            best = Some((attachment, distance));
        }
    };
    for edge in edges {
        let horizontal = matches!(edge, Edge::Top | Edge::Bottom);
        if horizontal && let Some(layout) = layout {
            for segment in &layout.segments {
                let (from, to) = if *edge == Edge::Top {
                    segment.top
                } else {
                    segment.bottom
                };
                let length = to - from;
                let fraction = if length > 0.0 { (point.x - from) / length } else { 0.0 };
                consider(EdgeAttachment {
                    edge: *edge,
                    region: Some(segment.index),
                    at: rounded_fraction(fraction),
                });
            }
            continue;
        }
        let fraction = if horizontal {
            if bounds.width > 0.0 {
                (point.x - bounds.x) / bounds.width
            } else {
                0.0
            }
        } else if bounds.height > 0.0 {
            (point.y - bounds.y) / bounds.height
        } else {
            0.0
        };
        consider(EdgeAttachment {
            edge: *edge,
            region: None,
            at: rounded_fraction(fraction),
        });
    }
    best.map(|(attachment, _)| attachment)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_count_is_kept_within_one_and_the_maximum() {
        assert_eq!(3, segment_count_of(Some(3.2), 5.0));
        assert_eq!(1, segment_count_of(Some(0.0), 5.0));
        assert_eq!(5, segment_count_of(Some(9.0), 5.0));
        assert_eq!(5, segment_count_of(None, 5.0));
    }

    #[test]
    fn unusable_boundaries_are_spread_evenly() {
        assert_eq!(vec![0.25, 0.6], boundary_fractions_of(Some(&[0.25, 0.6]), 3));
        assert_eq!(vec![1.0 / 3.0, 2.0 / 3.0], boundary_fractions_of(Some(&[0.6, 0.25]), 3));
        assert_eq!(vec![0.5], boundary_fractions_of(Some(&[0.2, 0.4]), 2));
    }

    #[test]
    fn a_banner_is_cut_into_chevrons_that_stay_inside_its_outline() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 40.0);
        let layout = segment_layout_of(bounds, 2, &[0.5], Divider::Chevron);
        assert_eq!(2, layout.segments.len());
        assert_eq!(
            vec![Point::new(80.0, 0.0), Point::new(100.0, 20.0), Point::new(80.0, 40.0)],
            layout.dividers[0].points
        );
        assert_eq!((0.0, 80.0), layout.segments[0].top);
        assert_eq!((80.0, 180.0), layout.segments[1].top);
        // The attachment at a segment's middle sits on the middle of its stretch.
        let at = EdgeAttachment {
            edge: Edge::Top,
            region: Some(1),
            at: 0.5,
        };
        assert_eq!(Point::new(130.0, 0.0), attachment_point_of(at, bounds, Some(&layout)));
        assert_eq!(
            Some(at),
            nearest_attachment(Point::new(130.0, -3.0), bounds, &[Edge::Top], Some(&layout))
        );
    }

    #[test]
    fn a_boundary_lands_on_the_step_and_never_beside_its_neighbours() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 40.0);
        let layout = segment_layout_of(bounds, 3, &[0.25, 0.75], Divider::Line);
        assert_eq!(60.0, boundary_landing(&layout, bounds, 0, 57.0, 10.0, 0.0));
        assert_eq!(140.0, boundary_landing(&layout, bounds, 0, 190.0, 10.0, 0.0));
        assert_eq!(10.0, boundary_landing(&layout, bounds, 0, -40.0, 10.0, 0.0));
    }
}
