//! What a diagram draws, as a scene (the parts of the standalone host's `DiagramCanvas.tsx` at 8b6ccfc that compute
//! what is drawn: `LibraryElement`, `renderShapeBody`, `declaredDecorations`, `declaredLabels`,
//! `declaredCompartments`, `LinkSymbol`, `LibraryConnection`, `routePath`, `attachmentPoint`, `elementBounds` and the
//! painting order of `DiagramCanvasCore`), with React, events and the DOM left out.
//!
//! The painting order is the standalone's: the background, then every line, then every element, frames first; within
//! an element its body, its decorations, its labels, its lists and its links, then the hooks a shell hit-tests; within
//! a line its stroke with its arrowheads, its label, and what rides along it.
//!
//! An element is drawn where the host's positions put it: a rectangle per element id, from the definition's placement
//! or the host's layout, which a scene does not compute. An element with no position is not drawn, unless its type
//! places it at the centre of the members it lists. A line is drawn when both its ends are, or as its stub where it
//! has no target and its relation states one.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

use adp_cel::Value;
use adp_disl::expressions::contexts;
use adp_disl::{DislDiagram, DislElement};
use serde_json::Value as Json;

use super::markers::marker_shape;
use super::{EdgePrimitive, Hook, HookKind, Owner, Primitive, Role, Scene, ShapePrimitive, TextRun};
use crate::connectors::{
    Axis, ConnectorBox, Edge, HorizontalSide, arc_path, edge_point_of, facing_anchors_between, forward_bezier_path,
    horizontal_bezier_path, midpoint_of, orthogonal_path, polyline_path, quadratic_bezier_path, side_anchor_of,
    spline_path, straight_path,
};
use crate::definition::binding::holds;
use crate::definition::compartments::{CompartmentContext, Folding, compartments_height, layout_compartments};
use crate::definition::decorations::{Glyph, LINK_SYMBOL_SIZE, circular_arrow_path, link_glyph_path, origin_point};
use crate::definition::labels::{LabelContext, TextAnchor, layout_labels};
use crate::definition::stub::stub_geometry;
use crate::definition::{
    AnchorKind, DiagramDefinition, EdgeSides, ElementType, EndAnchoring, RelationType, Resize, Route,
    RouteLabelPlacement, ShapeKind, Sizing, anchor_points,
};
use crate::evaluation::{Evaluator, Vars};
use crate::geometry::{Path, Point, Rect};
use crate::notation::bowed_arc::{SELF_LOOP_RADIUS, arc_between, bar_across, mark_beside};
use crate::notation::catalog::BuiltInRoute;
use crate::notation::geometry_expressions::{GeomScope, evaluate_condition, path_of, resolve_box, resolve_path};
use crate::shapes::BuiltInShape;
use crate::shapes::outline::{outline_edge_point, outline_of};
use crate::style::{Colour, Font, ResolvedStyle, Stroke, StyleResolver, Theme, merged_per_key, style_of};
use crate::text::TextMetrics;

/// The width and height an element has where nothing else says (`DEFAULT_WIDTH` and `DEFAULT_HEIGHT`).
const DEFAULT_WIDTH: f64 = 120.0;
const DEFAULT_HEIGHT: f64 = 40.0;

/// The width of a line's invisible hit path (`.canvas-connection-hit`).
const HIT_WIDTH: f64 = 14.0;

/// The size of a line's text (`.canvas-connection text`).
const LINE_FONT_SIZE: f64 = 11.0;

/// Where a line's label sits off its line where the relation states no offset: above it.
const LINE_LABEL_OFFSET: f64 = -6.0;

/// How far an edge strip reaches either side of its edge, and a resize handle's thickness.
const EDGE_STRIP_REACH: f64 = 4.0;
const RESIZE_HANDLE: f64 = 6.0;

/// The radius of an anchor's dot and press target.
const ANCHOR_RADIUS: f64 = 5.0;

/// The stroke a highlighted outline and line take (`HIGHLIGHT_STROKE`, `HIGHLIGHT_STROKE_WIDTH`).
const HIGHLIGHT_TOKEN: &str = "color.selected";
const HIGHLIGHT_VALUE: &str = "#7c3aed";
const HIGHLIGHT_STROKE_WIDTH: f64 = 3.0;

/// The two end boxes a line runs between.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RouteEnds {
    pub source: Rect,
    pub target: Rect,
}

/// The routing plugins a host draws (DISL section 6.10 `routing: { plugin }`): the path from `from` to `to` between
/// the end boxes, or none where the host has no such plugin, and the plugin's fallback routing is drawn.
pub trait RoutePlugins {
    fn route(&self, name: &str, from: Point, to: Point, ends: &RouteEnds) -> Option<Path>;
}

/// No routing plugins: every plugin routing is drawn by its fallback.
impl RoutePlugins for () {
    fn route(&self, _name: &str, _from: Point, _to: Point, _ends: &RouteEnds) -> Option<Path> {
        None
    }
}

/// The routing plugins the standalone host ships: `net.etalii.adp.generic.dependencyCurve`, the dependency graph's
/// curve (the standalone's `dependencyRoute`), a horizontal bezier between the facing sides while the dependency sits
/// clear to the right, and a forward loop out of the dependent's right into the dependency's left when it sits behind.
/// A tool description names the same curve by its capability, `routing.forwardBezier` (ATD §13.3).
#[derive(Debug, Clone, Copy, Default)]
pub struct StandardRoutePlugins;

/// The names `StandardRoutePlugins` draws the dependency graph's curve for: the plugin's, and the capability's.
const FORWARD_BEZIER: [&str; 2] = ["net.etalii.adp.generic.dependencyCurve", "routing.forwardBezier"];

impl RoutePlugins for StandardRoutePlugins {
    fn route(&self, name: &str, _from: Point, _to: Point, ends: &RouteEnds) -> Option<Path> {
        if !FORWARD_BEZIER.contains(&name) {
            return None;
        }
        let from = ConnectorBox::of(ends.source);
        let to = ConnectorBox::of(ends.target);
        let loops_back = ends.target.x < ends.source.right();
        if loops_back {
            let (a, b) = (
                side_anchor_of(from, HorizontalSide::Right),
                side_anchor_of(to, HorizontalSide::Left),
            );
            Some(forward_bezier_path(a, b))
        } else {
            let (a, b) = facing_anchors_between(from, to);
            Some(horizontal_bezier_path(a, b))
        }
    }
}

/// What a scene is drawn with besides the definition and the diagram.
pub struct SceneOptions<'a> {
    /// The ids of the selected elements and relations.
    pub selection: &'a [String],
    /// The headings the reader folded or opened.
    pub folding: &'a Folding,
    pub metrics: &'a dyn TextMetrics,
    pub plugins: &'a dyn RoutePlugins,
    /// What the view shows, by id, when something is drawn for one view of the diagram; none draws everything.
    pub shown: Option<&'a HashSet<String>>,
}

