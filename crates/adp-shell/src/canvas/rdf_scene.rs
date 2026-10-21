//! The scenes of the RDF family's four readings, whose module lays them out and hands the session cards, shapes and
//! statements (etalii.adp spec 030, task T083), drawn as `adp-scene` primitives so the one painter, hit test and
//! gesture arbiter serve them as they serve every other canvas.
//!
//! Ported from what the standalone host's `src/diagrams/rdf/client/` at `8b6ccfc` drew through its diagram library,
//! in the family notation's theme tokens (`w3c-rdf.dis`, `notation.theme`), light or dark:
//! - the data graph (`RdfCanvas.tsx`, `rdf.css`): a resource is a card 220 wide, its height following its content
//!   (a 30 header, a 16 badge line when it wears types, 16 per literal row, 10 below), its title centred 20 below its
//!   top, its type badges joined by ` · ` 40 below its top, and one `{predicate}: {value} {annotation}` line per row;
//!   a blank node's card is dashed and faded; a statement is a straight arrowed line labelled with its predicate 6
//!   above its middle; a resource card has an anchor in the middle of each side a statement is drawn from, a blank
//!   card none (the identity boundary starts at the gesture);
//! - the ontology (`OwlCanvas.tsx`, `owl.css`): classes, `owl:Thing` (at 0.55 of the size, dashed) and expressions
//!   are ellipses 190 by 70, doubled where an equivalence touches them, an expression's ring dashed and its label at
//!   11 with ` …` where it was elided, a malformed one outlined in the warning colour; a datatype is a box; an
//!   individual and the ontology header are cards as the data graph's; deprecated and external terms are faded; the
//!   seven axiom edges each have their own line (subclass dashed and arrowed, equivalence heavier, disjointness
//!   dotted, properties and assertions arrowed and labelled, an expression's link faint and dotted); every named node
//!   has an anchor in the middle of its left and right sides;
//! - the scheme (`SkosCanvas.tsx`, `skos.css`): a concept is a box 200 by 44, its label centred 30 below its top
//!   (italic for an alternate label standing in, monospaced and faded for the IRI's own form), its notation at its top
//!   left and its language chip at its top right; a scheme or collection is an unfilled region 260 by 40 with its
//!   label (a scheme's with its member count) and `ordered` for an ordered collection; hierarchy is a plain line,
//!   related dashed, a mapping dotted, arrowed and labelled; a concept's top anchor files it under another and its
//!   right anchor relates it;
//! - the shapes (`ShaclCanvas.tsx`, `shacl.css`): a shape is a rounded card 260 wide, 44 plus 22 per target and per
//!   row plus 12 high, its title (`{display} — {name}`) bold, its badges (deactivated, closed, its severity) at its top
//!   right, a sentence per target in italics, and a row per property constraint with its path, its summary from 110
//!   in and its cardinality at the right; a blank shape is dashed, a deactivated one faded; a reference between shapes
//!   is an arrowed, labelled line; nothing is connected from the canvas.
//!
//! Every reading's showing-N-of-M banner, an overlay at the top of the standalone host's canvas, is a line of text in
//! the warning colour above the drawing here. Every card is moved by its top-left, which is what the registration
//! stores (`onElementMoved`).

use std::collections::{HashMap, HashSet};

use adp_diagram::DiagramElement;
use adp_diagram_rdf::payload::{field, flag, list, number, records, text as payload_text};
use adp_scene::definition::labels::TextAnchor;
use adp_scene::notation::catalog::MarkerKind;
use adp_scene::scene::{EdgePrimitive, Hook, HookKind, Owner, Primitive, Role, ShapePrimitive, marker_shape};
use adp_scene::style::{Colour, Font, Stroke};
use adp_scene::{Path, Point, Rect, Scene};

use super::module_scene::{ANCHOR_RADIUS, AVERAGE_ADVANCE, ModuleScene, SELECTED, SELECTED_WIDTH, font, text};

/// The data graph's element types (`RdfElementMapper`).
pub const RDF_RESOURCE: &str = "w3c/rdf+resource";
pub const RDF_EDGE: &str = "w3c/rdf+edge";
pub const RDF_TRUNCATION: &str = "w3c/rdf+truncation";
/// The ontology's element types (`OwlElementMapper`).
pub const OWL_NODE: &str = "w3c/owl+node";
pub const OWL_EDGE: &str = "w3c/owl+edge";
pub const OWL_EXPRESSION: &str = "w3c/owl+expression";
pub const OWL_TRUNCATION: &str = "w3c/owl+truncation";
/// The scheme's element types (`SkosElementMapper`).
pub const SKOS_CONCEPT: &str = "w3c/skos+concept";
pub const SKOS_SCHEME: &str = "w3c/skos+scheme";
pub const SKOS_COLLECTION: &str = "w3c/skos+collection";
pub const SKOS_EDGE: &str = "w3c/skos+edge";
pub const SKOS_TRUNCATION: &str = "w3c/skos+truncation";
/// The shapes' element types (`ShaclElementMapper`).
pub const SHACL_SHAPE: &str = "w3c/shacl+shape";
pub const SHACL_EDGE: &str = "w3c/shacl+edge";
pub const SHACL_TRUNCATION: &str = "w3c/shacl+truncation";

