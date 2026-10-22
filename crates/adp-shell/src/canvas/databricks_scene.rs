//! The scene of a Databricks bundle, job or pipeline, which its module lays out and hands the session as boxes, target
//! frames and edges (etalii.adp spec 030, task T081), drawn as `adp-scene` primitives so the one painter, hit test and
//! gesture arbiter serve it as they serve every other canvas.
//!
//! Ported from what the standalone host's `src/diagrams/databricks/client/DatabricksCanvas.tsx`,
//! `databricksModel.ts` and `databricks.css` at `8b6ccfc` draw:
//! - a target frame is a dashed rectangle beneath everything, its name above it at 12, and `mode · default · n
//!   overrides` 18 below its top at 10;
//! - a box is 200 by 56, its name 24 below its top at 13 and its badges joined by ` · ` 48 below at 10, both inset 8;
//!   a condition task's corners are rounded 16, and a task a dependency names and no task declares is dashed, unfilled
//!   and faded;
//! - a dependency or a flow is a bezier from the right of what is waited for to the left of what waits, with a reach of
//!   30 and an arrowhead, an outcome edge drawn in its answer's colour with the answer at its middle; an override is
//!   straight and dashed, from the frame to the resource it overrides.
//!
//! Every box and frame is moved by its top-left, which is what the registration's `layout:` block stores
//! (`onElementMoved`). A task wears an anchor in the middle of either side, from which the job's dependency gesture is
//! drawn.

use std::collections::{HashMap, HashSet};

use adp_diagram::DiagramElement;
use adp_diagram_databricks::mapper::{
    EDGE_TYPE, FLOW_EDGE_TYPE, FRAME_HEIGHT, FRAME_WIDTH, NODE_HEIGHT, NODE_WIDTH, OVERRIDE_EDGE_TYPE, TARGET_TYPE,
    TASK_TYPE, is_edge,
};
use adp_diagram_databricks::theme::palette;
use adp_scene::definition::labels::TextAnchor;
use adp_scene::scene::{EdgePrimitive, Hook, HookKind, Owner, Primitive, Role, ShapePrimitive};
use adp_scene::style::{Colour, Stroke};
use adp_scene::{Path, Point, Rect, Scene};
use adp_theme::Kind;

use super::module_scene::{ANCHOR_RADIUS, ModuleScene, SELECTED_WIDTH, font, selection_colour, stroke, text};

/// The fixed reach of a layered edge's control points (`EDGE_REACH`).
const EDGE_REACH: f64 = 30.0;
/// How many characters of a box's name fit beside its inset at 13 (`truncate`).
const NAME_CAPACITY: usize = 26;

fn rect_of(element: &DiagramElement) -> Rect {
    if element.element_type == TARGET_TYPE {
        Rect::new(element.x, element.y, FRAME_WIDTH, FRAME_HEIGHT)
    } else {
        Rect::new(element.x, element.y, NODE_WIDTH, NODE_HEIGHT)
    }
}

fn shape(
    owner: &Owner,
    role: Role,
    path: Path,
    fill: Option<&str>,
    outline: Option<Stroke>,
    opacity: f64,
) -> Primitive {
    Primitive::Shape(ShapePrimitive {
        owner: owner.clone(),
        role,
        path,
        fill: fill.map(Colour::value),
        stroke: outline,
        opacity,
        tooltip: None,
    })
}

fn label(owner: &Owner, id: &str, value: String, at: Point, (size, colour): (f64, &str)) -> Primitive {
    text(
        owner,
        Role::Label {
            id: id.to_owned(),
            line: 0,
        },
        value,
        at,
        TextAnchor::Start,
        font(size, 400, Some(colour)),
    )
}

/// `name` whole when it fits, else cut with an ellipsis.
fn truncated(name: &str) -> String {
    if name.chars().count() <= NAME_CAPACITY {
        name.to_owned()
    } else {
        let kept: String = name.chars().take(NAME_CAPACITY - 1).collect();
        format!("{kept}…")
    }
}

/// What a target frame's badge line says: its mode, whether it is the default, and how many resources it overrides,
/// the parts that do not apply left out.
fn frame_badges(element: &DiagramElement) -> String {
    let count: usize = element.attribute("overrideCount").parse().unwrap_or_default();
    let overrides = match count {
        0 => String::new(),
        1 => "1 override".to_owned(),
        n => format!("{n} overrides"),
    };
    let default = if element.attribute("isDefault") == "true" {
        "default"
    } else {
        ""
    };
    [element.attribute("mode"), default, overrides.as_str()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" · ")
}

/// Where the segment from `from` towards the centre of `rect` meets its edge.
fn clipped(from: Point, rect: Rect) -> Point {
    let centre = rect.centre();
    let (dx, dy) = (centre.x - from.x, centre.y - from.y);
    let scale_x = if dx.abs() > f64::EPSILON {
        (rect.width / 2.0) / dx.abs()
    } else {
        f64::INFINITY
    };
    let scale_y = if dy.abs() > f64::EPSILON {
        (rect.height / 2.0) / dy.abs()
    } else {
        f64::INFINITY
    };
    let scale = scale_x.min(scale_y).min(1.0);
    if !scale.is_finite() {
        return centre;
    }
    Point::new(centre.x - dx * scale, centre.y - dy * scale)
}

