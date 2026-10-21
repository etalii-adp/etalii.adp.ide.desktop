//! What DISL's built-in shapes, markers and routings are drawn as (the standalone host's
//! `canvas/library/disl/shapeCatalog.ts` at 8b6ccfc).
//!
//! A shape is admitted only with the parameters the library draws: its hexagon is inset a quarter with flat ends, its
//! parallelogram leans a fifth to the right, its superellipse is the squircle of exponent 4 (`shapes::outline`). A
//! reference asking for other parameters is refused rather than drawn as something else; a parameter left out takes
//! DISL's default (Appendix B.2), which is then checked the same way.
//!
//! Where the two hosts differ: the standalone catalog has no `roundedRect`, which each standalone module draws as a
//! shape of its own family (the timeline's and the dependency graph's `span`). This host has no modules to say so,
//! and draws DISL's `roundedRect` as the library's rounded rectangle, at the `radius` the reference gives (DISL's
//! default 8), whatever other parameter it states.

use serde_json::{Map, Value as Json};

use crate::shapes::BuiltInShape;

/// Why a reference is refused.
pub type Refusal = String;

/// One catalog entry: the library's shape, the parameters its drawing has, and DISL's defaults for them.
struct Entry {
    name: &'static str,
    shape: BuiltInShape,
    draws: &'static [(&'static str, ParamValue)],
    defaults: &'static [(&'static str, ParamValue)],
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ParamValue {
    Number(f64),
    Text(&'static str),
}

impl ParamValue {
    fn matches(self, value: &Json) -> bool {
        match self {
            ParamValue::Number(n) => value.as_f64() == Some(n),
            ParamValue::Text(t) => value.as_str() == Some(t),
        }
    }

    fn to_json(self) -> Json {
        match self {
            ParamValue::Number(n) => serde_json::json!(n),
            ParamValue::Text(t) => Json::String(t.to_owned()),
        }
    }
}

const SHAPES: [Entry; 9] = [
    Entry {
        name: "none",
        shape: BuiltInShape::None,
        draws: &[],
        defaults: &[],
    },
    Entry {
        name: "rect",
        shape: BuiltInShape::Box,
        draws: &[],
        defaults: &[],
    },
    Entry {
        name: "ellipse",
        shape: BuiltInShape::Ellipse,
        draws: &[],
        defaults: &[],
    },
    Entry {
        name: "pill",
        shape: BuiltInShape::Pill,
        draws: &[],
        defaults: &[],
    },
    Entry {
        name: "diamond",
        shape: BuiltInShape::Diamond,
        draws: &[],
        defaults: &[],
    },
    Entry {
        name: "hexagon",
        shape: BuiltInShape::Hexagon,
        draws: &[
            ("inset", ParamValue::Number(0.25)),
            ("orientation", ParamValue::Text("flat")),
        ],
        defaults: &[
            ("inset", ParamValue::Number(0.25)),
            ("orientation", ParamValue::Text("flat")),
        ],
    },
    Entry {
        name: "parallelogram",
        shape: BuiltInShape::Parallelogram,
        draws: &[
            ("skew", ParamValue::Number(0.2)),
            ("direction", ParamValue::Text("right")),
        ],
        defaults: &[
            ("skew", ParamValue::Number(0.2)),
            ("direction", ParamValue::Text("right")),
        ],
    },
    Entry {
        name: "trapezoid",
        shape: BuiltInShape::Trapezoid,
        draws: &[
            ("inset", ParamValue::Number(0.15)),
            ("direction", ParamValue::Text("down")),
        ],
        defaults: &[
            ("inset", ParamValue::Number(0.2)),
            ("direction", ParamValue::Text("up")),
        ],
    },
    Entry {
        name: "superellipse",
        shape: BuiltInShape::Superellipse,
        draws: &[("exponent", ParamValue::Number(4.0))],
        defaults: &[("exponent", ParamValue::Number(4.0))],
    },
];

/// The DISL built-in shape names the library draws.
pub fn disl_shape_names() -> Vec<&'static str> {
    SHAPES.iter().map(|e| e.name).collect()
}

/// A shape reference split into its name and its parameters (DISL section 6.7).
pub fn shape_name_of(reference: &Json) -> (String, Map<String, Json>) {
    match reference {
        Json::String(name) => (name.clone(), Map::new()),
        Json::Object(o) => (
            o.get("type").and_then(Json::as_str).unwrap_or("").to_owned(),
            o.get("params").and_then(Json::as_object).cloned().unwrap_or_default(),
        ),
        _ => (String::new(), Map::new()),
    }
}

/// The library shape for a DISL built-in; none for a name the catalog does not know (a custom shape); refused for a
/// built-in asked for with parameters the library does not draw.
pub fn library_shape_of(reference: &Json) -> Result<Option<BuiltInShape>, Refusal> {
    let (name, params) = shape_name_of(reference);
    if name == "roundedRect" {
        return Ok(Some(BuiltInShape::RoundedRectangle));
    }
    let Some(entry) = SHAPES.iter().find(|e| e.name == name) else {
        return Ok(None);
    };
    let mut keys: Vec<&str> = params.keys().map(String::as_str).collect();
    for (key, _) in entry.draws {
        if !keys.contains(key) {
            keys.push(key);
        }
    }
    for key in keys {
        let Some((_, drawn)) = entry.draws.iter().find(|(k, _)| *k == key) else {
            return Err(format!(
                "The library draws \"{name}\" with no \"{key}\" parameter; the specification sets one."
            ));
        };
        let asked = params
            .get(key)
            .cloned()
            .or_else(|| entry.defaults.iter().find(|(k, _)| *k == key).map(|(_, v)| v.to_json()))
            .unwrap_or(Json::Null);
        if !drawn.matches(&asked) {
            return Err(format!(
                "The library draws \"{name}\" with {key} {}; the specification asks for {asked}.",
                drawn.to_json()
            ));
        }
    }
    Ok(Some(entry.shape))
}

