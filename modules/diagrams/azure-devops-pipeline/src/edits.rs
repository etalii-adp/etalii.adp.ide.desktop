//! What every pipeline edit asks first: where the element is, whether it may be touched, and whether the edit would
//! leave a pipeline that cannot run (etalii.adp spec 030, task T078).
//!
//! Ported from the standalone host's `src/diagrams/azure-devops-pipeline/backend/EtAlii.Adp.Diagram.AzureDevOpsPipeline/
//! PipelineEdits.cs` at `8b6ccfc`.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::graph;
use crate::model::{Location, LocationKind, PipelineModel};

/// The element an id names, or none when the model holds no such thing.
pub fn locate<'a>(model: &'a PipelineModel, element_id: &str) -> Option<Location<'a>> {
    for stage in &model.stages {
        if stage.id == element_id {
            return Some(Location {
                kind: LocationKind::Stage,
                stage,
                job: None,
                step: None,
            });
        }
        for job in &stage.jobs {
            if job.id == element_id {
                return Some(Location {
                    kind: LocationKind::Job,
                    stage,
                    job: Some(job),
                    step: None,
                });
            }
            if let Some(step) = job.steps.iter().find(|s| s.id == element_id) {
                return Some(Location {
                    kind: LocationKind::Step,
                    stage,
                    job: Some(job),
                    step: Some(step),
                });
            }
        }
    }
    None
}

/// Why this element may not be edited here, or empty when it may.
pub fn refusal_for(location: &Location<'_>) -> String {
    let target = location.target();
    if target.is_editable() {
        String::new()
    } else {
        target.read_only_reason()
    }
}

/// Whether making `element_id` depend on `depends_on` would close a loop.
pub fn would_cycle(model: &PipelineModel, element_id: &str, depends_on: &[String]) -> bool {
    let Some(location) = locate(model, element_id) else {
        return false;
    };
    let graph = if location.kind == LocationKind::Stage {
        graph::of_stages(model)
    } else {
        graph::of_jobs(location.stage)
    };
    let mut downstream: HashSet<String> = HashSet::new();
    let mut pending: VecDeque<String> = VecDeque::from([element_id.to_owned()]);
    while let Some(next) = pending.pop_front() {
        for dependent in graph.dependents_of(&next) {
            if downstream.insert(dependent.to_owned()) {
                pending.push_back(dependent.to_owned());
            }
        }
    }
    let by_name = names_to_ids(model, &location);
    depends_on.iter().any(|name| {
        by_name
            .get(&name.to_lowercase())
            .is_some_and(|id| downstream.contains(id) || id == element_id)
    })
}

/// Whether the pipeline would be left without a stage that waits for nothing, were `stage_id` to wait for
/// `depends_on`.
pub fn would_leave_no_starting_stage(model: &PipelineModel, stage_id: &str, depends_on: &[String]) -> bool {
    if depends_on.is_empty() {
        return false;
    }
    let graph = graph::of_stages(model);
    !model
        .stages
        .iter()
        .any(|stage| stage.id != stage_id && graph.dependencies_of(&stage.id).next().is_none())
}

/// The names an element at this level may depend on, lower case, mapped to their ids; the first of a name wins.
pub fn names_to_ids(model: &PipelineModel, location: &Location<'_>) -> HashMap<String, String> {
    let mut names = HashMap::new();
    if location.kind == LocationKind::Stage {
        for stage in model.stages.iter().filter(|s| !s.name.is_empty()) {
            names
                .entry(stage.name.to_lowercase())
                .or_insert_with(|| stage.id.clone());
        }
    } else {
        for job in location.stage.jobs.iter().filter(|j| !j.name.is_empty()) {
            names.entry(job.name.to_lowercase()).or_insert_with(|| job.id.clone());
        }
    }
    names
}
