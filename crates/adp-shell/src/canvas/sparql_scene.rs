//! The scene of a SPARQL query diagram, which its module projects and lays out and hands the session as nodes, edges,
//! regions, annotations, the header band and the truncation banner (etalii.adp spec 030, task T084), drawn as
//! `adp-scene` primitives so the one painter, hit test and gesture arbiter serve it as they serve every other canvas.
//!
//! Ported from what the standalone host's `src/diagrams/sparql/client/SparqlCanvas.tsx` and `sparql.css` at `8b6ccfc`
//! drew, in the theme's tokens, light or dark:
//! - a region (OPTIONAL, a UNION and its branches, MINUS, GRAPH, SERVICE, the template) is an unfilled frame at the
//!   size the module gave it, OPTIONAL and MINUS dashed 4 4, SERVICE and GRAPH dotted 2 3, its label above it at 11;
//!   regions are drawn first, so a frame is behind the patterns it holds;
//! - a node is a 170 by 64 box, its display in the middle (2 below centre) cut with an ellipsis when it does not fit,
//!   a literal's datatype or language tag or a variable's defining expression 10 above its bottom at 10, muted;
//!   a variable's box dashed 5 3 (an anonymous one also at 0.7), a literal's corners square, a collapsed subquery's
//!   outline 2 wide, and a projected variable marked with a short arrow (`→`) 8 in from its top-left;
//! - a triple pattern is a straight line between the two boxes' edges with an arrowhead, its predicate or property
//!   path at its middle 6 above it, a property path dashed 6 2;
//! - a FILTER, BIND or VALUES is its text as written at 11, muted, centred on what it annotates: 6 above a node's
//!   top-left, 12 in and 18 down a region's, or above the open canvas;
//! - the query's form and modifier rows are the header band, above the diagram; the banner says how much a cut query
//!   shows.
//!
//! The one gesture is a drag: every node and region is moved by its top-left, which is what the registration stores
//! (`onElementMoved`); the module refuses whatever cannot move with its own sentence. Nothing here can edit the query.

use std::collections::{HashMap, HashSet};

use adp_diagram::DiagramElement;
use adp_diagram_sparql::mapper::{
    ANNOTATION_TYPE, EDGE_TYPE, HEADER_TYPE, REGION_TYPE, TERM_TYPE, TRUNCATION_TYPE, VARIABLE_TYPE, modifier_rows,
};
use adp_diagram_sparql::theme::palette;
use adp_scene::definition::labels::TextAnchor;
use adp_scene::notation::catalog::MarkerKind;
use adp_scene::scene::{EdgePrimitive, Hook, HookKind, Owner, Primitive, Role, ShapePrimitive, marker_shape};
use adp_scene::style::{Colour, Stroke};
use adp_scene::{Path, Point, Rect, Scene};
use adp_theme::Kind;

use super::module_scene::{
    AVERAGE_ADVANCE, ModuleScene, SELECTED_WIDTH, edge_point, font, number, selection_colour, text,
};

/// A node's drawn size, in the module's canvas units (`NODE_WIDTH`, `NODE_HEIGHT`).
const NODE_WIDTH: f64 = 170.0;
const NODE_HEIGHT: f64 = 64.0;

/// An annotation's nominal bounds, which only the hit test needs (`ANNOTATION_WIDTH`, `ANNOTATION_HEIGHT`).
const ANNOTATION_WIDTH: f64 = 120.0;
const ANNOTATION_HEIGHT: f64 = 14.0;

/// Where the header band's line sits, above the root grid the layout starts below it.
const HEADER_Y: f64 = 24.0;

fn stroke(colour: &str, width: f64, dash: &[f64], opacity: f64) -> Stroke {
    Stroke {
        colour: Some(Colour::value(colour)),
        width,
        dash: dash.to_vec(),
        opacity,
    }
}

