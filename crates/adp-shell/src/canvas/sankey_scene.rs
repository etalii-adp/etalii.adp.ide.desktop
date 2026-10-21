//! The scene of a Sankey diagram, which its module lays out and hands the session as bars and bands (etalii.adp spec
//! 030, FR-018, task T077), drawn here as `adp-scene` primitives so the one painter, hit test and gesture arbiter serve
//! it as they serve every other canvas.
//!
//! Ported from what the standalone host's `src/diagrams/sankey/client/SankeyCanvas.tsx`, `sankey.css` and the
//! `--color-diagram-sankey-*` tokens of `src/client/src/index.css` at `8b6ccfc` draw:
//! - a node is a bar at the size and centre the module gave it, painted in its palette word's colour or its own hex;
//!   its name (13, bold, in its colour) 5 above its middle, its value in its format (12) 11 below and its note (11,
//!   italic, muted) 26 below, before the bar in the first column and after it elsewhere, 10 from its edge;
//! - a flow is a band filled at 0.42 opacity between the right edge of its source and the left edge of its target, as
//!   thick as its value all the way along: two horizontal S curves its thickness apart, each pulled half the distance
//!   it travels and at least 40, its centre leaving and reaching each bar where the module's fractions say;
//! - a drag moves a node up and down its column only (`COLUMN_LOCK`), and the position it reports is its bar's top left,
//!   as the client's `moveElementTo` sent it;
//! - a bar has an anchor in the middle of each side edge, from which a flow is drawn (`anchors: { kind: "along",
//!   edges: ["left", "right"] }`).
//!
//! Everything drawn was decided by the module: the canvas stays a renderer. A selected bar or band is outlined in the
//! selected colour.

use std::collections::{HashMap, HashSet};

use adp_diagram::DiagramElement;
use adp_scene::definition::labels::TextAnchor;
use adp_scene::scene::{Hook, HookKind, Owner, Primitive, Role, ShapePrimitive, TextRun};
use adp_scene::style::{Colour, Font, Stroke};
use adp_scene::{Path, Point, Rect, Scene};

use super::module_scene::ModuleScene;

/// The element types the Sankey module puts on the wire (`adp_diagram_sankey::mapper`).
const NODE_TYPE: &str = "etalii/sankey+node";
const FLOW_TYPE: &str = "etalii/sankey+flow";

/// The selected outline, as the scene builder paints it (`HIGHLIGHT_STROKE`).
const SELECTED: &str = "#7c3aed";

/// How far a label sits from its bar's edge.
const LABEL_GAP: f64 = 10.0;

/// How wide the field a rename is typed in is.
const NAME_FIELD_WIDTH: f64 = 180.0;

/// The radius of an anchor's dot, in canvas units.
const ANCHOR_RADIUS: f64 = 5.0;

/// A palette word's colour (`index.css`, the light theme's `--color-diagram-sankey-*`).
fn palette(word: &str) -> &'static str {
    match word {
        "slate" => "#2f5b8a",
        "purple" => "#86198f",
        "blue" => "#1d4ed8",
        "teal" => "#0f766e",
        "green" => "#15803d",
        "lime" => "#4d7c0f",
        "yellow" => "#a16207",
        "orange" => "#c2410c",
        "red" => "#a3151a",
        "pink" => "#be185d",
        "brown" => "#7c4a1e",
        _ => "#666666",
    }
}

/// What an element is painted in: its own hex, else its palette word's colour (`paintOf`).
fn paint_of(element: &DiagramElement) -> String {
    let custom = element.attribute("customColor");
    if custom.is_empty() {
        palette(element.attribute("color")).to_owned()
    } else {
        custom.to_owned()
    }
}

fn number(element: &DiagramElement, name: &str) -> f64 {
    element.attribute(name).parse().unwrap_or_default()
}

/// How strongly a band's curve pulls: half the distance it travels, so it leaves and arrives level (`pullOf`).
fn pull_of(from: Point, to: Point) -> f64 {
    ((to.x - from.x).abs() / 2.0).max(40.0)
}

