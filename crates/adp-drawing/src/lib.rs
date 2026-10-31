//! The recording painter (etalii.adp spec 035, T051, T127, ATD §2.4): a scene turned into `$defs/Drawing`, the drawing
//! a conformance fixture records and `atd validate` compares. It reads the scene a canvas paints, not pixels: each
//! element's body, its labels and its style; each relation's route, its ends and its style; the canvas's own bands and
//! texts as chrome; and the order everything is drawn in. Positions and sizes are in the canvas's units, rounded to
//! 0.01. Moved from the shell's canvas, so that a host without a window (`atd`, the language server, the validation
//! of a description) records what the shell's canvas draws.

use std::collections::{BTreeMap, HashMap, HashSet};

use adp_diagram::{DiagramDocuments, DiagramElement, DiagramSession, ElementKind};
use adp_disl::DislEnv;
use adp_scene::definition::compartments::Folding;
use adp_scene::geometry::PathCommand;
use adp_scene::scene::{Owner, Primitive, Role, ShapePrimitive};
use adp_scene::scene::{StandardRoutePlugins, default_size};
use adp_scene::style::{Colour, Stroke};
use adp_scene::{EstimatedMetrics, Evaluator, SceneOptions, Theme as SceneTheme, build_scene, compile_notation};
use adp_scene::{Point, Rect, Scene};
use serde_json::{Map, Value as Json, json};

fn round(value: f64) -> f64 {
    let rounded = (value * 100.0).round() / 100.0;
    if rounded == 0.0 { 0.0 } else { rounded }
}

fn number(value: f64) -> Json {
    let rounded = round(value);
    if rounded.fract() == 0.0 && rounded.abs() < 1e15 {
        json!(rounded as i64)
    } else {
        json!(rounded)
    }
}

fn bounds(rect: &Rect) -> Json {
    json!({"x": number(rect.x), "y": number(rect.y), "w": number(rect.width), "h": number(rect.height)})
}

fn point(at: Point) -> Json {
    json!({"x": number(at.x), "y": number(at.y)})
}

/// A path's points: where each command goes, curves and arcs by their end points.
fn points(path: &adp_scene::Path) -> Vec<Point> {
    path.commands
        .iter()
        .filter_map(|command| match command {
            PathCommand::MoveTo(to) | PathCommand::LineTo(to) => Some(*to),
            PathCommand::QuadTo { to, .. } | PathCommand::CubicTo { to, .. } | PathCommand::ArcTo { to, .. } => {
                Some(*to)
            }
            PathCommand::Close => None,
        })
        .collect()
}

/// The shape a body's path draws: a rectangle, a rounded rectangle, an ellipse, or a path.
fn shape_of(path: &adp_scene::Path) -> &'static str {
    let commands = &path.commands;
    let arcs = commands
        .iter()
        .filter(|c| matches!(c, PathCommand::ArcTo { .. }))
        .count();
    let curves = commands
        .iter()
        .filter(|c| matches!(c, PathCommand::QuadTo { .. } | PathCommand::CubicTo { .. }))
        .count();
    let lines = commands.iter().filter(|c| matches!(c, PathCommand::LineTo(_))).count();
    let ps = points(path);
    let axis_aligned = ps
        .windows(2)
        .all(|w| (w[0].x - w[1].x).abs() < 1e-9 || (w[0].y - w[1].y).abs() < 1e-9);
    if arcs == 0 && curves == 0 && (3..=4).contains(&lines) && axis_aligned {
        "rect"
    } else if arcs >= 2 && lines == 0 && curves == 0 || curves == 4 && lines == 0 {
        "ellipse"
    } else if (arcs == 4 || curves == 4) && lines >= 3 && axis_aligned {
        "roundRect"
    } else {
        "path"
    }
}

/// A paint as a colour, and the token it came from.
fn paint(colour: &Option<Colour>, key: &str, style: &mut Map<String, Json>, tokens: &mut Map<String, Json>) {
    let Some(colour) = colour else { return };
    if let Some(value) = &colour.value {
        style.insert(key.into(), json!(value));
    }
    if let Some(token) = &colour.token {
        tokens.insert(key.into(), json!(token));
    }
}

fn style_of(fill: &Option<Colour>, stroke: Option<&Stroke>, opacity: f64) -> Json {
    let (mut style, mut tokens) = (Map::new(), Map::new());
    paint(fill, "fill", &mut style, &mut tokens);
    if let Some(stroke) = stroke {
        paint(&stroke.colour, "stroke", &mut style, &mut tokens);
        style.insert("strokeWidth".into(), number(stroke.width));
        if !stroke.dash.is_empty() {
            style.insert(
                "dash".into(),
                Json::Array(stroke.dash.iter().map(|d| number(*d)).collect()),
            );
        }
    }
    if (opacity - 1.0).abs() > 1e-9 {
        style.insert("opacity".into(), number(opacity));
    }
    if !tokens.is_empty() {
        style.insert("tokens".into(), Json::Object(tokens));
    }
    Json::Object(style)
}

