//! The scene of a Helm chart diagram, which its module lays out in bands and hands the session as boxes and relations
//! (etalii.adp spec 030, task T080), drawn as `adp-scene` primitives so the one painter, hit test and gesture arbiter
//! serve it as they serve every other canvas.
//!
//! Ported from what the standalone host's `src/diagrams/helm-chart/client/HelmCanvas.tsx` at `8b6ccfc` drew through the
//! notation of `helm-chart.dis`, in its theme's tokens, light or dark:
//! - every node is a box at the size the module gave it, its name in the middle; one hue per kind, whole for the
//!   outline and mixed into a faint fill (chart, values, templates, dependencies, what `charts/` holds), the schema,
//!   the lock and the CRDs plain; an archive's outline is dashed, and a node whose file did not parse is outlined in
//!   the danger colour, dashed;
//! - a relation runs out of its source's right side into its target's left side as a horizontal curve with a filled
//!   arrowhead, in its kind's colour (`Overrides` and `Configures` dashed, `Includes` dotted), its label above its
//!   middle;
//! - a relation without a target (an unvendored dependency, an include no local partial defines) is a 48-unit stub out
//!   of its source's right side, dashed and faint, with what it names above it.
//!
//! Every box is moved by its top-left, which is what the registration stores (`onElementMoved`).

use std::collections::{HashMap, HashSet};

use adp_diagram::{DiagramElement, ElementKind};
use adp_diagram_helm_chart::mapper::{
    ARCHIVE_TYPE, CHART_TYPE, CRDS_TYPE, DEPENDENCY_TYPE, EDGE_TYPE, LOCK_TYPE, PARTIAL_TYPE, SCHEMA_TYPE,
    SUBCHART_TYPE, TEMPLATE_TYPE, VALUES_TYPE,
};
use adp_diagram_helm_chart::theme::palette;
use adp_scene::definition::labels::TextAnchor;
use adp_scene::notation::catalog::MarkerKind;
use adp_scene::scene::{EdgePrimitive, Hook, HookKind, Owner, Primitive, Role, ShapePrimitive, marker_shape};
use adp_scene::style::{Colour, Stroke};
use adp_scene::{Path, Point, Rect, Scene};
use adp_theme::Kind;

use super::module_scene::{ModuleScene, SELECTED_WIDTH, font, number, selection_colour, text};

/// What the canvas says of a folder that is no chart (`canvas.empty`).
const EMPTY: &str = "This folder is not a Helm chart: it has no Chart.yaml, so there is nothing to draw.";

/// The length of an open end's stub.
const STUB: f64 = 48.0;

fn stroke(colour: &str, width: f64, dash: &[f64], opacity: f64) -> Stroke {
    Stroke {
        colour: Some(Colour::value(colour)),
        width,
        dash: dash.to_vec(),
        opacity,
    }
}

/// Where a node is drawn: its top-left and the size the module gave it.
pub fn rect_of(element: &DiagramElement) -> Rect {
    Rect::new(
        element.x,
        element.y,
        number(element, "width"),
        number(element, "height"),
    )
}