/// The band itself, filled: its top and bottom edges the same S, `thickness` apart vertically (`bandPath`).
pub fn band_path(from: Point, to: Point, thickness: f64) -> Path {
    let half = thickness / 2.0;
    let pull = pull_of(from, to);
    let (c0, c1) = (from.x + pull, to.x - pull);
    Path::new()
        .move_to(Point::new(from.x, from.y - half))
        .cubic_to(
            Point::new(c0, from.y - half),
            Point::new(c1, to.y - half),
            Point::new(to.x, to.y - half),
        )
        .line_to(Point::new(to.x, to.y + half))
        .cubic_to(
            Point::new(c1, to.y + half),
            Point::new(c0, from.y + half),
            Point::new(from.x, from.y + half),
        )
        .close()
}

/// The band's centre line as points along it: what a press on the band is hit-tested against (`centrePath`).
fn centre_line(from: Point, to: Point) -> Vec<Point> {
    let pull = pull_of(from, to);
    let (c0, c1) = (Point::new(from.x + pull, from.y), Point::new(to.x - pull, to.y));
    (0..=16)
        .map(|step| {
            let t = f64::from(step) / 16.0;
            let u = 1.0 - t;
            let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            Point::new(
                a * from.x + b * c0.x + c * c1.x + d * to.x,
                a * from.y + b * c0.y + c * c1.y + d * to.y,
            )
        })
        .collect()
}

fn font(size: f64, weight: u16, italic: bool, colour: Option<&str>) -> Font {
    Font {
        family: None,
        size,
        weight,
        italic,
        colour: colour.map(Colour::value),
        underline: false,
    }
}

fn text(owner: &Owner, id: &str, text: String, position: Point, anchor: TextAnchor, font: Font) -> Primitive {
    Primitive::Text(TextRun {
        owner: owner.clone(),
        role: Role::Label {
            id: id.to_owned(),
            line: 0,
        },
        text,
        position,
        anchor,
        font,
        wrap: None,
        tooltip: None,
        editable: false,
    })
}

