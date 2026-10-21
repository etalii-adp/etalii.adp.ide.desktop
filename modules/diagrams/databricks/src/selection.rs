//! What one of the family's element ids names in a file's readings: the one vocabulary the menus, the property rows
//! and a selection answer from, so the three never disagree about what is selected (etalii.adp spec 030, task T081).
//!
//! Ported from the standalone host's `src/diagrams/databricks/backend/EtAlii.Adp.Diagram.Databricks/
//! DatabricksSelection.cs` and `DatabricksGestures.cs` at `8b6ccfc`, and the core's `GestureIds.cs`: a canvas gesture
//! names a place where nothing is yet as `new:{x},{y}`, and a finished connect gesture as `rel:{from}->{to}`.

use crate::model::{BundleTarget, DatabricksModel, JobModel, JobTask, PipelineLibrary, PipelineModel};

const PLACEMENT_PREFIX: &str = "new:";
const RELATION_PREFIX: &str = "rel:";

/// The id a canvas gesture names a place by (`GestureIds.Placement`).
pub fn placement_id(x: f64, y: f64) -> String {
    format!("{PLACEMENT_PREFIX}{x},{y}")
}

/// The id a finished connect gesture names its two ends by (`GestureIds.Relation`).
pub fn relation_id(from: &str, to: &str) -> String {
    format!("{RELATION_PREFIX}{from}->{to}")
}

/// Whether `id` is a well-formed placement: two numbers after `new:` (`GestureIds.TryParsePlacement`).
pub fn is_placement(id: &str) -> bool {
    let Some(rest) = id.strip_prefix(PLACEMENT_PREFIX) else {
        return false;
    };
    let parts: Vec<&str> = rest.split(',').collect();
    let number = |text: &str| {
        let lower = text.trim().to_ascii_lowercase();
        !lower.contains("inf") && !lower.contains("nan") && lower.parse::<f64>().is_ok()
    };
    parts.len() == 2 && number(parts[0]) && number(parts[1])
}

/// The two ends of a relation gesture, when `id` is one with both (`GestureIds.TryParseRelation`).
pub fn relation_of(id: &str) -> Option<(&str, &str)> {
    let body = id.strip_prefix(RELATION_PREFIX)?;
    let separator = body.find("->")?;
    if separator == 0 || separator + 2 >= body.len() {
        return None;
    }
    Some((&body[..separator], &body[separator + 2..]))
}

/// The task an id names, with the job declaring it.
pub fn task_of<'a>(model: &'a DatabricksModel, id: &str) -> Option<(&'a JobModel, &'a JobTask)> {
    let key = id.strip_prefix("task:")?;
    model
        .jobs
        .iter()
        .find_map(|job| job.tasks.iter().find(|t| t.key == key).map(|task| (job, task)))
}

/// The dependency edge an id names: `edge:{from}->{to}`.
pub fn edge_of(id: &str) -> Option<(&str, &str)> {
    let payload = id.strip_prefix("edge:")?;
    let separator = payload.find("->")?;
    if separator == 0 || separator >= payload.len() - 2 {
        return None;
    }
    Some((&payload[..separator], &payload[separator + 2..]))
}

/// Whether the id is the bundle node itself.
pub fn is_bundle(id: &str) -> bool {
    id == "bundle"
}

/// Whether the id is the pipeline node itself.
pub fn is_pipeline_node(id: &str) -> bool {
    id == "pipeline"
}

/// The library an id names, with the pipeline declaring it.
pub fn library_of<'a>(model: &'a DatabricksModel, id: &str) -> Option<(&'a PipelineModel, &'a PipelineLibrary)> {
    let path = id.strip_prefix("library:")?;
    model.pipelines.iter().find_map(|pipeline| {
        pipeline
            .libraries
            .iter()
            .find(|l| l.path == path)
            .map(|library| (pipeline, library))
    })
}

/// The target frame an id names.
pub fn target_of<'a>(model: &'a DatabricksModel, id: &str) -> Option<&'a BundleTarget> {
    let name = id.strip_prefix("target:")?;
    model.bundle.targets.iter().find(|t| t.name == name)
}

fn pipeline_name(pipeline: &PipelineModel) -> &str {
    if pipeline.name.is_empty() {
        "pipeline"
    } else {
        &pipeline.name
    }
}

/// A bundle's override edge, `override:{target}/{kind}/{key}`, as "target → kind/key", unless that target does not
/// override that resource.
fn override_edge_of(model: &DatabricksModel, id: &str) -> Option<String> {
    let rest = id.strip_prefix("override:")?;
    let target_end = rest.find('/').filter(|e| *e > 0)?;
    let (target_name, resource) = (&rest[..target_end], &rest[target_end + 1..]);
    let target = model.bundle.targets.iter().find(|t| t.name == target_name)?;
    target
        .overrides
        .iter()
        .any(|o| format!("{}/{}", o.kind, o.key) == resource)
        .then(|| format!("{target_name} → {resource}"))
}

/// A pipeline reading's flow edge, `flow:{library}->pipeline` or `flow:pipeline->target`, as "from → to".
fn flow_edge_of(model: &DatabricksModel, id: &str) -> Option<String> {
    let rest = id.strip_prefix("flow:")?;
    let pipeline = model.pipelines.first()?;
    let name = pipeline_name(pipeline);
    if id == "flow:pipeline->target" {
        return Some(format!("{name} → target"));
    }
    let library = rest.strip_suffix("->pipeline")?;
    pipeline
        .libraries
        .iter()
        .any(|l| l.path == library)
        .then(|| format!("{library} → {name}"))
}

/// The text an id reads as, or none when the file holds nothing by that id (`DatabricksSelection.Describe`).
pub fn describe(model: &DatabricksModel, id: &str) -> Option<String> {
    if let Some((_, task)) = task_of(model, id) {
        return Some(task.key.clone());
    }
    if let Some((from, to)) = edge_of(id) {
        let holds = model
            .jobs
            .iter()
            .flat_map(|j| &j.tasks)
            .any(|t| t.key == to && t.depends_on.iter().any(|d| d.task_key == from));
        return holds.then(|| format!("{from} → {to}"));
    }
    if is_bundle(id) {
        return Some(if model.bundle.name.is_empty() {
            "bundle".to_owned()
        } else {
            model.bundle.name.clone()
        });
    }
    if is_pipeline_node(id) {
        return model.pipelines.first().map(|p| pipeline_name(p).to_owned());
    }
    if let Some((_, library)) = library_of(model, id) {
        return Some(library.path.clone());
    }
    if let Some(target) = target_of(model, id) {
        return Some(target.name.clone());
    }
    if let Some(text) = override_edge_of(model, id) {
        return Some(text);
    }
    if let Some(text) = flow_edge_of(model, id) {
        return Some(text);
    }
    if let Some(key) = id.strip_prefix("cluster:") {
        return model
            .jobs
            .iter()
            .flat_map(|j| &j.clusters)
            .any(|c| c.key == key)
            .then(|| key.to_owned());
    }
    if let Some(pair) = id.strip_prefix("resource:") {
        return model
            .bundle
            .resources
            .iter()
            .any(|r| format!("{}/{}", r.kind, r.key) == pair)
            .then(|| pair[pair.find('/').map_or(0, |i| i + 1)..].to_owned());
    }
    None
}
