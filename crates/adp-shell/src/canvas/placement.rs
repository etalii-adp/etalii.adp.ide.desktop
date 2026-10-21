//! Where an element is drawn (etalii.adp spec 030, FR-018, task T055): a session's positions are in the diagram's
//! own units, the canvas draws in canvas units, and this is the one conversion between the two, both ways.
//!
//! The standalone host at `8b6ccfc` drew every element by its centre in canvas units (`elementBounds` in
//! `src/client/src/canvas/library/DiagramCanvas.tsx`), and each module converted its model's coordinates into that:
//! the dependency graph and the timeline put a node's top-left corner at (`x`, `row × 60`)
//! (`DependencyGraphRows.cs`, `DependencyGraphCanvas.tsx`), and rounded a dropped y to the nearest row. Here the
//! conversion is read from the specification instead of a module: a node type's `placement` names the attributes it
//! is placed by, its coordinate `system` and its `anchor`; the system names an axis for x and for y, whose `scale`
//! is the number of canvas units one unit of the axis takes, and its `snapping` says which axis lands on a grid. A
//! position the registration's `layout:` holds is already the element's centre in canvas units.
//!
//! A time axis (`kind: "time"`) is scaled as the standalone host's timeline canvas scaled it (`timelineScaleOf` in
//! `src/diagrams/timeline/client/TimelineCanvas.tsx`): once, when the diagram is first drawn with something on it, so
//! that the span of all its elements, a tenth to spare on either side, takes 1200 canvas units, and never again under
//! the person; an empty diagram by the axis's own `scale` (the timeline's 20 units a day). Where the tool type's module
//! places a node in coordinates of its own (the timeline's seconds across and 60 units a row down), its x is scaled by
//! the time axis and its y is already in canvas units; the module snaps what a gesture writes.
//!
//! An element's size is its type's (`default_size` in `adp-scene`), widened to its first label where the type sizes
//! by its content, as `elementBounds` widens it: the label's width at the label font plus 16; where the document keeps
//! a node's size (its notation binds it, `adp_diagram::sizing`), that size, within the notation's limits. A resize
//! carries one edge and keeps the far one, which it never crosses (`resizedBounds`), nor the limits.

use std::collections::HashMap;

use adp_diagram::{DiagramModule, Extent, Length, ResizedEdge};
use adp_disl::{DislDiagram, DislElement, DislSpecification, DislType};
use adp_layout::{LayoutAlgorithm, LayoutDefinition, LayoutElement, LayoutInput, LayoutMode, TreeLayout};
use adp_scene::definition::{ElementType, SizeRule, Sizing};
use adp_scene::scene::default_size;
use adp_scene::text::{LABEL_FONT_SIZE, TextMetrics};
use adp_scene::{DiagramDefinition, Evaluator, Point, Rect};
use serde_json::Value as Json;

/// What a content-sized element keeps beside its label (`elementBounds`: `widthOf(label) + 16`).
const CONTENT_PADDING: f64 = 16.0;

/// The gap between elements laid out in a row because nothing places them.
const UNPLACED_GAP: f64 = 40.0;

/// The canvas units a time axis's span, with its tenth to spare either side, is fitted across (`FALLBACK_WIDTH_PX`).
const TIME_FIT_WIDTH: f64 = 1200.0;

/// The shortest span a time axis is fitted to: a day (`timelineScaleOf`).
const DAY: f64 = 86_400.0;

/// What the canvas reads a node's coordinates with beside the specification.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Scales {
    /// Canvas units per second of a time axis, as it was frozen; none for the axis's own scale.
    pub time: Option<f64>,
    /// Whether the tool type's module places the node in coordinates of its own.
    pub module: bool,
}

/// The time axis across the specification's default coordinate system: canvas units per second by its own `scale`,
/// and the height of its ruler when it shows one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeAxis {
    pub units_per_second: f64,
    pub ruler: Option<f64>,
}

/// The time axis across the specification's default system, when its x axis is one (`coordinates.axes.<x>.kind`).
pub fn time_axis(specification: &DislSpecification) -> Option<TimeAxis> {
    let coordinates = specification.root().get("coordinates")?;
    let system = coordinates
        .get("default")
        .and_then(Json::as_str)
        .and_then(|name| coordinates.get("systems")?.get(name))?;
    let axis = coordinates.get("axes")?.get(system.get("x")?.as_str()?)?;
    time_axis_of(axis)
}