/// The size an element of `element_type` has where its position gives none: its fixed size, else its default with the
/// canvas's 120 by 40 for what that leaves open, kept within its minimum and maximum.
pub fn default_size(element_type: &ElementType) -> (f64, f64) {
    if let Some(fixed) = element_type.size.fixed {
        return fixed;
    }
    let rule = element_type.size;
    let clamp = |value: f64, min: Option<f64>, max: Option<f64>| {
        value.max(min.unwrap_or(f64::MIN)).min(max.unwrap_or(f64::MAX))
    };
    (
        clamp(rule.default.0.unwrap_or(DEFAULT_WIDTH), rule.min.0, rule.max.0),
        clamp(rule.default.1.unwrap_or(DEFAULT_HEIGHT), rule.min.1, rule.max.1),
    )
}

/// One element, placed.
struct Placed<'d> {
    element: DislElement,
    element_type: &'d ElementType,
    bounds: Rect,
    vars: Vars,
}

/// The built-in a type draws, for its text region and its outline; a custom shape reads as a box.
fn shape_of(element_type: &ElementType) -> BuiltInShape {
    match &element_type.shape {
        ShapeKind::BuiltIn(shape) => *shape,
        ShapeKind::Custom(_) => BuiltInShape::Box,
    }
}

/// Where a line touches an element (`attachmentPoint`): at the named anchor where it names one the element has, else
/// on the side facing `towards` where the element attaches sideways, else where the line from its centre towards
/// `towards` crosses its outline, else its box.
pub fn attachment_point(element_type: &ElementType, bounds: Rect, anchor: Option<&str>, towards: Point) -> Point {
    let anchors = &element_type.anchors;
    if let Some(anchor) = anchor
        && !anchors.drawn_by_edge
        && let Some((_, point)) = anchor_points(anchors, bounds)
            .into_iter()
            .find(|(name, _)| name == anchor)
    {
        return point;
    }
    let centre = bounds.centre();
    let box_ = ConnectorBox::of(bounds);
    match anchors.edge_sides {
        EdgeSides::Horizontal => {
            let side = if towards.x >= centre.x {
                HorizontalSide::Right
            } else {
                HorizontalSide::Left
            };
            return side_anchor_of(box_, side);
        }
        EdgeSides::Vertical => {
            let y = if towards.y >= centre.y {
                bounds.bottom()
            } else {
                bounds.y
            };
            return Point::new(centre.x, y);
        }
        EdgeSides::All => {}
    }
    let (dx, dy) = (towards.x - centre.x, towards.y - centre.y);
    let shape = shape_of(element_type);
    if let Some(point) = outline_edge_point(shape, bounds, dx, dy) {
        return point;
    }
    if anchors.drawn_by_edge && shape == BuiltInShape::Ellipse && (dx != 0.0 || dy != 0.0) {
        let (a, b) = (bounds.width / 2.0, bounds.height / 2.0);
        let scale = 1.0 / ((dx * dx) / (a * a) + (dy * dy) / (b * b)).sqrt();
        return Point::new(centre.x + dx * scale, centre.y + dy * scale);
    }
    edge_point_of(box_, dx, dy)
}

/// The selected colour: the theme's `color.selected` in the active mode, else the standalone host's violet.
fn highlight(theme: &Theme) -> Colour {
    Colour {
        token: Some(HIGHLIGHT_TOKEN.to_owned()),
        value: Some(
            theme
                .text_of(HIGHLIGHT_TOKEN)
                .unwrap_or_else(|| HIGHLIGHT_VALUE.to_owned()),
        ),
    }
}

/// The paint a highlighted outline takes: the selected colour, heavier than the notation's and never lighter.
fn highlighted(theme: &Theme, stroke: Option<Stroke>) -> Stroke {
    let declared = stroke.as_ref().map(|s| s.width);
    Stroke {
        colour: Some(highlight(theme)),
        width: declared
            .map(|w| (w + 1.0).max(HIGHLIGHT_STROKE_WIDTH))
            .unwrap_or(HIGHLIGHT_STROKE_WIDTH),
        dash: stroke.map(|s| s.dash).unwrap_or_default(),
        opacity: 1.0,
    }
}

struct Builder<'a> {
    definition: &'a DiagramDefinition,
    evaluator: &'a Evaluator,
    options: &'a SceneOptions<'a>,
    primitives: Vec<Primitive>,
}

/// The scene of `diagram` as `definition` draws it, with its elements at `positions` (corner-based rectangles by
/// element id).
pub fn build_scene(
    definition: &DiagramDefinition,
    evaluator: &Evaluator,
    diagram: &DislDiagram,
    positions: &HashMap<String, Rect>,
    options: &SceneOptions<'_>,
) -> Scene {
    build_scene_pieces(definition, evaluator, diagram, positions, options, None).scene(
        definition,
        evaluator,
        diagram,
        options.metrics,
    )
}

/// A scene in its pieces: what each line and each element draws, in painting order, with the box each element was
/// drawn in. Kept so that the scene of the same diagram with some of its elements elsewhere, as a drag or a resize in
/// flight previews it, is built again from the pieces that changed alone (etalii.adp spec 030, SC-006): a line is
/// drawn from its two ends' boxes and an element from its own, so the others are what they were.
#[derive(Debug, Clone, Default)]
pub struct ScenePieces {
    pieces: Vec<Piece>,
    /// The box each element was drawn in.
    bounds: HashMap<String, Rect>,
}

/// What one line or one element draws, and the number it was drawn under: a piece taken over from earlier pieces
/// keeps its number, so what a host derived from it holds as long as the number does.
#[derive(Debug, Clone)]
struct Piece {
    serial: u64,
    owner: Owner,
    primitives: Vec<Primitive>,
}

/// The number the next piece drawn is drawn under.
static NEXT_PIECE: AtomicU64 = AtomicU64::new(0);