/// The scene of `elements`, the Sankey module's rendering, with `selection` outlined and the nodes `moving` carried up
/// or down their column by the drag in flight.
pub fn build(
    elements: &[DiagramElement],
    selection: &[String],
    moving: Option<(&[String], (f64, f64))>,
) -> ModuleScene {
    let selected: HashSet<&str> = selection.iter().map(String::as_str).collect();
    let nodes: Vec<&DiagramElement> = elements.iter().filter(|e| e.element_type == NODE_TYPE).collect();
    let mut rects: HashMap<String, Rect> = HashMap::new();
    for node in &nodes {
        rects.insert(
            node.id.clone(),
            Rect::centred(
                Point::new(node.x, node.y),
                number(node, "width"),
                number(node, "height"),
            ),
        );
    }
    let mut drawn = rects.clone();
    if let Some((ids, (_, dy))) = moving {
        // A node moves up and down its column, never out of it.
        for id in ids {
            if let Some(rect) = drawn.get_mut(id) {
                *rect = Rect::new(rect.x, rect.y + dy, rect.width, rect.height);
            }
        }
    }

    let mut primitives = Vec::new();
    let mut lines = Vec::new();

    // The bands first, under the bars they join.
    for flow in elements.iter().filter(|e| e.element_type == FLOW_TYPE) {
        let (Some(source), Some(target)) = (
            flow.source.as_deref().and_then(|s| drawn.get(s)),
            flow.target.as_deref().and_then(|t| drawn.get(t)),
        ) else {
            continue;
        };
        let from = Point::new(source.right(), source.y + number(flow, "sourceAt") * source.height);
        let to = Point::new(target.x, target.y + number(flow, "targetAt") * target.height);
        let thickness = number(flow, "thickness");
        let is_selected = selected.contains(flow.id.as_str());
        primitives.push(Primitive::Shape(ShapePrimitive {
            owner: Owner::Relation(flow.id.clone()),
            role: Role::Body,
            path: band_path(from, to, thickness),
            fill: Some(Colour::value(&paint_of(flow))),
            stroke: is_selected.then(|| Stroke {
                colour: Some(Colour::value(SELECTED)),
                width: 2.0,
                dash: Vec::new(),
                opacity: 1.0,
            }),
            opacity: 0.42,
            tooltip: Some(flow.attribute("displayValue").to_owned()),
        }));
        lines.push((flow.id.clone(), centre_line(from, to), (thickness / 2.0).max(5.0)));
    }

    // The bars, with their labels beside them.
    let mut boxes = Vec::new();
    let mut anchors = Vec::new();
    let mut labels = Vec::new();
    for node in &nodes {
        let Some(rect) = drawn.get(&node.id).copied() else {
            continue;
        };
        let owner = Owner::Element(node.id.clone());
        let paint = paint_of(node);
        let is_selected = selected.contains(node.id.as_str());
        primitives.push(Primitive::Shape(ShapePrimitive {
            owner: owner.clone(),
            role: Role::Body,
            path: Path::rounded_rect(rect, 0.0),
            fill: Some(Colour::value(&paint)),
            stroke: is_selected.then(|| Stroke {
                colour: Some(Colour::value(SELECTED)),
                width: 3.0,
                dash: Vec::new(),
                opacity: 1.0,
            }),
            opacity: if moving.is_some_and(|(ids, _)| ids.contains(&node.id)) {
                0.75
            } else {
                1.0
            },
            tooltip: None,
        }));
        let left = node.attribute("side") == "left";
        let (x, anchor) = if left {
            (rect.x - LABEL_GAP, TextAnchor::End)
        } else {
            (rect.right() + LABEL_GAP, TextAnchor::Start)
        };
        let middle = rect.centre().y;
        // Where a rename is typed: over the name, on the bar's side of it.
        labels.push((
            node.id.clone(),
            "name".to_owned(),
            if left {
                Rect::new(x - NAME_FIELD_WIDTH, middle - 20.0, NAME_FIELD_WIDTH, 20.0)
            } else {
                Rect::new(x, middle - 20.0, NAME_FIELD_WIDTH, 20.0)
            },
        ));
        primitives.push(text(
            &owner,
            "name",
            node.attribute("name").to_owned(),
            Point::new(x, middle - 5.0),
            anchor,
            font(13.0, 700, false, Some(&paint)),
        ));
        primitives.push(text(
            &owner,
            "value",
            node.attribute("displayValue").to_owned(),
            Point::new(x, middle + 11.0),
            anchor,
            font(12.0, 400, false, None),
        ));
        let note = node.attribute("note");
        if !note.is_empty() {
            primitives.push(text(
                &owner,
                "note",
                note.to_owned(),
                Point::new(x, middle + 26.0),
                anchor,
                font(11.0, 400, true, Some("#6b7280")),
            ));
        }
        if is_selected {
            primitives.push(Primitive::Hook(Hook {
                owner: owner.clone(),
                kind: HookKind::Selection,
                area: rect,
            }));
        }
        for (name, at) in [
            ("left", Point::new(rect.x, middle)),
            ("right", Point::new(rect.right(), middle)),
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
        boxes.push((node.id.clone(), rect));
    }

    // A drag reports a bar's top left, as the client did.
    let top_left = rects.keys().cloned().collect();
    ModuleScene {
        scene: Scene {
            primitives,
            empty_text: None,
        },
        rects,
        boxes,
        lines,
        labels,
        anchors,
        top_left,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_band_is_as_thick_as_its_value_at_both_ends() {
        let path = band_path(Point::new(0.0, 100.0), Point::new(296.0, 50.0), 20.0);
        let bounds = path.bounds().unwrap();
        assert_eq!((bounds.x, bounds.right()), (0.0, 296.0));
        assert_eq!(path.start(), Some(Point::new(0.0, 90.0)));
        let line = centre_line(Point::new(0.0, 100.0), Point::new(296.0, 50.0));
        assert_eq!(line.first(), Some(&Point::new(0.0, 100.0)));
        assert_eq!(line.last(), Some(&Point::new(296.0, 50.0)));
    }

    #[test]
    fn a_palette_word_is_painted_in_its_theme_colour_and_a_hex_as_written() {
        let mut node = DiagramElement::node("a", NODE_TYPE);
        node.attributes.push(("color".into(), "blue".into()));
        node.attributes.push(("customColor".into(), String::new()));
        assert_eq!(paint_of(&node), "#1d4ed8");
        node.attributes[1].1 = "#aa5500".into();
        assert_eq!(paint_of(&node), "#aa5500");
    }
}
