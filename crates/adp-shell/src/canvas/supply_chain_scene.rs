//! The scene of a supply chain diagram, which its module lays out and hands the session as groups, cards and flows
//! (etalii.adp spec 030, task T076), drawn as `adp-scene` primitives so the one painter, hit test and gesture arbiter
//! serve it as they serve every other canvas.
//!
//! Ported from what the standalone host's `src/diagrams/supply-chain/client/SupplyChainCanvas.tsx` and
//! `supply-chain.css` at `8b6ccfc` draw, with the colours of `supply-chain.dis`'s theme:
//! - a group is a dashed frame beneath everything, its name 26 below its top at 13 and bold, and its member count at
//!   its right;
//! - a node is a card with a header band in its stage's colour, the stage's title in capitals 16 below its top, its
//!   name 46 below, its amount 74 below at 20 and bold, and a share bar along its foot;
//! - a flow is a band as broad as its weight, a horizontal S from the supplier's facing side to the consumer's through
//!   the lanes the layout reserved for it, with an arrowhead at the consumer, its product above its middle and its
//!   volume below;
//! - while something is selected, its chain is drawn at full strength and the rest dimmed: the module's trace
//!   ([`TraceGraph::of_elements`]) over what it was given.
//!
//! Every card and frame is moved by its top-left, which is what the document holds (`onElementMoved`).

use std::collections::{HashMap, HashSet};

use adp_diagram::DiagramElement;
use adp_diagram_supply_chain::mapper::{FLOW_TYPE, GROUP_TYPE, PREFIX};
use adp_diagram_supply_chain::trace::{DIMMED, DOWNSTREAM, SupplyChainTrace, TraceGraph, UPSTREAM};
use adp_scene::definition::labels::TextAnchor;
use adp_scene::scene::{EdgePrimitive, Hook, HookKind, Owner, Primitive, Role, ShapePrimitive};
use adp_scene::style::{Colour, Stroke};
use adp_scene::{Path, Point, Rect, Scene};

use super::module_scene::{ANCHOR_RADIUS, ModuleScene, SELECTED, SELECTED_WIDTH, font, number, stroke, text};

/// The header colour of `stage` (`supply-chain.dis`, `color.supply.<stage>`).
fn stage_colour(stage: &str) -> &'static str {
    match stage {
        "Source" => "#7c4a1e",
        "Processor" => "#b91c1c",
        "Producer" => "#1d4ed8",
        "Integrator" => "#0f766e",
        "Hub" => "#475569",
        "Outlet" => "#15803d",
        _ => "#be185d",
    }
}

const TEXT: &str = "#0f172a";
const MUTED: &str = "#64748b";
const SURFACE: &str = "#ffffff";
const BORDER: &str = "#e2e8f0";
const FLOW: &str = "#64748b";
const HEADER: f64 = 24.0;
const INSET: f64 = 14.0;
/// How far a lane runs straight either side of its slot's middle: half a card.
const LANE_HALF: f64 = 96.0;
/// The opacity of what is off the traced chain.
const DIM: f64 = 0.22;

/// A band's width for a flow's weight: 3 for a trickle, 18 for the heaviest in its unit (`bandWidthOf`).
pub fn band_width(weight: f64) -> f64 {
    3.0 + (weight.clamp(0.0, 1.0) * 15.0).round()
}

fn faded(base: f64, mark: &str) -> f64 {
    if mark == DIMMED { base * DIM } else { base }
}

fn rect_of(element: &DiagramElement) -> Rect {
    Rect::centred(
        Point::new(element.x, element.y),
        number(element, "width"),
        number(element, "height"),
    )
}

/// The stops a flow passes through: its two ends on the facing sides, and in and out of every lane (`flowStopsOf`).
fn stops(source: Rect, target: Rect, lanes: &[Point]) -> Vec<Point> {
    let forward = target.centre().x >= source.centre().x;
    let start = Point::new(if forward { source.right() } else { source.x }, source.centre().y);
    let end = Point::new(if forward { target.x } else { target.right() }, target.centre().y);
    let mut points = vec![start];
    for lane in lanes {
        points.push(Point::new(lane.x - LANE_HALF, lane.y));
        points.push(Point::new(lane.x + LANE_HALF, lane.y));
    }
    points.push(end);
    points
}

