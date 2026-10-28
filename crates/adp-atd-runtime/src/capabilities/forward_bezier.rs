//! `routing.forwardBezier` (ATD §13.3): the curve the desktop host draws the dependency graph's dependencies with,
//! `StandardRoutePlugins` in `adp-scene`, called (not rewritten) for each edge of `$defs/RoutingInput`.

use adp_scene::scene::{RouteEnds, RoutePlugins, StandardRoutePlugins};
use adp_scene::{Path, PathCommand, Point, Rect};
use serde_json::{Map, Value as Json, json};

use super::RoutingCapability;

/// The capability's name, as `StandardRoutePlugins` answers it.
const NAME: &str = "routing.forwardBezier";

pub struct ForwardBezier;

impl RoutingCapability for ForwardBezier {
    fn route(&self, from: Point, to: Point, ends: &RouteEnds) -> Option<Path> {
        StandardRoutePlugins.route(NAME, from, to, ends)
    }

    fn routing(&self, input: &Json, _args: &Json) -> Result<Json, String> {
        let mut routes = Map::new();
        for edge in input["edges"].as_array().ok_or("The input has no `edges`.")? {
            let id = edge["id"].as_str().ok_or("An edge has no `id`.")?;
            let ends = RouteEnds {
                source: rect(&edge["source"]).ok_or_else(|| format!("Edge `{id}` has no source box."))?,
                target: rect(&edge["target"]).ok_or_else(|| format!("Edge `{id}` has no target box."))?,
            };
            let origin = Point::new(0.0, 0.0);
            let path = self
                .route(origin, origin, &ends)
                .ok_or_else(|| format!("Edge `{id}` is not routed."))?;
            routes.insert(
                id.to_owned(),
                Json::Array(path.commands.iter().filter_map(command).collect()),
            );
        }
        Ok(json!({ "routes": routes }))
    }
}

/// A box of `$defs/RoutingInput`.
fn rect(bounds: &Json) -> Option<Rect> {
    Some(Rect::new(
        bounds["x"].as_f64()?,
        bounds["y"].as_f64()?,
        bounds["width"].as_f64()?,
        bounds["height"].as_f64()?,
    ))
}

/// To two decimals, a midpoint away from zero and no negative zero, as the fixtures were recorded.
fn round2(value: f64) -> f64 {
    let rounded = (value * 100.0).round() / 100.0;
    if rounded == 0.0 { 0.0 } else { rounded }
}

fn point(at: Point) -> Json {
    json!({"x": round2(at.x), "y": round2(at.y)})
}

/// One command of a route (`$defs/RoutingResult`): its operation and its points, control points first.
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