impl ScenePieces {
    /// The scene of these pieces: the background, where the definition draws one round what the pieces draw, then
    /// the pieces in their order.
    pub fn scene(
        &self,
        definition: &DiagramDefinition,
        evaluator: &Evaluator,
        diagram: &DislDiagram,
        metrics: &dyn TextMetrics,
    ) -> Scene {
        let mut primitives: Vec<Primitive> =
            Vec::with_capacity(self.pieces.iter().map(|p| p.primitives.len()).sum::<usize>() + 1);
        for piece in &self.pieces {
            primitives.extend(piece.primitives.iter().cloned());
        }
        if let Some(background) = &definition.background {
            let vars = evaluator.diagram_vars(diagram);
            let resolver = StyleResolver { evaluator, vars: &vars };
            let content = primitives
                .iter()
                .filter_map(|p| p.bounds(metrics))
                .reduce(|a, b| a.union(&b))
                .unwrap_or_default()
                .inflate(40.0);
            if let Some(fill) = resolver.paint(background) {
                primitives.insert(
                    0,
                    Primitive::Shape(ShapePrimitive {
                        owner: Owner::Canvas,
                        role: Role::Background,
                        path: Path::polygon(&[
                            Point::new(content.x, content.y),
                            Point::new(content.right(), content.y),
                            Point::new(content.right(), content.bottom()),
                            Point::new(content.x, content.bottom()),
                        ]),
                        fill: Some(fill),
                        stroke: None,
                        opacity: 1.0,
                        tooltip: None,
                    }),
                );
            }
        }
        Scene {
            primitives,
            empty_text: if diagram.nodes().is_empty() {
                definition.empty_text.clone()
            } else {
                None
            },
        }
    }

    /// Each piece in painting order, the background left out: the number it was drawn under and what it draws. A
    /// piece keeps its number while it is taken over unchanged, and one drawn again gets a new one.
    pub fn iter(&self) -> impl Iterator<Item = (u64, &[Primitive])> {
        self.pieces.iter().map(|p| (p.serial, p.primitives.as_slice()))
    }
}

/// The pieces of the scene of `diagram` as `definition` draws it, with its elements at `positions`. Where `previous`
/// holds the pieces of the same diagram, definition, evaluator and options with other positions, an element drawn in
/// the box it was drawn in there and a line whose two ends are are taken from it rather than drawn again.
pub fn build_scene_pieces(
    definition: &DiagramDefinition,
    evaluator: &Evaluator,
    diagram: &DislDiagram,
    positions: &HashMap<String, Rect>,
    options: &SceneOptions<'_>,
    previous: Option<ScenePieces>,
) -> ScenePieces {
    let mut builder = Builder {
        definition,
        evaluator,
        options,
        primitives: Vec::new(),
    };
    let placed = builder.place(diagram, positions);
    let by_id: HashMap<String, usize> = placed.iter().enumerate().map(|(i, p)| (p.element.id(), i)).collect();
    let bounds: HashMap<String, Rect> = placed.iter().map(|p| (p.element.id(), p.bounds)).collect();
    // The piece `previous` has at `index`, where it is `owner`'s: the pieces come in the same order for the same
    // diagram.
    let (mut earlier_pieces, earlier_bounds) = match previous {
        Some(p) => (p.pieces, Some(p.bounds)),
        None => (Vec::new(), None),
    };
    let mut earlier = |index: usize, owner: &Owner| {
        earlier_pieces
            .get_mut(index)
            .filter(|p| &p.owner == owner)
            .map(|p| (p.serial, std::mem::take(&mut p.primitives)))
    };
    let fresh = |builder: &mut Builder| {
        (
            NEXT_PIECE.fetch_add(1, Ordering::Relaxed),
            std::mem::take(&mut builder.primitives),
        )
    };
    let unmoved = |id: Option<String>| {
        earlier_bounds.as_ref().is_some_and(|p| match id {
            Some(id) => bounds.get(&id) == p.get(&id),
            None => true,
        })
    };
    let mut pieces: Vec<Piece> = Vec::new();

    for relation in diagram.relations() {
        // A relation a budget cuts, or one an end of which it cuts, is not drawn (DISL section 3.2.1).
        if !diagram.is_drawn(&relation) || options.shown.is_some_and(|shown| !shown.contains(&relation.id())) {
            continue;
        }
        let owner = Owner::Relation(relation.id());
        let reused = (unmoved(relation.source().map(|e| e.id())) && unmoved(relation.target().map(|e| e.id())))
            .then(|| earlier(pieces.len(), &owner))
            .flatten();
        let (serial, primitives) = match reused {
            Some(piece) => piece,
            None => {
                builder.relation(&relation, &placed, &by_id);
                fresh(&mut builder)
            }
        };
        pieces.push(Piece {
            serial,
            owner,
            primitives,
        });
    }
    // Frames first, so enclosures draw under their members; then the rest in model order.
    let (frames, others): (Vec<&Placed>, Vec<&Placed>) = placed
        .iter()
        .partition(|p| p.element_type.shape == ShapeKind::BuiltIn(BuiltInShape::Frame));
    for element in frames.into_iter().chain(others) {
        let id = element.element.id();
        let owner = Owner::Element(id.clone());
        let reused = unmoved(Some(id)).then(|| earlier(pieces.len(), &owner)).flatten();
        let (serial, primitives) = match reused {
            Some(piece) => piece,
            None => {
                builder.element(element);
                fresh(&mut builder)
            }
        };
        pieces.push(Piece {
            serial,
            owner,
            primitives,
        });
    }
    ScenePieces { pieces, bounds }
}

impl<'a> Builder<'a> {
    fn selected(&self, id: &str) -> bool {
        self.options.selection.iter().any(|s| s == id)
    }

    fn resolver<'v>(&'v self, vars: &'v Vars) -> StyleResolver<'v> {
        StyleResolver {
            evaluator: self.evaluator,
            vars,
        }
    }

    // ---- placing ---------------------------------------------------------------------------------------------------