/// A time axis whose values are instants (`valueType: datetime`), held as seconds since the epoch; a month index
/// (the hype cycle's `yearMonth`) is an axis of plain numbers.
fn time_axis_of(axis: &Json) -> Option<TimeAxis> {
    if axis.get("kind").and_then(Json::as_str) != Some("time")
        || axis.get("valueType").and_then(Json::as_str) != Some("datetime")
    {
        return None;
    }
    let scale = axis.get("scale");
    let size = scale.and_then(|s| s.get("size")).and_then(Json::as_f64).unwrap_or(20.0);
    let unit = match scale.and_then(|s| s.get("unit")).and_then(Json::as_str) {
        Some("second") => 1.0,
        Some("minute") => 60.0,
        Some("hour") => 3_600.0,
        Some("week") => 7.0 * DAY,
        _ => DAY,
    };
    let ruler = axis
        .get("ruler")
        .filter(|r| r.get("visible").and_then(Json::as_bool) == Some(true))
        .map(|r| r.get("size").and_then(Json::as_f64).unwrap_or(24.0));
    Some(TimeAxis {
        units_per_second: size / unit,
        ruler,
    })
}

/// The scale `timelineScaleOf` freezes for elements spanning `spans` (each a begin and an end, in seconds): their
/// whole span, at least a day, a tenth to spare either side, across 1200 canvas units; none for no element.
pub fn fitted_time_scale(spans: impl IntoIterator<Item = (f64, f64)>) -> Option<f64> {
    let (begin, end) = spans.into_iter().fold(None, |known: Option<(f64, f64)>, (b, e)| {
        Some(match known {
            Some((low, high)) => (low.min(b), high.max(e.max(b))),
            None => (b, e.max(b)),
        })
    })?;
    let span = (end - begin).max(DAY);
    Some(TIME_FIT_WIDTH / (span * 1.2))
}

/// How one element's position in the diagram's units maps onto the canvas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mapping {
    /// Canvas units per unit of the x and the y axis.
    pub scale: (f64, f64),
    /// Whether the position is the element's top-left corner rather than its centre.
    pub top_left: bool,
    /// The grid a moved or dropped position lands on, in the axis's own units, per axis.
    pub snap: (Option<f64>, Option<f64>),
}

impl Default for Mapping {
    /// A registration's position: the centre, in canvas units, on no grid.
    fn default() -> Self {
        Mapping {
            scale: (1.0, 1.0),
            top_left: false,
            snap: (None, None),
        }
    }
}

impl Mapping {
    /// The mapping of `element`, read from the specification's notation and coordinates and `scales`.
    pub fn of(specification: &DislSpecification, element: &DislElement, scales: Scales) -> Mapping {
        Self::of_type(specification, &element.element_type(), scales)
    }

    /// The mapping of a node of `node_type`.
    pub fn of_type(specification: &DislSpecification, node_type: &DislType, scales: Scales) -> Mapping {
        let root = specification.root();
        let Some(nodes) = root
            .get("notation")
            .and_then(|n| n.get("nodes"))
            .and_then(Json::as_object)
        else {
            return Mapping::default();
        };
        let declared = nodes
            .get(&node_type.name)
            .or_else(|| nodes.iter().find(|(name, _)| node_type.is_a(name)).map(|(_, n)| n));
        let Some(placement) = declared.and_then(|d| d.get("placement")) else {
            return Mapping::default();
        };
        let bound = |axis: &str| placement.get(axis).and_then(|a| a.get("attribute")).is_some();
        if !bound("x") || !bound("y") {
            return Mapping::default();
        }
        let top_left = placement.get("anchor").and_then(Json::as_str) == Some("top-left");
        let system = placement
            .get("system")
            .and_then(Json::as_str)
            .and_then(|name| root.get("coordinates")?.get("systems")?.get(name));
        let axis_of = |axis: &str| {
            system
                .and_then(|s| s.get(axis))
                .and_then(Json::as_str)
                .and_then(|name| root.get("coordinates")?.get("axes")?.get(name))
        };
        let axis_scale = |axis: &str| -> f64 {
            let declared = axis_of(axis);
            if let Some(time) = declared.and_then(time_axis_of) {
                return scales.time.unwrap_or(time.units_per_second);
            }
            if scales.module {
                // The module's own coordinates are canvas units already, but for time.
                return 1.0;
            }
            declared
                .and_then(|a| a.get("scale"))
                .and_then(Json::as_f64)
                .filter(|s| *s > 0.0)
                .unwrap_or(1.0)
        };
        let snap = |axis: &str| -> Option<f64> {
            if scales.module {
                // The module lands what a gesture writes on its own lattice: a day, a row.
                return None;
            }
            let snapping = system?.get("snapping")?;
            if snapping.get("mode").and_then(Json::as_str) == Some("never") {
                return None;
            }
            snapping
                .get(axis)?
                .get("grid")?
                .get("spacing")?
                .as_f64()
                .filter(|s| *s > 0.0)
        };
        Mapping {
            scale: (axis_scale("x"), axis_scale("y")),
            top_left,
            snap: (snap("x"), snap("y")),
        }
    }

