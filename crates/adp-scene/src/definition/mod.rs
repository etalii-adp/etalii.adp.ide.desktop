//! The diagram definition: what a notation compiles into, and what a scene is drawn from (the standalone host's
//! `canvas/library/definition/diagramDefinition.ts` at 8b6ccfc, as far as it states what is drawn).
//!
//! Plain data, as the standalone's is, with one difference that runs through all of it: where the standalone states a
//! payload path or a template, because its library has no CEL, this definition keeps the specification's own
//! [`Binding`]: an attribute, CEL, or a literal, which the scene evaluates per element. Classes do not exist here,
//! since no stylesheet paints this host: a type keeps its notation's style, which [`crate::style`] resolves.
//!
//! What the standalone definition states that is not drawing is left out: actions and their wire ids, the toolbox,
//! snapping, the layout modes, the filter box, the switches, the drop target and the connect gesture's rules beyond
//! the endpoint constraints a scene marks.

pub mod binding;
pub mod compartments;
pub mod decorations;
pub mod labels;
pub mod stub;

use adp_disl::DislSpecification;
use serde_json::{Map, Value as Json};

pub use binding::Binding;

use crate::connectors::Edge;
use crate::geometry::Point;
use crate::notation::catalog::{BuiltInRoute, MarkerKind};
use crate::shapes::BuiltInShape;
use compartments::CompartmentDeclaration;
use decorations::{DecorationDeclaration, LinkDeclaration};
use labels::LabelDeclaration;
use stub::StubDeclaration;

/// A CEL condition in the element context, as written.
pub type Condition = String;

/// A notation compiled for drawing.
#[derive(Debug, Clone)]
pub struct DiagramDefinition {
    pub specification: DislSpecification,
    /// The notation as the specification writes it, for its named styles.
    pub notation: Json,
    pub element_types: Vec<ElementType>,
    pub relation_types: Vec<RelationType>,
    /// What the canvas says when its document holds nothing to draw (DISL section 6.13 `canvas.empty`).
    pub empty_text: Option<String>,
    /// The canvas's background paint (`canvas.background`), as written.
    pub background: Option<Json>,
}

impl DiagramDefinition {
    /// The element type of the DISL node type `name`.
    pub fn element_type(&self, name: &str) -> Option<&ElementType> {
        self.element_types.iter().find(|t| t.name == name)
    }

    /// The relation type of the DISL relation `name`.
    pub fn relation_type(&self, name: &str) -> Option<&RelationType> {
        self.relation_types.iter().find(|t| t.name == name)
    }
}

/// What a node's body is drawn as.
#[derive(Debug, Clone, PartialEq)]
pub enum ShapeKind {
    BuiltIn(BuiltInShape),
    /// A custom shape of the notation (DISL section 6.8), drawn from its own geometry.
    Custom(CustomShape),
}

/// A custom shape and the parameters a node binds it with.
#[derive(Debug, Clone, PartialEq)]
pub struct CustomShape {
    pub name: String,
    /// The shape's declaration under `notation.shapes`.
    pub declaration: Json,
    /// The parameters the node's reference binds, each Bindable.
    pub params: Map<String, Json>,
}

/// Which sides a line may reach when it attaches by edge rather than at a named anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EdgeSides {
    #[default]
    All,
    Horizontal,
    Vertical,
}

/// A compass position on the bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compass {
    N,
    Ne,
    E,
    Se,
    S,
    Sw,
    W,
    Nw,
}

impl Compass {
    pub fn of(id: &str) -> Option<Compass> {
        Some(match id {
            "n" => Compass::N,
            "ne" => Compass::Ne,
            "e" => Compass::E,
            "se" => Compass::Se,
            "s" => Compass::S,
            "sw" => Compass::Sw,
            "w" => Compass::W,
            "nw" => Compass::Nw,
            _ => return None,
        })
    }

    /// The position as fractions of the bounds.
    pub fn fraction(self) -> (f64, f64) {
        match self {
            Compass::N => (0.5, 0.0),
            Compass::Ne => (1.0, 0.0),
            Compass::E => (1.0, 0.5),
            Compass::Se => (1.0, 1.0),
            Compass::S => (0.5, 1.0),
            Compass::Sw => (0.0, 1.0),
            Compass::W => (0.0, 0.5),
            Compass::Nw => (0.0, 0.0),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Compass::N => "n",
            Compass::Ne => "ne",
            Compass::E => "e",
            Compass::Se => "se",
            Compass::S => "s",
            Compass::Sw => "sw",
            Compass::W => "w",
            Compass::Nw => "nw",
        }
    }
}