    /// Every node the definition draws, at its position: its height at least what its lists need, and an element with
    /// no position of its own at the centre of the members it lists, where its type says so.
    fn place(&self, diagram: &DislDiagram, positions: &HashMap<String, Rect>) -> Vec<Placed<'a>> {
        let mut placed: Vec<Placed<'a>> = Vec::new();
        let mut centred: Vec<(DislElement, &'a ElementType, String)> = Vec::new();
        for element in diagram.nodes() {
            let Some(element_type) = self.definition.element_type(&element.type_name()) else {
                continue;
            };
            // A node a truncating budget cuts is not drawn (DISL section 3.2.1).
            if !diagram.is_drawn(&element) || self.options.shown.is_some_and(|shown| !shown.contains(&element.id())) {
                continue;
            }
            let vars = self.evaluator.element_vars(&element);
            match positions.get(&element.id()) {
                Some(rect) => {
                    let context = CompartmentContext {
                        evaluator: self.evaluator,
                        vars: &vars,
                        folding: self.options.folding,
                        metrics: self.options.metrics,
                    };
                    let rows = compartments_height(&element_type.compartments, &context).unwrap_or(0.0);
                    let bounds = Rect::new(rect.x, rect.y, rect.width, rect.height.max(rows));
                    placed.push(Placed {
                        element,
                        element_type,
                        bounds,
                        vars,
                    });
                }
                None => {
                    if let Some(attribute) = &element_type.centre_of {
                        centred.push((element, element_type, attribute.clone()));
                    }
                }
            }
        }
        for (element, element_type, attribute) in centred {
            let members: Vec<String> = match element.value_of(&attribute) {
                Value::List(items) => items
                    .iter()
                    .filter_map(|item| match DislElement::from_value(item) {
                        Some(member) => Some(member.id()),
                        None => item.as_str().map(str::to_owned),
                    })
                    .collect(),
                _ => Vec::new(),
            };
            let centres: Vec<Point> = members
                .iter()
                .filter_map(|id| placed.iter().find(|p| &p.element.id() == id).map(|p| p.bounds.centre()))
                .collect();
            if centres.is_empty() || centres.len() != members.len() {
                continue;
            }
            let n = centres.len() as f64;
            let centre = Point::new(
                centres.iter().map(|c| c.x).sum::<f64>() / n,
                centres.iter().map(|c| c.y).sum::<f64>() / n,
            );
            let (width, height) = default_size(element_type);
            let vars = self.evaluator.element_vars(&element);
            placed.push(Placed {
                element,
                element_type,
                bounds: Rect::centred(centre, width, height),
                vars,
            });
        }
        placed
    }

    // ---- elements --------------------------------------------------------------------------------------------------

    /// The element's style: its type's, with every conditional style that holds merged over it.
    fn element_style(&self, placed: &Placed) -> ResolvedStyle {
        let mut style = placed.element_type.style.clone();
        for (when, conditional) in &placed.element_type.conditions {
            if holds(self.evaluator, Some(when), contexts::ELEMENT, &placed.vars) {
                style = merged_per_key(&style, conditional);
            }
        }
        self.resolver(&placed.vars).resolve(Some(&style), true)
    }

    fn shape(
        &mut self,
        owner: &Owner,
        role: Role,
        path: Path,
        style: &ResolvedStyle,
        stroke: Option<Stroke>,
        tooltip: Option<String>,
    ) {
        self.primitives.push(Primitive::Shape(ShapePrimitive {
            owner: owner.clone(),
            role,
            path,
            fill: style.fill.clone(),
            stroke,
            opacity: style.opacity,
            tooltip,
        }));
    }

    fn element(&mut self, placed: &Placed) {
        let id = placed.element.id();
        let owner = Owner::Element(id.clone());
        let element_type = placed.element_type;
        let bounds = placed.bounds;
        let selected = self.selected(&id);
        let style = self.element_style(placed);
        let stroke = if selected {
            Some(highlighted(self.evaluator.theme(), style.stroke.clone()))
        } else {
            style.stroke.clone()
        };
        let tooltip = element_type
            .tooltip
            .as_ref()
            .and_then(|t| t.element_text(self.evaluator, &placed.vars));

        self.body(placed, &owner, &style, stroke, tooltip);
        self.decorations(placed, &owner, &style);
        self.labels(placed, &owner, &style);
        self.compartments(placed, &owner, &style);
        self.links(placed, &owner, &style);

        if let AnchorKind::Along { edges, .. } = &element_type.anchors.kind {
            for edge in edges {
                let area = match edge {
                    Edge::Top => Rect::new(
                        bounds.x,
                        bounds.y - EDGE_STRIP_REACH,
                        bounds.width,
                        2.0 * EDGE_STRIP_REACH,
                    ),
                    Edge::Bottom => Rect::new(
                        bounds.x,
                        bounds.bottom() - EDGE_STRIP_REACH,
                        bounds.width,
                        2.0 * EDGE_STRIP_REACH,
                    ),
                    Edge::Left => Rect::new(
                        bounds.x - EDGE_STRIP_REACH,
                        bounds.y,
                        2.0 * EDGE_STRIP_REACH,
                        bounds.height,
                    ),
                    Edge::Right => Rect::new(
                        bounds.right() - EDGE_STRIP_REACH,
                        bounds.y,
                        2.0 * EDGE_STRIP_REACH,
                        bounds.height,
                    ),
                };
                self.hook(
                    &owner,
                    HookKind::EdgeStrip {
                        edge: *edge,
                        region: None,
                    },
                    area,
                );
            }
        }
        if selected {
            self.hook(&owner, HookKind::Selection, bounds);
            if element_type.sizing == Sizing::User {
                let half = RESIZE_HANDLE / 2.0;
                self.hook(
                    &owner,
                    HookKind::ResizeHandle(Edge::Left),
                    Rect::new(bounds.x - half, bounds.y, RESIZE_HANDLE, bounds.height),
                );
                self.hook(
                    &owner,
                    HookKind::ResizeHandle(Edge::Right),
                    Rect::new(bounds.right() - half, bounds.y, RESIZE_HANDLE, bounds.height),
                );
                if element_type.resize == Resize::Both {
                    self.hook(
                        &owner,
                        HookKind::ResizeHandle(Edge::Top),
                        Rect::new(bounds.x, bounds.y - half, bounds.width, RESIZE_HANDLE),
                    );
                    self.hook(
                        &owner,
                        HookKind::ResizeHandle(Edge::Bottom),
                        Rect::new(bounds.x, bounds.bottom() - half, bounds.width, RESIZE_HANDLE),
                    );
                }
            }
        }
        for (name, point) in anchor_points(&element_type.anchors, bounds) {
            self.hook(
                &owner,
                HookKind::Anchor {
                    name,
                    visible: element_type.anchors.visible,
                },
                Rect::centred(point, 2.0 * ANCHOR_RADIUS, 2.0 * ANCHOR_RADIUS),
            );
        }
    }

    fn hook(&mut self, owner: &Owner, kind: HookKind, area: Rect) {
        self.primitives.push(Primitive::Hook(Hook {
            owner: owner.clone(),
            kind,
            area,
        }));
    }