    /// The box an element of `size` at `position` (in the diagram's units) takes on the canvas.
    pub fn to_canvas(self, position: (f64, f64), size: (f64, f64)) -> Rect {
        let (x, y) = (position.0 * self.scale.0, position.1 * self.scale.1);
        if self.top_left {
            Rect::new(x, y, size.0, size.1)
        } else {
            Rect::new(x - size.0 / 2.0, y - size.1 / 2.0, size.0, size.1)
        }
    }

    /// The position, in the diagram's units and on its grid, of an element whose box on the canvas is `bounds`.
    pub fn to_domain(self, bounds: Rect) -> (f64, f64) {
        let point = if self.top_left {
            Point::new(bounds.x, bounds.y)
        } else {
            bounds.centre()
        };
        self.point_to_domain(point)
    }

    /// A point on the canvas in the diagram's units, on its grid: where a drop at that point puts what it makes.
    pub fn point_to_domain(self, point: Point) -> (f64, f64) {
        let snapped = |value: f64, grid: Option<f64>| match grid {
            Some(spacing) => {
                // Halves away from zero, never a negative zero (`dependency-graph.md`, Row rounding).
                let rounded = (value / spacing).round() * spacing;
                if rounded == 0.0 { 0.0 } else { rounded }
            }
            None => value,
        };
        (
            snapped(point.x / self.scale.0, self.snap.0),
            snapped(point.y / self.scale.1, self.snap.1),
        )
    }
}

/// The mapping a drop on the empty canvas uses: that of the first node type placed by its attributes, else the
/// registration's.
pub fn drop_mapping(specification: &DislSpecification, diagram: &DislDiagram, scales: Scales) -> Mapping {
    let metamodel = specification.metamodel();
    metamodel
        .types
        .values()
        .filter(|t| !t.is_abstract())
        .map(|t| Mapping::of_type(specification, t, scales))
        .find(|m| *m != Mapping::default())
        .or_else(|| diagram.nodes().first().map(|n| Mapping::of(specification, n, scales)))
        .unwrap_or_default()
}

/// The size an element is drawn at: its type's, widened to its first label where it sizes by its content.
pub fn size_of(
    element_type: &ElementType,
    evaluator: &Evaluator,
    element: &DislElement,
    metrics: &dyn TextMetrics,
) -> (f64, f64) {
    let (width, height) = default_size(element_type);
    if element_type.sizing != Sizing::Content || element_type.size.fixed.is_some() {
        return (width, height);
    }
    let vars = evaluator.element_vars(element);
    let label = element_type
        .labels
        .first()
        .and_then(|l| l.text.element_text(evaluator, &vars))
        .unwrap_or_default();
    let fitted = metrics.width(&label, LABEL_FONT_SIZE) + CONTENT_PADDING;
    let max = element_type.size.max.0.unwrap_or(f64::MAX);
    (width.max(fitted).min(max), height)
}

/// The size a resize never goes below where the notation states no minimum (`resizedBounds`: one unit).
const MIN_RESIZED: f64 = 1.0;

/// The size an element with a stored `extent` is drawn at: the stored axes, scaled as its position is where they are
/// in the axis's units, within the notation's limits; the other axes as `size` has them.
pub fn stored_size(size: (f64, f64), extent: &Extent, mapping: Mapping, rule: &SizeRule) -> (f64, f64) {
    let canvas = |length: Option<Length>, scale: f64, fallback: f64| match length {
        Some(Length::Canvas(v)) => v,
        Some(Length::Domain(v)) => v * scale,
        None => fallback,
    };
    let clamp = |v: f64, min: Option<f64>, max: Option<f64>| v.min(max.unwrap_or(f64::MAX)).max(min.unwrap_or(0.0));
    (
        clamp(canvas(extent.width, mapping.scale.0, size.0), rule.min.0, rule.max.0),
        clamp(canvas(extent.height, mapping.scale.1, size.1), rule.min.1, rule.max.1),
    )
}