/// An anchor a fraction of the way along one side, clockwise, and its name.
#[derive(Debug, Clone, PartialEq)]
pub struct SideFraction {
    pub side: Edge,
    pub at: f64,
    pub name: Option<String>,
}

/// Where lines attach to an element.
#[derive(Debug, Clone, PartialEq)]
pub enum AnchorKind {
    /// By edge intersection.
    Edge,
    Compass(Vec<Compass>),
    Sides(Vec<SideFraction>),
    /// Anywhere along the named edges, in segments where the shape has them.
    Along {
        edges: Vec<Edge>,
        segments: bool,
    },
}

/// Where lines attach to an element, and whether its anchors are drawn and take a connection.
#[derive(Debug, Clone, PartialEq)]
pub struct AnchorSet {
    pub kind: AnchorKind,
    pub edge_sides: EdgeSides,
    pub enabled: bool,
    pub visible: bool,
    /// Every end on the element is drawn by edge intersection, whatever anchor started it.
    pub drawn_by_edge: bool,
}

impl AnchorSet {
    pub fn edge() -> AnchorSet {
        AnchorSet {
            kind: AnchorKind::Edge,
            edge_sides: EdgeSides::All,
            enabled: true,
            visible: true,
            drawn_by_edge: false,
        }
    }
}

/// Where an element type's size comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sizing {
    Model,
    Content,
    User,
}

/// Which edges a user-sized element lets the reader drag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resize {
    Width,
    Both,
}

/// A node's size as its notation states it (DISL section 6.9 `size`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SizeRule {
    pub default: (Option<f64>, Option<f64>),
    pub fixed: Option<(f64, f64)>,
    pub min: (Option<f64>, Option<f64>),
    pub max: (Option<f64>, Option<f64>),
}

/// How an element reaches a screen reader.
#[derive(Debug, Clone, PartialEq)]
pub struct Accessibility {
    pub role: Option<String>,
    pub name: Option<Binding>,
}

/// One kind of thing a diagram draws boxes for.
#[derive(Debug, Clone, PartialEq)]
pub struct ElementType {
    /// The DISL node type.
    pub name: String,
    pub shape: ShapeKind,
    /// The node's style, its named styles and their `extends` merged (an empty object for none).
    pub style: Json,
    /// The node's conditional styles (DISL section 6.6): each `when`, and its style resolved.
    pub conditions: Vec<(Condition, Json)>,
    pub labels: Vec<LabelDeclaration>,
    pub decorations: Vec<DecorationDeclaration>,
    pub compartments: Vec<CompartmentDeclaration>,
    pub links: Vec<LinkDeclaration>,
    pub tooltip: Option<Binding>,
    pub accessibility: Option<Accessibility>,
    pub anchors: AnchorSet,
    pub sizing: Sizing,
    pub resize: Resize,
    pub draggable: bool,
    pub deletable: bool,
    /// Whether a toolbox drop that creates one opens its label for editing.
    pub edit_on_drop: bool,
    /// The attribute listing the members at whose centre an element with no position is drawn.
    pub centre_of: Option<String>,
    pub size: SizeRule,
}

/// The way a line is routed.
#[derive(Debug, Clone, PartialEq)]
pub enum Route {
    BuiltIn(BuiltInRoute),
    /// DISL's `bezier` with `startDirection: right` and `endDirection: left`: a horizontal bezier that loops forward
    /// and back when the target begins before the source ends.
    ForwardBezier,
    /// DISL's `curved`: a bowed arc.
    BowedArc(BowedArcDeclaration),
    /// A routing plugin (DISL section 6.10, `routing: { plugin }`), drawn by the host's plugin where it has one and by
    /// the plugin's declared fallback otherwise.
    Plugin {
        name: String,
        fallback: Box<Route>,
    },
}

/// A curved line and what rides along it.
#[derive(Debug, Clone, PartialEq)]
pub struct BowedArcDeclaration {
    pub bow: f64,
    /// The bow where a condition on the relation holds, the first that does.
    pub variants: Vec<(Condition, f64)>,
    pub self_loop_radius: Option<f64>,
    pub bars: Vec<ArcBars>,
    pub marks: Vec<ArcMark>,
}

/// Bars across the line at fractions along it, as one group.
#[derive(Debug, Clone, PartialEq)]
pub struct ArcBars {
    pub at: Vec<f64>,
    pub length: f64,
    pub when: Option<Condition>,
}

