//! `arrange.selfOrganising` from the causal loop diagram's Arrange diagram
//! (`modules/diagrams/causal-loop-diagram/src/self_organizing.rs`, Meyer's self-organizing graphs made deterministic),
//! on every diagram of its parity scripts: the elements with their sizes and the relations between them in, and a
//! `move` of each element to the centre the arrangement gives it out, or the refusal as a finding. One more fixture
//! records the refusal of a diagram past the budget, on the first diagram with a budget one short of its size.

use std::path::{Path, PathBuf};

use adp_diagram_causal_loop::metrics::Metrics;
use adp_diagram_causal_loop::reading::Model;
use adp_diagram_causal_loop::self_organizing::{self, DEFAULT_BUDGET, SEPARATION_ROUNDS};
use serde_json::{Value as Json, json};

use super::documents::{opened, round2};
use super::{Recorded, write_fixture};

/// Records the fixtures of `arrange.selfOrganising` into `cases`.
pub fn record(cases: &Path) -> Result<Vec<PathBuf>, String> {
    let metrics = Metrics::default();
    let mut written = Vec::new();
    let documents = opened(adp_diagram_causal_loop::ORIGIN, false)?;
    for document in &documents {
        let model = Model::of(&document.diagram);
        written.push(fixture(
            cases,
            &document.name,
            &document.source,
            &model,
            &metrics,
            DEFAULT_BUDGET,
        )?);
    }
    if let Some(first) = documents.first() {
        let model = Model::of(&first.diagram);
        let budget = model.variables.len().saturating_sub(1);
        written.push(fixture(
            cases,
            &format!("{}-over-budget", first.name),
            &first.source,
            &model,
            &metrics,
            budget,
        )?);
    }
    Ok(written)
}

fn fixture(
    cases: &Path,
    case: &str,
    source: &str,
    model: &Model,
    metrics: &Metrics,
    budget: usize,
) -> Result<PathBuf, String> {
    let height = metrics.height();
    let elements: Vec<Json> = model
        .variables
        .iter()
        .map(|variable| {
            json!({
                "id": variable.element_id(), "width": round2(metrics.width_of(variable.display())), "height": round2(height),
            })
        })
        .collect();
    let relations: Vec<Json> = model
        .links
        .iter()
        .map(|link| json!({"source": format!("variable:{}", link.from), "target": format!("variable:{}", link.to)}))
        .collect();
    let output = match self_organizing::compute(model, metrics, budget, SEPARATION_ROUNDS) {
        Ok(boxes) => {
            let actions: Vec<Json> = model
                .variables
                .iter()
                .filter_map(|variable| {
                    let rect = boxes.get(&variable.name)?;
                    Some(json!({
                        "kind": "move", "target": variable.element_id(),
                        "x": round2(rect.center_x()), "y": round2(rect.center_y()),
                    }))
                })
                .collect();
            json!({"actions": actions})
        }
        Err(refusal) => json!({"findings": [{"severity": "error", "message": refusal}]}),
    };
    let from = Recorded {
        capability: "arrange.selfOrganising",
        code: "modules/diagrams/causal-loop-diagram/src/self_organizing.rs",
        source,
        note: "Each element's id is the causal loop diagram's `variable:<name>`; a relation names the ids of its ends.",
    };
    write_fixture(
        cases,
        case,
        &from,
        json!({"elements": elements, "relations": relations}),
        json!({"budget": budget, "separation": metrics.separation, "rounds": SEPARATION_ROUNDS}),
        output,
    )
}