    /// The body (`renderShapeBody`): the built-in's silhouette, or the custom shape's own geometry.
    fn body(
        &mut self,
        placed: &Placed,
        owner: &Owner,
        style: &ResolvedStyle,
        stroke: Option<Stroke>,
        tooltip: Option<String>,
    ) {
        let bounds = placed.bounds;
        let centre = bounds.centre();
        let shape = match &placed.element_type.shape {
            ShapeKind::Custom(custom) => {
                if !self.custom_body(placed, owner, style, custom, stroke.clone(), tooltip.clone()) {
                    // One bad shape cannot blank the canvas: it is drawn as the fallback box (`fallbackBox`).
                    let fallback = ResolvedStyle {
                        fill: None,
                        ..style.clone()
                    };
                    let dashed = Stroke {
                        colour: Some(Colour::value("currentColor")),
                        width: 1.0,
                        dash: vec![4.0, 2.0],
                        opacity: 1.0,
                    };
                    self.shape(
                        owner,
                        Role::Body,
                        Path::rounded_rect(bounds, 0.0),
                        &fallback,
                        Some(dashed),
                        tooltip,
                    );
                }
                return;
            }
            ShapeKind::BuiltIn(shape) => *shape,
        };
        let radius = style.corner_radius;
        let path = match shape {
            BuiltInShape::None => return,
            BuiltInShape::Moment => {
                let r = (bounds.width.min(bounds.height) / 2.0).max(2.0);
                Path::polygon(&[
                    Point::new(centre.x - r, centre.y),
                    Point::new(centre.x, centre.y - r),
                    Point::new(centre.x + r, centre.y),
                    Point::new(centre.x, centre.y + r),
                ])
            }
            BuiltInShape::DoubleEllipse => {
                self.shape(owner, Role::Body, Path::ellipse(bounds), style, stroke.clone(), tooltip);
                let inner = Rect::centred(
                    centre,
                    2.0 * (bounds.width / 2.0 - 4.0).max(1.0),
                    2.0 * (bounds.height / 2.0 - 4.0).max(1.0),
                );
                self.shape(owner, Role::BodyPart, Path::ellipse(inner), style, stroke, None);
                return;
            }
            BuiltInShape::Box | BuiltInShape::Span | BuiltInShape::StyledBox | BuiltInShape::Frame => {
                // The library's box rounds its corners by 4 where the style states no radius (`BoxElement`).
                Path::rounded_rect(bounds, radius.unwrap_or(4.0))
            }
            BuiltInShape::RoundedRectangle => {
                let param = self.shape_param(placed, "radius");
                Path::rounded_rect(bounds, param.or(radius).unwrap_or(8.0))
            }
            BuiltInShape::Person => {
                // The head first, so that the body covers it where they overlap (DISL 0.5, *The person shape*).
                let head = Rect::centred(Point::new(centre.x, bounds.y - 8.0), 20.0, 20.0);
                self.shape(owner, Role::BodyPart, Path::ellipse(head), style, stroke.clone(), None);
                let param = self.shape_param(placed, "radius");
                let radius = param.or(radius).unwrap_or(8.0);
                Path::rounded_rect(bounds, radius.min(bounds.width.min(bounds.height) / 2.0))
            }
            BuiltInShape::Pill => Path::rounded_rect(bounds, bounds.height / 2.0),
            BuiltInShape::CenteredBox => Path::rounded_rect(bounds, 6.0),
            BuiltInShape::Ellipse => Path::ellipse(bounds),
            BuiltInShape::Symbol => {
                let r = (bounds.width.min(bounds.height) / 2.0).max(2.0);
                Path::ellipse(Rect::centred(centre, 2.0 * r, 2.0 * r))
            }
            BuiltInShape::Cylinder => {
                let body = Rect::new(bounds.x, bounds.y + 6.0, bounds.width, bounds.height - 12.0);
                self.shape(
                    owner,
                    Role::Body,
                    Path::rounded_rect(body, 0.0),
                    style,
                    stroke.clone(),
                    tooltip,
                );
                for y in [bounds.y + 6.0, bounds.bottom() - 6.0] {
                    let cap = Rect::centred(Point::new(centre.x, y), bounds.width, 12.0);
                    self.shape(owner, Role::BodyPart, Path::ellipse(cap), style, stroke.clone(), None);
                }
                return;
            }
            BuiltInShape::Diamond
            | BuiltInShape::Hexagon
            | BuiltInShape::Parallelogram
            | BuiltInShape::Trapezoid
            | BuiltInShape::Superellipse
            | BuiltInShape::Diode
            | BuiltInShape::ArrowBanner => Path::polygon(&outline_of(shape, bounds)),
        };
        self.shape(owner, Role::Body, path, style, stroke, tooltip);
    }

    /// A shape parameter the node's reference binds, evaluated for the element.
    fn shape_param(&self, placed: &Placed, name: &str) -> Option<f64> {
        let reference = self
            .definition
            .notation
            .get("nodes")
            .and_then(|n| n.get(&placed.element_type.name))
            .and_then(|n| n.get("shape"))?;
        let value = reference.get("params").and_then(|p| p.get(name))?;
        self.resolver(&placed.vars).number(value)
    }

