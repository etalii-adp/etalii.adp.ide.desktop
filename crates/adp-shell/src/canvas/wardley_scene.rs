//! The scene of a Wardley map its module draws itself (etalii.adp spec 030, task T070): what
//! `adp_diagram_wardley_map::mapper` renders in the map's 0..1 space, drawn on a square of 1000 canvas units as
//! `adp-scene` primitives, so the one painter, hit test and gesture arbiter serve it as they serve every other canvas.
//!
//! Ported from what the standalone host's `src/diagrams/wardley-map/client/WardleyCanvas.tsx` and `wardley.css` at
//! `8b6ccfc` draw:
//! - the evolution axis divided into its four stages, each a band of increasing opacity behind a dashed (6 6)
//!   separator, labelled at font size 20, with the value chain's axis labelled "Value chain", "Visible" at its top
//!   and "Invisible" at its bottom, and the horizontal one "Evolution", both axis lines of width 2;
//! - an attitude as a dashed (4 4) rectangle at fill opacity 0.05, labelled with its kind, its corners in any order;
//! - a dependency as a muted line of width 1.5, a flow of width 3, neither with a marker nor selectable;
//! - an evolution as a dashed (8 6) line of width 2 to a dashed (4 3) ring at the target maturity;
//! - a component as a circle of radius 9, an anchor as a filled square, a submap as a ring inside a fainter ring, the
//!   name beside the mark at (9 + 6, 4) or at the document's own label offset, the decorators and "inertia" as one
//!   muted line 14 under it at 12, and inertia as a bar of width 4 at the mark's right + 4;
//! - an accelerator as a rule 16 wide with its name at (22, 4), a deaccelerator's dashed; a note as text at 14; an
//!   annotation as a circle of radius 11 carrying its number at every place it is pinned.
//!
//! Only components, anchors, submaps and pipeline children are dragged; a drag is clamped to the map by the module's
//! command. Every element but a link is selected by a click.

use std::collections::{HashMap, HashSet};

use adp_diagram::DiagramElement;
use adp_diagram_wardley_map::mapper;
use adp_scene::definition::labels::TextAnchor;
use adp_scene::scene::{EdgePrimitive, Hook, HookKind, Owner, Primitive, Role, ShapePrimitive, TextRun};
use adp_scene::style::{Colour, Font, Stroke};
use adp_scene::{Path, Point, Rect, Scene};

use super::module_scene::ModuleScene;
use super::placement::Mapping;

/// The side of the square the map's 0..1 space is drawn on, in canvas units (`SPACE`).
pub const SPACE: f64 = 1000.0;
/// The radius of a mark (`DOT`).
const DOT: f64 = 9.0;
/// The selected outline, as the scene builder paints it.
const SELECTED: &str = "#7c3aed";
/// The muted colour of a dependency, a stage label and a decorator line.
const MUTED: &str = "#6b7280";
/// The band colour, drawn at each stage's opacity.
const BAND: &str = "#9ca3af";
/// The opacity of each stage's band, genesis first (`wardley.css`), over a faint tint so a label stays legible in
/// either theme.
const BAND_OPACITY: [f64; 4] = [0.35, 0.5, 0.65, 0.8];
/// The tint every band's opacity is taken of.
const BAND_TINT: f64 = 0.15;

fn number(element: &DiagramElement, name: &str) -> f64 {
    element.attribute(name).parse().unwrap_or_default()
}

fn at(x: f64, y: f64) -> Point {
    Point::new(x * SPACE, y * SPACE)
}

fn font(size: f64, weight: u16, colour: Option<&str>) -> Font {
    Font {
        family: None,
        size,
        weight,
        italic: false,
        colour: colour.map(Colour::value),
        underline: false,
    }
}

fn stroke(colour: Option<&str>, width: f64, dash: &[f64]) -> Stroke {
    Stroke {
        colour: colour.map(Colour::value),
        width,
        dash: dash.to_vec(),
        opacity: 1.0,
    }
}

