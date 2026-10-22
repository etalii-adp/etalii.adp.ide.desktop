//! The functional decomposition graph as its module hands it over, drawn as `adp-scene` primitives (etalii.adp spec
//! 030, task T072), beside the C4 views in [`super::module_scene`].
//!
//! Ported from what the standalone host's `src/diagrams/functional-decomposition-graph/client/FdgCanvas.tsx` and
//! `fdg.css` at `8b6ccfc` draw, in the geometry `definitions/diagrams/functional-decomposition-graph.dis` states:
//! - a UI element is a superellipse (four cubic quarters, control factor 0.919), a data element a parallelogram with
//!   its top shifted right, an action a trapezoid narrower at the bottom, a function a diode (square on the left,
//!   closed on the right by a semicircle of radius `min(h / 2, w)`) and a Comment a box, each filled with its type's
//!   theme colour, at the size the module gave it;
//! - a named element's name in the middle, cut with an ellipsis where it does not fit; a Comment's text wrapped
//!   inside it, line by line;
//! - a named element has an anchor on each edge a connection is drawn from; a Comment has none, since no relation
//!   takes it;
//! - a connection is a cubic curve from edge to edge with a filled arrow at the child, its name at the midpoint.
//!
//! The module decides everything drawn; positions it sends are centres, and a move writes a top-left, which is the
//! mapping the canvas gives these elements.

use std::collections::{HashMap, HashSet};

use adp_diagram::{DiagramElement, ElementKind};
use adp_diagram_functional_decomposition_graph::theme::palette;
use adp_scene::definition::labels::TextAnchor;
use adp_scene::notation::catalog::MarkerKind;
use adp_scene::scene::{EdgePrimitive, Hook, HookKind, Owner, Primitive, Role, ShapePrimitive, TextRun, marker_shape};
use adp_scene::style::{Colour, Font, Stroke};
use adp_scene::{Path, Point, Rect};
use adp_theme::Kind;

/// What the module prefixes every element and connection type with (`adp_diagram_functional_decomposition_graph`).
pub const PREFIX: &str = "etalii/functional-decomposition-graph+";

const SELECTED_WIDTH: f64 = 3.0;
const ANCHOR_RADIUS: f64 = 5.0;
const FONT_SIZE: f64 = 13.0;
const AVERAGE_ADVANCE: f64 = 0.55;
const LINE_HEIGHT: f64 = 16.0;

/// Whether `element` is one the functional decomposition graph's module sends.
pub fn is_fdg(element: &DiagramElement) -> bool {
    element.element_type.starts_with(PREFIX)
}

fn type_of(element: &DiagramElement) -> &str {
    &element.element_type[PREFIX.len()..]
}

fn number(element: &DiagramElement, name: &str) -> f64 {
    element.attribute(name).parse().unwrap_or_default()
}

/// The box an element is drawn in: centred on what the module sent, at its size.
pub fn rect_of(element: &DiagramElement) -> Rect {
    Rect::centred(
        Point::new(element.x, element.y),
        number(element, "width"),
        number(element, "height"),
    )
}

/// The outline of an element of `element_type` in `b`.
pub fn outline(element_type: &str, b: Rect) -> Path {
    let (x, y, w, h) = (b.x, b.y, b.width, b.height);
    let at = |fx: f64, fy: f64| Point::new(x + w * fx, y + h * fy);
    match element_type {
        "ui-element" => Path::new()
            .move_to(at(1.0, 0.5))
            .cubic_to(at(1.0, 0.9595), at(0.9595, 1.0), at(0.5, 1.0))
            .cubic_to(at(0.0405, 1.0), at(0.0, 0.9595), at(0.0, 0.5))
            .cubic_to(at(0.0, 0.0405), at(0.0405, 0.0), at(0.5, 0.0))
            .cubic_to(at(0.9595, 0.0), at(1.0, 0.0405), at(1.0, 0.5))
            .close(),
        "data-element" => Path::polygon(&[at(0.2, 0.0), at(1.0, 0.0), at(0.8, 1.0), at(0.0, 1.0)]),
        "action" => Path::polygon(&[at(0.0, 0.0), at(1.0, 0.0), at(0.85, 1.0), at(0.15, 1.0)]),
        "function" => {
            let r = (h / 2.0).min(w);
            Path::new()
                .move_to(Point::new(x, y))
                .line_to(Point::new(x + w - r, y))
                .arc_to(r, r, 0.0, false, true, Point::new(x + w - r, y + h))
                .line_to(Point::new(x, y + h))
                .close()
        }
        _ => Path::rounded_rect(b, 2.0),
    }
}