/// The family's origins, as the shell routes a diagram to its scene.
pub const ORIGINS: [&str; 4] = ["w3c/rdf", "w3c/owl", "w3c/skos", "w3c/shacl"];

/// A data-graph card's width, and its parts stacked (`RdfCanvas.tsx`, `OwlCanvas.tsx`).
pub const CARD_WIDTH: f64 = 220.0;
const HEADER_HEIGHT: f64 = 30.0;
const BADGES_HEIGHT: f64 = 16.0;
const ROW_HEIGHT: f64 = 16.0;
const FOOTER_PADDING: f64 = 10.0;

/// An ontology shape's size, and `owl:Thing`'s scale of it (`OwlCanvas.tsx`).
pub const SHAPE_WIDTH: f64 = 190.0;
pub const SHAPE_HEIGHT: f64 = 70.0;
const THING_SCALE: f64 = 0.55;

/// A concept's and a region's size (`SkosCanvas.tsx`).
pub const CONCEPT_WIDTH: f64 = 200.0;
pub const CONCEPT_HEIGHT: f64 = 44.0;
pub const REGION_WIDTH: f64 = 260.0;
pub const REGION_HEIGHT: f64 = 40.0;

/// A shape card's width and its line pitch (`ShaclCanvas.tsx`).
pub const SHAPE_CARD_WIDTH: f64 = 260.0;
const SHACL_HEADER_HEIGHT: f64 = 30.0;
const SHACL_LINE_HEIGHT: f64 = 22.0;

/// The size a node's label is set at (`LABEL_FONT_SIZE`).
const LABEL_FONT_SIZE: f64 = 13.0;

/// The theme's tokens (`notation.theme` of the family's specifications).
struct Palette {
    surface: &'static str,
    text: &'static str,
    muted: &'static str,
    border: &'static str,
    outline: &'static str,
    primary: &'static str,
    warning: &'static str,
}

const LIGHT: Palette = Palette {
    surface: "#ffffff",
    text: "#0f172a",
    muted: "#64748b",
    border: "#e2e8f0",
    outline: "#8892a6",
    primary: "#32cd32",
    warning: "#b45309",
};

const DARK: Palette = Palette {
    surface: "#1e293b",
    text: "#f1f5f9",
    muted: "#94a3b8",
    border: "#334155",
    outline: "#8892a6",
    primary: "#32cd32",
    warning: "#fbbf24",
};

fn stroke(colour: &str, width: f64, dash: &[f64]) -> Stroke {
    Stroke {
        colour: Some(Colour::value(colour)),
        width,
        dash: dash.to_vec(),
        opacity: 1.0,
    }
}

fn styled(size: f64, weight: u16, colour: &str, italic: bool, mono: bool) -> Font {
    Font {
        family: mono.then(|| "ui-monospace, monospace".to_owned()),
        italic,
        ..font(size, weight, Some(colour))
    }
}

/// `text` whole when it fits `width` at `size`, else cut with an ellipsis (`fitToCapacity(capacityOf(...))`).
pub fn fit(text: &str, width: f64, size: f64) -> String {
    let capacity = ((width / (size * AVERAGE_ADVANCE) + 1e-9).floor() as usize).max(1);
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

/// A literal row as its card lists it: `{predicate}: {value} {annotation}`, without a trailing space.
fn row_line(row: &[(String, String)]) -> String {
    format!(
        "{}: {} {}",
        field(row, "predicate"),
        field(row, "value"),
        field(row, "annotation")
    )
    .trim_end()
    .to_owned()
}

/// The height of a card with `badges` and `rows` (`nodeHeightOf`, `nodeSizeOf`).
fn card_height(badges: bool, rows: usize) -> f64 {
    HEADER_HEIGHT + if badges { BADGES_HEIGHT } else { 0.0 } + rows as f64 * ROW_HEIGHT + FOOTER_PADDING
}

/// Where a data-graph card is drawn: its top-left and the size its content gives it.
pub fn rdf_rect(element: &DiagramElement) -> Rect {
    let height = card_height(!list(element, "typeBadges").is_empty(), records(element, "rows").len());
    Rect::new(element.x, element.y, CARD_WIDTH, height)
}

/// An ontology node's kind (`kindOf`, and an expression's `operator` or `restriction`).
pub fn owl_kind(element: &DiagramElement) -> &'static str {
    let kind = payload_text(element, "kind");
    if element.element_type == OWL_EXPRESSION {
        return if kind == "operator" { "operator" } else { "restriction" };
    }
    match kind {
        "datatype" => "datatype",
        "individual" => "individual",
        "thing" => "thing",
        "ontology" => "ontology",
        _ => "class",
    }
}

fn is_owl_card(kind: &str) -> bool {
    matches!(kind, "individual" | "ontology")
}