/// The scene of `elements`, a Helm chart view's rendering, with `selection` outlined, the elements `moving` carried by
/// the drag in flight, in the palette of the active theme, `theme`.
pub fn build(
    elements: &[DiagramElement],
    selection: &[String],
    moving: Option<(&[String], (f64, f64))>,
    theme: Kind,
) -> ModuleScene {
    let palette = palette(theme);
    let highlight = selection_colour(theme);
    let selected: HashSet<&str> = selection.iter().map(String::as_str).collect();
    let mut rects: HashMap<String, Rect> = HashMap::new();
    for element in elements.iter().filter(|e| e.kind == ElementKind::Node) {
        rects.insert(element.id.clone(), rect_of(element));
    }
    let mut drawn = rects.clone();
    if let Some((ids, (dx, dy))) = moving {
        for id in ids {
            if let Some(rect) = drawn.get_mut(id) {
                *rect = Rect::new(rect.x + dx, rect.y + dy, rect.width, rect.height);
            }
        }
    }

    let mut primitives = Vec::new();
    let mut boxes = Vec::new();
    let mut lines = Vec::new();
    let mut labels = Vec::new();

    // The relations first, beneath the boxes they join.
    for relation in elements.iter().filter(|e| e.element_type == EDGE_TYPE) {
        let Some(source) = relation.source.as_deref().and_then(|s| drawn.get(s)) else {
            continue;
        };
        let owner = Owner::Relation(relation.id.clone());
        let is_selected = selected.contains(relation.id.as_str());
        let kind = relation.attribute("edge.kind");
        let (colour, dash): (&str, &[f64]) = match kind {
            "DECLARES" => (palette.dependency, &[]),
            "RESOLVES" => (palette.subchart, &[]),
            "OVERRIDES" | "CONFIGURES" => (palette.values, &[5.0, 4.0]),
            "INCLUDES" => (palette.template, &[2.0, 3.0]),
            _ => (palette.muted, &[]),
        };
        let from = Point::new(source.right(), source.centre().y);
        let label = relation.attribute("edge.label");
        let Some(target) = relation.target.as_deref().and_then(|t| drawn.get(t)) else {
            // An open end: the stub, with what it names above it.
            let to = Point::new(from.x + STUB, from.y);
            let line = if is_selected {
                stroke(highlight, SELECTED_WIDTH, &[3.0, 3.0], 1.0)
            } else {
                stroke(colour, 1.5, &[3.0, 3.0], 0.7)
            };
            primitives.push(Primitive::Edge(EdgePrimitive {
                owner: owner.clone(),
                path: Path::polyline(&[from, to]),
                end: None,
                stroke: line,
                start: None,
                hit_width: 10.0,
                selectable: true,
                tooltip: None,
            }));
            lines.push((relation.id.clone(), vec![from, to], 5.0));
            let suffix = if kind == "INCLUDES" {
                " (not defined here)"
            } else {
                " (unvendored)"
            };
            primitives.push(text(
                &owner,
                Role::LineLabel,
                format!("{label}{suffix}"),
                Point::new(from.x + 8.0, from.y - 6.0),
                TextAnchor::Start,
                font(11.0, 400, Some(palette.muted)),
            ));
            continue;
        };
        let to = Point::new(target.x, target.centre().y);
        let bend = ((to.x - from.x).abs() / 2.0).max(24.0);
        let path =
            Path::new()
                .move_to(from)
                .cubic_to(Point::new(from.x + bend, from.y), Point::new(to.x - bend, to.y), to);
        let line = if is_selected {
            stroke(highlight, SELECTED_WIDTH, dash, 1.0)
        } else {
            stroke(colour, 1.5, dash, 1.0)
        };
        primitives.push(Primitive::Edge(EdgePrimitive {
            owner: owner.clone(),
            path,
            end: marker_shape(MarkerKind::Arrow, to, 0.0, line.width),
            stroke: line,
            start: None,
            hit_width: 10.0,
            selectable: true,
            tooltip: None,
        }));
        let middle = Point::new((from.x + to.x) / 2.0, (from.y + to.y) / 2.0);
        lines.push((relation.id.clone(), vec![from, middle, to], 5.0));
        if !label.is_empty() {
            primitives.push(text(
                &owner,
                Role::LineLabel,
                label.to_owned(),
                Point::new(middle.x, middle.y - 6.0),
                TextAnchor::Middle,
                font(11.0, if is_selected { 600 } else { 400 }, Some(palette.muted)),
            ));
        }
    }

    // The boxes, at the sizes the module gave them.
    for node in elements.iter().filter(|e| e.kind == ElementKind::Node) {
        let Some(rect) = drawn.get(&node.id).copied() else {
            continue;
        };
        let owner = Owner::Element(node.id.clone());
        let hue = match node.element_type.as_str() {
            CHART_TYPE => Some(palette.chart),
            VALUES_TYPE => Some(palette.values),
            TEMPLATE_TYPE | PARTIAL_TYPE => Some(palette.template),
            DEPENDENCY_TYPE => Some(palette.dependency),
            SUBCHART_TYPE | ARCHIVE_TYPE => Some(palette.subchart),
            SCHEMA_TYPE | LOCK_TYPE | CRDS_TYPE => None,
            _ => None,
        };
        let (fill, colour) = match hue {
            Some(hue) => (format!("{hue}24"), hue),
            None => (palette.plain_fill.to_owned(), palette.muted),
        };
        let dash: &[f64] = if node.element_type == ARCHIVE_TYPE {
            &[6.0, 3.0]
        } else {
            &[]
        };
        let outline = if selected.contains(node.id.as_str()) {
            stroke(highlight, SELECTED_WIDTH, dash, 1.0)
        } else if node.attribute("unreadable") == "true" {
            // A node whose file did not parse: marked, never hidden.
            stroke(palette.danger, 1.5, &[4.0, 3.0], 1.0)
        } else {
            stroke(colour, 1.5, dash, 1.0)
        };
        let moved = moving.is_some_and(|(ids, _)| ids.contains(&node.id));
        primitives.push(Primitive::Shape(ShapePrimitive {
            owner: owner.clone(),
            role: Role::Body,
            path: Path::rounded_rect(rect, 4.0),
            fill: Some(Colour::value(&fill)),
            stroke: Some(outline),
            opacity: if moved { 0.75 } else { 1.0 },
            tooltip: Some(tooltip(node)),
        }));
        primitives.push(text(
            &owner,
            Role::Label {
                id: "name".to_owned(),
                line: 0,
            },
            node.attribute("name").to_owned(),
            Point::new(rect.centre().x, rect.centre().y + 4.5),
            TextAnchor::Middle,
            font(13.0, 400, Some(palette.text)),
        ));
        labels.push((node.id.clone(), "name".to_owned(), rect));
        if selected.contains(node.id.as_str()) {
            primitives.push(Primitive::Hook(Hook {
                owner: owner.clone(),
                kind: HookKind::Selection,
                area: rect,
            }));
        }
        boxes.push((node.id.clone(), rect));
    }

    let empty_text = (!elements.iter().any(|e| e.kind == ElementKind::Node)).then(|| EMPTY.to_owned());
    ModuleScene {
        scene: Scene { primitives, empty_text },
        top_left: rects.keys().cloned().collect(),
        rects,
        boxes,
        lines,
        labels,
        anchors: Vec::new(),
    }
}

/// What a node's tooltip says (`notation.nodes.*.tooltip`): its kind and name.
fn tooltip(node: &DiagramElement) -> String {
    let kind = match node.element_type.as_str() {
        CHART_TYPE => "Chart",
        VALUES_TYPE => "Values",
        SCHEMA_TYPE => "Values schema",
        TEMPLATE_TYPE | PARTIAL_TYPE => "Template",
        CRDS_TYPE => "CRDs",
        DEPENDENCY_TYPE => "Dependency",
        SUBCHART_TYPE => "Subchart",
        ARCHIVE_TYPE => "Archive",
        LOCK_TYPE => "Lock",
        _ => "Element",
    };
    format!("{kind} {}", node.attribute("name"))
}