    /// A custom shape drawn from its geometry: its path or outline, and each of its parts whose `when` holds. False
    /// where it states none or an expression of it fails.
    fn custom_body(
        &mut self,
        placed: &Placed,
        owner: &Owner,
        style: &ResolvedStyle,
        custom: &crate::definition::CustomShape,
        stroke: Option<Stroke>,
        tooltip: Option<String>,
    ) -> bool {
        let bounds = placed.bounds;
        let mut scope = GeomScope::new(bounds.width, bounds.height);
        scope.element = Some(placed.element.to_value());
        let resolver = self.resolver(&placed.vars);
        for (name, declared) in custom
            .declaration
            .get("params")
            .and_then(Json::as_object)
            .into_iter()
            .flatten()
        {
            if let Some(default) = declared.get("default").and_then(|d| resolver.bound(d)) {
                scope.p.insert(name.clone(), default);
            }
        }
        for (name, value) in &custom.params {
            if let Some(value) = resolver.bound(value) {
                scope.p.insert(name.clone(), value);
            }
        }
        let origin = Point::new(bounds.x, bounds.y);
        let outline = custom
            .declaration
            .get("outline")
            .or_else(|| custom.declaration.get("path"))
            .and_then(|o| o.get("segments"))
            .and_then(Json::as_array);
        let parts = custom.declaration.get("parts").and_then(Json::as_array);
        if outline.is_none() && parts.is_none() {
            return false;
        }
        let mut drawn: Vec<Primitive> = Vec::new();
        if let Some(segments) = outline {
            let Ok(resolved) = resolve_path(self.evaluator, segments, &scope) else {
                return false;
            };
            drawn.push(Primitive::Shape(ShapePrimitive {
                owner: owner.clone(),
                role: Role::Body,
                path: path_of(&resolved, origin),
                fill: style.fill.clone(),
                stroke: stroke.clone(),
                opacity: style.opacity,
                tooltip: tooltip.clone(),
            }));
        }
        for part in parts.into_iter().flatten() {
            let Ok(shown) = evaluate_condition(self.evaluator, part.get("when"), &scope) else {
                return false;
            };
            if !shown {
                continue;
            }
            let part_id = part.get("id").and_then(Json::as_str).unwrap_or("").to_owned();
            let (x, y, w, h) = match part.get("box") {
                Some(box_) => match resolve_box(self.evaluator, box_, &scope) {
                    Ok(resolved) => resolved,
                    Err(_) => return false,
                },
                None => (0.0, 0.0, bounds.width, bounds.height),
            };
            let area = Rect::new(origin.x + x, origin.y + y, w, h);
            let path = match part.get("shape") {
                Some(Json::Object(o)) => {
                    let segments = o.get("path").and_then(|p| p.get("segments")).and_then(Json::as_array);
                    let mut inner = GeomScope::new(w, h);
                    inner.p = scope.p.clone();
                    inner.element = scope.element.clone();
                    match segments.map(|s| resolve_path(self.evaluator, s, &inner)) {
                        Some(Ok(resolved)) => path_of(&resolved, Point::new(area.x, area.y)),
                        _ => return false,
                    }
                }
                Some(Json::String(name)) => {
                    match crate::notation::catalog::library_shape_of(&Json::String(name.clone())) {
                        Ok(Some(BuiltInShape::Ellipse)) => Path::ellipse(area),
                        Ok(Some(BuiltInShape::Box)) | Ok(Some(BuiltInShape::None)) | Ok(None) | Err(_) => {
                            Path::rounded_rect(area, 0.0)
                        }
                        Ok(Some(BuiltInShape::RoundedRectangle)) => Path::rounded_rect(area, 8.0),
                        Ok(Some(BuiltInShape::Pill)) => Path::rounded_rect(area, area.height / 2.0),
                        Ok(Some(shape)) => Path::polygon(&outline_of(shape, area)),
                    }
                }
                _ => Path::rounded_rect(area, 0.0),
            };
            let part_style = part
                .get("style")
                .and_then(|s| style_of(&self.definition.notation, s))
                .map(|s| resolver.resolve(Some(&s), true));
            let part_style = part_style.as_ref().unwrap_or(style);
            drawn.push(Primitive::Shape(ShapePrimitive {
                owner: owner.clone(),
                role: Role::Part(part_id),
                path,
                fill: part_style.fill.clone(),
                stroke: part_style.stroke.clone(),
                opacity: part_style.opacity,
                tooltip: part
                    .get("tooltip")
                    .and_then(crate::definition::Binding::of)
                    .and_then(|t| t.element_text(self.evaluator, &placed.vars)),
            }));
        }
        self.primitives.extend(drawn);
        true
    }

    /// The ornaments a type declares (`declaredDecorations`), each while its condition holds.
    fn decorations(&mut self, placed: &Placed, owner: &Owner, style: &ResolvedStyle) {
        let colour = style.stroke.as_ref().and_then(|s| s.colour.clone());
        for decoration in &placed.element_type.decorations {
            if !holds(
                self.evaluator,
                decoration.when.as_deref(),
                contexts::ELEMENT,
                &placed.vars,
            ) {
                continue;
            }
            let at = origin_point(decoration.origin, placed.bounds);
            let tooltip = decoration
                .tooltip
                .as_ref()
                .and_then(|t| t.element_text(self.evaluator, &placed.vars))
                .or_else(|| decoration.label.clone());
            let role = Role::Decoration(decoration.id.clone());
            match decoration.glyph {
                Glyph::Circle => {
                    let path = Path::ellipse(Rect::centred(at, 2.0 * decoration.radius, 2.0 * decoration.radius));
                    self.primitives.push(Primitive::Shape(ShapePrimitive {
                        owner: owner.clone(),
                        role,
                        path,
                        fill: colour.clone(),
                        stroke: None,
                        opacity: 1.0,
                        tooltip,
                    }));
                }
                Glyph::CircularArrow => {
                    let mirrored = holds(
                        self.evaluator,
                        decoration.mirrored.as_deref().or(Some("false")),
                        contexts::ELEMENT,
                        &placed.vars,
                    );
                    let path = circular_arrow_path(at, decoration.radius, !mirrored);
                    let line = Stroke {
                        colour: colour.clone(),
                        width: 1.5,
                        dash: Vec::new(),
                        opacity: 1.0,
                    };
                    let head = path.end_direction().and_then(|d| {
                        marker_shape(
                            decoration.marker_end,
                            path.end().unwrap_or(at),
                            d.y.atan2(d.x),
                            line.width,
                        )
                    });
                    self.primitives.push(Primitive::Shape(ShapePrimitive {
                        owner: owner.clone(),
                        role: role.clone(),
                        path,
                        fill: None,
                        stroke: Some(line),
                        opacity: 1.0,
                        tooltip,
                    }));
                    if let Some(head) = head {
                        self.primitives.push(Primitive::Shape(ShapePrimitive {
                            owner: owner.clone(),
                            role,
                            path: head.path,
                            fill: colour.clone(),
                            stroke: None,
                            opacity: 1.0,
                            tooltip: None,
                        }));
                    }
                }
            }
        }
    }

    /// The lines a type's labels declare (`declaredLabels`), each in the element's font with the label's own over it.
    fn labels(&mut self, placed: &Placed, owner: &Owner, style: &ResolvedStyle) {
        let resolver = self.resolver(&placed.vars);
        let fonts = |declaration: &crate::definition::labels::LabelDeclaration| -> Font {
            resolver.font(
                declaration.style.as_ref().and_then(|s| s.get("font")),
                style.font.clone(),
            )
        };
        let context = LabelContext {
            evaluator: self.evaluator,
            vars: &placed.vars,
            bounds: placed.bounds,
            shape: shape_of(placed.element_type),
            metrics: self.options.metrics,
        };
        let lines = layout_labels(&placed.element_type.labels, &context, &fonts);
        for line in lines {
            let declaration = &placed.element_type.labels[line.declaration_index];
            self.primitives.push(Primitive::Text(TextRun {
                owner: owner.clone(),
                role: Role::Label {
                    id: declaration.id.clone(),
                    line: line.line_index,
                },
                text: line.text,
                position: Point::new(line.x, line.y),
                anchor: line.anchor,
                font: line.font,
                wrap: line.region,
                tooltip: line.tooltip,
                editable: line.editable,
            }));
        }
    }