/// Where an ontology node is drawn (`nodeSizeOf`).
pub fn owl_rect(element: &DiagramElement) -> Rect {
    let kind = owl_kind(element);
    let (width, height) = if is_owl_card(kind) {
        (
            CARD_WIDTH,
            card_height(!list(element, "badges").is_empty(), records(element, "rows").len()),
        )
    } else if kind == "thing" {
        (SHAPE_WIDTH * THING_SCALE, SHAPE_HEIGHT * THING_SCALE)
    } else {
        (SHAPE_WIDTH, SHAPE_HEIGHT)
    };
    Rect::new(element.x, element.y, width, height)
}

/// Where a concept or a region is drawn.
pub fn skos_rect(element: &DiagramElement) -> Rect {
    if element.element_type == SKOS_CONCEPT {
        Rect::new(element.x, element.y, CONCEPT_WIDTH, CONCEPT_HEIGHT)
    } else {
        Rect::new(element.x, element.y, REGION_WIDTH, REGION_HEIGHT)
    }
}

/// Where a shape card is drawn (`shapeHeight`).
pub fn shacl_rect(element: &DiagramElement) -> Rect {
    let lines = records(element, "targets").len() + records(element, "rows").len();
    Rect::new(
        element.x,
        element.y,
        SHAPE_CARD_WIDTH,
        44.0 + lines as f64 * SHACL_LINE_HEIGHT + 12.0,
    )
}

/// A target chip's kind: the proto enum's number, or its name as JSON writes it.
fn target_kind(chip: &[(String, String)]) -> u8 {
    match field(chip, "kind") {
        "1" | "SHACL_TARGET_KIND_PROTO_CLASS" => 1,
        "2" | "SHACL_TARGET_KIND_PROTO_NODE" => 2,
        "3" | "SHACL_TARGET_KIND_PROTO_SUBJECTS_OF" => 3,
        "4" | "SHACL_TARGET_KIND_PROTO_OBJECTS_OF" => 4,
        _ => 5,
    }
}

/// What a target chip says (`targetWords`).
pub fn target_words(chip: &[(String, String)]) -> String {
    let term = field(chip, "termDisplay");
    match target_kind(chip) {
        1 => format!("targets class {term}"),
        2 => format!("targets node {term}"),
        3 => format!("targets subjects of {term}"),
        4 => format!("targets objects of {term}"),
        _ => "targets its own instances".to_owned(),
    }
}

/// The ends a statement joins: its payload's, else the element's own.
fn ends(edge: &DiagramElement) -> (String, String) {
    let from = payload_text(edge, "fromElementId");
    let to = payload_text(edge, "toElementId");
    (
        if from.is_empty() {
            edge.source.clone().unwrap_or_default()
        } else {
            from.to_owned()
        },
        if to.is_empty() {
            edge.target.clone().unwrap_or_default()
        } else {
            to.to_owned()
        },
    )
}

/// Where the line from the centre of `rect` towards `towards` leaves it, round or square.
fn boundary_point(rect: Rect, round: bool, towards: Point) -> Point {
    let centre = rect.centre();
    let (dx, dy) = (towards.x - centre.x, towards.y - centre.y);
    if dx == 0.0 && dy == 0.0 {
        return centre;
    }
    let (a, b) = (rect.width / 2.0, rect.height / 2.0);
    let scale = if round {
        1.0 / ((dx / a).powi(2) + (dy / b).powi(2)).sqrt()
    } else {
        let sx = if dx == 0.0 { f64::INFINITY } else { a / dx.abs() };
        let sy = if dy == 0.0 { f64::INFINITY } else { b / dy.abs() };
        sx.min(sy)
    };
    Point::new(centre.x + dx * scale, centre.y + dy * scale)
}

/// How one statement is drawn.
struct LineStyle<'a> {
    colour: &'a str,
    width: f64,
    dash: &'a [f64],
    opacity: f64,
    arrow: bool,
    label: String,
}

/// What a reading's builder gathers.
struct Builder<'a> {
    palette: &'static Palette,
    selected: HashSet<&'a str>,
    moving: Option<(&'a [String], (f64, f64))>,
    /// Every node's box as placed, and as drawn with the drag in flight; whether it is round.
    rects: HashMap<String, Rect>,
    drawn: HashMap<String, (Rect, bool)>,
    primitives: Vec<Primitive>,
    boxes: Vec<(String, Rect)>,
    lines: Vec<(String, Vec<Point>, f64)>,
    anchors: Vec<(String, String, Rect)>,
}