fn font_of(font: &adp_scene::style::Font) -> String {
    let mut out = format!("{} {}", number(font.size), font.weight);
    if font.italic {
        out.push_str(" italic");
    }
    if let Some(family) = &font.family {
        out.push(' ');
        out.push_str(family);
    }
    out
}

/// A label as the scene sets it: its lines by index, where its first line stands, and its font.
type Label = (Vec<(usize, String)>, Option<Point>, Option<String>);

/// What is known of a drawn element or relation while the scene is read.
#[derive(Default)]
struct Drawn {
    body: Option<ShapePrimitive>,
    extent: Option<Rect>,
    labels: BTreeMap<String, Label>,
    route: Vec<Point>,
    markers: (Option<String>, Option<String>),
    stroke: Option<Stroke>,
}

fn label_entries(labels: &BTreeMap<String, Label>) -> Vec<Json> {
    labels
        .iter()
        .map(|(id, (lines, at, font))| {
            let mut lines = lines.clone();
            lines.sort();
            let text: Vec<String> = lines.into_iter().map(|(_, t)| t).collect();
            let mut label = Map::new();
            label.insert("id".into(), json!(id));
            label.insert("text".into(), json!(text.join("\n")));
            if let Some(font) = font {
                label.insert("font".into(), json!(font));
            }
            if let Some(at) = at {
                label.insert("position".into(), point(*at));
            }
            Json::Object(label)
        })
        .collect()
}

fn marker_name(kind: adp_scene::notation::catalog::MarkerKind) -> &'static str {
    use adp_scene::notation::catalog::MarkerKind::*;
    match kind {
        None => "none",
        Arrow => "arrow",
        OpenArrow => "openArrow",
        Diamond => "diamond",
        Circle => "circle",
    }
}

/// The drawing of a scene: its elements, by the ids and types the diagram's elements give them, its relations with
/// their ends, the canvas's chrome, and the drawing order (`$defs/Drawing`).
pub fn record(scene: &Scene, elements: &[DiagramElement], view: &str) -> Json {
    let by_id: HashMap<&str, &DiagramElement> = elements.iter().map(|e| (e.id.as_str(), e)).collect();
    let mut drawn: BTreeMap<(bool, String), Drawn> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut chrome: Vec<Json> = Vec::new();
    let mention = |id: &str, order: &mut Vec<String>| {
        if !order.iter().any(|o| o == id) {
            order.push(id.to_owned());
        }
    };
    for primitive in &scene.primitives {
        let (relation, id) = match primitive.owner() {
            Owner::Element(id) => (false, id.clone()),
            Owner::Relation(id) => (true, id.clone()),
            Owner::Canvas => {
                let id = format!("chrome:{}", chrome.len() + 1);
                match primitive {
                    Primitive::Shape(shape) if shape.role != Role::Background => {
                        let Some(area) = shape.path.bounds() else { continue };
                        chrome.push(json!({
                            "id": id, "kind": "band", "bounds": bounds(&area),
                            "style": style_of(&shape.fill, shape.stroke.as_ref(), shape.opacity),
                        }));
                    }
                    Primitive::Text(text) => {
                        chrome.push(json!({
                            "id": id, "kind": "legend", "text": text.text,
                            "bounds": bounds(&Rect::new(text.position.x, text.position.y, 0.0, 0.0)),
                        }));
                    }
                    _ => continue,
                }
                mention(&id, &mut order);
                continue;
            }
        };
        if matches!(primitive, Primitive::Hook(_)) {
            continue;
        }
        mention(&id, &mut order);
        let entry = drawn.entry((relation, id)).or_default();
        match primitive {
            Primitive::Shape(shape) => {
                if let Some(area) = shape.path.bounds() {
                    entry.extent = Some(entry.extent.map_or(area, |e| e.union(&area)));
                }
                if shape.role == Role::Body && entry.body.is_none() {
                    entry.body = Some(shape.clone());
                }
            }
            Primitive::Text(text) => {
                let (label, line) = match &text.role {
                    Role::Label { id, line } => (id.clone(), *line),
                    Role::LineLabel => ("label".to_owned(), 0),
                    _ => continue,
                };
                let slot = entry.labels.entry(label).or_insert_with(|| (Vec::new(), None, None));
                slot.0.push((line, text.text.clone()));
                if slot.1.is_none() {
                    slot.1 = Some(text.position);
                    slot.2 = Some(font_of(&text.font));
                }
            }
            Primitive::Edge(edge) => {
                if entry.route.is_empty() {
                    entry.route = points(&edge.path);
                    entry.stroke = Some(edge.stroke.clone());
                    entry.markers = (
                        Some(edge.start.as_ref().map_or("none", |m| marker_name(m.kind)).to_owned()),
                        Some(edge.end.as_ref().map_or("none", |m| marker_name(m.kind)).to_owned()),
                    );
                }
            }
            Primitive::Hook(_) => {}
        }
    }

    let (mut nodes, mut edges) = (Vec::new(), Vec::new());
    for ((relation, id), entry) in drawn {
        let element = by_id.get(id.as_str());
        let kind = element.map(|e| e.element_type.clone()).unwrap_or_default();
        let labels = label_entries(&entry.labels);
        if relation {
            let mut edge = Map::new();
            edge.insert("id".into(), json!(id));
            edge.insert("type".into(), json!(kind));
            edge.insert(
                "source".into(),
                json!(element.and_then(|e| e.source.clone()).unwrap_or_default()),
            );
            edge.insert(
                "target".into(),
                json!(element.and_then(|e| e.target.clone()).unwrap_or_default()),
            );
            // A relation drawn as a band rather than a line runs along its band's middle.
            let route = if entry.route.is_empty() {
                entry
                    .extent
                    .map(|r| {
                        vec![
                            Point::new(r.x, r.y + r.height / 2.0),
                            Point::new(r.right(), r.y + r.height / 2.0),
                        ]
                    })
                    .unwrap_or_default()
            } else {
                entry.route.clone()
            };
            edge.insert("route".into(), Json::Array(route.into_iter().map(point).collect()));
            if let (Some(source), Some(target)) = &entry.markers {
                edge.insert("markers".into(), json!({"source": source, "target": target}));
            }
            if !labels.is_empty() {
                edge.insert("labels".into(), Json::Array(labels));
            }
            let style = match (&entry.stroke, &entry.body) {
                (Some(stroke), _) => style_of(&None, Some(stroke), stroke.opacity),
                (None, Some(body)) => style_of(&body.fill, body.stroke.as_ref(), body.opacity),
                (None, None) => json!({}),
            };
            edge.insert("style".into(), style);
            edges.push(Json::Object(edge));
        } else {
            // An element drawn as text only (a note on a map) is where its text is.
            let area = entry
                .body
                .as_ref()
                .and_then(|b| b.path.bounds())
                .or(entry.extent)
                .or_else(|| Rect::around(entry.labels.values().filter_map(|(_, at, _)| *at)));
            let Some(area) = area else { continue };
            let mut node = Map::new();
            node.insert("id".into(), json!(id));
            node.insert("type".into(), json!(kind));
            node.insert(
                "shape".into(),
                json!(entry.body.as_ref().map_or("path", |b| shape_of(&b.path))),
            );
            node.insert("bounds".into(), bounds(&area));
            if !labels.is_empty() {
                node.insert("labels".into(), Json::Array(labels));
            }
            if let Some(body) = &entry.body {
                node.insert("style".into(), style_of(&body.fill, body.stroke.as_ref(), body.opacity));
            }
            let children: Vec<Json> = elements
                .iter()
                .filter(|e| e.parent.as_deref() == Some(id.as_str()))
                .map(|e| json!(e.id))
                .collect();
            if !children.is_empty() {
                node.insert("children".into(), Json::Array(children));
            }
            nodes.push(Json::Object(node));
        }
    }
    json!({
        "$schema": "https://etalii.net/adp/atd/schema/0.1/atd.schema.json#/$defs/Drawing",
        "view": view,
        "elements": nodes,
        "edges": edges,
        "chrome": chrome,
        "order": order,
    })
}