    /// A node's lists (`declaredCompartments`): each heading, and each row of an open group, with its link.
    fn compartments(&mut self, placed: &Placed, owner: &Owner, style: &ResolvedStyle) {
        if placed.element_type.compartments.is_empty() {
            return;
        }
        let context = CompartmentContext {
            evaluator: self.evaluator,
            vars: &placed.vars,
            folding: self.options.folding,
            metrics: self.options.metrics,
        };
        let laid_out = layout_compartments(&placed.element_type.compartments, &context, placed.bounds);
        let font = Font {
            size: crate::text::LABEL_FONT_SIZE,
            ..style.font.clone()
        };
        for heading in laid_out.headings {
            let glyph = if heading.collapsed { "\u{25B8}" } else { "\u{25BE}" };
            self.hook(
                owner,
                HookKind::Heading {
                    compartment: heading.compartment_id.clone(),
                    key: heading.key.clone(),
                },
                heading.bounds,
            );
            self.primitives.push(Primitive::Text(TextRun {
                owner: owner.clone(),
                role: Role::CompartmentHeading {
                    compartment: heading.compartment_id,
                    key: heading.key,
                },
                text: format!("{glyph} {} ({})", heading.title, heading.count),
                position: Point::new(
                    heading.bounds.x,
                    heading.bounds.y + heading.bounds.height / 2.0 + font.size / 3.0,
                ),
                anchor: TextAnchor::Start,
                font: font.clone(),
                wrap: None,
                tooltip: None,
                editable: false,
            }));
        }
        for row in laid_out.rows {
            self.hook(
                owner,
                HookKind::Row {
                    compartment: row.compartment_id.clone(),
                    row: row.id.clone(),
                },
                row.bounds,
            );
            let trimmed = row.text != row.full_text;
            self.primitives.push(Primitive::Text(TextRun {
                owner: owner.clone(),
                role: Role::CompartmentRow {
                    compartment: row.compartment_id.clone(),
                    row: row.id.clone(),
                },
                text: row.text,
                position: Point::new(row.text_x, row.bounds.y + row.bounds.height / 2.0 + font.size / 3.0),
                anchor: TextAnchor::Start,
                font: font.clone(),
                wrap: None,
                tooltip: trimmed.then_some(row.full_text),
                editable: false,
            }));
            if let Some(link) = row.link {
                let x = row.bounds.right() - LINK_SYMBOL_SIZE;
                let y = row.bounds.y + (row.bounds.height - LINK_SYMBOL_SIZE) / 2.0;
                self.link_symbol(
                    owner,
                    &format!("{}-{}", row.compartment_id, row.id.unwrap_or_default()),
                    x,
                    y,
                    &link,
                    style,
                );
            }
        }
    }

    fn link_symbol(&mut self, owner: &Owner, id: &str, x: f64, y: f64, link: &str, style: &ResolvedStyle) {
        self.hook(
            owner,
            HookKind::Link { link: link.to_owned() },
            Rect::new(x, y, LINK_SYMBOL_SIZE, LINK_SYMBOL_SIZE),
        );
        self.primitives.push(Primitive::Shape(ShapePrimitive {
            owner: owner.clone(),
            role: Role::Link(id.to_owned()),
            path: link_glyph_path(x, y),
            fill: None,
            stroke: Some(Stroke {
                colour: style.font.colour.clone(),
                width: 1.2,
                dash: Vec::new(),
                opacity: 1.0,
            }),
            opacity: 1.0,
            tooltip: Some(format!("Open {link}")),
        }));
    }

    /// The link symbols a type carries, each where the element has a link.
    fn links(&mut self, placed: &Placed, owner: &Owner, style: &ResolvedStyle) {
        for declaration in &placed.element_type.links {
            let link = crate::definition::binding::text_of(&placed.element.value_of(&declaration.attribute));
            if link.is_empty() {
                continue;
            }
            let x = placed.bounds.right() - declaration.right;
            let y = placed.bounds.y + declaration.top;
            self.link_symbol(owner, &declaration.id, x, y, &link, style);
        }
    }

    // ---- relations -------------------------------------------------------------------------------------------------

    /// The anchor an end is drawn at: the relation's fixed anchor for it, the one nearest the other end where it names
    /// several the element has.
    fn end_anchor(anchoring: &EndAnchoring, element_type: &ElementType, bounds: Rect, other: Point) -> Option<String> {
        let EndAnchoring::Fixed(ids) = anchoring else {
            return None;
        };
        let points = anchor_points(&element_type.anchors, bounds);
        ids.iter()
            .filter_map(|id| points.iter().find(|(name, _)| name == id))
            .min_by(|a, b| a.1.distance(other).total_cmp(&b.1.distance(other)))
            .map(|(name, _)| name.clone())
    }

    fn end_point(anchoring: &EndAnchoring, placed: &Placed, other: &Placed) -> Point {
        if *anchoring == EndAnchoring::Center {
            return placed.bounds.centre();
        }
        let towards = other.bounds.centre();
        let anchor = Self::end_anchor(anchoring, placed.element_type, placed.bounds, towards);
        attachment_point(placed.element_type, placed.bounds, anchor.as_deref(), towards)
    }

    fn line_stroke(&self, relation_type: &RelationType, vars: &Vars, style: Option<&Json>) -> (Stroke, ResolvedStyle) {
        let resolved = self.resolver(vars).resolve(style.or(Some(&relation_type.style)), false);
        let stroke = resolved.stroke.clone().unwrap_or(Stroke {
            colour: Some(Colour::token(self.evaluator.theme(), "color.border")),
            width: 1.0,
            dash: Vec::new(),
            opacity: 1.0,
        });
        (stroke, resolved)
    }

    fn line_font(&self, resolved: &ResolvedStyle, selected: bool) -> Font {
        let mut font = Font {
            size: LINE_FONT_SIZE,
            ..resolved.font.clone()
        };
        if selected {
            font.colour = Some(highlight(self.evaluator.theme()));
        }
        font
    }

