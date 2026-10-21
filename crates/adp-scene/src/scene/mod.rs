//! A scene: a flat, ordered list of drawing primitives in diagram coordinates, painted first to last, and its culling
//! to a viewport. What the standalone host's `DiagramCanvas.tsx` (at 8b6ccfc) renders as SVG, as data a shell paints
//! with its own toolkit: filled and stroked paths with their outline geometry, text runs with their font, alignment and
//! wrapping box, lines with their route and their arrowheads, and the hooks the shell's gestures are hit-tested
//! against (anchors, resize handles, edge strips, a line's wide hit path).
//!
//! Every primitive names what it belongs to ([`Owner`]) and what part of it it is ([`Role`]), so a shell can find an
//! element's body for a selection outline or a line's label for its inline editor without knowing the notation.

mod builder;
mod markers;

pub use builder::{
    RouteEnds, RoutePlugins, SceneOptions, StandardRoutePlugins, attachment_point, build_scene, default_size,
};
pub use markers::marker_shape;

use crate::connectors::Edge;
use crate::definition::labels::TextAnchor;
use crate::geometry::{Path, Point, Rect};
use crate::notation::catalog::MarkerKind;
use crate::style::{Colour, Font, Stroke};

/// What a primitive belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Owner {
    /// The canvas itself: its background.
    Canvas,
    /// A node, by its id.
    Element(String),
    /// A relation, by its id.
    Relation(String),
}

/// What part of its owner a primitive is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Role {
    Background,
    /// A node's body.
    Body,
    /// The second part of a body that has one: a double ellipse's inner ring, a cylinder's caps.
    BodyPart,
    /// One part of a custom shape (DISL section 6.8 `parts`), by its id.
    Part(String),
    /// One segment of an arrow banner.
    Segment(usize),
    /// The divider after a segment.
    Divider(usize),
    /// A label's line, by the label's id and the line's index.
    Label {
        id: String,
        line: usize,
    },
    /// A decoration (an icon or a badge), by its id.
    Decoration(String),
    /// A link symbol, by its id.
    Link(String),
    /// A list's heading, by the list's id and the group's key.
    CompartmentHeading {
        compartment: String,
        key: String,
    },
    /// A list's row, by the list's id and the row's id.
    CompartmentRow {
        compartment: String,
        row: Option<String>,
    },
    /// A line, with its arrowheads.
    Line,
    /// A line's label.
    LineLabel,
    /// The bars across a curved line.
    Bars,
    /// A mark beside a curved line.
    Mark,
}

/// A filled and stroked path.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapePrimitive {
    pub owner: Owner,
    pub role: Role,
    pub path: Path,
    pub fill: Option<Colour>,
    pub stroke: Option<Stroke>,
    pub opacity: f64,
    pub tooltip: Option<String>,
}

/// A run of text on one line: `position` is the anchor point on its baseline.
#[derive(Debug, Clone, PartialEq)]
pub struct TextRun {
    pub owner: Owner,
    pub role: Role,
    pub text: String,
    pub position: Point,
    pub anchor: TextAnchor,
    pub font: Font,
    /// The box a wrapped text was laid out in.
    pub wrap: Option<Rect>,
    pub tooltip: Option<String>,
    /// Whether the inline editor may open over it.
    pub editable: bool,
}

/// An arrowhead or other end decoration, in diagram coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct MarkerShape {
    pub kind: MarkerKind,
    /// The end of the line it sits on.
    pub tip: Point,
    /// The direction it points, in radians, clockwise from the positive x axis (y runs down).
    pub angle: f64,
    pub path: Path,
    /// Filled with the line's colour; otherwise stroked with it.
    pub filled: bool,
}

/// A line between two elements, or a stub out of one.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgePrimitive {
    pub owner: Owner,
    pub path: Path,
    pub stroke: Stroke,
    pub start: Option<MarkerShape>,
    pub end: Option<MarkerShape>,
    /// The width of the invisible line a press is hit-tested against.
    pub hit_width: f64,
    pub selectable: bool,
    pub tooltip: Option<String>,
}

/// What a hook is for.
#[derive(Debug, Clone, PartialEq)]
pub enum HookKind {
    /// A named anchor a connection starts from, and whether its dot is drawn at rest.
    Anchor { name: String, visible: bool },
    /// A stretch of edge a connection attaches along.
    EdgeStrip { edge: Edge, region: Option<usize> },
    /// The edge a selected, user-sized element is resized by.
    ResizeHandle(Edge),
    /// The outline drawn round a selected element.
    Selection,
    /// A list's heading, which a press folds or unfolds.
    Heading { compartment: String, key: String },
    /// A list's row, which a press selects.
    Row { compartment: String, row: Option<String> },
    /// A link symbol, which a press opens.
    Link { link: String },
}