/// The scene a diagram's notation draws for `elements`, as a recording takes it: nothing selected, laid out with the
/// estimated text metrics that do not depend on the fonts of a machine, as the `insta` scene snapshots of `adp-disl`
/// are.
pub fn notation_scene(
    session: &DiagramSession,
    documents: &DiagramDocuments,
    elements: &[DiagramElement],
) -> Result<Scene, String> {
    let document = documents.get(session.body_path()).ok_or("the diagram is not open")?;
    let specification = document.tool().definition.specification();
    let definition = compile_notation(specification).map_err(|e| e.to_string())?;
    let evaluator = Evaluator::new(
        &definition.specification,
        SceneTheme::of(&definition.specification, None),
        DislEnv::default(),
    );
    let diagram = &document.model().diagram;
    let module = document.tool().module.as_deref();
    let positions: HashMap<String, Rect> = elements
        .iter()
        .filter(|e| e.kind == ElementKind::Node)
        .filter_map(|e| {
            let measured = module.and_then(|m| m.size_of(document, &diagram.element_by_id(&e.id)?));
            let Some((width, height)) = measured else {
                let (width, height) = default_size(definition.element_type(&e.element_type)?);
                return Some((e.id.clone(), Rect::new(e.x, e.y, width, height)));
            };
            // A size the module measures goes with a position that is the node's centre.
            Some((
                e.id.clone(),
                Rect::new(e.x - width / 2.0, e.y - height / 2.0, width, height),
            ))
        })
        .collect();
    // The view draws what its viewpoint shows (DISL section 3.5).
    let shown: Option<HashSet<String>> = session
        .uses_viewpoint(specification)
        .then(|| diagram.shown_in(session.viewpoint()).iter().map(|e| e.id()).collect());
    let folding = Folding::default();
    let options = SceneOptions {
        selection: &[],
        folding: &folding,
        metrics: &EstimatedMetrics,
        plugins: &StandardRoutePlugins,
        shown: shown.as_ref(),
    };
    Ok(build_scene(&definition, &evaluator, diagram, &positions, &options))
}
