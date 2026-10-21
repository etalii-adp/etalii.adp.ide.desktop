//! Where a node's size is kept, and what a resize writes (etalii.adp spec 030, FR-018, task T056; DISL sections 5.8
//! and 6.9).
//!
//! The standalone host at `8b6ccfc` raised `element-resized` from its canvas (`DiagramCanvas.tsx`, the `resize`
//! gesture and `resizedBounds`) and left each module to say what a size is: the functional decomposition graph and the
//! hype cycle's notes set a width and a height property (`FdgCanvas.tsx`, `GhgCanvas.tsx`), the timeline and the hype
//! cycle's trends set the begin or the end the dragged edge now stands at (`TimelineCanvas.tsx`). Here the
//! specification says it, per axis, and a module only where its coordinates are not plain numbers:
//!
//! - `size.width` or `size.height` bound to an attribute (DISL section 6.9, a bindable number): the extent in canvas
//!   units;
//! - `placement.x2` or `placement.y2` bound to an attribute (section 5.8): the far edge, in the axis's own units;
//! - `placement.width` or `placement.height` bound to an attribute: the extent, in the axis's own units.
//!
//! A node whose notation keeps its size nowhere cannot be resized: the registration's `layout:` holds positions only
//! (FBL section 8.3), so a resize of it is refused with a sentence rather than lost.

use serde_json::Value as Json;

/// One axis of a node's stored size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Length {
    /// In canvas units (`size.width`).
    Canvas(f64),
    /// In the units of the placement's axis, scaled onto the canvas as a position is (`placement.width`, `x2 - x`).
    Domain(f64),
}

/// A node's stored size, per axis; none where it is the notation's.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Extent {
    pub width: Option<Length>,
    pub height: Option<Length>,
}

/// The attribute one axis of a node's size is kept in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SizeSource {
    /// The extent in canvas units (`size.width`, `size.height`).
    Size(String),
    /// The far edge, in the axis's units (`placement.x2`, `placement.y2`).
    Far(String),
    /// The extent in the axis's units (`placement.width`, `placement.height`).
    Span(String),
}

impl SizeSource {
    pub fn attribute(&self) -> &str {
        match self {
            SizeSource::Size(a) | SizeSource::Far(a) | SizeSource::Span(a) => a,
        }
    }
}

/// Where a node type keeps its width and its height.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SizeSources {
    pub width: Option<SizeSource>,
    pub height: Option<SizeSource>,
}

impl SizeSources {
    /// Read from a node's notation (`notation.nodes.<type>`).
    pub fn of(declared: &Json) -> SizeSources {
        let bound = |section: &str, key: &str| -> Option<String> {
            Some(declared.get(section)?.get(key)?.get("attribute")?.as_str()?.to_owned())
        };
        let axis = |size: &str, far: &str| {
            bound("size", size)
                .map(SizeSource::Size)
                .or_else(|| bound("placement", far).map(SizeSource::Far))
                .or_else(|| bound("placement", size).map(SizeSource::Span))
        };
        SizeSources {
            width: axis("width", "x2"),
            height: axis("height", "y2"),
        }
    }
}

/// The edge a resize drags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizedEdge {
    Left,
    Right,
    Top,
    Bottom,
}

impl ResizedEdge {
    /// Whether the edge changes the width.
    pub fn is_horizontal(self) -> bool {
        matches!(self, ResizedEdge::Left | ResizedEdge::Right)
    }
}

/// A finished resize: where the node now is and how large, in the units a move takes (the position) and in canvas
/// units (the size), with its far edges in the axis's units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Resize {
    pub edge: ResizedEdge,
    /// The position, as a move to it would write it.
    pub x: f64,
    pub y: f64,
    /// The right and the bottom edge, in the axis's units.
    pub right: f64,
    pub bottom: f64,
    /// The size, in canvas units.
    pub width: f64,
    pub height: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_size_binding_comes_before_a_far_edge_and_a_span() {
        let note = serde_json::json!({
            "size": { "width": { "attribute": "width" }, "height": { "attribute": "height" } },
            "placement": { "x": { "attribute": "at" }, "x2": { "attribute": "stop" } }
        });
        assert_eq!(
            SizeSources::of(&note),
            SizeSources {
                width: Some(SizeSource::Size("width".into())),
                height: Some(SizeSource::Size("height".into())),
            }
        );
        let period =
            serde_json::json!({ "placement": { "x2": { "attribute": "end" }, "height": { "attribute": "rows" } } });
        assert_eq!(
            SizeSources::of(&period),
            SizeSources {
                width: Some(SizeSource::Far("end".into())),
                height: Some(SizeSource::Span("rows".into())),
            }
        );
        assert_eq!(
            SizeSources::of(&serde_json::json!({ "size": { "default": [1, 2] } })),
            SizeSources::default()
        );
    }
}
