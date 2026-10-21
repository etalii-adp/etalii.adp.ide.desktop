//! A custom shape's geometry (DISL sections 2.5 c and 6.8; the standalone host's
//! `canvas/library/disl/geomExpr.ts` at 8b6ccfc): GeomExprs, numbers or CEL over the shape context's `w`, `h` and
//! the shape's parameters `p`, evaluated into a path and boxes.
//!
//! The standalone evaluates GeomExprs only in its tests, with a subset reader of its own, to prove that the library
//! shape a module draws a custom shape as has the custom shape's geometry. This host draws the custom shape itself,
//! so its GeomExprs are evaluated where the scene is built, by the runtime's CEL in the shape context
//! ([`contexts::SHAPE`]); what lies outside that context is refused by the CEL compiler, naming it, rather than by a
//! subset reader. A percentage (`"50%"`), which DISL allows where a number goes, is that fraction of the box's width
//! for a horizontal coordinate and of its height for a vertical one.

use adp_cel::{Map, Value};
use adp_disl::expressions::contexts;
use serde_json::Value as Json;

use crate::evaluation::{Evaluator, Vars};
use crate::geometry::{Path, Point};

/// The shape context: the box's width and height, the shape's parameters, and the element it draws.
#[derive(Debug, Clone)]
pub struct GeomScope {
    pub w: f64,
    pub h: f64,
    pub p: Map,
    /// `self`, where the shape draws an element.
    pub element: Option<Value>,
}

impl GeomScope {
    pub fn new(w: f64, h: f64) -> Self {
        GeomScope {
            w,
            h,
            p: Map::default(),
            element: None,
        }
    }

    fn vars(&self, evaluator: &Evaluator) -> Vars {
        let mut vars = Vars::new();
        vars.insert("w".to_owned(), Value::Double(self.w));
        vars.insert("h".to_owned(), Value::Double(self.h));
        vars.insert("p".to_owned(), Value::map(self.p.clone()));
        vars.insert("self".to_owned(), self.element.clone().unwrap_or(Value::Null));
        vars.insert("env".to_owned(), evaluator.env_value());
        vars
    }
}

/// Which dimension a percentage is a fraction of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeomAxis {
    Horizontal,
    Vertical,
}

impl GeomAxis {
    /// The axis of a coordinate by its key (`x`, `rx`, `w`, `x1`, `cx` are horizontal).
    pub fn of_key(key: &str) -> GeomAxis {
        if matches!(key, "x" | "rx" | "w" | "x1" | "x2" | "cx") {
            GeomAxis::Horizontal
        } else {
            GeomAxis::Vertical
        }
    }
}

fn value_of(evaluator: &Evaluator, expression: &Json, scope: &GeomScope, axis: GeomAxis) -> Result<Value, String> {
    match expression {
        Json::Number(n) => Ok(Value::Double(n.as_f64().unwrap_or(0.0))),
        Json::Bool(b) => Ok(Value::Bool(*b)),
        Json::String(text) => {
            if let Some(percent) = text.strip_suffix('%').and_then(|t| t.trim().parse::<f64>().ok()) {
                let whole = match axis {
                    GeomAxis::Horizontal => scope.w,
                    GeomAxis::Vertical => scope.h,
                };
                return Ok(Value::Double(whole * percent / 100.0));
            }
            evaluator.evaluate(contexts::SHAPE, text, &scope.vars(evaluator))
        }
        Json::Object(o) => match o.get("cel").and_then(Json::as_str) {
            Some(cel) => evaluator.evaluate(contexts::SHAPE, cel, &scope.vars(evaluator)),
            None => Err(format!("The GeomExpr {expression} is neither a number nor CEL.")),
        },
        other => Err(format!("The GeomExpr {other} is neither a number nor CEL.")),
    }
}

/// A GeomExpr's number in a scope; a literal number is itself.
pub fn evaluate_geom(
    evaluator: &Evaluator,
    expression: &Json,
    scope: &GeomScope,
    axis: GeomAxis,
) -> Result<f64, String> {
    match value_of(evaluator, expression, scope, axis)? {
        Value::Int(i) => Ok(i as f64),
        Value::Double(d) => Ok(d),
        Value::Bool(_) => Err(format!("The GeomExpr {expression} is a condition, not a number.")),
        other => Err(format!("The GeomExpr {expression} gives {other}, not a number.")),
    }
}

