//! What Arrange diagram does: Meyer's self-organizing graphs with every source of randomness replaced by a function
//! of the document and the iteration index (the standalone host's `SelfOrganizingLayout.cs` and
//! `SelfOrganizingResult.cs` at `8b6ccfc`; the specification's layout `selfOrganizing`, the plugin
//! `net.etalii.adp.systems.cld`).
//!
//! The variables start on a phyllotaxis spiral in model order, the stimuli are a two-dimensional Halton sequence over
//! the rectangle around it, the best-matching unit's ties go to the earlier variable, and the neighbourhood is
//! measured in hops over the links read as undirected: a variable in another component is not pulled at all. After a
//! fixed 2000 iterations the boxes still overlapping are pushed apart by the shared overlap resolver in at most 200
//! rounds; where they cannot be, or the diagram is past the budget, the arrangement is refused with the size it
//! failed at. The same document gives the same positions in every process. The capability `arrange.selfOrganising`,
//! which this code moved to (etalii.adp spec 076, T033), does it.

use std::collections::HashMap;

use adp_atd_runtime::capabilities::self_organising::{self as capability, ArrangedElement};
use adp_layout::Rect;

use crate::metrics::Metrics;
use crate::reading::{Model, Variable};

pub use capability::{DEFAULT_BUDGET, SEPARATION_ROUNDS, could_not_separate, halton, initial_positions, over_budget};

/// Where each variable was placed, by name, or why the layout declined.
pub type Arrangement = Result<HashMap<String, Rect>, String>;

/// Arranges `model`'s variables, with `budget` and `rounds` as the standalone host's parameters.
pub fn compute(model: &Model, metrics: &Metrics, budget: usize, rounds: usize) -> Arrangement {
    let elements = elements(&model.variables, metrics);
    let relations = relations(model);
    let height = metrics.height();
    let centres = capability::compute(&elements, &relations, height, metrics.separation, budget, rounds)?;
    Ok(model
        .variables
        .iter()
        .zip(elements.iter().zip(centres))
        .map(|(variable, (element, (x, y)))| {
            (
                variable.name.clone(),
                Rect::new(x - element.width / 2.0, y - height / 2.0, element.width, height),
            )
        })
        .collect())
}

/// Pushes overlapping variables apart, every box one height; how many pairs still overlap when the rounds are spent.
pub fn separate(positions: &mut [(f64, f64)], widths: &[f64], height: f64, separation: f64, rounds: usize) -> usize {
    capability::separate(positions, widths, height, separation, rounds)
}

/// Hop counts between every pair of variables over the links read as undirected; -1 where unreachable.
pub fn graph_distances(model: &Model, variables: &[Variable]) -> Vec<Vec<i32>> {
    let elements: Vec<ArrangedElement> = variables
        .iter()
        .map(|v| ArrangedElement {
            id: v.name.clone(),
            width: 0.0,
        })
        .collect();
    capability::graph_distances(&elements, &relations(model))
}

fn elements(variables: &[Variable], metrics: &Metrics) -> Vec<ArrangedElement> {
    variables
        .iter()
        .map(|v| ArrangedElement {
            id: v.name.clone(),
            width: metrics.width_of(v.display()),
        })
        .collect()
}

fn relations(model: &Model) -> Vec<(String, String)> {
    model.links.iter().map(|l| (l.from.clone(), l.to.clone())).collect()
}