/// Where the line from the centre of `rect` towards `towards` leaves it.
fn edge_point(rect: Rect, towards: Point) -> Point {
    let centre = rect.centre();
    let (dx, dy) = (towards.x - centre.x, towards.y - centre.y);
    if dx == 0.0 && dy == 0.0 {
        return centre;
    }
    let scale_x = if dx == 0.0 {
        f64::INFINITY
    } else {
        (rect.width / 2.0) / dx.abs()
    };
    let scale_y = if dy == 0.0 {
        f64::INFINITY
    } else {
        (rect.height / 2.0) / dy.abs()
    };
    let scale = scale_x.min(scale_y);
    Point::new(centre.x + dx * scale, centre.y + dy * scale)
}

/// `text` whole when `capacity` characters of it fit, else cut with an ellipsis.
fn fit(text: &str, capacity: usize) -> String {
    let characters: Vec<char> = text.chars().collect();
    if characters.len() <= capacity {
        return text.to_owned();
    }
    if capacity <= 1 {
        return "…".to_owned();
    }
    let kept: String = characters[..capacity - 1].iter().collect();
    format!("{kept}…")
}

/// A Comment's text broken at its own newlines and then at spaces into lines no wider than `per_line` characters.
fn wrapped(text: &str, per_line: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut current = String::new();
        for word in paragraph.trim_end_matches('\r').split(' ') {
            let length = current.chars().count();
            if length > 0 && length + 1 + word.chars().count() > per_line {
                lines.push(std::mem::take(&mut current));
            } else if length > 0 {
                current.push(' ');
            }
            current.push_str(word);
        }
        lines.push(current);
    }
    lines
}

fn font(colour: &str, weight: u16) -> Font {
    Font {
        family: None,
        size: FONT_SIZE,
        weight,
        italic: false,
        colour: Some(Colour::value(colour)),
        underline: false,
    }
}

fn stroke(colour: &str, width: f64) -> Stroke {
    Stroke {
        colour: Some(Colour::value(colour)),
        width,
        dash: Vec::new(),
        opacity: 1.0,
    }
}

fn text(owner: &Owner, role: Role, content: String, position: Point, font: Font) -> Primitive {
    Primitive::Text(TextRun {
        owner: owner.clone(),
        role,
        text: content,
        position,
        anchor: TextAnchor::Middle,
        font,
        wrap: None,
        tooltip: None,
        editable: false,
    })
}

/// What drawing the graph adds to a module's scene.
#[derive(Default)]
pub struct Drawn {
    pub primitives: Vec<Primitive>,
    pub boxes: Vec<(String, Rect)>,
    pub lines: Vec<(String, Vec<Point>, f64)>,
    pub labels: Vec<(String, String, Rect)>,
    pub anchors: Vec<(String, String, Rect)>,
}