/// The markers the library draws (the standalone's `MarkerKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarkerKind {
    None,
    /// The solid triangle.
    Arrow,
    /// The open V.
    OpenArrow,
    Diamond,
    Circle,
}

/// DISL's marker (Appendix B.3) as the library draws it. DISL's `arrow` is the open V and `arrowFilled` the solid
/// triangle; the library's names cross over.
pub fn library_marker_of(marker: &str) -> Result<MarkerKind, Refusal> {
    match marker {
        "none" => Ok(MarkerKind::None),
        "arrow" => Ok(MarkerKind::OpenArrow),
        "arrowFilled" => Ok(MarkerKind::Arrow),
        other => Err(format!("The library draws no marker for DISL's \"{other}\".")),
    }
}

/// The library's built-in routes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltInRoute {
    Straight,
    Polyline,
    Orthogonal,
    Arc,
    QuadraticBezier,
    CubicBezier,
    Spline,
}

/// DISL's edge routing (section 6.10) by the library's route.
pub fn library_route_of(routing: &str) -> Result<BuiltInRoute, Refusal> {
    match routing {
        "straight" => Ok(BuiltInRoute::Straight),
        "polyline" => Ok(BuiltInRoute::Polyline),
        "orthogonal" => Ok(BuiltInRoute::Orthogonal),
        "bezier" => Ok(BuiltInRoute::CubicBezier),
        "spline" => Ok(BuiltInRoute::Spline),
        other => Err(format!("The library draws no route for DISL's \"{other}\" routing.")),
    }
}

#[cfg(test)]
mod tests {
    //! The standalone host's `shapeCatalog.test.ts`; the outline half is in `shapes::outline`.
    use super::*;
    use serde_json::json;

    #[test]
    fn every_name_is_a_shape_the_library_has() {
        for name in disl_shape_names() {
            let reference = if name == "trapezoid" {
                json!({ "type": name, "params": { "inset": 0.15, "direction": "down" } })
            } else {
                json!(name)
            };
            let shape = library_shape_of(&reference).unwrap().unwrap();
            assert!(BuiltInShape::ALL.contains(&shape), "{name}");
        }
    }

    #[test]
    fn a_built_in_is_admitted_at_the_parameters_the_library_draws_its_defaults_included() {
        assert_eq!(Ok(Some(BuiltInShape::Box)), library_shape_of(&json!("rect")));
        assert_eq!(Ok(Some(BuiltInShape::Hexagon)), library_shape_of(&json!("hexagon")));
        assert_eq!(
            Ok(Some(BuiltInShape::Superellipse)),
            library_shape_of(&json!({ "type": "superellipse", "params": { "exponent": 4 } }))
        );
        assert_eq!(
            Ok(Some(BuiltInShape::Parallelogram)),
            library_shape_of(&json!({ "type": "parallelogram", "params": { "skew": 0.2, "direction": "right" } }))
        );
        assert_eq!(Ok(None), library_shape_of(&json!("phasedBanner")));
        assert_eq!(
            Ok(Some(BuiltInShape::RoundedRectangle)),
            library_shape_of(&json!({ "type": "roundedRect", "params": { "radius": 6 } }))
        );
    }

    #[test]
    fn a_built_in_asked_for_with_other_parameters_is_refused() {
        let refused = |reference: Json| library_shape_of(&reference).unwrap_err();
        let exponent = refused(json!({ "type": "superellipse", "params": { "exponent": 3 } }));
        assert!(
            exponent.contains("exponent 4") && exponent.contains("asks for 3"),
            "{exponent}"
        );
        assert!(refused(json!({ "type": "hexagon", "params": { "orientation": "pointy" } })).contains("orientation"));
        assert!(refused(json!({ "type": "rect", "params": { "radius": 4 } })).contains("no \"radius\" parameter"));
        assert!(refused(json!("trapezoid")).contains("inset 0.15"));
    }

    #[test]
    fn disls_marker_names_cross_over_to_the_librarys() {
        assert_eq!(Ok(MarkerKind::Arrow), library_marker_of("arrowFilled"));
        assert_eq!(Ok(MarkerKind::OpenArrow), library_marker_of("arrow"));
        assert_eq!(Ok(MarkerKind::None), library_marker_of("none"));
        assert!(library_marker_of("erMany").unwrap_err().contains("no marker"));
    }

    #[test]
    fn disls_routings_map_to_the_librarys_routes() {
        assert_eq!(Ok(BuiltInRoute::CubicBezier), library_route_of("bezier"));
        assert_eq!(Ok(BuiltInRoute::Orthogonal), library_route_of("orthogonal"));
        assert!(library_route_of("manhattan").unwrap_err().contains("no route"));
    }
}