impl<'a> Builder<'a> {
    fn new(selection: &'a [String], moving: Option<(&'a [String], (f64, f64))>, dark: bool) -> Self {
        Builder {
            palette: if dark { &DARK } else { &LIGHT },
            selected: selection.iter().map(String::as_str).collect(),
            moving,
            rects: HashMap::new(),
            drawn: HashMap::new(),
            primitives: Vec::new(),
            boxes: Vec::new(),
            lines: Vec::new(),
            anchors: Vec::new(),
        }
    }

    /// Places the node `id` at `rect`, carried by the drag in flight when it is moving.
    fn place(&mut self, id: &str, rect: Rect, round: bool) {
        self.rects.insert(id.to_owned(), rect);
        let mut drawn = rect;
        if let Some((ids, (dx, dy))) = self.moving
            && ids.iter().any(|m| m == id)
        {
            drawn = Rect::new(rect.x + dx, rect.y + dy, rect.width, rect.height);
        }
        self.drawn.insert(id.to_owned(), (drawn, round));
    }

    fn drawn(&self, id: &str) -> Option<Rect> {
        self.drawn.get(id).map(|(rect, _)| *rect)
    }

    fn is_selected(&self, id: &str) -> bool {
        self.selected.contains(id)
    }

    fn is_moving(&self, id: &str) -> bool {
        self.moving.is_some_and(|(ids, _)| ids.iter().any(|m| m == id))
    }

    /// One straight statement between two drawn nodes, from boundary to boundary.
    fn statement(&mut self, edge: &DiagramElement, style: LineStyle<'_>) {
        let (from_id, to_id) = ends(edge);
        let (Some(&(from, from_round)), Some(&(to, to_round))) = (self.drawn.get(&from_id), self.drawn.get(&to_id))
        else {
            return;
        };
        let start = boundary_point(from, from_round, to.centre());
        let end = boundary_point(to, to_round, from.centre());
        let owner = Owner::Relation(edge.id.clone());
        let is_selected = self.is_selected(&edge.id);
        let mut line = if is_selected {
            stroke(SELECTED, SELECTED_WIDTH, style.dash)
        } else {
            stroke(style.colour, style.width, style.dash)
        };
        if !is_selected {
            line.opacity = style.opacity;
        }
        let angle = (end.y - start.y).atan2(end.x - start.x);
        self.primitives.push(Primitive::Edge(EdgePrimitive {
            owner: owner.clone(),
            path: Path::polyline(&[start, end]),
            end: if style.arrow {
                marker_shape(MarkerKind::Arrow, end, angle, line.width)
            } else {
                None
            },
            stroke: line,
            start: None,
            hit_width: 10.0,
            selectable: true,
            tooltip: None,
        }));
        let middle = Point::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
        self.lines.push((edge.id.clone(), vec![start, middle, end], 5.0));
        if !style.label.is_empty() {
            self.primitives.push(text(
                &owner,
                Role::LineLabel,
                style.label,
                Point::new(middle.x, middle.y - 6.0),
                TextAnchor::Middle,
                font(11.0, if is_selected { 600 } else { 400 }, Some(self.palette.muted)),
            ));
        }
    }

    /// A node's body: its outline, or the selected one, its selection hook and its hit box.
    fn body(&mut self, id: &str, path: Path, fill: Option<String>, outline: Stroke, opacity: f64, tooltip: String) {
        let rect = self.drawn(id).unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));
        let owner = Owner::Element(id.to_owned());
        let outline = if self.is_selected(id) {
            Stroke {
                colour: Some(Colour::value(SELECTED)),
                width: SELECTED_WIDTH,
                ..outline
            }
        } else {
            outline
        };
        let moved = if self.is_moving(id) { 0.75 } else { 1.0 };
        self.primitives.push(Primitive::Shape(ShapePrimitive {
            owner: owner.clone(),
            role: Role::Body,
            path,
            fill: fill.map(|f| Colour::value(&f)),
            stroke: Some(outline),
            opacity: opacity * moved,
            tooltip: Some(tooltip),
        }));
        if self.is_selected(id) {
            self.primitives.push(Primitive::Hook(Hook {
                owner,
                kind: HookKind::Selection,
                area: rect,
            }));
        }
        self.boxes.push((id.to_owned(), rect));
    }

    /// A line of text owned by node `id`.
    #[allow(clippy::too_many_arguments)]
    fn label(
        &mut self,
        id: &str,
        label: &str,
        line: usize,
        content: String,
        at: Point,
        anchor: TextAnchor,
        font: Font,
    ) {
        if content.is_empty() {
            return;
        }
        self.primitives.push(text(
            &Owner::Element(id.to_owned()),
            Role::Label {
                id: label.to_owned(),
                line,
            },
            content,
            at,
            anchor,
            font,
        ));
    }

    /// The named anchors a connection is drawn from, each the middle of one side of node `id`.
    fn side_anchors(&mut self, id: &str, sides: &[(&str, &str)]) {
        let Some(rect) = self.drawn(id) else { return };
        for (name, side) in sides {
            let at = match *side {
                "left" => Point::new(rect.x, rect.centre().y),
                "right" => Point::new(rect.right(), rect.centre().y),
                "top" => Point::new(rect.centre().x, rect.y),
                _ => Point::new(rect.centre().x, rect.bottom()),
            };
            let area = Rect::centred(at, 2.0 * ANCHOR_RADIUS, 2.0 * ANCHOR_RADIUS);
            self.primitives.push(Primitive::Hook(Hook {
                owner: Owner::Element(id.to_owned()),
                kind: HookKind::Anchor {
                    name: (*name).to_owned(),
                    visible: false,
                },
                area,
            }));
            self.anchors.push((id.to_owned(), (*name).to_owned(), area));
        }
    }

    /// A card's header, badges and rows, stacked as the data graph's and the ontology's cards stack them.
    fn card_text(
        &mut self,
        id: &str,
        display: &str,
        badges: &[String],
        rows: &[Vec<(String, String)>],
        truncate: bool,
    ) {
        let Some(rect) = self.drawn(id) else { return };
        let palette = self.palette;
        let inner = rect.width - 16.0;
        self.label(
            id,
            "display",
            0,
            fit(display, inner, LABEL_FONT_SIZE),
            Point::new(rect.centre().x, rect.y + HEADER_HEIGHT / 2.0 + 5.0),
            TextAnchor::Middle,
            font(LABEL_FONT_SIZE, 400, Some(palette.text)),
        );
        if !badges.is_empty() {
            let joined = badges.join(" · ");
            self.label(
                id,
                "badges",
                0,
                if truncate { fit(&joined, inner, 11.0) } else { joined },
                Point::new(rect.x + 8.0, rect.y + HEADER_HEIGHT - 6.0 + BADGES_HEIGHT),
                TextAnchor::Start,
                font(11.0, 400, Some(palette.primary)),
            );
        }
        let start = HEADER_HEIGHT + if badges.is_empty() { 0.0 } else { BADGES_HEIGHT } + ROW_HEIGHT - 4.0;
        for (index, row) in rows.iter().enumerate() {
            let line = row_line(row);
            self.label(
                id,
                "rows",
                index,
                if truncate { fit(&line, inner, 11.0) } else { line },
                Point::new(rect.x + 8.0, rect.y + start + index as f64 * ROW_HEIGHT),
                TextAnchor::Start,
                font(11.0, 400, Some(palette.muted)),
            );
        }
    }

    /// The showing-N-of-M banner above the drawing, from the truncation element when the module sent one.
    fn banner(&mut self, elements: &[DiagramElement], truncation_type: &str, sentence: impl Fn(i64, i64) -> String) {
        let Some(truncation) = elements.iter().find(|e| e.element_type == truncation_type) else {
            return;
        };
        let (shown, total) = (number(truncation, "shown"), number(truncation, "total"));
        let (mut left, mut top, mut right) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY);
        for (rect, _) in self.drawn.values() {
            left = left.min(rect.x);
            top = top.min(rect.y);
            right = right.max(rect.right());
        }
        if !left.is_finite() {
            (left, top, right) = (0.0, 0.0, 0.0);
        }
        self.primitives.push(text(
            &Owner::Canvas,
            Role::Label {
                id: "truncation".to_owned(),
                line: 0,
            },
            sentence(shown, total),
            Point::new((left + right) / 2.0, top - 16.0),
            TextAnchor::Middle,
            font(12.0, 400, Some(self.palette.warning)),
        ));
    }

    fn finish(self, empty_text: Option<String>) -> ModuleScene {
        let top_left = self.rects.keys().cloned().collect();
        ModuleScene {
            scene: Scene {
                primitives: self.primitives,
                empty_text,
            },
            rects: self.rects,
            boxes: self.boxes,
            lines: self.lines,
            labels: Vec::new(),
            anchors: self.anchors,
            top_left,
        }
    }
}

