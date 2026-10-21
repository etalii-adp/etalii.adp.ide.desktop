//! Ornaments a node draws beside its shape, and the link symbols it carries (the standalone host's
//! `canvas/library/definition/decorations.ts` at 8b6ccfc, and `LinkSymbol` of `DiagramCanvas.tsx`): drawn, never
//! interactive, except a link, whose one press opens what it names.
//!
//! DISL gives a node two kinds of ornament the standalone compiles: its icon (section 6.14), drawn by the library's
//! own glyph for its name, and its badges (section 6.9), a mark shown while its condition holds or a link symbol
//! opening an attribute. Those are the glyphs ported here: the circle and the circular arrow.

use super::{Binding, Condition};
use crate::geometry::{Path, Point, Rect, round2};
use crate::notation::catalog::MarkerKind;

/// The glyphs a decoration draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    Circle,
    /// A near-complete circle opening at its top and sweeping into its arrowhead: a feedback loop's turn.
    CircularArrow,
}

/// What a decoration's position is measured from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DecorationOrigin {
    /// An offset from the element's centre.
    Centre { dx: f64, dy: f64 },
    /// `inset` in from the top-left or the top-right corner, both ways.
    Corner { right: bool, inset: f64 },
}

/// An ornament attached to an element.
#[derive(Debug, Clone, PartialEq)]
pub struct DecorationDeclaration {
    /// The DISL icon or badge it is drawn for.
    pub id: String,
    pub glyph: Glyph,
    pub origin: DecorationOrigin,
    pub radius: f64,
    pub marker_end: MarkerKind,
    /// A circular arrow drawn mirrored, turning anticlockwise, while this holds.
    pub mirrored: Option<Condition>,
    pub when: Option<Condition>,
    pub tooltip: Option<Binding>,
    /// The accessible name, for a reader who cannot see the mark.
    pub label: Option<String>,
}

/// A link symbol: drawn where the element has a link at its attribute, and nowhere else.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkDeclaration {
    pub id: String,
    /// The attribute holding the link.
    pub attribute: String,
    /// The symbol's top-left corner, measured from the element's top-right corner: leftwards and down.
    pub right: f64,
    pub top: f64,
}

/// The side of the square a link symbol is drawn and pressed in.
pub const LINK_SYMBOL_SIZE: f64 = 14.0;

/// A decoration's point on an element.
pub fn origin_point(origin: DecorationOrigin, bounds: Rect) -> Point {
    match origin {
        DecorationOrigin::Centre { dx, dy } => {
            let centre = bounds.centre();
            Point::new(centre.x + dx, centre.y + dy)
        }
        DecorationOrigin::Corner { right, inset } => Point::new(
            if right {
                bounds.right() - inset
            } else {
                bounds.x + inset
            },
            bounds.y + inset,
        ),
    }
}

/// The circular arrow round a centre: from the top, 1.7 of a half turn clockwise (or anticlockwise), as two arcs
/// through its middle (`circularArrowPath`), its points to two decimals.
pub fn circular_arrow_path(centre: Point, radius: f64, clockwise: bool) -> Path {
    let start = -std::f64::consts::FRAC_PI_2;
    let end = start + if clockwise { 1.0 } else { -1.0 } * std::f64::consts::PI * 1.7;
    let middle = (start + end) / 2.0;
    let at = |angle: f64| {
        Point::new(
            round2(centre.x + radius * angle.cos()),
            round2(centre.y + radius * angle.sin()),
        )
    };
    Path::new()
        .move_to(at(start))
        .arc_to(radius, radius, 0.0, false, clockwise, at(middle))
        .arc_to(radius, radius, 0.0, false, clockwise, at(end))
}

/// The chain a link symbol draws in its square at `(x, y)`.
pub fn link_glyph_path(x: f64, y: f64) -> Path {
    let p = |dx: f64, dy: f64| Point::new(x + dx, y + dy);
    Path::new()
        .move_to(p(5.5, 8.5))
        .line_to(p(8.5, 5.5))
        .move_to(p(6.2, 4.2))
        .line_to(p(7.5, 2.9))
        .arc_to(2.4, 2.4, 0.0, false, true, p(10.9, 6.3))
        .line_to(p(9.6, 7.6))
        .move_to(p(7.8, 9.8))
        .line_to(p(6.5, 11.1))
        .arc_to(2.4, 2.4, 0.0, false, true, p(3.1, 7.7))
        .line_to(p(4.4, 6.4))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::PathCommand;

    #[test]
    fn a_circular_arrow_opens_at_the_top_and_turns_the_way_it_is_asked() {
        let clockwise = circular_arrow_path(Point::new(0.0, 0.0), 10.0, true);
        assert_eq!(Some(Point::new(0.0, -10.0)), clockwise.start());
        let end = clockwise.end().unwrap();
        // 1.7 half turns from the top, clockwise in a y-down plane, ends left of the top.
        assert!(end.x < 0.0 && end.y < 0.0, "{end:?}");
        let mirrored = circular_arrow_path(Point::new(0.0, 0.0), 10.0, false);
        assert!(mirrored.end().unwrap().x > 0.0);
        assert!(matches!(mirrored.commands[1], PathCommand::ArcTo { sweep: false, .. }));
    }

    #[test]
    fn a_badge_sits_inset_from_its_corner() {
        let bounds = Rect::new(0.0, 0.0, 100.0, 40.0);
        assert_eq!(
            Point::new(91.0, 9.0),
            origin_point(
                DecorationOrigin::Corner {
                    right: true,
                    inset: 9.0
                },
                bounds
            )
        );
        assert_eq!(
            Point::new(50.0, -14.0),
            origin_point(DecorationOrigin::Centre { dx: 0.0, dy: -34.0 }, bounds)
        );
    }
}
