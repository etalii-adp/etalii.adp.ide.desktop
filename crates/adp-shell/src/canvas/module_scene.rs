//! The scene of a diagram its module draws itself (etalii.adp spec 030, FR-018, task T061): the C4 views, whose
//! module lays them out and hands the session boxes, lines, boundaries and the view's title and key as
//! [`DiagramElement`]s, drawn here as `adp-scene` primitives so the one painter, hit test and gesture arbiter serve
//! them as they serve every other canvas.
//!
//! Ported from what the standalone host's `src/diagrams/c4/client/C4Canvas.tsx` and `c4.css` at `8b6ccfc` draw:
//! - an element is a styled box at the size and in the palette the module gave it (a rounded box, a box, a person, a
//!   cylinder), its name 22 below its top at 13 and bold, its bracketed type line 38 below at 10, cut with an ellipsis
//!   when it does not fit, and its description from 58 below at 10, broken at spaces into lines 14 apart, each no
//!   wider than the card holds at 0.55 of the font size per character less 12 either side;
//! - an element has an anchor in the middle of each edge, drawn while it is selected or under the pointer, which a
//!   connection is drawn from (`anchors: { kind: "edge" }`);
//! - a boundary is a dashed rectangle with `{name} [{kind}]` above it, drawn under everything and never picked;
//! - a relationship is a dashed line with an arrowhead, between the two boxes' edges, labelled at its midpoint 6
//!   above it with `{order}. {description} [{technology}]`;
//! - the view's title above the diagram and its key below it, each entry a swatch of the colour it explains.
//!
//! Everything drawn was decided by the module: the canvas stays a renderer. A selected element or line is outlined in
//! the selected colour, as every other canvas outlines it.

use std::collections::{HashMap, HashSet};

use adp_diagram::{DiagramElement, ElementKind};
use adp_diagram_c4::theme as c4_theme;
use adp_scene::definition::labels::TextAnchor;
use adp_scene::notation::catalog::MarkerKind;
use adp_scene::scene::{EdgePrimitive, Hook, HookKind, Owner, Primitive, Role, ShapePrimitive, TextRun, marker_shape};
use adp_scene::style::{Colour, Font, Stroke};
use adp_scene::{Path, Point, Rect, Scene};
use adp_theme::Kind;

use super::fdg_scene;

/// The element types the C4 module puts on the wire (`adp_diagram_c4::mapper`).
const NODE_TYPE: &str = "c4/model+node";
const RELATIONSHIP_TYPE: &str = "c4/model+relationship";
const BOUNDARY_TYPE: &str = "c4/model+boundary";
const VIEW_TYPE: &str = "c4/model+view";

/// The selected outline, as the scene builder paints it (`HIGHLIGHT_STROKE`), in the light theme and in the dark one,
/// where the standalone host's `index.css` lifts the same violet so it still reads on the dark canvas (`--color-selected`).
pub(super) const SELECTED: &str = "#7c3aed";
pub(super) const SELECTED_DARK: &str = "#a78bfa";
pub(super) const SELECTED_WIDTH: f64 = 3.0;

/// The colour a selected element is outlined in, in the active theme `kind`.
pub(super) fn selection_colour(kind: Kind) -> &'static str {
    match kind {
        Kind::Light => SELECTED,
        Kind::Dark => SELECTED_DARK,
    }
}

/// The average advance of a character, as a fraction of the font size (`textMetrics.ts`).
pub(super) const AVERAGE_ADVANCE: f64 = 0.55;

/// A module's view as the canvas draws and hit-tests it.
pub struct ModuleScene {
    pub scene: Scene,
    /// Every element's box, by id, in canvas units: what a drag moves.
    pub rects: HashMap<String, Rect>,
    /// Each element's box in painting order, the topmost last.
    pub boxes: Vec<(String, Rect)>,
    /// Each relationship's line, and how far from it a press still hits it.
    pub lines: Vec<(String, Vec<Point>, f64)>,
    /// Each element's name line: what the inline editor of a rename opens over.
    pub labels: Vec<(String, String, Rect)>,
    /// Each card's anchors on the middle of its four edges: element, anchor, area (`anchors: { kind: "edge" }`).
    pub anchors: Vec<(String, String, Rect)>,
    /// The elements whose module writes a move as their top-left rather than their centre (the functional
    /// decomposition graph's, `FdgCanvas.tsx`'s `onElementMoved`).
    pub top_left: Vec<String>,
}