/// A part's `when` in a scope; an absent `when` holds.
pub fn evaluate_condition(evaluator: &Evaluator, expression: Option<&Json>, scope: &GeomScope) -> Result<bool, String> {
    let Some(expression) = expression else {
        return Ok(true);
    };
    match value_of(evaluator, expression, scope, GeomAxis::Horizontal)? {
        Value::Bool(b) => Ok(b),
        _ => Err(format!("The GeomExpr {expression} is a number, not a condition.")),
    }
}

/// One resolved path command.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResolvedSegment {
    Move {
        x: f64,
        y: f64,
    },
    Line {
        x: f64,
        y: f64,
    },
    Arc {
        rx: f64,
        ry: f64,
        rotation: f64,
        large_arc: bool,
        sweep: bool,
        x: f64,
        y: f64,
    },
    Close,
}

/// A custom shape's path with every GeomExpr evaluated.
pub fn resolve_path(
    evaluator: &Evaluator,
    segments: &[Json],
    scope: &GeomScope,
) -> Result<Vec<ResolvedSegment>, String> {
    let at = |segment: &Json, key: &str| -> Result<f64, String> {
        let expression = segment
            .get(key)
            .ok_or_else(|| format!("The path segment {segment} has no \"{key}\"."))?;
        evaluate_geom(evaluator, expression, scope, GeomAxis::of_key(key))
    };
    segments
        .iter()
        .map(|segment| match segment.get("op").and_then(Json::as_str) {
            Some("M") => Ok(ResolvedSegment::Move {
                x: at(segment, "x")?,
                y: at(segment, "y")?,
            }),
            Some("L") => Ok(ResolvedSegment::Line {
                x: at(segment, "x")?,
                y: at(segment, "y")?,
            }),
            Some("A") => Ok(ResolvedSegment::Arc {
                rx: at(segment, "rx")?,
                ry: at(segment, "ry")?,
                rotation: at(segment, "rotation")?,
                large_arc: segment.get("largeArc").and_then(Json::as_bool).unwrap_or(false),
                sweep: segment.get("sweep").and_then(Json::as_bool).unwrap_or(false),
                x: at(segment, "x")?,
                y: at(segment, "y")?,
            }),
            Some("Z") => Ok(ResolvedSegment::Close),
            other => Err(format!(
                "Path command \"{}\" is outside the shape-context subset.",
                other.unwrap_or("")
            )),
        })
        .collect()
}

/// A box with every GeomExpr evaluated: `(x, y, w, h)` in the shape's own coordinates.
pub fn resolve_box(evaluator: &Evaluator, box_: &Json, scope: &GeomScope) -> Result<(f64, f64, f64, f64), String> {
    let at = |key: &str| -> Result<f64, String> {
        let expression = box_
            .get(key)
            .ok_or_else(|| format!("The box {box_} has no \"{key}\"."))?;
        evaluate_geom(evaluator, expression, scope, GeomAxis::of_key(key))
    };
    Ok((at("x")?, at("y")?, at("w")?, at("h")?))
}

/// The resolved segments as a path, its shape coordinates moved to `origin`.
pub fn path_of(segments: &[ResolvedSegment], origin: Point) -> Path {
    let at = |x: f64, y: f64| Point::new(origin.x + x, origin.y + y);
    let mut path = Path::new();
    for segment in segments {
        path = match *segment {
            ResolvedSegment::Move { x, y } => path.move_to(at(x, y)),
            ResolvedSegment::Line { x, y } => path.line_to(at(x, y)),
            ResolvedSegment::Arc {
                rx,
                ry,
                rotation,
                large_arc,
                sweep,
                x,
                y,
            } => path.arc_to(rx, ry, rotation, large_arc, sweep, at(x, y)),
            ResolvedSegment::Close => path.close(),
        };
    }
    path
}