/// `bounds` with one edge carried `delta` canvas units along its axis: the far edge stays where it is and is never
/// crossed, and the size stays within the notation's limits (at least one unit where it states no minimum).
pub fn resized(bounds: Rect, edge: ResizedEdge, delta: f64, rule: &SizeRule) -> Rect {
    let horizontal = edge.is_horizontal();
    let (start, length) = if horizontal {
        (bounds.x, bounds.width)
    } else {
        (bounds.y, bounds.height)
    };
    let (min, max) = if horizontal {
        (rule.min.0, rule.max.0)
    } else {
        (rule.min.1, rule.max.1)
    };
    let min = min.unwrap_or(MIN_RESIZED).max(MIN_RESIZED);
    let max = max.unwrap_or(f64::MAX).max(min);
    let (start, length) = match edge {
        ResizedEdge::Left | ResizedEdge::Top => {
            let end = start + length;
            let carried = (length - delta).clamp(min, max);
            (end - carried, carried)
        }
        ResizedEdge::Right | ResizedEdge::Bottom => (start, (length + delta).clamp(min, max)),
    };
    if horizontal {
        Rect::new(start, bounds.y, length, bounds.height)
    } else {
        Rect::new(bounds.x, start, bounds.width, length)
    }
}

/// Every node's box on the canvas and the mapping it was placed by, by element id.
#[derive(Debug, Clone, Default)]
pub struct Placed {
    pub rects: HashMap<String, Rect>,
    pub mappings: HashMap<String, Mapping>,
    /// Each node's type, by id.
    pub types: HashMap<String, String>,
}

