//! A relation without a target, drawn as a stub (DISL sections 4.9 and 6.10 *Stubs*; the standalone host's
//! `canvas/library/definition/stub.ts` at 8b6ccfc): a straight line of its length out of the middle of one side of
//! its source, ending nowhere, with its label beside it.

use super::Binding;
use crate::connectors::Edge;
use crate::definition::labels::TextAnchor;
use crate::geometry::{Point, Rect};

/// How far off its line a stub's label sits where it does not say: where a line's label sits.
const STUB_LABEL_DISTANCE: f64 = 6.0;

/// How far past the source a stub's label begins where it does not say.
const STUB_LABEL_INSET: f64 = 8.0;

/// The height of a line's text, by which a label below a downward stub clears its end.
const STUB_TEXT_HEIGHT: f64 = 11.0;

/// A relation's stub.
#[derive(Debug, Clone, PartialEq)]
pub struct StubDeclaration {
    pub length: f64,
    pub side: Edge,
    /// The stub's own style (its `extends` resolved), drawn in place of its relation's.
    pub style: Option<serde_json::Value>,
    pub label: Option<StubLabel>,
}

/// A stub's label.
#[derive(Debug, Clone, PartialEq)]
pub struct StubLabel {
    pub text: Binding,
    pub below: bool,
    pub distance: Option<f64>,
    pub inset: Option<f64>,
}

/// Where a stub is drawn: its line, and where its label's text is anchored.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StubGeometry {
    pub from: Point,
    pub to: Point,
    pub label: (Point, TextAnchor),
}

/// A stub's line out of the middle of one side of its source, and its label's anchor: sideways, the label runs along
/// the line from its inset, above or below it; upwards or downwards, it is centred beyond the free end.
pub fn stub_geometry(source: Rect, stub: &StubDeclaration) -> StubGeometry {
    let centre = source.centre();
    let distance = stub
        .label
        .as_ref()
        .and_then(|l| l.distance)
        .unwrap_or(STUB_LABEL_DISTANCE);
    let inset = stub.label.as_ref().and_then(|l| l.inset).unwrap_or(STUB_LABEL_INSET);
    let below = stub.label.as_ref().is_some_and(|l| l.below);
    let off = if below { distance + STUB_TEXT_HEIGHT } else { -distance };
    match stub.side {
        Edge::Right => {
            let from = Point::new(source.right(), centre.y);
            StubGeometry {
                from,
                to: Point::new(from.x + stub.length, centre.y),
                label: (Point::new(from.x + inset, centre.y + off), TextAnchor::Start),
            }
        }
        Edge::Left => {
            let from = Point::new(source.x, centre.y);
            StubGeometry {
                from,
                to: Point::new(from.x - stub.length, centre.y),
                label: (Point::new(from.x - inset, centre.y + off), TextAnchor::End),
            }
        }
        Edge::Top => {
            let from = Point::new(centre.x, source.y);
            let to = Point::new(centre.x, from.y - stub.length);
            StubGeometry {
                from,
                to,
                label: (Point::new(centre.x, to.y - distance), TextAnchor::Middle),
            }
        }
        Edge::Bottom => {
            let from = Point::new(centre.x, source.bottom());
            let to = Point::new(centre.x, from.y + stub.length);
            StubGeometry {
                from,
                to,
                label: (
                    Point::new(centre.x, to.y + distance + STUB_TEXT_HEIGHT),
                    TextAnchor::Middle,
                ),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stub(side: Edge, below: bool) -> StubDeclaration {
        StubDeclaration {
            length: 48.0,
            side,
            style: None,
            label: Some(StubLabel {
                text: Binding::Literal("missing".to_owned()),
                below,
                distance: None,
                inset: None,
            }),
        }
    }

    #[test]
    fn a_stub_leaves_the_middle_of_its_side_with_its_label_along_or_beyond_it() {
        let source = Rect::new(0.0, 0.0, 100.0, 40.0);
        let right = stub_geometry(source, &stub(Edge::Right, false));
        assert_eq!(
            (Point::new(100.0, 20.0), Point::new(148.0, 20.0)),
            (right.from, right.to)
        );
        assert_eq!((Point::new(108.0, 14.0), TextAnchor::Start), right.label);
        let left = stub_geometry(source, &stub(Edge::Left, true));
        assert_eq!(Point::new(-48.0, 20.0), left.to);
        assert_eq!((Point::new(-8.0, 37.0), TextAnchor::End), left.label);
        let down = stub_geometry(source, &stub(Edge::Bottom, false));
        assert_eq!(
            (Point::new(50.0, 88.0), Point::new(50.0, 105.0)),
            (down.to, down.label.0)
        );
        let up = stub_geometry(source, &stub(Edge::Top, false));
        assert_eq!((Point::new(50.0, -48.0), Point::new(50.0, -54.0)), (up.to, up.label.0));
    }
}