/// Whether `origin` is one of the family's readings, which the shell draws with [`build`].
pub fn is_rdf(origin: &str) -> bool {
    ORIGINS.contains(&origin)
}

/// The scene of `elements`, the reading `origin`'s rendering, with `selection` outlined, the elements `moving`
/// carried by the drag in flight, in the dark palette when `dark`.
pub fn build(
    origin: &str,
    elements: &[DiagramElement],
    selection: &[String],
    moving: Option<(&[String], (f64, f64))>,
    dark: bool,
) -> ModuleScene {
    match origin {
        "w3c/owl" => build_owl(elements, selection, moving, dark),
        "w3c/skos" => build_skos(elements, selection, moving, dark),
        "w3c/shacl" => build_shacl(elements, selection, moving, dark),
        _ => build_rdf(elements, selection, moving, dark),
    }
}

/// The data graph (`RdfCanvas.tsx`).
pub fn build_rdf(
    elements: &[DiagramElement],
    selection: &[String],
    moving: Option<(&[String], (f64, f64))>,
    dark: bool,
) -> ModuleScene {
    let mut b = Builder::new(selection, moving, dark);
    let palette = b.palette;
    let nodes: Vec<&DiagramElement> = elements.iter().filter(|e| e.element_type == RDF_RESOURCE).collect();
    for node in &nodes {
        b.place(&node.id, rdf_rect(node), false);
    }
    for edge in elements.iter().filter(|e| e.element_type == RDF_EDGE) {
        b.statement(
            edge,
            LineStyle {
                colour: palette.muted,
                width: 1.5,
                dash: &[],
                opacity: 1.0,
                arrow: true,
                label: payload_text(edge, "predicate").to_owned(),
            },
        );
    }
    for node in &nodes {
        let rect = b.drawn(&node.id).expect("placed");
        let blank = flag(node, "blank");
        let display = payload_text(node, "display");
        b.body(
            &node.id,
            Path::rounded_rect(rect, 4.0),
            Some(palette.surface.to_owned()),
            stroke(palette.outline, 1.0, if blank { &[5.0, 3.0] } else { &[] }),
            if blank { 0.75 } else { 1.0 },
            display.to_owned(),
        );
        b.card_text(
            &node.id,
            display,
            &list(node, "typeBadges"),
            &records(node, "rows"),
            false,
        );
        if !blank {
            b.side_anchors(&node.id, &[("w", "left"), ("e", "right")]);
        }
    }
    b.banner(elements, RDF_TRUNCATION, |shown, total| {
        format!("Showing {shown} of {total} resources — edits are withheld on this truncated view")
    });
    b.finish(None)
}