fn text(owner: &Owner, role: Role, value: String, position: Point, anchor: TextAnchor, font: Font) -> Primitive {
    Primitive::Text(TextRun {
        owner: owner.clone(),
        role,
        text: value,
        position,
        anchor,
        font,
        wrap: None,
        tooltip: None,
        editable: false,
    })
}

fn label(id: &str) -> Role {
    Role::Label {
        id: id.to_owned(),
        line: 0,
    }
}

fn shape(owner: &Owner, role: Role, path: Path, fill: Option<&str>, line: Option<Stroke>, opacity: f64) -> Primitive {
    Primitive::Shape(ShapePrimitive {
        owner: owner.clone(),
        role,
        path,
        fill: fill.map(Colour::value),
        stroke: line,
        opacity,
        tooltip: None,
    })
}

fn line(owner: &Owner, from: Point, to: Point, line: Stroke) -> Primitive {
    Primitive::Edge(EdgePrimitive {
        owner: owner.clone(),
        path: Path::polyline(&[from, to]),
        stroke: line,
        start: None,
        end: None,
        hit_width: 0.0,
        selectable: false,
        tooltip: None,
    })
}

/// The scene of `elements`, a Wardley view's rendering, with `selection` outlined and the elements `moving` carried
/// by the drag in flight; with it, how each dragged mark's centre maps back onto the map's 0..1 space.
pub fn build(
    elements: &[DiagramElement],
    selection: &[String],
    moving: Option<(&[String], (f64, f64))>,
) -> (ModuleScene, HashMap<String, Mapping>) {
    let selected: HashSet<&str> = selection.iter().map(String::as_str).collect();
    let offset = |id: &str| -> (f64, f64) {
        match moving {
            Some((ids, delta)) if ids.iter().any(|m| m == id) => delta,
            _ => (0.0, 0.0),
        }
    };
    let mut primitives = Vec::new();
    let mut rects = HashMap::new();
    let mut mappings = HashMap::new();
    let mut boxes = Vec::new();
    let mut labels = Vec::new();
    let canvas = Owner::Canvas;

    // The evolution axis: the stages' bands and separators, then the two axes and their labels.
    if let Some(axis) = elements.iter().find(|e| e.element_type == mapper::EVOLUTION_AXIS) {
        for (index, opacity) in BAND_OPACITY.into_iter().enumerate() {
            let stage = axis.attribute(&format!("stage.{index}"));
            let mut parts = stage.split('|');
            let (Some(name), Some(start), Some(end)) = (parts.next(), parts.next(), parts.next()) else {
                continue;
            };
            let (start, end) = (start.parse().unwrap_or(0.0), end.parse().unwrap_or(1.0));
            primitives.push(shape(
                &canvas,
                Role::Background,
                Path::rounded_rect(Rect::new(start * SPACE, 0.0, (end - start) * SPACE, SPACE), 0.0),
                Some(BAND),
                None,
                opacity * BAND_TINT,
            ));
            if index > 0 {
                primitives.push(line(
                    &canvas,
                    at(start, 0.0),
                    at(start, 1.0),
                    stroke(Some(MUTED), 1.0, &[6.0, 6.0]),
                ));
            }
            primitives.push(text(
                &canvas,
                label("stage"),
                name.to_owned(),
                Point::new((start + end) / 2.0 * SPACE, SPACE + 30.0),
                TextAnchor::Middle,
                font(20.0, 400, Some(MUTED)),
            ));
        }
        primitives.push(line(&canvas, at(0.0, 0.0), at(0.0, 1.0), stroke(None, 2.0, &[])));
        primitives.push(line(&canvas, at(0.0, 1.0), at(1.0, 1.0), stroke(None, 2.0, &[])));
        for (value, position, anchor) in [
            ("Visible", Point::new(-12.0, 14.0), TextAnchor::End),
            ("Invisible", Point::new(-12.0, SPACE), TextAnchor::End),
            ("Value chain", Point::new(-12.0, SPACE / 2.0), TextAnchor::End),
            ("Evolution", Point::new(SPACE, SPACE + 60.0), TextAnchor::End),
        ] {
            primitives.push(text(
                &canvas,
                label("axis"),
                value.to_owned(),
                position,
                anchor,
                font(14.0, 600, None),
            ));
        }
        let title = axis.attribute("title");
        if !title.is_empty() {
            primitives.push(text(
                &canvas,
                label("title"),
                title.to_owned(),
                Point::new(0.0, -30.0),
                TextAnchor::Start,
                font(18.0, 600, None),
            ));
        }
    }

    // Attitudes, behind everything they cover.
    for attitude in elements.iter().filter(|e| e.element_type == mapper::ATTITUDE) {
        let owner = Owner::Element(attitude.id.clone());
        let (x1, y1) = (attitude.x, attitude.y);
        let (x2, y2) = (number(attitude, "oppositeX"), number(attitude, "oppositeY"));
        let area = Rect::new(
            x1.min(x2) * SPACE,
            y1.min(y2) * SPACE,
            (x1 - x2).abs() * SPACE,
            (y1 - y2).abs() * SPACE,
        );
        let is_selected = selected.contains(attitude.id.as_str());
        primitives.push(shape(
            &owner,
            Role::Body,
            Path::rounded_rect(area, 0.0),
            Some(MUTED),
            Some(stroke(
                is_selected.then_some(SELECTED),
                if is_selected { 3.0 } else { 1.0 },
                &[4.0, 4.0],
            )),
            0.05,
        ));
        primitives.push(shape(
            &owner,
            Role::BodyPart,
            Path::rounded_rect(area, 0.0),
            None,
            Some(stroke(
                is_selected.then_some(SELECTED),
                if is_selected { 3.0 } else { 1.0 },
                &[4.0, 4.0],
            )),
            1.0,
        ));
        primitives.push(text(
            &owner,
            label("kind"),
            attitude.attribute("kind").to_owned(),
            Point::new(area.x + 6.0, area.y + 16.0),
            TextAnchor::Start,
            font(12.0, 600, Some(MUTED)),
        ));
        boxes.push((attitude.id.clone(), area));
    }

    // Where each mark is drawn, a drag in flight included.
    let centre_of = |element: &DiagramElement| {
        let (dx, dy) = offset(&element.id);
        Point::new(element.x * SPACE + dx, element.y * SPACE + dy)
    };
    let marks: HashMap<&str, Point> = elements
        .iter()
        .filter(|e| e.element_type == mapper::ELEMENT)
        .map(|e| (e.id.as_str(), centre_of(e)))
        .collect();

    // The links, under the marks they join, and never picked.
    for link in elements.iter().filter(|e| e.element_type == mapper::LINK) {
        let (Some(from), Some(to)) = (
            link.source.as_deref().and_then(|s| marks.get(s)),
            link.target.as_deref().and_then(|t| marks.get(t)),
        ) else {
            continue;
        };
        let flow = link.attribute("isFlow") == "true";
        let owner = Owner::Relation(link.id.clone());
        primitives.push(line(
            &owner,
            *from,
            *to,
            if flow {
                stroke(None, 3.0, &[])
            } else {
                stroke(Some(MUTED), 1.5, &[])
            },
        ));
        let context = link.attribute("context");
        if !context.is_empty() {
            primitives.push(text(
                &owner,
                Role::LineLabel,
                context.to_owned(),
                Point::new((from.x + to.x) / 2.0, (from.y + to.y) / 2.0 - 6.0),
                TextAnchor::Middle,
                font(11.0, 400, Some(MUTED)),
            ));
        }
    }

    // The marks: components, anchors, submaps and pipeline children, each with its evolution, its name and its
    // strategy line.
    for element in elements.iter().filter(|e| e.element_type == mapper::ELEMENT) {
        let owner = Owner::Element(element.id.clone());
        let centre = marks[element.id.as_str()];
        let is_selected = selected.contains(element.id.as_str());
        let outline = if is_selected {
            stroke(Some(SELECTED), 3.0, &[])
        } else {
            stroke(None, 2.0, &[])
        };
        let mark = Rect::centred(centre, 2.0 * DOT, 2.0 * DOT);
        if !element.attribute("evolveMaturity").is_empty() {
            let target = Point::new(number(element, "evolveMaturity") * SPACE, centre.y);
            primitives.push(line(&owner, centre, target, stroke(Some(MUTED), 2.0, &[8.0, 6.0])));
            primitives.push(shape(
                &owner,
                Role::Decoration("evolve".to_owned()),
                Path::ellipse(Rect::centred(target, 2.0 * DOT, 2.0 * DOT)),
                None,
                Some(stroke(Some(MUTED), 2.0, &[4.0, 3.0])),
                1.0,
            ));
            let becomes = element.attribute("evolveName");
            if !becomes.is_empty() {
                primitives.push(text(
                    &owner,
                    label("evolve"),
                    becomes.to_owned(),
                    Point::new(target.x + DOT + 6.0, target.y + 4.0),
                    TextAnchor::Start,
                    font(13.0, 400, Some(MUTED)),
                ));
            }
        }
        match element.attribute("kind") {
            "anchor" => primitives.push(shape(
                &owner,
                Role::Body,
                Path::rounded_rect(mark, 0.0),
                Some("#111827"),
                Some(outline),
                1.0,
            )),
            "submap" => {
                primitives.push(shape(
                    &owner,
                    Role::BodyPart,
                    Path::ellipse(mark.inflate(5.0)),
                    None,
                    Some(stroke(None, 1.0, &[])),
                    0.6,
                ));
                primitives.push(shape(
                    &owner,
                    Role::Body,
                    Path::ellipse(mark),
                    Some("#ffffff"),
                    Some(outline),
                    1.0,
                ));
            }
            _ => primitives.push(shape(
                &owner,
                Role::Body,
                Path::ellipse(mark),
                Some("#ffffff"),
                Some(outline),
                1.0,
            )),
        }
        if element.attribute("inertia") == "true" {
            let x = mark.right() + 4.0;
            primitives.push(line(
                &owner,
                Point::new(x, mark.y - 6.0),
                Point::new(x, mark.bottom() + 6.0),
                stroke(None, 4.0, &[]),
            ));
        }
        let name_at = if element.attribute("labelOffsetX").is_empty() {
            Point::new(centre.x + DOT + 6.0, centre.y + 4.0)
        } else {
            Point::new(
                centre.x + number(element, "labelOffsetX"),
                centre.y + number(element, "labelOffsetY"),
            )
        };
        let name = element.attribute("name").to_owned();
        let name_width = name.chars().count() as f64 * 14.0 * 0.55;
        primitives.push(text(
            &owner,
            label("name"),
            name,
            name_at,
            TextAnchor::Start,
            font(
                14.0,
                if is_selected { 600 } else { 400 },
                is_selected.then_some(SELECTED),
            ),
        ));
        labels.push((
            element.id.clone(),
            "name".to_owned(),
            Rect::new(name_at.x, name_at.y - 14.0, name_width.max(20.0), 18.0),
        ));
        let mut strategy: Vec<String> = element
            .attribute("decorators")
            .split(", ")
            .filter(|d| !d.is_empty())
            .map(str::to_owned)
            .collect();
        if element.attribute("inertia") == "true" {
            strategy.push("inertia".to_owned());
        }
        if !strategy.is_empty() {
            primitives.push(text(
                &owner,
                label("strategy"),
                strategy.join(" · "),
                Point::new(name_at.x, name_at.y + 14.0),
                TextAnchor::Start,
                font(12.0, 400, Some(MUTED)),
            ));
        }
        if is_selected {
            primitives.push(Primitive::Hook(Hook {
                owner: owner.clone(),
                kind: HookKind::Selection,
                area: mark,
            }));
        }
        let home = Rect::centred(Point::new(element.x * SPACE, element.y * SPACE), 2.0 * DOT, 2.0 * DOT);
        rects.insert(element.id.clone(), home);
        mappings.insert(
            element.id.clone(),
            Mapping {
                scale: (SPACE, SPACE),
                ..Mapping::default()
            },
        );
        boxes.push((element.id.clone(), mark.inflate(3.0)));
    }

    // Accelerators, notes and annotations.
    for accelerator in elements.iter().filter(|e| e.element_type == mapper::ACCELERATOR) {
        let owner = Owner::Element(accelerator.id.clone());
        let centre = Point::new(accelerator.x * SPACE, accelerator.y * SPACE);
        let is_selected = selected.contains(accelerator.id.as_str());
        let dash: &[f64] = if accelerator.attribute("isDeaccelerator") == "true" {
            &[4.0, 3.0]
        } else {
            &[]
        };
        primitives.push(line(
            &owner,
            centre,
            Point::new(centre.x + 16.0, centre.y),
            stroke(is_selected.then_some(SELECTED), 3.0, dash),
        ));
        primitives.push(text(
            &owner,
            label("name"),
            accelerator.attribute("name").to_owned(),
            Point::new(centre.x + 22.0, centre.y + 4.0),
            TextAnchor::Start,
            font(13.0, 400, is_selected.then_some(SELECTED)),
        ));
        boxes.push((
            accelerator.id.clone(),
            Rect::new(centre.x - 2.0, centre.y - 8.0, 22.0, 16.0),
        ));
    }
    for note in elements.iter().filter(|e| e.element_type == mapper::NOTE) {
        let owner = Owner::Element(note.id.clone());
        let position = Point::new(note.x * SPACE, note.y * SPACE);
        let value = note.attribute("text").to_owned();
        let width = value.chars().count() as f64 * 14.0 * 0.55;
        let is_selected = selected.contains(note.id.as_str());
        primitives.push(text(
            &owner,
            label("text"),
            value,
            position,
            TextAnchor::Start,
            font(14.0, 400, is_selected.then_some(SELECTED)),
        ));
        boxes.push((
            note.id.clone(),
            Rect::new(position.x, position.y - 14.0, width.max(20.0), 18.0),
        ));
    }
    for annotation in elements.iter().filter(|e| e.element_type == mapper::ANNOTATION) {
        let owner = Owner::Element(annotation.id.clone());
        let is_selected = selected.contains(annotation.id.as_str());
        let points: Vec<Point> = annotation
            .attribute("occurrences")
            .split(';')
            .filter_map(|pair| {
                let (x, y) = pair.split_once(' ')?;
                Some(at(x.parse().ok()?, y.parse().ok()?))
            })
            .collect();
        for point in points {
            let ring = Rect::centred(point, 22.0, 22.0);
            primitives.push(Primitive::Shape(ShapePrimitive {
                owner: owner.clone(),
                role: Role::Body,
                path: Path::ellipse(ring),
                fill: Some(Colour::value("#ffffff")),
                stroke: Some(stroke(is_selected.then_some(SELECTED), 1.5, &[])),
                opacity: 1.0,
                tooltip: Some(annotation.attribute("text").to_owned()),
            }));
            primitives.push(text(
                &owner,
                label("number"),
                annotation.attribute("number").to_owned(),
                Point::new(point.x, point.y + 4.0),
                TextAnchor::Middle,
                font(12.0, 600, None),
            ));
            boxes.push((annotation.id.clone(), ring));
        }
    }

    let empty_text = None;
    (
        ModuleScene {
            scene: Scene { primitives, empty_text },
            rects,
            boxes,
            lines: Vec::new(),
            labels,
            anchors: Vec::new(),
            top_left: Vec::new(),
        },
        mappings,
    )
}
