//! `routing.forwardBezier` from the routing plugin the desktop host draws the dependency graph's curve with
//! (`StandardRoutePlugins` in `crates/adp-scene/src/scene/builder.rs`, over `crates/adp-scene/src/connectors.rs`), on
//! every diagram of the dependency graph's parity scripts: each dependency's two end boxes as the canvas draws them
//! in, and its path out, a move and one cubic curve with its two control points.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use adp_scene::geometry::PathCommand;
use adp_scene::scene::{RouteEnds, RoutePlugins, StandardRoutePlugins};
use adp_scene::{Point, Rect};
use serde_json::{Map, Value as Json, json};

use super::documents::{opened, round2};
use super::{Recorded, write_fixture};

/// The tool type whose curve this is.
const ORIGIN: &str = "generic/dependencies";
/// The routing plugin the dependency graph names for it.
const PLUGIN: &str = "net.etalii.adp.generic.dependencyCurve";
/// The relation type it routes.
const RELATION: &str = "DependsOn";

/// Records the fixtures of `routing.forwardBezier` into `cases`.
pub fn record(cases: &Path) -> Result<Vec<PathBuf>, String> {
    let mut written = Vec::new();
    for document in opened(ORIGIN, true)? {
        let drawing = document.drawing.as_ref().ok_or("the dependency graph draws")?;
        let boxes: HashMap<&str, Rect> = drawing["elements"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|element| Some((element["id"].as_str()?, rect(&element["bounds"])?)))
            .collect();
        let mut edges = Vec::new();
        let mut routes = Map::new();
        for edge in drawing["edges"].as_array().into_iter().flatten() {
            if edge["type"].as_str() != Some(RELATION) {
                continue;
            }
            let (Some(id), Some(source), Some(target)) = (
                edge["id"].as_str(),
                edge["source"].as_str().and_then(|s| boxes.get(s)),
                edge["target"].as_str().and_then(|t| boxes.get(t)),
            ) else {
                continue;
            };
            let ends = RouteEnds {
                source: *source,
                target: *target,
            };
            let path = StandardRoutePlugins
                .route(PLUGIN, Point::new(0.0, 0.0), Point::new(0.0, 0.0), &ends)
                .ok_or("the dependency curve routes every dependency")?;
            edges.push(json!({"id": id, "source": box_of(source), "target": box_of(target)}));
            routes.insert(
                id.to_owned(),
                Json::Array(path.commands.iter().filter_map(command).collect()),
            );
        }
        let from = Recorded {
            capability: "routing.forwardBezier",
            code: "crates/adp-scene/src/scene/builder.rs (StandardRoutePlugins) and crates/adp-scene/src/connectors.rs",
            source: &document.source,
            note: "The end boxes are the drawing's, as the canvas draws them.",
        };
        written.push(write_fixture(
            cases,
            &document.name,
            &from,
            json!({"edges": edges}),
            json!({}),
            json!({"routes": routes}),
        )?);
    }
    Ok(written)
}

/// A drawing's box as a rectangle.
fn rect(bounds: &Json) -> Option<Rect> {
    Some(Rect::new(
        bounds["x"].as_f64()?,
        bounds["y"].as_f64()?,
        bounds["w"].as_f64()?,
        bounds["h"].as_f64()?,
    ))
}

fn box_of(rect: &Rect) -> Json {
    json!({"x": round2(rect.x), "y": round2(rect.y), "width": round2(rect.width), "height": round2(rect.height)})
}

fn point(at: Point) -> Json {
    json!({"x": round2(at.x), "y": round2(at.y)})
}

/// One command of a route: its operation and its points, control points first.
fn command(command: &PathCommand) -> Option<Json> {
    Some(match command {
        PathCommand::MoveTo(to) => json!({"op": "move", "points": [point(*to)]}),
        PathCommand::LineTo(to) => json!({"op": "line", "points": [point(*to)]}),
        PathCommand::QuadTo { control, to } => json!({"op": "quad", "points": [point(*control), point(*to)]}),
        PathCommand::CubicTo { first, second, to } => {
            json!({"op": "cubic", "points": [point(*first), point(*second), point(*to)]})
        }
        PathCommand::ArcTo { .. } | PathCommand::Close => return None,
    })
}
