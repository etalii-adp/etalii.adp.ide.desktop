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
//! An element's size is its type's (`default_size` in `adp-scene`), widened to its first label where the type sizes
//! by its content, as `elementBounds` widens it: the label's width at the label font plus 16.

use std::collections::HashMap;

use adp_disl::{DislDiagram, DislElement, DislSpecification, DislType};
use adp_layout::{LayoutAlgorithm, LayoutDefinition, LayoutElement, LayoutInput, LayoutMode, TreeLayout};
use adp_scene::definition::{ElementType, Sizing};
use adp_scene::scene::default_size;
use adp_scene::text::{LABEL_FONT_SIZE, TextMetrics};
use adp_scene::{DiagramDefinition, Evaluator, Point, Rect};
use serde_json::Value as Json;

/// What a content-sized element keeps beside its label (`elementBounds`: `widthOf(label) + 16`).
const CONTENT_PADDING: f64 = 16.0;

/// The gap between elements laid out in a row because nothing places them.
const UNPLACED_GAP: f64 = 40.0;

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
    /// The mapping of `element`, read from the specification's notation and coordinates.
    pub fn of(specification: &DislSpecification, element: &DislElement) -> Mapping {
        Self::of_type(specification, &element.element_type())
    }

    /// The mapping of a node of `node_type`.
    pub fn of_type(specification: &DislSpecification, node_type: &DislType) -> Mapping {
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
        let axis_scale = |axis: &str| -> f64 {
            system
                .and_then(|s| s.get(axis))
                .and_then(Json::as_str)
                .and_then(|name| root.get("coordinates")?.get("axes")?.get(name))
                .and_then(|a| a.get("scale"))
                .and_then(Json::as_f64)
                .filter(|s| *s > 0.0)
                .unwrap_or(1.0)
        };
        let snap = |axis: &str| -> Option<f64> {
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
pub fn drop_mapping(specification: &DislSpecification, diagram: &DislDiagram) -> Mapping {
    let metamodel = specification.metamodel();
    metamodel
        .types
        .values()
        .filter(|t| !t.is_abstract())
        .map(|t| Mapping::of_type(specification, t))
        .find(|m| *m != Mapping::default())
        .or_else(|| diagram.nodes().first().map(|n| Mapping::of(specification, n)))
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
/// its members is the scene's to place.
pub fn place(
    definition: &DiagramDefinition,
    evaluator: &Evaluator,
    diagram: &DislDiagram,
    positions: &HashMap<String, (f64, f64)>,
    metrics: &dyn TextMetrics,
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
        let size = size_of(element_type, evaluator, &element, metrics);
        let mapping = Mapping::of(specification, &element);
        match positions.get(&element.id()) {
            Some(position) => {
                placed.rects.insert(element.id(), mapping.to_canvas(*position, size));
                placed.mappings.insert(element.id(), mapping);
            }
            None => unplaced.push((element, size)),
        }
    }
    if unplaced.is_empty() {
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
}