/// Where the shell hit-tests a gesture, or draws chrome: not part of the notation's drawing.
#[derive(Debug, Clone, PartialEq)]
pub struct Hook {
    pub owner: Owner,
    pub kind: HookKind,
    pub area: Rect,
}

/// One drawing primitive.
#[derive(Debug, Clone, PartialEq)]
pub enum Primitive {
    Shape(ShapePrimitive),
    Text(TextRun),
    Edge(EdgePrimitive),
    Hook(Hook),
}

impl Primitive {
    pub fn owner(&self) -> &Owner {
        match self {
            Primitive::Shape(s) => &s.owner,
            Primitive::Text(t) => &t.owner,
            Primitive::Edge(e) => &e.owner,
            Primitive::Hook(h) => &h.owner,
        }
    }

    /// A rectangle holding everything the primitive draws, for culling: a stroke's half width and a marker
    /// included, and a text's measured box estimated from its font.
    pub fn bounds(&self, metrics: &dyn crate::text::TextMetrics) -> Option<Rect> {
        match self {
            Primitive::Shape(s) => {
                let half = s.stroke.as_ref().map(|k| k.width / 2.0).unwrap_or(0.0);
                s.path.bounds().map(|b| b.inflate(half))
            }
            Primitive::Text(t) => {
                if let Some(wrap) = t.wrap {
                    return Some(wrap);
                }
                let line = crate::definition::labels::LaidOutLabel {
                    text: t.text.clone(),
                    x: t.position.x,
                    y: t.position.y,
                    anchor: t.anchor,
                    font: t.font.clone(),
                    editable: false,
                    tooltip: None,
                    region: None,
                    declaration_index: 0,
                    line_index: 0,
                };
                Some(crate::definition::labels::label_box_of(metrics, &line))
            }
            Primitive::Edge(e) => {
                let half = e.hit_width.max(e.stroke.width) / 2.0;
                let mut bounds = e.path.bounds()?.inflate(half);
                for marker in [&e.start, &e.end].into_iter().flatten() {
                    if let Some(marked) = marker.path.bounds() {
                        bounds = bounds.union(&marked);
                    }
                }
                Some(bounds)
            }
            Primitive::Hook(h) => Some(h.area),
        }
    }

    /// The role of a shape, a text or a line.
    pub fn role(&self) -> Option<&Role> {
        match self {
            Primitive::Shape(s) => Some(&s.role),
            Primitive::Text(t) => Some(&t.role),
            Primitive::Edge(_) => Some(&Role::Line),
            Primitive::Hook(_) => None,
        }
    }
}

/// A diagram drawn as primitives, painted in order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Scene {
    pub primitives: Vec<Primitive>,
    /// What the canvas says when the diagram holds nothing to draw.
    pub empty_text: Option<String>,
}

impl Scene {
    /// A rectangle holding everything drawn but the background: what fitting the view to the diagram fits.
    pub fn bounds(&self, metrics: &dyn crate::text::TextMetrics) -> Option<Rect> {
        self.primitives
            .iter()
            .filter(|p| p.owner() != &Owner::Canvas)
            .filter_map(|p| p.bounds(metrics))
            .reduce(|a, b| a.union(&b))
    }

    /// The primitives an owner has, in painting order.
    pub fn of<'a>(&'a self, owner: &Owner) -> impl Iterator<Item = &'a Primitive> + 'a {
        let owner = owner.clone();
        self.primitives.iter().filter(move |p| *p.owner() == owner)
    }

    /// The scene less what lies wholly outside `viewport`; the canvas's background is always kept, and order is kept.
    pub fn cull(&self, viewport: Rect, metrics: &dyn crate::text::TextMetrics) -> Scene {
        Scene {
            primitives: self
                .primitives
                .iter()
                .filter(|p| p.owner() == &Owner::Canvas || p.bounds(metrics).is_some_and(|b| b.intersects(&viewport)))
                .cloned()
                .collect(),
            empty_text: self.empty_text.clone(),
        }
    }
}

/// The scene less what lies wholly outside `viewport` (see [`Scene::cull`]).
pub fn cull(scene: &Scene, viewport: Rect, metrics: &dyn crate::text::TextMetrics) -> Scene {
    scene.cull(viewport, metrics)
}