/// The ontology (`OwlCanvas.tsx`).
pub fn build_owl(
    elements: &[DiagramElement],
    selection: &[String],
    moving: Option<(&[String], (f64, f64))>,
    dark: bool,
) -> ModuleScene {
    let mut b = Builder::new(selection, moving, dark);
    let palette = b.palette;
    let nodes: Vec<&DiagramElement> = elements
        .iter()
        .filter(|e| e.element_type == OWL_NODE || e.element_type == OWL_EXPRESSION)
        .collect();
    let edges: Vec<&DiagramElement> = elements.iter().filter(|e| e.element_type == OWL_EDGE).collect();
    for node in &nodes {
        let kind = owl_kind(node);
        b.place(
            &node.id,
            owl_rect(node),
            !matches!(kind, "datatype" | "individual" | "ontology"),
        );
    }
    // Equivalence doubles the ring of both its ends (`hasEquivalence`).
    let doubled: HashSet<String> = edges
        .iter()
        .filter(|e| payload_text(e, "kind") == "equivalent")
        .flat_map(|e| {
            let (from, to) = ends(e);
            [from, to]
        })
        .collect();
    for edge in &edges {
        let kind = payload_text(edge, "kind");
        let (width, dash, opacity, arrow, labelled): (f64, &[f64], f64, bool, bool) = match kind {
            "equivalent" => (2.0, &[], 1.0, false, false),
            "disjoint" => (1.5, &[1.0, 5.0], 1.0, false, false),
            "object-property" | "datatype-property" | "assertion" => (1.5, &[], 1.0, true, true),
            "expression" => (1.5, &[2.0, 3.0], 0.7, false, false),
            _ => (1.5, &[6.0, 4.0], 1.0, true, false),
        };
        b.statement(
            edge,
            LineStyle {
                colour: palette.muted,
                width,
                dash,
                opacity,
                arrow,
                label: if labelled {
                    payload_text(edge, "label").to_owned()
                } else {
                    String::new()
                },
            },
        );
    }
    for node in &nodes {
        let rect = b.drawn(&node.id).expect("placed");
        let kind = owl_kind(node);
        let expression = node.element_type == OWL_EXPRESSION;
        let display = if expression {
            payload_text(node, "label")
        } else {
            payload_text(node, "display")
        };
        let faded = flag(node, "deprecated") || flag(node, "external");
        let malformed = flag(node, "malformed");
        let outline_colour = if malformed { palette.warning } else { palette.border };
        let dash: &[f64] = match kind {
            "thing" => &[4.0, 3.0],
            "operator" | "restriction" => &[3.0, 2.0],
            _ => &[],
        };
        let opacity = if faded { 0.55 } else { 1.0 };
        let round = !matches!(kind, "datatype" | "individual" | "ontology");
        let path = if round {
            Path::ellipse(rect)
        } else {
            Path::rounded_rect(rect, if is_owl_card(kind) { 4.0 } else { 0.0 })
        };
        b.body(
            &node.id,
            path,
            Some(palette.surface.to_owned()),
            stroke(outline_colour, 1.0, dash),
            opacity,
            display.to_owned(),
        );
        if round && doubled.contains(&node.id) {
            // The notation's equivalence: a second ring just inside the first.
            let inset = Rect::new(rect.x + 4.0, rect.y + 4.0, rect.width - 8.0, rect.height - 8.0);
            b.primitives.push(Primitive::Shape(ShapePrimitive {
                owner: Owner::Element(node.id.clone()),
                role: Role::BodyPart,
                path: Path::ellipse(inset),
                fill: None,
                stroke: Some(stroke(outline_colour, 1.0, dash)),
                opacity,
                tooltip: None,
            }));
        }
        if is_owl_card(kind) {
            b.card_text(&node.id, display, &list(node, "badges"), &records(node, "rows"), true);
        } else {
            let round_label = if expression && flag(node, "elided") {
                format!("{display} …")
            } else {
                display.to_owned()
            };
            let size = if expression { 11.0 } else { LABEL_FONT_SIZE };
            b.label(
                &node.id,
                "label",
                0,
                fit(&round_label, rect.width - 16.0, LABEL_FONT_SIZE),
                Point::new(rect.centre().x, rect.centre().y + size / 3.0),
                TextAnchor::Middle,
                font(size, 400, Some(palette.text)),
            );
        }
        if !expression {
            b.side_anchors(&node.id, &[("axiom-left", "left"), ("axiom-right", "right")]);
        }
    }
    b.banner(elements, OWL_TRUNCATION, |shown, total| {
        format!("Showing {shown} of {total} elements — edits are withheld on this truncated view")
    });
    b.finish(None)
}