/// Text beside the line at a fraction along it.
#[derive(Debug, Clone, PartialEq)]
pub struct ArcMark {
    pub text: Binding,
    pub at: f64,
    pub left: bool,
    pub distance: f64,
}

/// Where a line's label sits along it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteLabelPlacement {
    Source,
    Midpoint,
    Target,
}

/// A line's label.
#[derive(Debug, Clone, PartialEq)]
pub struct RouteLabel {
    pub text: Binding,
    pub placement: RouteLabelPlacement,
    /// Below the line for a positive offset; the canvas draws a label 6 above the line where none is stated.
    pub offset: Option<f64>,
    pub editable: bool,
}

/// Which anchors an end may attach to.
#[derive(Debug, Clone, PartialEq)]
pub enum EndpointAnchors {
    Any,
    Edge,
    Named(Vec<String>),
}

/// Which element types one end of a relation may attach to, and on which anchors.
#[derive(Debug, Clone, PartialEq)]
pub struct EndpointConstraint {
    pub element_types: Vec<String>,
    pub anchors: EndpointAnchors,
}

/// How many of a relation a pair of elements may have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerPair {
    Ordered,
    Unordered,
}

/// The ends of a relation.
#[derive(Debug, Clone, PartialEq)]
pub struct Endpoints {
    pub source: EndpointConstraint,
    pub target: EndpointConstraint,
    pub allow_self: bool,
    pub per_pair: Option<PerPair>,
    pub max_from_source: Option<usize>,
    pub max_into_target: Option<usize>,
}

/// How one end of a drawn line attaches (DISL section 6.10 `anchoring`).
#[derive(Debug, Clone, PartialEq)]
pub enum EndAnchoring {
    /// At one of the named anchors of the end's node, the one nearest the other end where several are named.
    Fixed(Vec<String>),
    /// At the centre of the end's node.
    Center,
    /// By the edge rule of the end's node.
    Edge,
}

/// One kind of line a diagram draws.
#[derive(Debug, Clone, PartialEq)]
pub struct RelationType {
    /// The DISL relation.
    pub name: String,
    pub route: Route,
    pub start_marker: MarkerKind,
    pub end_marker: MarkerKind,
    /// The line's style: the edge's style with its `line.stroke` merged over it as `stroke`.
    pub style: Json,
    pub corner_radius: Option<f64>,
    pub label: Option<RouteLabel>,
    pub stub: Option<StubDeclaration>,
    pub selectable: bool,
    pub movable_ends: bool,
    pub hide_when_attachment_hidden: bool,
    pub endpoints: Endpoints,
    pub empty_release: bool,
    pub source_anchoring: EndAnchoring,
    pub target_anchoring: EndAnchoring,
}

/// A point on a side, for a [`SideFraction`]: `at` runs clockwise round the bounds.
pub fn side_point(side: Edge, at: f64, bounds: crate::geometry::Rect) -> Point {
    match side {
        Edge::Top => Point::new(bounds.x + at * bounds.width, bounds.y),
        Edge::Right => Point::new(bounds.right(), bounds.y + at * bounds.height),
        Edge::Bottom => Point::new(bounds.x + (1.0 - at) * bounds.width, bounds.bottom()),
        Edge::Left => Point::new(bounds.x, bounds.y + (1.0 - at) * bounds.height),
    }
}

/// The anchors an anchor set draws, by name, on `bounds` (`anchorPoints`): none for edge and along attachment.
pub fn anchor_points(anchors: &AnchorSet, bounds: crate::geometry::Rect) -> Vec<(String, Point)> {
    match &anchors.kind {
        AnchorKind::Edge | AnchorKind::Along { .. } => Vec::new(),
        AnchorKind::Compass(positions) => positions
            .iter()
            .map(|position| {
                let (fx, fy) = position.fraction();
                (
                    position.name().to_owned(),
                    Point::new(bounds.x + fx * bounds.width, bounds.y + fy * bounds.height),
                )
            })
            .collect(),
        AnchorKind::Sides(fractions) => fractions
            .iter()
            .map(|fraction| {
                let name = fraction
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("{}:{}", edge_name(fraction.side), crate::geometry::num(fraction.at)));
                (name, side_point(fraction.side, fraction.at, bounds))
            })
            .collect(),
    }
}

/// An edge's name as DISL writes it.
pub fn edge_name(edge: Edge) -> &'static str {
    match edge {
        Edge::Top => "top",
        Edge::Bottom => "bottom",
        Edge::Left => "left",
        Edge::Right => "right",
    }
}