    fn relation(&mut self, relation: &DislElement, placed: &[Placed], by_id: &HashMap<String, usize>) {
        let Some(relation_type) = self.definition.relation_type(&relation.type_name()) else {
            return;
        };
        let id = relation.id();
        let owner = Owner::Relation(id.clone());
        let selected = self.selected(&id);
        let vars = self.evaluator.element_vars(relation);
        let source = relation.source().and_then(|s| by_id.get(&s.id())).map(|i| &placed[*i]);
        let target = relation.target().and_then(|t| by_id.get(&t.id())).map(|i| &placed[*i]);

        if relation.target().is_none()
            && let (Some(stub), Some(source)) = (&relation_type.stub, source)
        {
            let geometry = stub_geometry(source.bounds, stub);
            let (mut stroke, resolved) = self.line_stroke(
                relation_type,
                &vars,
                stub.style
                    .as_ref()
                    .map(|s| {
                        // The stub's own style, over its relation's.
                        merged_per_key(&relation_type.style, s)
                    })
                    .as_ref(),
            );
            if selected {
                stroke = highlighted(self.evaluator.theme(), Some(stroke));
            }
            self.primitives.push(Primitive::Edge(EdgePrimitive {
                owner: owner.clone(),
                path: straight_path(geometry.from, geometry.to),
                stroke,
                start: None,
                end: None,
                hit_width: HIT_WIDTH,
                selectable: relation_type.selectable,
                tooltip: None,
            }));
            if let Some(text) = stub
                .label
                .as_ref()
                .and_then(|l| l.text.element_text(self.evaluator, &vars))
                .filter(|t| !t.is_empty())
            {
                self.primitives.push(Primitive::Text(TextRun {
                    owner,
                    role: Role::LineLabel,
                    text,
                    position: geometry.label.0,
                    anchor: geometry.label.1,
                    font: self.line_font(&resolved, selected),
                    wrap: None,
                    tooltip: None,
                    editable: false,
                }));
            }
            return;
        }

        let (Some(source), Some(target)) = (source, target) else {
            return;
        };
        let from = Self::end_point(&relation_type.source_anchoring, source, target);
        let to = Self::end_point(&relation_type.target_anchoring, target, source);
        let ends = RouteEnds {
            source: source.bounds,
            target: target.bounds,
        };
        let (path, from, to, arc) = self.route(&relation_type.route, relation_type, from, to, &ends, &vars);
        let (mut stroke, resolved) = self.line_stroke(relation_type, &vars, None);
        if selected {
            stroke = highlighted(self.evaluator.theme(), Some(stroke));
        }
        let start = path
            .start_direction()
            .and_then(|d| marker_shape(relation_type.start_marker, from, (-d.y).atan2(-d.x), stroke.width));
        let end = path
            .end_direction()
            .and_then(|d| marker_shape(relation_type.end_marker, to, d.y.atan2(d.x), stroke.width));
        self.primitives.push(Primitive::Edge(EdgePrimitive {
            owner: owner.clone(),
            path,
            stroke: stroke.clone(),
            start,
            end,
            hit_width: HIT_WIDTH,
            selectable: relation_type.selectable,
            tooltip: None,
        }));

        if let Some(label) = &relation_type.label
            && let Some(text) = label.text.element_text(self.evaluator, &vars).filter(|t| !t.is_empty())
        {
            let at = match label.placement {
                RouteLabelPlacement::Source => from,
                RouteLabelPlacement::Target => to,
                RouteLabelPlacement::Midpoint => midpoint_of(from, to),
            };
            self.primitives.push(Primitive::Text(TextRun {
                owner: owner.clone(),
                role: Role::LineLabel,
                text,
                position: Point::new(at.x, at.y + label.offset.unwrap_or(LINE_LABEL_OFFSET)),
                anchor: TextAnchor::Middle,
                font: self.line_font(&resolved, selected),
                wrap: None,
                tooltip: None,
                editable: label.editable,
            }));
        }

        if let (Route::BowedArc(declaration), Some(arc)) = (&relation_type.route, arc) {
            for bars in &declaration.bars {
                if !holds(self.evaluator, bars.when.as_deref(), contexts::ELEMENT, &vars) {
                    continue;
                }
                let mut path = Path::new();
                for t in &bars.at {
                    let (a, b) = bar_across(&arc, *t, bars.length);
                    path = path.move_to(a).line_to(b);
                }
                self.primitives.push(Primitive::Shape(ShapePrimitive {
                    owner: owner.clone(),
                    role: Role::Bars,
                    path,
                    fill: None,
                    stroke: Some(stroke.clone()),
                    opacity: 1.0,
                    tooltip: None,
                }));
            }
            for mark in &declaration.marks {
                let Some(text) = mark.text.element_text(self.evaluator, &vars).filter(|t| !t.is_empty()) else {
                    continue;
                };
                self.primitives.push(Primitive::Text(TextRun {
                    owner: owner.clone(),
                    role: Role::Mark,
                    text,
                    position: mark_beside(&arc, mark.at, mark.left, mark.distance),
                    anchor: TextAnchor::Middle,
                    font: self.line_font(&resolved, selected),
                    wrap: None,
                    tooltip: None,
                    editable: false,
                }));
            }
        }
    }

    /// The line's path (`routePath`), and where it really starts and ends: a bowed arc clips its own ends to the
    /// boxes. The arc itself, for what rides along it.
    fn route(
        &self,
        route: &Route,
        relation_type: &RelationType,
        from: Point,
        to: Point,
        ends: &RouteEnds,
        vars: &Vars,
    ) -> (Path, Point, Point, Option<crate::notation::bowed_arc::BowedArc>) {
        match route {
            Route::BuiltIn(built_in) => {
                let path = match built_in {
                    BuiltInRoute::Straight => straight_path(from, to),
                    BuiltInRoute::Polyline => polyline_path(from, to, &[]),
                    BuiltInRoute::Orthogonal => {
                        let on_top_or_bottom = |point: Point, box_: Rect| {
                            let near = |a: f64, b: f64| (a - b).abs() < 0.5;
                            (near(point.y, box_.y) || near(point.y, box_.bottom()))
                                && !(near(point.x, box_.x) || near(point.x, box_.right()))
                        };
                        let axis = if on_top_or_bottom(from, ends.source) && on_top_or_bottom(to, ends.target) {
                            Axis::Vertical
                        } else {
                            Axis::Horizontal
                        };
                        orthogonal_path(from, to, relation_type.corner_radius.unwrap_or(0.0), axis)
                    }
                    BuiltInRoute::Arc => arc_path(from, to, 0.25),
                    BuiltInRoute::QuadraticBezier => quadratic_bezier_path(from, to),
                    BuiltInRoute::CubicBezier => horizontal_bezier_path(from, to),
                    BuiltInRoute::Spline => spline_path(from, to, &[]),
                };
                (path, from, to, None)
            }
            Route::ForwardBezier => {
                let path = if to.x < from.x {
                    forward_bezier_path(from, to)
                } else {
                    horizontal_bezier_path(from, to)
                };
                (path, from, to, None)
            }
            Route::BowedArc(declaration) => {
                let bow = declaration
                    .variants
                    .iter()
                    .find(|(when, _)| holds(self.evaluator, Some(when), contexts::ELEMENT, vars))
                    .map(|(_, bow)| *bow)
                    .unwrap_or(declaration.bow);
                let arc = arc_between(
                    ends.source,
                    ends.target,
                    bow,
                    declaration.self_loop_radius.unwrap_or(SELF_LOOP_RADIUS),
                );
                (arc.path.clone(), arc.start, arc.finish, Some(arc))
            }
            Route::Plugin { name, fallback } => match self.options.plugins.route(name, from, to, ends) {
                Some(path) => {
                    let (start, end) = (path.start().unwrap_or(from), path.end().unwrap_or(to));
                    (path, start, end, None)
                }
                None => self.route(fallback, relation_type, from, to, ends, vars),
            },
        }
    }
}
