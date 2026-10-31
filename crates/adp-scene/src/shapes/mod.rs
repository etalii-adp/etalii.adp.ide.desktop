//! The built-in silhouettes a node's body is drawn as (the standalone host's `BuiltInShape` in
//! `canvas/library/definition/diagramDefinition.ts` at 8b6ccfc), their one outline, and the arrow banner cut into
//! segments.

pub mod outline;
pub mod segments;

/// The built-in silhouettes of the standalone canvas library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltInShape {
    /// No body at all: the element is its labels and decorations.
    None,
    Box,
    CenteredBox,
    Ellipse,
    Frame,
    Span,
    StyledBox,
    Symbol,
    RoundedRectangle,
    Diamond,
    Hexagon,
    Pill,
    Parallelogram,
    Cylinder,
    /// A point in time: a diamond of the smaller side.
    Moment,
    /// The squircle |x/a|^4 + |y/b|^4 = 1.
    Superellipse,
    /// Narrower at the bottom than the top.
    Trapezoid,
    /// A rectangle closed on the right by a semicircle.
    Diode,
    /// A ring inside the ellipse.
    DoubleEllipse,
    /// A banner closed on the right by a point, which may be cut into segments.
    ArrowBanner,
    /// A rounded box that carries the label, with a round head above it (DISL 0.5, section 6.4).
    Person,
}

impl BuiltInShape {
    /// Every built-in shape.
    pub const ALL: [BuiltInShape; 21] = [
        BuiltInShape::None,
        BuiltInShape::Box,
        BuiltInShape::CenteredBox,
        BuiltInShape::Ellipse,
        BuiltInShape::Frame,
        BuiltInShape::Span,
        BuiltInShape::StyledBox,
        BuiltInShape::Symbol,
        BuiltInShape::RoundedRectangle,
        BuiltInShape::Diamond,
        BuiltInShape::Hexagon,
        BuiltInShape::Pill,
        BuiltInShape::Parallelogram,
        BuiltInShape::Superellipse,
        BuiltInShape::Trapezoid,
        BuiltInShape::Diode,
        BuiltInShape::Cylinder,
        BuiltInShape::Moment,
        BuiltInShape::DoubleEllipse,
        BuiltInShape::ArrowBanner,
        BuiltInShape::Person,
    ];

    /// The name the standalone library gives it.
    pub fn name(self) -> &'static str {
        match self {
            BuiltInShape::None => "none",
            BuiltInShape::Box => "box",
            BuiltInShape::CenteredBox => "centered-box",
            BuiltInShape::Ellipse => "ellipse",
            BuiltInShape::Frame => "frame",
            BuiltInShape::Span => "span",
            BuiltInShape::StyledBox => "styled-box",
            BuiltInShape::Symbol => "symbol",
            BuiltInShape::RoundedRectangle => "rounded-rectangle",
            BuiltInShape::Diamond => "diamond",
            BuiltInShape::Hexagon => "hexagon",
            BuiltInShape::Pill => "pill",
            BuiltInShape::Parallelogram => "parallelogram",
            BuiltInShape::Cylinder => "cylinder",
            BuiltInShape::Moment => "moment",
            BuiltInShape::Superellipse => "superellipse",
            BuiltInShape::Trapezoid => "trapezoid",
            BuiltInShape::Diode => "diode",
            BuiltInShape::DoubleEllipse => "double-ellipse",
            BuiltInShape::ArrowBanner => "arrow-banner",
            BuiltInShape::Person => "person",
        }
    }
}