fn arrowhead(owner: &Owner, tip: Point, from: Point, colour: &str) -> Primitive {
    let (dx, dy) = (tip.x - from.x, tip.y - from.y);
    let length = (dx * dx + dy * dy).sqrt().max(f64::EPSILON);
    let (ux, uy) = (dx / length, dy / length);
    let back = Point::new(tip.x - ux * 8.0, tip.y - uy * 8.0);
    let (nx, ny) = (-uy * 4.5, ux * 4.5);
    shape(
        owner,
        Role::Decoration("head".to_owned()),
        Path::new()
            .move_to(tip)
            .line_to(Point::new(back.x + nx, back.y + ny))
            .line_to(Point::new(back.x - nx, back.y - ny))
            .close(),
        Some(colour),
        None,
        1.0,
    )
}

/// The scene of `elements`, a Databricks view's rendering, with `selection` outlined, in the palette of the active
/// theme, `theme`.
pub fn build(elements: &[DiagramElement], selection: &[String], theme: Kind) -> ModuleScene {
    let palette = palette(theme);
    let highlight = selection_colour(theme);
    let selected: HashSet<&str> = selection.iter().map(String::as_str).collect();
    let mut rects: HashMap<String, Rect> = HashMap::new();
    for element in elements.iter().filter(|e| !is_edge(&e.element_type)) {
        rects.insert(element.id.clone(), rect_of(element));
    }

    let mut primitives = Vec::new();
    let mut boxes = Vec::new();
    let mut lines = Vec::new();
    let mut labels = Vec::new();
    let mut anchors = Vec::new();

    // Frames first, so everything they enclose paints on top.
    for frame in elements.iter().filter(|e| e.element_type == TARGET_TYPE) {
        let rect = rects[&frame.id];
        let owner = Owner::Element(frame.id.clone());
        let outline = if selected.contains(frame.id.as_str()) {
            stroke(Some(highlight), SELECTED_WIDTH, &[6.0, 4.0])
        } else {
            stroke(Some(palette.border), 1.0, &[6.0, 4.0])
        };
        primitives.push(shape(
            &owner,
            Role::Body,
            Path::rounded_rect(rect, 0.0),
            None,
            Some(outline),
            1.0,
        ));
        primitives.push(label(
            &owner,
            "name",
            frame.attribute("label").to_owned(),
            Point::new(rect.x, rect.y - 6.0),
            (12.0, palette.muted),
        ));
        primitives.push(label(
            &owner,
            "badges",
            frame_badges(frame),
            Point::new(rect.x + 12.0, rect.y + 18.0),
            (10.0, palette.muted),
        ));
        labels.push((
            frame.id.clone(),
            "name".to_owned(),
            Rect::new(rect.x, rect.y - 20.0, rect.width, 18.0),
        ));
        if selected.contains(frame.id.as_str()) {
            primitives.push(Primitive::Hook(Hook {
                owner: owner.clone(),
                kind: HookKind::Selection,
                area: rect,
            }));
        }
        boxes.push((frame.id.clone(), rect));
    }

    for node in elements
        .iter()
        .filter(|e| !is_edge(&e.element_type) && e.element_type != TARGET_TYPE)
    {
        let rect = rects[&node.id];
        let owner = Owner::Element(node.id.clone());
        let missing = node.attribute("unresolved") == "true";
        let is_selected = selected.contains(node.id.as_str());
        let dash: &[f64] = if missing { &[5.0, 4.0] } else { &[] };
        let outline = if is_selected {
            stroke(Some(highlight), SELECTED_WIDTH, dash)
        } else {
            stroke(Some(palette.border), 1.0, dash)
        };
        let radius = if node.attribute("kind") == "condition" {
            16.0
        } else {
            4.0
        };
        primitives.push(shape(
            &owner,
            Role::Body,
            Path::rounded_rect(rect, radius),
            (!missing).then_some(palette.surface),
            Some(outline),
            if missing { 0.7 } else { 1.0 },
        ));
        primitives.push(label(
            &owner,
            "name",
            truncated(node.attribute("label")),
            Point::new(rect.x + 8.0, rect.y + NODE_HEIGHT / 2.0 - 4.0),
            (13.0, palette.text),
        ));
        let worn: Vec<&str> = node.attribute("badges").lines().filter(|b| !b.is_empty()).collect();
        if !worn.is_empty() {
            primitives.push(label(
                &owner,
                "badges",
                worn.join(" · "),
                Point::new(rect.x + 8.0, rect.y + NODE_HEIGHT - 8.0),
                (10.0, palette.muted),
            ));
        }
        labels.push((
            node.id.clone(),
            "name".to_owned(),
            Rect::new(rect.x, rect.y, rect.width, NODE_HEIGHT),
        ));
        if is_selected {
            primitives.push(Primitive::Hook(Hook {
                owner: owner.clone(),
                kind: HookKind::Selection,
                area: rect,
            }));
        }
        if node.element_type == TASK_TYPE {
            // The job's dependency gesture is drawn from either side of a task.
            for (name, at) in [
                ("left", Point::new(rect.x, rect.centre().y)),
                ("right", Point::new(rect.right(), rect.centre().y)),
            ] {
                let area = Rect::centred(at, 2.0 * ANCHOR_RADIUS, 2.0 * ANCHOR_RADIUS);
                primitives.push(Primitive::Hook(Hook {
                    owner: owner.clone(),
                    kind: HookKind::Anchor {
                        name: name.to_owned(),
                        visible: false,
                    },
                    area,
                }));
                anchors.push((node.id.clone(), name.to_owned(), area));
            }
        }
        boxes.push((node.id.clone(), rect));
    }

    // The edges, over the boxes they join.
    for edge in elements.iter().filter(|e| is_edge(&e.element_type)) {
        let (Some(source), Some(target)) = (
            edge.source.as_deref().and_then(|s| rects.get(s)),
            edge.target.as_deref().and_then(|t| rects.get(t)),
        ) else {
            continue;
        };
        let owner = Owner::Relation(edge.id.clone());
        let outcome = edge.attribute("outcome");
        let colour = if selected.contains(edge.id.as_str()) {
            highlight
        } else {
            match outcome {
                "true" => palette.success,
                "false" => palette.danger,
                _ => palette.edge,
            }
        };
        let (path, sampled, head_from, tip) = if edge.element_type == OVERRIDE_EDGE_TYPE {
            let from = clipped(target.centre(), *source);
            let to = clipped(from, *target);
            (Path::new().move_to(from).line_to(to), vec![from, to], from, to)
        } else {
            let from = Point::new(source.right(), source.centre().y);
            let to = Point::new(target.x, target.centre().y);
            let (c1, c2) = (
                Point::new(from.x + EDGE_REACH, from.y),
                Point::new(to.x - EDGE_REACH, to.y),
            );
            let sampled: Vec<Point> = (0..=12)
                .map(|step| {
                    let t = f64::from(step) / 12.0;
                    let u = 1.0 - t;
                    Point::new(
                        u * u * u * from.x + 3.0 * u * u * t * c1.x + 3.0 * u * t * t * c2.x + t * t * t * to.x,
                        u * u * u * from.y + 3.0 * u * u * t * c1.y + 3.0 * u * t * t * c2.y + t * t * t * to.y,
                    )
                })
                .collect();
            (Path::new().move_to(from).cubic_to(c1, c2, to), sampled, c2, to)
        };
        let dash = if edge.element_type == OVERRIDE_EDGE_TYPE {
            vec![3.0, 3.0]
        } else {
            Vec::new()
        };
        primitives.push(Primitive::Edge(EdgePrimitive {
            owner: owner.clone(),
            path,
            stroke: Stroke {
                colour: Some(Colour::value(colour)),
                width: 1.5,
                dash,
                opacity: 1.0,
            },
            start: None,
            end: None,
            hit_width: 10.0,
            selectable: true,
            tooltip: None,
        }));
        primitives.push(arrowhead(&owner, tip, head_from, colour));
        if !outcome.is_empty() && (edge.element_type == EDGE_TYPE || edge.element_type == FLOW_EDGE_TYPE) {
            let middle = sampled[sampled.len() / 2];
            primitives.push(text(
                &owner,
                Role::Label {
                    id: "outcome".to_owned(),
                    line: 0,
                },
                outcome.to_owned(),
                Point::new(middle.x, middle.y - 6.0),
                TextAnchor::Middle,
                font(10.0, 400, Some(colour)),
            ));
        }
        lines.push((edge.id.clone(), sampled, 5.0));
    }

    let empty_text = elements
        .is_empty()
        .then(|| "This file declares nothing this diagram draws.".to_owned());
    ModuleScene {
        scene: Scene { primitives, empty_text },
        top_left: rects.keys().cloned().collect(),
        rects,
        boxes,
        lines,
        labels,
        anchors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frames_badge_line_leaves_out_what_does_not_apply() {
        let mut frame = DiagramElement::node("target:prod", TARGET_TYPE);
        frame.attributes = vec![
            ("mode".into(), "production".into()),
            ("isDefault".into(), "false".into()),
            ("overrideCount".into(), "2".into()),
        ];
        assert_eq!(frame_badges(&frame), "production · 2 overrides");
        frame.attributes = vec![
            ("mode".into(), String::new()),
            ("isDefault".into(), "true".into()),
            ("overrideCount".into(), "1".into()),
        ];
        assert_eq!(frame_badges(&frame), "default · 1 override");
    }

    #[test]
    fn an_override_is_drawn_from_edge_to_edge() {
        let source = Rect::new(0.0, 0.0, 100.0, 100.0);
        let target = Rect::new(300.0, 0.0, 100.0, 100.0);
        let from = clipped(target.centre(), source);
        assert_eq!(from, Point::new(100.0, 50.0));
        assert_eq!(clipped(from, target), Point::new(300.0, 50.0));
    }
}
