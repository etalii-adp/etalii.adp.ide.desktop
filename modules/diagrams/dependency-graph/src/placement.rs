//! Where a node is drawn, and what a gesture that puts it somewhere writes (the standalone host's
//! `DependencyGraphElementMapper.cs`, `Element` and `Placement`, and
//! `Commands/SetDependencyGraphPlacementCommandHandler.cs` at `8b6ccfc`).
//!
//! A node is drawn with its top-left corner at its `x` as written and at its row's y (`row × 60`). A gesture's point is
//! the horizontal coordinate as sent and the row nearest its y; the placement sets both `x` and `row`, as the standalone
//! command does, and a dependency has no position of its own to set.

use adp_cel::{Map, Value};
use adp_diagram::ModulePlacement;
use adp_disl::DislElement;

use crate::rows;

/// What a gesture that would move a dependency answers: a curve follows its ends.
pub const NOT_MOVABLE: &str = "That element is not something this graph can move.";

/// A node's `x` as the model reads it: 0 for one that does not read as a number.
pub fn x_of(node: &DislElement) -> f64 {
    match node.value_of("x") {
        Value::Double(x) => x,
        Value::Int(x) => x as f64,
        _ => 0.0,
    }
}

/// A node's row as the model reads it: 0 for one that does not read as a whole number.
pub fn row_of(node: &DislElement) -> i64 {
    match node.value_of("row") {
        Value::Int(row) => row,
        _ => 0,
    }
}

/// Where `node` is drawn: its `x` across and its row's y down, in canvas units.
pub fn position(node: &DislElement) -> (f64, f64) {
    (x_of(node), rows::to_y(row_of(node)))
}

/// The placement a point means: the horizontal coordinate as sent, and the row the y snaps to
/// (`DependencyGraphElementMapper.Placement`).
pub fn placed(x: f64, y: f64) -> (f64, i64) {
    (x, rows::to_nearest_row(y))
}

/// What a gesture to (`x`, `y`) writes of `node`, with the bounds its placement constraints judge, or why not.
pub fn placement(node: &DislElement, x: f64, y: f64) -> Result<ModulePlacement, String> {
    if node.is_relation() {
        return Err(NOT_MOVABLE.to_owned());
    }
    let (x, row) = placed(x, y);
    let attributes: Map = [("x", Value::Double(x)), ("row", Value::Int(row))]
        .into_iter()
        .collect();
    let bounds = |x: f64, row: i64| -> Map {
        [("x", Value::Double(x)), ("y", Value::Double(rows::to_y(row)))]
            .into_iter()
            .collect()
    };
    Ok(ModulePlacement {
        attributes,
        old_bounds: bounds(x_of(node), row_of(node)),
        new_bounds: bounds(x, row),
    })
}

/// The `position` an addition at the canvas point (`x`, `y`) reads: the point's x, and the row nearest its y, which
/// the specification's `int(math.round(double(position.y)))` keeps.
pub fn position_at(x: f64, y: f64) -> Map {
    [("x", Value::Double(x)), ("y", Value::Int(rows::to_nearest_row(y)))]
        .into_iter()
        .collect()
}