/// The horizontal S between each stop and the next (`flowPath`), as a path and as points sampled along it.
fn flow_path(stops: &[Point]) -> (Path, Vec<Point>) {
    let mut path = Path::new().move_to(stops[0]);
    let mut sampled = vec![stops[0]];
    for pair in stops.windows(2) {
        let (start, end) = (pair[0], pair[1]);
        let direction = if end.x >= start.x { 1.0 } else { -1.0 };
        let pull = f64::max(48.0, (end.x - start.x).abs() / 2.0);
        let (c1, c2) = (
            Point::new(start.x + direction * pull, start.y),
            Point::new(end.x - direction * pull, end.y),
        );
        path = path.cubic_to(c1, c2, end);
        for step in 1..=12 {
            let t = f64::from(step) / 12.0;
            let u = 1.0 - t;
            sampled.push(Point::new(
                u * u * u * start.x + 3.0 * u * u * t * c1.x + 3.0 * u * t * t * c2.x + t * t * t * end.x,
                u * u * u * start.y + 3.0 * u * u * t * c1.y + 3.0 * u * t * t * c2.y + t * t * t * end.y,
            ));
        }
    }
    (path, sampled)
}

fn lanes_of(flow: &DiagramElement) -> Vec<Point> {
    flow.attribute("lanes")
        .split(';')
        .filter_map(|lane| {
            let (x, y) = lane.split_once(',')?;
            Some(Point::new(x.parse().ok()?, y.parse().ok()?))
        })
        .collect()
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

/// The scene of `elements`, a supply chain view's rendering, traced through the first of `selection`, with the
/// elements `moving` carried by the drag in flight.
pub fn build(
    elements: &[DiagramElement],
    selection: &[String],
    moving: Option<(&[String], (f64, f64))>,
) -> ModuleScene {
    let selected: HashSet<&str> = selection.iter().map(String::as_str).collect();
    let trace = SupplyChainTrace::over(
        &TraceGraph::of_elements(elements),
        selection.first().map(String::as_str),
    );
    let mut rects: HashMap<String, Rect> = HashMap::new();
    for element in elements.iter().filter(|e| e.element_type != FLOW_TYPE) {
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
    let mut anchors = Vec::new();

    // The frames first, beneath the flows that cross them.
    for group in elements.iter().filter(|e| e.element_type == GROUP_TYPE) {
        let rect = drawn[&group.id];
        let owner = Owner::Element(group.id.clone());
        let mark = trace.of(&group.id);
        let dash: &[f64] = if mark == "selected" || mark == "related" {
            &[]
        } else {
            &[6.0, 4.0]
        };
        let outline = if selected.contains(group.id.as_str()) {
            stroke(Some(SELECTED), SELECTED_WIDTH, dash)
        } else {
            stroke(Some(BORDER), 1.5, dash)
        };
        primitives.push(shape(
            &owner,
            Role::Body,
            Path::rounded_rect(rect, 8.0),
            Some("#0000000f"),
            Some(outline),
            faded(1.0, mark),
        ));
        primitives.push(text(
            &owner,
            Role::Label {
                id: "name".to_owned(),
                line: 0,
            },
            group.attribute("name").to_owned(),
            Point::new(rect.x + 16.0, rect.y + 26.0),
            TextAnchor::Start,
            font(13.0, 700, None),
        ));
        primitives.push(text(
            &owner,
            Role::Label {
                id: "members".to_owned(),
                line: 0,
            },
            group.attribute("membersText").to_owned(),
            Point::new(rect.right() - 16.0, rect.y + 26.0),
            TextAnchor::End,
            font(11.0, 400, None),
        ));
        labels.push((
            group.id.clone(),
            "name".to_owned(),
            Rect::new(rect.x, rect.y + 10.0, rect.width, 24.0),
        ));
        boxes.push((group.id.clone(), rect));
    }

    // The flows, beneath the cards they join.
    for flow in elements.iter().filter(|e| e.element_type == FLOW_TYPE) {
        let (Some(source), Some(target)) = (
            flow.source.as_deref().and_then(|s| drawn.get(s)),
            flow.target.as_deref().and_then(|t| drawn.get(t)),
        ) else {
            continue;
        };
        let owner = Owner::Relation(flow.id.clone());
        let mark = trace.of(&flow.id);
        let width = band_width(number(flow, "weight"));
        let points = stops(*source, *target, &lanes_of(flow));
        let (path, sampled) = flow_path(&points);
        let opacity = if mark == UPSTREAM || mark == DOWNSTREAM {
            0.9
        } else {
            0.45
        };
        let band = if selected.contains(flow.id.as_str()) {
            Stroke {
                colour: Some(Colour::value(SELECTED)),
                width,
                dash: Vec::new(),
                opacity: 0.9,
            }
        } else {
            Stroke {
                colour: Some(Colour::value(FLOW)),
                width,
                dash: Vec::new(),
                opacity: faded(opacity, mark),
            }
        };
        primitives.push(Primitive::Edge(EdgePrimitive {
            owner: owner.clone(),
            path,
            stroke: band,
            start: None,
            end: None,
            hit_width: width.max(10.0),
            selectable: true,
            tooltip: Some(flow.attribute("product").to_owned()).filter(|p| !p.is_empty()),
        }));
        // The arrowhead, square to the consumer's side.
        let end = points[points.len() - 1];
        let direction = if end.x >= points[points.len() - 2].x { 1.0 } else { -1.0 };
        let head = 7.0 + width / 3.0;
        primitives.push(shape(
            &owner,
            Role::Decoration("head".to_owned()),
            Path::new()
                .move_to(end)
                .line_to(Point::new(end.x - direction * head, end.y - head * 0.7))
                .line_to(Point::new(end.x - direction * head, end.y + head * 0.7))
                .close(),
            Some(FLOW),
            None,
            faded(0.9, mark),
        ));
        lines.push((flow.id.clone(), sampled, width.max(10.0) / 2.0));

        let lanes = lanes_of(flow);
        let middle = if lanes.is_empty() {
            Point::new((points[0].x + end.x) / 2.0, (points[0].y + end.y) / 2.0)
        } else {
            lanes[lanes.len() / 2]
        };
        let product = flow.attribute("product");
        if !product.is_empty() {
            primitives.push(text(
                &owner,
                Role::Label {
                    id: "product".to_owned(),
                    line: 0,
                },
                product.to_owned(),
                Point::new(middle.x, middle.y - (9.0 + 6.0)),
                TextAnchor::Middle,
                font(12.0, 400, Some(TEXT)),
            ));
        }
        let volume = flow.attribute("volumeText");
        if !volume.is_empty() {
            primitives.push(text(
                &owner,
                Role::Label {
                    id: "volume".to_owned(),
                    line: 0,
                },
                volume.to_owned(),
                Point::new(middle.x, middle.y + width / 2.0 + 16.0),
                TextAnchor::Middle,
                font(10.0, 600, Some(TEXT)),
            ));
        }
    }

    // The cards.
    for node in elements
        .iter()
        .filter(|e| e.element_type.starts_with(PREFIX) && e.element_type != GROUP_TYPE && e.element_type != FLOW_TYPE)
    {
        let rect = drawn[&node.id];
        let owner = Owner::Element(node.id.clone());
        let mark = trace.of(&node.id);
        let opacity = faded(
            if moving.is_some_and(|(ids, _)| ids.contains(&node.id)) {
                0.75
            } else {
                1.0
            },
            mark,
        );
        let outline = if selected.contains(node.id.as_str()) {
            stroke(Some(SELECTED), SELECTED_WIDTH, &[])
        } else if mark == UPSTREAM || mark == DOWNSTREAM {
            stroke(Some(MUTED), 2.5, &[])
        } else {
            stroke(Some(BORDER), 1.5, &[])
        };
        let colour = stage_colour(node.attribute("stage"));
        primitives.push(shape(
            &owner,
            Role::Body,
            Path::rounded_rect(rect, 8.0),
            Some(SURFACE),
            Some(outline),
            opacity,
        ));
        primitives.push(shape(
            &owner,
            Role::Part("header".to_owned()),
            Path::new()
                .move_to(Point::new(rect.x, rect.y + HEADER))
                .line_to(Point::new(rect.x, rect.y + 8.0))
                .cubic_to(
                    Point::new(rect.x, rect.y + 3.6),
                    Point::new(rect.x + 3.6, rect.y),
                    Point::new(rect.x + 8.0, rect.y),
                )
                .line_to(Point::new(rect.right() - 8.0, rect.y))
                .cubic_to(
                    Point::new(rect.right() - 3.6, rect.y),
                    Point::new(rect.right(), rect.y + 3.6),
                    Point::new(rect.right(), rect.y + 8.0),
                )
                .line_to(Point::new(rect.right(), rect.y + HEADER))
                .close(),
            Some(colour),
            None,
            opacity,
        ));
        let track = Rect::new(rect.x + INSET, rect.bottom() - 12.0, rect.width - 2.0 * INSET, 4.0);
        primitives.push(shape(
            &owner,
            Role::Part("barTrack".to_owned()),
            Path::rounded_rect(track, 2.0),
            Some(BORDER),
            None,
            opacity,
        ));
        if node.attribute("hasQuantity") == "true" {
            let share = number(node, "share").min(1.0);
            primitives.push(shape(
                &owner,
                Role::Part("bar".to_owned()),
                Path::rounded_rect(Rect::new(track.x, track.y, (track.width * share).max(4.0), 4.0), 2.0),
                Some(colour),
                None,
                opacity,
            ));
        }
        for (id, value, dy, size, weight, colour) in [
            ("stage", node.attribute("stageTitle"), 16.0, 10.0, 700, "#ffffff"),
            ("name", node.attribute("name"), 46.0, 13.0, 600, TEXT),
        ] {
            primitives.push(text(
                &owner,
                Role::Label {
                    id: id.to_owned(),
                    line: 0,
                },
                value.to_owned(),
                Point::new(rect.x + INSET, rect.y + dy),
                TextAnchor::Start,
                font(size, weight, Some(colour)),
            ));
        }
        let unit = node.attribute("unit");
        let amount = if node.attribute("hasQuantity") == "true" && !unit.is_empty() {
            format!("{} {unit}", node.attribute("amount"))
        } else {
            node.attribute("amount").to_owned()
        };
        primitives.push(text(
            &owner,
            Role::Label {
                id: "amount".to_owned(),
                line: 0,
            },
            amount,
            Point::new(rect.x + INSET, rect.y + 74.0),
            TextAnchor::Start,
            font(20.0, 700, Some(TEXT)),
        ));
        labels.push((
            node.id.clone(),
            "name".to_owned(),
            Rect::new(rect.x, rect.y + 30.0, rect.width, 22.0),
        ));
        if selected.contains(node.id.as_str()) {
            primitives.push(Primitive::Hook(Hook {
                owner: owner.clone(),
                kind: HookKind::Selection,
                area: rect,
            }));
        }
        // A flow is drawn from the middle of either side.
        for (name, at) in [
            ("e", Point::new(rect.right(), rect.centre().y)),
            ("w", Point::new(rect.x, rect.centre().y)),
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

    let empty_text = (!elements.iter().any(|e| e.element_type != FLOW_TYPE))
        .then(|| "This supply chain has no node yet. Drop a stage from the toolbox.".to_owned());
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
    fn a_band_is_as_broad_as_its_weight() {
        assert_eq!(3.0, band_width(0.0));
        assert_eq!(18.0, band_width(1.0));
        assert_eq!(11.0, band_width(0.5));
    }

    #[test]
    fn a_long_flow_runs_in_and_out_of_each_lane() {
        let points = stops(
            Rect::new(0.0, 0.0, 192.0, 96.0),
            Rect::new(640.0, 0.0, 192.0, 96.0),
            &[Point::new(416.0, 48.0)],
        );
        assert_eq!(
            vec![
                Point::new(192.0, 48.0),
                Point::new(320.0, 48.0),
                Point::new(512.0, 48.0),
                Point::new(640.0, 48.0)
            ],
            points
        );
    }
}