/// `display` cut with an ellipsis to what `width` holds at `size` (`truncate: true`).
fn truncated(display: &str, width: f64, size: f64) -> String {
    let fits = ((width / (size * AVERAGE_ADVANCE)).floor() as usize).max(1);
    if display.chars().count() <= fits {
        return display.to_owned();
    }
    let kept: String = display.chars().take(fits.saturating_sub(1)).collect();
    format!("{kept}…")
}

/// Where an element is drawn: a region's frame at the size the module gave it, a node's box at its fixed size.
pub fn rect_of(element: &DiagramElement) -> Rect {
    if element.element_type == REGION_TYPE {
        Rect::new(
            element.x,
            element.y,
            number(element, "width"),
            number(element, "height"),
        )
    } else {
        Rect::new(element.x, element.y, NODE_WIDTH, NODE_HEIGHT)
    }
}

fn is_node(element: &DiagramElement) -> bool {
    element.element_type == VARIABLE_TYPE || element.element_type == TERM_TYPE
}

/// The scene of `elements`, a SPARQL query view's rendering, with `selection` outlined, the elements `moving` carried
/// by the drag in flight, in the palette of the active theme, `theme`.
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
    for element in elements.iter().filter(|e| e.element_type == REGION_TYPE || is_node(e)) {
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
    let moved = |id: &str| moving.is_some_and(|(ids, _)| ids.iter().any(|m| m == id));

    let mut primitives = Vec::new();
    let mut boxes = Vec::new();
    let mut lines = Vec::new();
    let mut labels = Vec::new();

    // The header band: the form, then each modifier row, on one line above the diagram.
    if let Some(header) = elements.iter().find(|e| e.element_type == HEADER_TYPE) {
        let owner = Owner::Element(header.id.clone());
        let form = header.attribute("form").to_owned();
        let mut x = 0.0;
        let advance = |s: &str| s.chars().count() as f64 * 12.0 * AVERAGE_ADVANCE + 12.0;
        x += advance(&form);
        primitives.push(text(
            &owner,
            Role::Label {
                id: "form".to_owned(),
                line: 0,
            },
            form,
            Point::new(0.0, HEADER_Y),
            TextAnchor::Start,
            font(12.0, 600, Some(palette.primary)),
        ));
        for (i, row) in modifier_rows(header).into_iter().enumerate() {
            let width = advance(&row);
            primitives.push(text(
                &owner,
                Role::Label {
                    id: "modifier".to_owned(),
                    line: i,
                },
                row,
                Point::new(x, HEADER_Y),
                TextAnchor::Start,
                font(12.0, 400, Some(palette.muted)),
            ));
            x += width;
        }
    }

    // The frames first, behind the patterns they contain.
    for region in elements.iter().filter(|e| e.element_type == REGION_TYPE) {
        let Some(rect) = drawn.get(&region.id).copied() else {
            continue;
        };
        let owner = Owner::Element(region.id.clone());
        let dash: &[f64] = match region.attribute("kind") {
            "optional" | "minus" => &[4.0, 4.0],
            "service" | "graph" => &[2.0, 3.0],
            _ => &[],
        };
        let outline = if selected.contains(region.id.as_str()) {
            stroke(highlight, SELECTED_WIDTH, dash, 1.0)
        } else {
            stroke(palette.border, 1.0, dash, 1.0)
        };
        primitives.push(Primitive::Shape(ShapePrimitive {
            owner: owner.clone(),
            role: Role::Body,
            path: Path::rounded_rect(rect, 0.0),
            fill: None,
            stroke: Some(outline),
            opacity: if moved(&region.id) { 0.75 } else { 1.0 },
            tooltip: Some(region.attribute("label").to_owned()),
        }));
        primitives.push(text(
            &owner,
            Role::Label {
                id: "label".to_owned(),
                line: 0,
            },
            region.attribute("label").to_owned(),
            Point::new(rect.x, rect.y - 4.0),
            TextAnchor::Start,
            font(11.0, 400, Some(palette.muted)),
        ));
        if selected.contains(region.id.as_str()) {
            primitives.push(Primitive::Hook(Hook {
                owner: owner.clone(),
                kind: HookKind::Selection,
                area: rect,
            }));
        }
        boxes.push((region.id.clone(), rect));
    }

    // The triple patterns, beneath the boxes they join.
    for edge in elements.iter().filter(|e| e.element_type == EDGE_TYPE) {
        let (Some(source), Some(target)) = (
            edge.source.as_deref().and_then(|s| drawn.get(s)).copied(),
            edge.target.as_deref().and_then(|t| drawn.get(t)).copied(),
        ) else {
            continue;
        };
        let owner = Owner::Relation(edge.id.clone());
        let from = edge_point(source, target.centre());
        let to = edge_point(target, source.centre());
        let dash: &[f64] = if edge.attribute("isPath") == "true" {
            &[6.0, 2.0]
        } else {
            &[]
        };
        let is_selected = selected.contains(edge.id.as_str());
        let line = if is_selected {
            stroke(highlight, SELECTED_WIDTH, dash, 1.0)
        } else {
            stroke(palette.border, 1.5, dash, 1.0)
        };
        let angle = (to.y - from.y).atan2(to.x - from.x);
        primitives.push(Primitive::Edge(EdgePrimitive {
            owner: owner.clone(),
            path: Path::polyline(&[from, to]),
            end: marker_shape(MarkerKind::Arrow, to, angle, line.width),
            stroke: line,
            start: None,
            hit_width: 10.0,
            selectable: true,
            tooltip: None,
        }));
        let middle = Point::new((from.x + to.x) / 2.0, (from.y + to.y) / 2.0);
        lines.push((edge.id.clone(), vec![from, to], 5.0));
        let label = edge.attribute("label");
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

    // The nodes: what the query asks for, what it pins down, and what a subquery hands back.
    for node in elements.iter().filter(|e| is_node(e)) {
        let Some(rect) = drawn.get(&node.id).copied() else {
            continue;
        };
        let owner = Owner::Element(node.id.clone());
        let variable = node.element_type == VARIABLE_TYPE;
        let anonymous = variable && node.attribute("anonymous") == "true";
        let literal = !variable && node.attribute("kind") == "literal";
        let subquery = !variable && node.attribute("annotation") == "subquery";
        let dash: &[f64] = if variable { &[5.0, 3.0] } else { &[] };
        let outline = if selected.contains(node.id.as_str()) {
            stroke(highlight, SELECTED_WIDTH, dash, 1.0)
        } else {
            stroke(palette.border, if subquery { 2.0 } else { 1.0 }, dash, 1.0)
        };
        let opacity = match (anonymous, moved(&node.id)) {
            (true, _) => 0.7,
            (false, true) => 0.75,
            (false, false) => 1.0,
        };
        let (display, annotation) = if variable {
            (node.attribute("name"), node.attribute("definingExpression"))
        } else {
            (node.attribute("display"), node.attribute("annotation"))
        };
        primitives.push(Primitive::Shape(ShapePrimitive {
            owner: owner.clone(),
            role: Role::Body,
            path: Path::rounded_rect(rect, if literal { 0.0 } else { 4.0 }),
            fill: Some(Colour::value(palette.surface)),
            stroke: Some(outline),
            opacity,
            tooltip: Some(if variable {
                display.to_owned()
            } else {
                node.attribute("full").to_owned()
            }),
        }));
        primitives.push(text(
            &owner,
            Role::Label {
                id: "display".to_owned(),
                line: 0,
            },
            truncated(display, rect.width - 16.0, 13.0),
            Point::new(rect.centre().x, rect.y + NODE_HEIGHT / 2.0 + 2.0 + 4.5),
            TextAnchor::Middle,
            font(13.0, 400, Some(palette.text)),
        ));
        labels.push((node.id.clone(), "display".to_owned(), rect));
        if !annotation.is_empty() {
            primitives.push(text(
                &owner,
                Role::Label {
                    id: "annotation".to_owned(),
                    line: 0,
                },
                truncated(annotation, rect.width - 16.0, 10.0),
                Point::new(rect.x + 8.0, rect.y + NODE_HEIGHT - 10.0),
                TextAnchor::Start,
                font(10.0, 400, Some(palette.muted)),
            ));
        }
        if variable && node.attribute("projected") == "true" {
            // What leaves the query, marked so the header never has to be consulted: the `→` glyph at 12, drawn as a
            // short arrow on the glyph's middle line, since the canvas font has no arrows.
            let (from, to) = (
                Point::new(rect.x + 8.0, rect.y + 12.0),
                Point::new(rect.x + 18.0, rect.y + 12.0),
            );
            let mark = stroke(palette.primary, 1.2, &[], 1.0);
            primitives.push(Primitive::Edge(EdgePrimitive {
                owner: owner.clone(),
                path: Path::polyline(&[from, to]),
                end: marker_shape(MarkerKind::Arrow, to, 0.0, mark.width),
                stroke: mark,
                start: None,
                hit_width: 0.0,
                selectable: false,
                tooltip: None,
            }));
        }
        if selected.contains(node.id.as_str()) {
            primitives.push(Primitive::Hook(Hook {
                owner: owner.clone(),
                kind: HookKind::Selection,
                area: rect,
            }));
        }
        boxes.push((node.id.clone(), rect));
    }

    // The badges last, so what they annotate never paints over them.
    for annotation in elements.iter().filter(|e| e.element_type == ANNOTATION_TYPE) {
        let attached = annotation.attribute("attachedTo");
        let anchor = match drawn.get(attached) {
            Some(rect) if elements.iter().any(|e| e.id == attached && is_node(e)) => Point::new(rect.x, rect.y - 6.0),
            Some(rect) => Point::new(rect.x + 12.0, rect.y + 18.0),
            // A root-scope filter constrains the whole query and floats above its patterns.
            None => Point::new(0.0, -10.0),
        };
        let owner = Owner::Element(annotation.id.clone());
        let is_selected = selected.contains(annotation.id.as_str());
        primitives.push(text(
            &owner,
            Role::Label {
                id: "text".to_owned(),
                line: 0,
            },
            annotation.attribute("text").to_owned(),
            Point::new(anchor.x, anchor.y + 4.0),
            TextAnchor::Middle,
            font(
                11.0,
                if is_selected { 600 } else { 400 },
                Some(if is_selected { highlight } else { palette.muted }),
            ),
        ));
        // The badge is centred on its anchor, as the canvas library centres an element on its position.
        let width =
            (annotation.attribute("text").chars().count() as f64 * 11.0 * AVERAGE_ADVANCE).max(ANNOTATION_WIDTH);
        boxes.push((
            annotation.id.clone(),
            Rect::new(
                anchor.x - width / 2.0,
                anchor.y - ANNOTATION_HEIGHT / 2.0,
                width,
                ANNOTATION_HEIGHT,
            ),
        ));
    }

    // The banner over a view the sanity bound cut.
    if let Some(banner) = elements.iter().find(|e| e.element_type == TRUNCATION_TYPE) {
        primitives.push(text(
            &Owner::Element(banner.id.clone()),
            Role::Label {
                id: "banner".to_owned(),
                line: 0,
            },
            format!(
                "Showing {} of {} elements — this query is larger than the diagram draws",
                banner.attribute("shown"),
                banner.attribute("total")
            ),
            Point::new(0.0, HEADER_Y + 20.0),
            TextAnchor::Start,
            font(12.0, 400, Some(palette.warning)),
        ));
    }

    let top_left = rects.keys().cloned().collect();
    ModuleScene {
        scene: Scene {
            primitives,
            empty_text: None,
        },
        top_left,
        rects,
        boxes,
        lines,
        labels,
        anchors: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::truncated;

    #[test]
    fn a_long_display_is_cut_with_an_ellipsis() {
        assert_eq!(truncated("?x", 154.0, 13.0), "?x");
        let long = "a".repeat(40);
        let cut = truncated(&long, 154.0, 13.0);
        assert!(cut.ends_with('…') && cut.chars().count() == 21, "{cut}");
    }
}