/// Places every node of `diagram` that `definition` draws: at its authored position where it has one, by the tree
/// layout when no node of the diagram has one, and in a row under the rest otherwise. A node drawn at the centre of
/// its members is the scene's to place. Where a module's layout gave the `positions` (`laid_out`), a node it left
/// out, under a collapsed branch, is not drawn at all. A node the tool type's `module` places is read in its
/// coordinates, a time axis at `time_scale` canvas units a second.
#[allow(clippy::too_many_arguments)]
pub fn place(
    definition: &DiagramDefinition,
    evaluator: &Evaluator,
    diagram: &DislDiagram,
    positions: &HashMap<String, (f64, f64)>,
    extents: &HashMap<String, Extent>,
    metrics: &dyn TextMetrics,
    laid_out: bool,
    module: Option<&dyn DiagramModule>,
    time_scale: Option<f64>,
) -> Placed {
    let specification = &definition.specification;
    let mut placed = Placed::default();
    let mut unplaced: Vec<(DislElement, (f64, f64))> = Vec::new();
    for element in diagram.nodes() {
        let Some(element_type) = definition.element_type(&element.type_name()) else {
            continue;
        };
        if element_type.centre_of.is_some() && !positions.contains_key(&element.id()) {
            continue;
        }
        placed.types.insert(element.id(), element.type_name());
        let scales = Scales {
            time: time_scale,
            module: module.is_some_and(|m| m.position(&element).is_some()),
        };
        let mapping = Mapping::of(specification, &element, scales);
        let mut size = size_of(element_type, evaluator, &element, metrics);
        if let Some(extent) = extents.get(&element.id()) {
            size = stored_size(size, extent, mapping, &element_type.size);
        }
        match positions.get(&element.id()) {
            Some(position) => {
                placed.rects.insert(element.id(), mapping.to_canvas(*position, size));
                placed.mappings.insert(element.id(), mapping);
            }
            None => unplaced.push((element, size)),
        }
    }
    if unplaced.is_empty() || laid_out {
        return placed;
    }
    if placed.rects.is_empty() {
        let input = LayoutInput {
            elements: unplaced
                .iter()
                .map(|(element, size)| {
                    let mut entry = LayoutElement::new(element.id(), 0.0, 0.0, size.0, size.1);
                    entry.parent_id = element.parent().map(|p| p.id());
                    entry.element_type = Some(element.type_name());
                    entry
                })
                .collect(),
            connections: diagram
                .relations()
                .iter()
                .filter_map(|r| Some(adp_layout::LayoutConnection::new(r.source()?.id(), r.target()?.id())))
                .collect(),
        };
        let layout = TreeLayout.place(&input, &LayoutDefinition::with_modes(&[LayoutMode::Tree]), None);
        if let Some(layout) = layout {
            for (element, size) in &unplaced {
                if let Some(at) = layout.get(&element.id()) {
                    placed
                        .rects
                        .insert(element.id(), Mapping::default().to_canvas((at.x, at.y), *size));
                    placed.mappings.insert(element.id(), Mapping::default());
                }
            }
            return placed;
        }
    }
    let bottom = placed.rects.values().map(|r| r.bottom()).fold(f64::MIN, f64::max);
    let left = placed.rects.values().map(|r| r.x).fold(f64::MAX, f64::min);
    let (mut x, y) = if bottom == f64::MIN {
        (0.0, 0.0)
    } else {
        (left, bottom + UNPLACED_GAP)
    };
    for (element, size) in unplaced {
        if placed.rects.contains_key(&element.id()) {
            continue;
        }
        placed.rects.insert(element.id(), Rect::new(x, y, size.0, size.1));
        placed.mappings.insert(element.id(), Mapping::default());
        x += size.0 + UNPLACED_GAP;
    }
    placed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_axis_scales_and_snaps_and_a_top_left_anchor_is_kept() {
        let rows = Mapping {
            scale: (1.0, 60.0),
            top_left: true,
            snap: (None, Some(1.0)),
        };
        let bounds = rows.to_canvas((270.5, 4.0), (160.0, 36.0));
        assert_eq!(bounds, Rect::new(270.5, 240.0, 160.0, 36.0));
        assert_eq!(rows.to_domain(Rect::new(300.0, 200.0, 160.0, 36.0)), (300.0, 3.0));
        assert_eq!(
            rows.point_to_domain(Point::new(10.0, -20.0)),
            (10.0, 0.0),
            "never a negative zero"
        );
        let centred = Mapping::default();
        assert_eq!(
            centred.to_canvas((0.0, 0.0), (90.0, 40.0)),
            Rect::new(-45.0, -20.0, 90.0, 40.0)
        );
        assert_eq!(centred.to_domain(Rect::new(-45.0, -20.0, 90.0, 40.0)), (0.0, 0.0));
    }

    #[test]
    fn a_time_axis_is_fitted_once_to_its_span_and_an_empty_one_takes_its_own_scale() {
        let declared = time_axis_of(&serde_json::json!({
            "kind": "time",
            "valueType": "datetime",
            "scale": { "unit": "day", "size": 20 },
            "ruler": { "visible": true, "position": "bottom", "size": 24 }
        }))
        .unwrap();
        assert_eq!(declared.units_per_second, 20.0 / 86_400.0, "20 units a day");
        assert_eq!(declared.ruler, Some(24.0));
        assert_eq!(
            time_axis_of(&serde_json::json!({ "kind": "linear", "scale": 60 })),
            None
        );
        // A hundred days, a tenth either side, across 1200 units: ten units a day.
        let fitted = fitted_time_scale([(0.0, 50.0 * DAY), (20.0 * DAY, 100.0 * DAY)]).unwrap();
        assert!((fitted * DAY - 10.0).abs() < 1e-9, "{}", fitted * DAY);
        // A single moment spans a day.
        assert!((fitted_time_scale([(5.0, 5.0)]).unwrap() * DAY - 1000.0).abs() < 1e-9);
        assert_eq!(fitted_time_scale([]), None);
    }

    #[test]
    fn a_resize_keeps_the_far_edge_never_crosses_it_and_stays_within_the_limits() {
        let rule = SizeRule {
            min: (Some(80.0), None),
            max: (Some(300.0), Some(48.0)),
            ..SizeRule::default()
        };
        let box_ = Rect::new(100.0, 50.0, 160.0, 48.0);
        assert_eq!(
            resized(box_, ResizedEdge::Right, 40.0, &rule),
            Rect::new(100.0, 50.0, 200.0, 48.0)
        );
        assert_eq!(
            resized(box_, ResizedEdge::Left, 30.0, &rule),
            Rect::new(130.0, 50.0, 130.0, 48.0),
            "the right edge stays put"
        );
        assert_eq!(
            resized(box_, ResizedEdge::Left, 500.0, &rule),
            Rect::new(180.0, 50.0, 80.0, 48.0),
            "the minimum stops the edge before the far one"
        );
        assert_eq!(
            resized(box_, ResizedEdge::Right, 900.0, &rule).width,
            300.0,
            "nor past the maximum"
        );
        assert_eq!(resized(box_, ResizedEdge::Bottom, 20.0, &rule).height, 48.0);
        assert_eq!(
            resized(box_, ResizedEdge::Top, 100.0, &SizeRule::default()),
            Rect::new(100.0, 97.0, 160.0, 1.0),
            "one unit where no minimum is stated"
        );
    }
}