/// The scheme (`SkosCanvas.tsx`).
pub fn build_skos(
    elements: &[DiagramElement],
    selection: &[String],
    moving: Option<(&[String], (f64, f64))>,
    dark: bool,
) -> ModuleScene {
    let mut b = Builder::new(selection, moving, dark);
    let palette = b.palette;
    // Regions first, schemes before collections, so the concepts they hold draw over them.
    let regions: Vec<&DiagramElement> = elements
        .iter()
        .filter(|e| e.element_type == SKOS_SCHEME)
        .chain(elements.iter().filter(|e| e.element_type == SKOS_COLLECTION))
        .collect();
    let concepts: Vec<&DiagramElement> = elements.iter().filter(|e| e.element_type == SKOS_CONCEPT).collect();
    for node in regions.iter().chain(concepts.iter()) {
        b.place(&node.id, skos_rect(node), false);
    }
    for region in &regions {
        let rect = b.drawn(&region.id).expect("placed");
        let label = payload_text(region, "label");
        b.body(
            &region.id,
            Path::rounded_rect(rect, 0.0),
            None,
            stroke(palette.outline, 1.0, &[8.0, 6.0]),
            1.0,
            label.to_owned(),
        );
        let title = if region.element_type == SKOS_SCHEME {
            format!("{label} ({})", number(region, "memberCount"))
        } else {
            label.to_owned()
        };
        b.label(
            &region.id,
            "label",
            0,
            fit(&title, rect.width - 16.0, LABEL_FONT_SIZE),
            Point::new(rect.centre().x, rect.y + REGION_HEIGHT / 2.0 + 5.0),
            TextAnchor::Middle,
            font(LABEL_FONT_SIZE, 400, Some(palette.text)),
        );
        if flag(region, "ordered") {
            b.label(
                &region.id,
                "kind",
                0,
                "ordered".to_owned(),
                Point::new(rect.x + 8.0, rect.y + REGION_HEIGHT - 6.0),
                TextAnchor::Start,
                font(10.0, 400, Some(palette.muted)),
            );
        }
    }
    for edge in elements.iter().filter(|e| e.element_type == SKOS_EDGE) {
        let kind = number(edge, "kind");
        let (dash, arrow, label): (&[f64], bool, String) = match kind {
            0 => (&[], false, String::new()),
            2 => (&[2.0, 3.0], true, payload_text(edge, "predicate").to_owned()),
            _ => (&[6.0, 4.0], false, String::new()),
        };
        b.statement(
            edge,
            LineStyle {
                colour: palette.muted,
                width: 1.5,
                dash,
                opacity: 1.0,
                arrow,
                label,
            },
        );
    }
    for concept in &concepts {
        let rect = b.drawn(&concept.id).expect("placed");
        let blank = flag(concept, "blank");
        let label = payload_text(concept, "label");
        b.body(
            &concept.id,
            Path::rounded_rect(rect, 4.0),
            Some(palette.surface.to_owned()),
            stroke(palette.outline, 1.0, if blank { &[5.0, 3.0] } else { &[] }),
            if blank { 0.75 } else { 1.0 },
            label.to_owned(),
        );
        // A label standing in for a missing preferred one is italic; the IRI's own form monospaced and faded.
        let label_font = match number(concept, "labelKind") {
            1 => styled(LABEL_FONT_SIZE, 400, palette.text, true, false),
            2 => styled(11.0, 400, palette.muted, false, true),
            _ => font(LABEL_FONT_SIZE, 400, Some(palette.text)),
        };
        b.label(
            &concept.id,
            "label",
            0,
            fit(label, rect.width - 16.0, LABEL_FONT_SIZE),
            Point::new(rect.centre().x, rect.y + CONCEPT_HEIGHT / 2.0 + 8.0),
            TextAnchor::Middle,
            label_font,
        );
        b.label(
            &concept.id,
            "notation",
            0,
            payload_text(concept, "notation").to_owned(),
            Point::new(rect.x + 8.0, rect.y + 14.0),
            TextAnchor::Start,
            font(10.0, 400, Some(palette.primary)),
        );
        if flag(concept, "languageChip") {
            b.label(
                &concept.id,
                "language",
                0,
                payload_text(concept, "languageTag").to_owned(),
                Point::new(rect.right() - 8.0, rect.y + 14.0),
                TextAnchor::End,
                font(10.0, 400, Some(palette.muted)),
            );
        }
        if !blank {
            b.side_anchors(&concept.id, &[("file", "top"), ("relate", "right")]);
        }
    }
    b.banner(elements, SKOS_TRUNCATION, |shown, total| {
        format!("Showing {shown} of {total} terms — edits are withheld on this truncated view")
    });
    b.finish(None)
}