/// The radius of an anchor's dot, in canvas units (the scene builder's `ANCHOR_RADIUS`).
pub(super) const ANCHOR_RADIUS: f64 = 5.0;

pub(super) fn number(element: &DiagramElement, name: &str) -> f64 {
    element.attribute(name).parse().unwrap_or_default()
}

/// How many characters of `font_size` fit across `width` (`capacityOf`).
fn capacity_of(width: f64, font_size: f64) -> usize {
    ((width / (font_size * AVERAGE_ADVANCE) + 1e-9).floor() as usize).max(1)
}

/// How many characters of the card's two small lines fit across a card `width` wide (`charactersAcross`).
fn characters_across(width: f64) -> usize {
    capacity_of(width - 2.0 * 12.0, 10.0)
}

/// `text` whole when it fits `capacity`, else cut with an ellipsis (`fitToCapacity`).
fn fit_to_capacity(text: &str, capacity: usize) -> String {
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

/// A description broken at spaces into lines no wider than the card holds; a word longer than a line is cut
/// (`descriptionLines`).
pub fn description_lines(description: &str, width: f64) -> Vec<String> {
    let per_line = characters_across(width);
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in description.split_whitespace() {
        let mut word: Vec<char> = word.chars().collect();
        let length = current.chars().count();
        if length > 0 && length + 1 + word.len() <= per_line {
            current.push(' ');
            current.extend(word.iter());
            continue;
        }
        if length > 0 {
            lines.push(std::mem::take(&mut current));
        }
        while word.len() > per_line {
            lines.push(word[..per_line].iter().collect());
            word = word[per_line..].to_vec();
        }
        current = word.into_iter().collect();
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

pub(super) fn font(size: f64, weight: u16, colour: Option<&str>) -> Font {
    Font {
        family: None,
        size,
        weight,
        italic: false,
        colour: colour.map(Colour::value),
        underline: false,
    }
}

pub(super) fn text(
    owner: &Owner,
    role: Role,
    text: String,
    position: Point,
    anchor: TextAnchor,
    font: Font,
) -> Primitive {
    Primitive::Text(TextRun {
        owner: owner.clone(),
        role,
        text,
        position,
        anchor,
        font,
        wrap: None,
        tooltip: None,
        editable: false,
    })
}

pub(super) fn stroke(colour: Option<&str>, width: f64, dash: &[f64]) -> Stroke {
    Stroke {
        colour: colour.map(Colour::value),
        width,
        dash: dash.to_vec(),
        opacity: 1.0,
    }
}

/// The outline of a card of `shape`: a person's corners rounder than a box's, a cylinder with its top cap.
fn silhouette(shape: &str, bounds: Rect) -> (Path, Option<Path>) {
    match shape {
        "Box" => (Path::rounded_rect(bounds, 0.0), None),
        "Person" => (Path::rounded_rect(bounds, 18.0), None),
        "Cylinder" => {
            let ry = (bounds.height / 8.0).min(10.0);
            let (left, right, top, bottom) = (bounds.x, bounds.right(), bounds.y + ry, bounds.bottom() - ry);
            let rx = bounds.width / 2.0;
            let body = Path::new()
                .move_to(Point::new(left, top))
                .arc_to(rx, ry, 0.0, false, true, Point::new(right, top))
                .line_to(Point::new(right, bottom))
                .arc_to(rx, ry, 0.0, false, true, Point::new(left, bottom))
                .close();
            let cap =
                Path::new()
                    .move_to(Point::new(left, top))
                    .arc_to(rx, ry, 0.0, false, false, Point::new(right, top));
            (body, Some(cap))
        }
        _ => (Path::rounded_rect(bounds, 6.0), None),
    }
}

/// Where the line from the centre of `rect` towards `towards` leaves it.
pub(super) fn edge_point(rect: Rect, towards: Point) -> Point {
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

/// The scene of `elements`, a module view's rendering, with `selection` outlined and the elements `moving` carried
/// by the drag in flight, in the colours of the active theme, `theme`; a C4 element keeps the colours its model's
/// styles give it in either theme, as Structurizr draws them.
pub fn build(
    elements: &[DiagramElement],
    selection: &[String],
    moving: Option<(&[String], (f64, f64))>,
    theme: Kind,
) -> ModuleScene {
    let highlight = selection_colour(theme);
    let selected: HashSet<&str> = selection.iter().map(String::as_str).collect();
    let mut rects: HashMap<String, Rect> = HashMap::new();
    for element in elements.iter().filter(|e| e.element_type == NODE_TYPE) {
        let (width, height) = (number(element, "width"), number(element, "height"));
        rects.insert(
            element.id.clone(),
            Rect::centred(Point::new(element.x, element.y), width, height),
        );
    }
    let fdg_nodes: Vec<&DiagramElement> = elements
        .iter()
        .filter(|e| fdg_scene::is_fdg(e) && e.kind == ElementKind::Node)
        .collect();
    for element in &fdg_nodes {
        rects.insert(element.id.clone(), fdg_scene::rect_of(element));
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
    let mut anchors = Vec::new();

    // Boundaries first, so everything they enclose draws on top of them; they are read, never picked.
    for boundary in elements.iter().filter(|e| e.element_type == BOUNDARY_TYPE) {
        let rect = Rect::centred(
            Point::new(boundary.x, boundary.y),
            number(boundary, "width"),
            number(boundary, "height"),
        );
        primitives.push(Primitive::Shape(ShapePrimitive {
            owner: Owner::Canvas,
            role: Role::Body,
            path: Path::rounded_rect(rect, 0.0),
            fill: None,
            stroke: Some(stroke(None, 1.5, &[8.0, 6.0])),
            opacity: 0.8,
            tooltip: None,
        }));
        primitives.push(text(
            &Owner::Canvas,
            Role::Label {
                id: "boundary".to_owned(),
                line: 0,
            },
            format!("{} [{}]", boundary.attribute("name"), boundary.attribute("kind")),
            Point::new(rect.x, rect.y - 6.0),
            TextAnchor::Start,
            font(11.0, 400, None),
        ));
    }

    // The lines, under the cards they join, from edge to edge.
    for relationship in elements.iter().filter(|e| e.element_type == RELATIONSHIP_TYPE) {
        let (Some(source), Some(target)) = (
            relationship.source.as_deref().and_then(|s| drawn.get(s)),
            relationship.target.as_deref().and_then(|t| drawn.get(t)),
        ) else {
            continue;
        };
        let from = edge_point(*source, target.centre());
        let to = edge_point(*target, source.centre());
        let owner = Owner::Relation(relationship.id.clone());
        let is_selected = selected.contains(relationship.id.as_str());
        let line = if is_selected {
            stroke(Some(highlight), SELECTED_WIDTH, &[6.0, 4.0])
        } else {
            stroke(None, 1.5, &[6.0, 4.0])
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
        lines.push((relationship.id.clone(), vec![from, to], 5.0));
        let description = relationship.attribute("description");
        let technology = relationship.attribute("technology");
        let order = relationship.attribute("interactionOrder");
        let label = if technology.is_empty() {
            description.to_owned()
        } else {
            format!("{description} [{technology}]")
        };
        if !label.is_empty() {
            let label = if order.is_empty() {
                label
            } else {
                format!("{order}. {label}")
            };
            primitives.push(text(
                &owner,
                Role::LineLabel,
                label,
                Point::new((from.x + to.x) / 2.0, (from.y + to.y) / 2.0 - 6.0),
                TextAnchor::Middle,
                font(
                    10.0,
                    if is_selected { 600 } else { 400 },
                    is_selected.then_some(highlight),
                ),
            ));
        }
    }

    // The cards, at the sizes and in the palette the module resolved.
    for element in elements.iter().filter(|e| e.element_type == NODE_TYPE) {
        let Some(rect) = drawn.get(&element.id).copied() else {
            continue;
        };
        let owner = Owner::Element(element.id.clone());
        let background = Some(element.attribute("background")).filter(|b| !b.is_empty());
        let colour = Some(element.attribute("color"))
            .filter(|c| !c.is_empty())
            .unwrap_or(c4_theme::LIGHT_TEXT);
        let is_selected = selected.contains(element.id.as_str());
        let outline = if is_selected {
            stroke(Some(highlight), SELECTED_WIDTH, &[])
        } else {
            stroke(background.or(Some(c4_theme::SOFTWARE_SYSTEM_BACKGROUND)), 1.0, &[])
        };
        let (body, cap) = silhouette(element.attribute("shape"), rect);
        primitives.push(Primitive::Shape(ShapePrimitive {
            owner: owner.clone(),
            role: Role::Body,
            path: body,
            fill: Some(Colour::value(
                background.unwrap_or(c4_theme::SOFTWARE_SYSTEM_BACKGROUND),
            )),
            stroke: Some(outline),
            opacity: if moving.is_some_and(|(ids, _)| ids.contains(&element.id)) {
                0.75
            } else {
                1.0
            },
            tooltip: None,
        }));
        if let Some(cap) = cap {
            primitives.push(Primitive::Shape(ShapePrimitive {
                owner: owner.clone(),
                role: Role::BodyPart,
                path: cap,
                fill: None,
                stroke: Some(stroke(Some(colour), 1.0, &[])),
                opacity: 0.6,
                tooltip: None,
            }));
        }
        let centre = rect.centre().x;
        let name_at = Point::new(centre, rect.y + 22.0);
        primitives.push(text(
            &owner,
            Role::Label {
                id: "name".to_owned(),
                line: 0,
            },
            element.attribute("name").to_owned(),
            name_at,
            TextAnchor::Middle,
            font(13.0, 600, Some(colour)),
        ));
        labels.push((
            element.id.clone(),
            "name".to_owned(),
            Rect::new(rect.x, rect.y + 6.0, rect.width, 20.0),
        ));
        let type_line = element.attribute("typeLine");
        if !type_line.is_empty() {
            primitives.push(text(
                &owner,
                Role::Label {
                    id: "type".to_owned(),
                    line: 0,
                },
                fit_to_capacity(type_line, characters_across(rect.width)),
                Point::new(centre, rect.y + 38.0),
                TextAnchor::Middle,
                font(10.0, 400, Some(colour)),
            ));
        }
        for (index, line) in description_lines(element.attribute("description"), rect.width)
            .into_iter()
            .enumerate()
        {
            primitives.push(text(
                &owner,
                Role::Label {
                    id: "description".to_owned(),
                    line: index,
                },
                line,
                Point::new(centre, rect.y + 58.0 + 14.0 * index as f64),
                TextAnchor::Middle,
                font(10.0, 400, Some(colour)),
            ));
        }
        if is_selected {
            primitives.push(Primitive::Hook(Hook {
                owner: owner.clone(),
                kind: HookKind::Selection,
                area: rect,
            }));
        }
        // An anchor on each edge, drawn while the card is selected or under the pointer.
        let centre_y = rect.centre().y;
        for (name, at) in [
            ("top", Point::new(centre, rect.y)),
            ("right", Point::new(rect.right(), centre_y)),
            ("bottom", Point::new(centre, rect.bottom())),
            ("left", Point::new(rect.x, centre_y)),
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
            anchors.push((element.id.clone(), name.to_owned(), area));
        }
        boxes.push((element.id.clone(), rect));
    }

    // The title C4 requires above the diagram, and the key it requires below it.
    let extent = boxes
        .iter()
        .map(|(_, r)| *r)
        .chain(
            elements
                .iter()
                .filter(|e| e.element_type == BOUNDARY_TYPE)
                .map(|b| Rect::centred(Point::new(b.x, b.y), number(b, "width"), number(b, "height"))),
        )
        .reduce(|a, b| a.union(&b));
    if let (Some(view), Some(extent)) = (elements.iter().find(|e| e.element_type == VIEW_TYPE), extent) {
        primitives.push(text(
            &Owner::Canvas,
            Role::Label {
                id: "title".to_owned(),
                line: 0,
            },
            view.attribute("title").to_owned(),
            Point::new(extent.x, extent.y - 30.0),
            TextAnchor::Start,
            font(15.0, 600, None),
        ));
        let mut x = extent.x;
        let y = extent.bottom() + 30.0;
        let entries: Vec<(String, String)> = (0..)
            .map_while(|index| {
                let label = view.attribute(&format!("legend.{index}.label"));
                (!label.is_empty()).then(|| {
                    (
                        label.to_owned(),
                        view.attribute(&format!("legend.{index}.background")).to_owned(),
                    )
                })
            })
            .collect();
        if !entries.is_empty() {
            primitives.push(text(
                &Owner::Canvas,
                Role::Label {
                    id: "key".to_owned(),
                    line: 0,
                },
                "Key".to_owned(),
                Point::new(x, y),
                TextAnchor::Start,
                font(11.0, 600, None),
            ));
            x += 36.0;
            for (index, (label, background)) in entries.into_iter().enumerate() {
                primitives.push(Primitive::Shape(ShapePrimitive {
                    owner: Owner::Canvas,
                    role: Role::Decoration(format!("swatch.{index}")),
                    path: Path::rounded_rect(Rect::new(x, y - 10.0, 12.0, 12.0), 3.0),
                    fill: Some(Colour::value(&background)),
                    stroke: Some(stroke(None, 1.0, &[])),
                    opacity: 1.0,
                    tooltip: None,
                }));
                x += 17.0;
                let width = label.chars().count() as f64 * 11.0 * AVERAGE_ADVANCE;
                primitives.push(text(
                    &Owner::Canvas,
                    Role::Label {
                        id: "key".to_owned(),
                        line: index + 1,
                    },
                    label,
                    Point::new(x, y),
                    TextAnchor::Start,
                    font(11.0, 400, None),
                ));
                x += width + 12.0;
            }
        }
    }

    // The functional decomposition graph's elements and connections, over everything else.
    let moving_ids: HashSet<&str> = moving
        .map(|(ids, _)| ids.iter().map(String::as_str).collect())
        .unwrap_or_default();
    let graph = fdg_scene::draw(elements, &drawn, &selected, &moving_ids, theme);
    primitives.extend(graph.primitives);
    boxes.extend(graph.boxes);
    lines.extend(graph.lines);
    labels.extend(graph.labels);
    anchors.extend(graph.anchors);

    let empty_text = (!elements
        .iter()
        .any(|e| e.kind == ElementKind::Node && (e.element_type == NODE_TYPE || fdg_scene::is_fdg(e))))
    .then(|| "This view shows nothing yet.".to_owned());
    ModuleScene {
        scene: Scene { primitives, empty_text },
        rects,
        boxes,
        lines,
        labels,
        anchors,
        top_left: fdg_nodes.iter().map(|e| e.id.clone()).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_description_breaks_at_spaces_within_the_card_and_a_long_word_is_cut() {
        // 240 wide: (240 - 24) / 5.5 = 39 characters a line.
        let lines = description_lines("Serves the single-page application and its static assets.", 240.0);
        assert_eq!(
            lines,
            vec!["Serves the single-page application and", "its static assets."]
        );
        assert_eq!(description_lines(&"x".repeat(50), 240.0).len(), 2);
        assert_eq!(
            fit_to_capacity("[Container: Java 21 and Spring Boot 3]", 10),
            "[Containe…"
        );
    }

    #[test]
    fn a_line_leaves_a_box_at_its_edge() {
        let rect = Rect::new(0.0, 0.0, 100.0, 50.0);
        assert_eq!(edge_point(rect, Point::new(200.0, 25.0)), Point::new(100.0, 25.0));
        assert_eq!(edge_point(rect, Point::new(50.0, -100.0)), Point::new(50.0, 0.0));
    }
}