#[cfg(test)]
mod tests {
    //! The standalone host's `geomExpr.test.ts`: the same scope and the same answers, through CEL.
    use super::*;
    use crate::style::Theme;
    use adp_disl::{DislEnv, DislLoader, DislSpecification};
    use serde_json::json;

    fn evaluator() -> Evaluator {
        let specification: DislSpecification = DislLoader::load(
            r#"{ "disl": "0.3", "language": { "id": "test", "version": "1.0.0" },
                 "metamodel": { "types": { "Card": {} } }, "notation": { "nodes": { "Card": { "shape": "rect" } } } }"#,
        )
        .specification
        .unwrap();
        Evaluator::new(&specification, Theme::default(), DislEnv::default())
    }

    fn scope() -> GeomScope {
        let mut scope = GeomScope::new(96.0, 32.0);
        scope.p.insert("count", Value::Int(3));
        scope.p.insert("b1", Value::Double(0.25));
        scope.p.insert("b2", Value::Double(0.5));
        scope
    }

    fn geom(expression: Json) -> f64 {
        evaluate_geom(&evaluator(), &expression, &scope(), GeomAxis::Horizontal).unwrap()
    }

    fn condition(expression: Option<Json>) -> bool {
        evaluate_condition(&evaluator(), expression.as_ref(), &scope()).unwrap()
    }

    #[test]
    fn numbers_the_box_the_parameters_arithmetic_and_min_max_are_read() {
        assert_eq!(12.0, geom(json!(12)));
        assert_eq!(80.0, geom(json!("w - min(h / 2.0, w / 2.0)")));
        assert_eq!(30.0, geom(json!("w * p.b1 - max(1, 2) * -3")));
        assert_eq!(64.0, geom(json!("(w + h) / 2")));
        assert_eq!(48.0, geom(json!("50%")));
        assert_eq!(
            8.0,
            evaluate_geom(&evaluator(), &json!("25%"), &scope(), GeomAxis::Vertical).unwrap()
        );
    }

    #[test]
    fn conditions_and_the_conditional_the_custom_shapes_use_are_read() {
        assert!(condition(Some(json!("p.count >= 2"))));
        assert!(condition(Some(json!("p.count == 4 || !(w < h)"))));
        assert!(!condition(Some(json!("p.count >= 2 && p.count < 3"))));
        assert!(condition(None));
        assert_eq!(96.0, geom(json!("p.count == 3 ? w : w * p.b2")));
        assert_eq!(48.0, geom(json!("p.count == 1 ? w : w * p.b2")));
    }

    #[test]
    fn a_condition_where_a_number_stands_and_an_unbound_parameter_are_refused() {
        let e = evaluator();
        assert!(
            evaluate_geom(&e, &json!("w > h"), &scope(), GeomAxis::Horizontal)
                .unwrap_err()
                .contains("is a condition")
        );
        assert!(
            evaluate_condition(&e, Some(&json!("w")), &scope())
                .unwrap_err()
                .contains("is a number")
        );
        assert!(evaluate_geom(&e, &json!("w +"), &scope(), GeomAxis::Horizontal).is_err());
        assert!(evaluate_geom(&e, &json!("q.b3"), &scope(), GeomAxis::Horizontal).is_err());
    }

    #[test]
    fn a_path_and_a_box_are_resolved() {
        let e = evaluator();
        let segments = [
            json!({ "op": "M", "x": 0, "y": 0 }),
            json!({ "op": "L", "x": "w", "y": "h / 2.0" }),
            json!({ "op": "Z" }),
        ];
        assert_eq!(
            vec![
                ResolvedSegment::Move { x: 0.0, y: 0.0 },
                ResolvedSegment::Line { x: 96.0, y: 16.0 },
                ResolvedSegment::Close
            ],
            resolve_path(&e, &segments, &scope()).unwrap()
        );
        assert_eq!(
            (6.0, 8.0, 84.0, 16.0),
            resolve_box(
                &e,
                &json!({ "x": 6, "y": "h * 0.25", "w": "w - 12", "h": "h / 2" }),
                &scope()
            )
            .unwrap()
        );
    }
}