/// Draws the graph's connections and then its elements, at the boxes `drawn` gives them (a drag in flight included),
/// `selected` outlined, in the palette of the active theme, `theme`, the elements `moving` faded.
pub fn draw(
    elements: &[DiagramElement],
    drawn: &HashMap<String, Rect>,
    selected: &HashSet<&str>,
    moving: &HashSet<&str>,
    theme: Kind,
) -> Drawn {
    let palette = palette(theme);
    let highlight = super::module_scene::selection_colour(theme);
    let mut out = Drawn::default();

    for connection in elements.iter().filter(|e| is_fdg(e) && e.kind == ElementKind::Relation) {
        let (Some(source), Some(target)) = (
            connection.source.as_deref().and_then(|s| drawn.get(s)),
            connection.target.as_deref().and_then(|t| drawn.get(t)),
        ) else {
            continue;
        };
        let from = edge_point(*source, target.centre());
        let to = edge_point(*target, source.centre());
        // A cubic curve bowed along the longer axis, as `cubic-bezier` routes it.
        let (dx, dy) = (to.x - from.x, to.y - from.y);
        let (first, second) = if dx.abs() >= dy.abs() {
            (Point::new(from.x + dx / 2.0, from.y), Point::new(to.x - dx / 2.0, to.y))
        } else {
            (Point::new(from.x, from.y + dy / 2.0), Point::new(to.x, to.y - dy / 2.0))
        };
        let owner = Owner::Relation(connection.id.clone());
        let is_selected = selected.contains(connection.id.as_str());
        let line = if is_selected {
            stroke(highlight, SELECTED_WIDTH)
        } else {
            stroke(palette.border, 1.5)
        };
        let angle = (to.y - second.y).atan2(to.x - second.x);
        out.primitives.push(Primitive::Edge(EdgePrimitive {
            owner: owner.clone(),
            path: Path::new().move_to(from).cubic_to(first, second, to),
            end: marker_shape(MarkerKind::Arrow, to, angle, line.width),
            stroke: line,
            start: None,
            hit_width: 10.0,
            selectable: true,
            tooltip: None,
        }));
        out.lines
            .push((connection.id.clone(), vec![from, first, second, to], 5.0));
        let name = connection.attribute("name");
        if !name.is_empty() {
            let mid = Point::new(
                0.125 * from.x + 0.375 * first.x + 0.375 * second.x + 0.125 * to.x,
                0.125 * from.y + 0.375 * first.y + 0.375 * second.y + 0.125 * to.y,
            );
            out.primitives.push(text(
                &owner,
                Role::LineLabel,
                name.to_owned(),
                Point::new(mid.x, mid.y - 6.0),
                font(if is_selected { highlight } else { palette.text }, 400),
            ));
        }
    }

    for element in elements.iter().filter(|e| is_fdg(e) && e.kind == ElementKind::Node) {
        let Some(rect) = drawn.get(&element.id).copied() else {
            continue;
        };
        let element_type = type_of(element);
        let owner = Owner::Element(element.id.clone());
        let is_selected = selected.contains(element.id.as_str());
        out.primitives.push(Primitive::Shape(ShapePrimitive {
            owner: owner.clone(),
            role: Role::Body,
            path: outline(element_type, rect),
            fill: Some(Colour::value(palette.fill(element_type))),
            stroke: Some(if is_selected {
                stroke(highlight, SELECTED_WIDTH)
            } else {
                stroke(palette.border, 1.0)
            }),
            opacity: if moving.contains(element.id.as_str()) {
                0.75
            } else {
                1.0
            },
            tooltip: None,
        }));
        let per_line = ((rect.width - 12.0) / (FONT_SIZE * AVERAGE_ADVANCE)).floor().max(1.0) as usize;
        if element_type == "comment" {
            for (index, line) in wrapped(element.attribute("text"), per_line).into_iter().enumerate() {
                out.primitives.push(text(
                    &owner,
                    Role::Label {
                        id: "text".to_owned(),
                        line: index,
                    },
                    line,
                    Point::new(rect.centre().x, rect.y + 20.0 + LINE_HEIGHT * index as f64),
                    font(palette.text, 400),
                ));
            }
            out.labels.push((
                element.id.clone(),
                "text".to_owned(),
                Rect::new(rect.x, rect.y + 4.0, rect.width, rect.height - 8.0),
            ));
        } else {
            out.primitives.push(text(
                &owner,
                Role::Label {
                    id: "name".to_owned(),
                    line: 0,
                },
                fit(element.attribute("name"), per_line),
                Point::new(rect.centre().x, rect.centre().y + FONT_SIZE / 3.0),
                font(palette.text, 600),
            ));
            out.labels.push((
                element.id.clone(),
                "name".to_owned(),
                Rect::new(rect.x, rect.centre().y - 10.0, rect.width, 20.0),
            ));
            // An anchor on each edge a connection is drawn from; a Comment, which no relation takes, has none.
            let (centre_x, centre_y) = (rect.centre().x, rect.centre().y);
            for (name, at) in [
                ("top", Point::new(centre_x, rect.y)),
                ("right", Point::new(rect.right(), centre_y)),
                ("bottom", Point::new(centre_x, rect.bottom())),
                ("left", Point::new(rect.x, centre_y)),
            ] {
                let area = Rect::centred(at, 2.0 * ANCHOR_RADIUS, 2.0 * ANCHOR_RADIUS);
                out.primitives.push(Primitive::Hook(Hook {
                    owner: owner.clone(),
                    kind: HookKind::Anchor {
                        name: name.to_owned(),
                        visible: false,
                    },
                    area,
                }));
                out.anchors.push((element.id.clone(), name.to_owned(), area));
            }
        }
        if is_selected {
            out.primitives.push(Primitive::Hook(Hook {
                owner: owner.clone(),
                kind: HookKind::Selection,
                area: rect,
            }));
        }
        out.boxes.push((element.id.clone(), rect));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_comment_breaks_at_its_newlines_and_then_at_spaces() {
        assert_eq!(
            wrapped("Photos upload last.\nThey are the largest of all.", 12),
            ["Photos", "upload last.", "They are the", "largest of", "all."]
        );
    }

    #[test]
    fn each_type_has_its_outline() {
        let b = Rect::new(0.0, 0.0, 160.0, 48.0);
        assert_eq!(outline("action", b).bounds(), Some(b));
        assert_eq!(outline("data-element", b).bounds(), Some(b));
        assert_eq!(outline("ui-element", b).bounds().map(|r| r.width.round()), Some(160.0));
        assert!(outline("function", b).bounds().is_some());
    }
}