/// The shapes (`ShaclCanvas.tsx`).
pub fn build_shacl(
    elements: &[DiagramElement],
    selection: &[String],
    moving: Option<(&[String], (f64, f64))>,
    dark: bool,
) -> ModuleScene {
    let mut b = Builder::new(selection, moving, dark);
    let palette = b.palette;
    let shapes: Vec<&DiagramElement> = elements.iter().filter(|e| e.element_type == SHACL_SHAPE).collect();
    for shape in &shapes {
        b.place(&shape.id, shacl_rect(shape), false);
    }
    for edge in elements.iter().filter(|e| e.element_type == SHACL_EDGE) {
        b.statement(
            edge,
            LineStyle {
                colour: palette.muted,
                width: 1.5,
                dash: &[],
                opacity: 1.0,
                arrow: true,
                label: payload_text(edge, "label").to_owned(),
            },
        );
    }
    for shape in &shapes {
        let rect = b.drawn(&shape.id).expect("placed");
        let blank = flag(shape, "blank");
        let deactivated = flag(shape, "deactivated");
        let display = payload_text(shape, "display");
        let name = payload_text(shape, "name");
        b.body(
            &shape.id,
            Path::rounded_rect(rect, 6.0),
            Some(palette.surface.to_owned()),
            stroke(palette.outline, 1.0, if blank { &[4.0, 3.0] } else { &[] }),
            if deactivated { 0.55 } else { 1.0 },
            display.to_owned(),
        );
        let title = if name.is_empty() {
            display.to_owned()
        } else {
            format!("{display} — {name}")
        };
        b.label(
            &shape.id,
            "title",
            0,
            fit(&title, rect.width - 16.0, LABEL_FONT_SIZE),
            Point::new(rect.centre().x, rect.y + SHACL_HEADER_HEIGHT / 2.0 + 5.0),
            TextAnchor::Middle,
            font(LABEL_FONT_SIZE, 600, Some(palette.text)),
        );
        let badges: Vec<&str> = [
            if deactivated { "deactivated" } else { "" },
            if flag(shape, "closed") { "closed" } else { "" },
            payload_text(shape, "severity"),
        ]
        .into_iter()
        .filter(|badge| !badge.is_empty())
        .collect();
        b.label(
            &shape.id,
            "badges",
            0,
            badges.join(" · "),
            Point::new(rect.right() - 8.0, rect.y + SHACL_HEADER_HEIGHT / 2.0 + 5.0),
            TextAnchor::End,
            font(11.0, 400, Some(palette.muted)),
        );
        let targets = records(shape, "targets");
        for (index, chip) in targets.iter().enumerate() {
            b.label(
                &shape.id,
                "targets",
                index,
                target_words(chip),
                Point::new(
                    rect.x + 8.0,
                    rect.y + SHACL_HEADER_HEIGHT + SHACL_LINE_HEIGHT - 6.0 + index as f64 * SHACL_LINE_HEIGHT,
                ),
                TextAnchor::Start,
                styled(11.0, 400, palette.text, true, false),
            );
        }
        let start = SHACL_HEADER_HEIGHT + (targets.len() + 1) as f64 * SHACL_LINE_HEIGHT - 6.0;
        for (index, row) in records(shape, "rows").iter().enumerate() {
            let y = rect.y + start + index as f64 * SHACL_LINE_HEIGHT;
            let sparql = field(row, "sparql") == "true";
            let path = if sparql {
                "SPARQL constraint".to_owned()
            } else {
                field(row, "path").to_owned()
            };
            b.label(
                &shape.id,
                "rows",
                index,
                path,
                Point::new(rect.x + 8.0, y),
                TextAnchor::Start,
                styled(11.0, 400, palette.text, sparql, false),
            );
            b.label(
                &shape.id,
                "summary",
                index,
                fit(field(row, "summary"), rect.width - 110.0 - 48.0, 10.0),
                Point::new(rect.x + 110.0, y),
                TextAnchor::Start,
                font(10.0, 400, Some(palette.muted)),
            );
            b.label(
                &shape.id,
                "cardinality",
                index,
                field(row, "cardinality").to_owned(),
                Point::new(rect.right() - 8.0, y),
                TextAnchor::End,
                font(10.0, 400, Some(palette.muted)),
            );
        }
    }
    b.banner(elements, SHACL_TRUNCATION, |shown, total| {
        format!("Showing {shown} of {total} shapes. Edits are withheld while the view is partial.")
    });
    b.finish(None)
}
