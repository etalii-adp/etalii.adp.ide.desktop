//! A pipeline as what the canvas draws: its stages, the jobs of the open ones, the steps of the open jobs, the arrows
//! of what waits for what, and the templates that could not be followed (etalii.adp spec 030, task T078).
//!
//! Ported from the standalone host's `src/diagrams/azure-devops-pipeline/backend/EtAlii.Adp.Diagram.AzureDevOpsPipeline/
//! PipelineElementMapper.cs` and the payload of `api/azure-pipeline.proto` at `8b6ccfc`, with the badges the client's
//! `pipelineIndicators.ts` derived from it. The payload's fields are the element's attributes as text. An element is
//! placed at its centre, in canvas units; `width` and `height` are its size.

use std::collections::HashSet;

use adp_diagram::{DiagramElement, ElementKind};

use crate::graph::{self, PipelineEdge, PipelineGraph};
use crate::layout::{self, Arrangement, PipelineMetrics, Placement};
use crate::model::{PipelineJob, PipelineModel, PipelineStage, PipelineStep, PoolOrigin, StrategyKind};

pub const STAGE_TYPE: &str = "azure-devops/pipeline+stage";
pub const JOB_TYPE: &str = "azure-devops/pipeline+job";
pub const STEP_TYPE: &str = "azure-devops/pipeline+step";
pub const EDGE_TYPE: &str = "azure-devops/pipeline+edge";
pub const TEMPLATE_TYPE: &str = "azure-devops/pipeline+template";

/// The id of the arrow `edge` draws (`PipelineElementMapper.EdgeId`).
pub fn edge_id(edge: &PipelineEdge) -> String {
    if edge.from_id.is_empty() {
        format!("edge:?{}->{}", edge.from_name, edge.to_id)
    } else {
        format!("edge:{}->{}", edge.from_id, edge.to_id)
    }
}

fn flag(value: bool) -> String {
    if value { "true" } else { "false" }.to_owned()
}

fn number(value: f64) -> String {
    format!("{value}")
}

fn node(id: &str, element_type: &str, x: f64, y: f64, width: f64, height: f64) -> DiagramElement {
    let mut element = DiagramElement::node(id, element_type);
    element.x = x + width / 2.0;
    element.y = y + height / 2.0;
    element.placed = true;
    element.attributes.push(("width".to_owned(), number(width)));
    element.attributes.push(("height".to_owned(), number(height)));
    element
}

fn push(element: &mut DiagramElement, name: &str, value: impl Into<String>) {
    element.attributes.push((name.to_owned(), value.into()));
}

/// The badges an element wears, as `glyph|title` lines: the client's `indicatorsOf`.
fn indicators(element: &DiagramElement) -> String {
    let mut out: Vec<String> = Vec::new();
    if element.attribute("enabled") == "false" {
        out.push("⊘|Disabled: this will not run.".to_owned());
    }
    if element.attribute("triggerIsManual") == "true" {
        out.push("▶|Manual trigger: this waits to be started by a person.".to_owned());
    }
    let condition = element.attribute("condition");
    if !condition.is_empty() {
        out.push(format!("?|Runs only when: {condition}"));
    }
    if element.attribute("continuesOnError") == "true" {
        out.push("!|Continues on error: a failure here does not fail the run.".to_owned());
    }
    let multiplicity: i32 = element.attribute("multiplicity").parse().unwrap_or(1);
    if multiplicity > 1 {
        out.push(format!(
            "×{multiplicity}|Runs {multiplicity} times, once per matrix entry."
        ));
    } else if element.attribute("indeterminate") == "true" && element.attribute("kind") != "stage" {
        out.push("×?|How many of these run is decided when the pipeline runs.".to_owned());
    }
    if element.attribute("fromTemplate") == "true" {
        out.push(format!("⇱|From {}: edit it there.", element.attribute("templatePath")));
    }
    out.join("\n")
}

fn finish(mut element: DiagramElement) -> DiagramElement {
    let badges = indicators(&element);
    element.attributes.push(("indicators".to_owned(), badges));
    element
}

/// The elements a view with `expanded` open shows, in the order they are drawn: stages, their jobs and steps and the
/// arrows between the jobs, then the arrows between the stages, then the templates not followed.
pub fn visible(model: &PipelineModel, expanded: &HashSet<String>) -> Vec<DiagramElement> {
    let metrics = PipelineMetrics::default();
    let arrangement = layout::arrange_pipeline(model, &metrics, expanded);
    let stage_graph = graph::of_stages(model);
    let mut elements = Vec::new();
    for stage in &model.stages {
        let Some(placement) = arrangement.of(&stage.id) else {
            continue;
        };
        let open = expanded.contains(&stage.id);
        elements.push(stage_element(stage, placement, &stage_graph, open));
        if !open {
            continue;
        }
        for job in &stage.jobs {
            let Some(job_placement) = arrangement.of(&job.id) else {
                continue;
            };
            let job_open = expanded.contains(&job.id);
            elements.push(job_element(job, stage, job_placement, job_open));
            if !job_open {
                continue;
            }
            elements.extend(steps_of(
                job,
                job_placement.x + metrics.padding,
                job_placement.y + metrics.header_height + metrics.padding,
                &metrics,
            ));
        }
        elements.extend(edges_of(&graph::of_jobs(stage), &arrangement, &stage.id));
    }
    elements.extend(edges_of(&stage_graph, &arrangement, ""));
    elements.extend(unresolved_templates(model, &arrangement, &metrics));
    elements
}

fn steps_of(job: &PipelineJob, x: f64, y: f64, metrics: &PipelineMetrics) -> Vec<DiagramElement> {
    job.steps
        .iter()
        .enumerate()
        .map(|(index, step)| {
            step_element(
                step,
                job,
                x,
                y + index as f64 * (metrics.job_height + metrics.vertical_gap),
                metrics,
            )
        })
        .collect()
}

fn step_element(step: &PipelineStep, job: &PipelineJob, x: f64, y: f64, metrics: &PipelineMetrics) -> DiagramElement {
    let mut element = node(&step.id, STEP_TYPE, x, y, metrics.job_width, metrics.job_height);
    element.parent = Some(job.id.clone());
    push(&mut element, "name", "");
    push(&mut element, "displayName", step.label());
    push(&mut element, "kind", "step");
    push(&mut element, "condition", step.execution.condition.clone());
    push(&mut element, "indeterminate", flag(false));
    push(&mut element, "fromTemplate", flag(step.is_from_template()));
    push(&mut element, "templatePath", step.template.clone());
    push(&mut element, "enabled", flag(!step.execution.is_disabled()));
    push(
        &mut element,
        "continuesOnError",
        flag(step.execution.continues_on_error()),
    );
    push(&mut element, "multiplicity", "1");
    push(&mut element, "hook", step.hook.clone());
    push(&mut element, "firstLine", (step.lines.start + 1).to_string());
    push(&mut element, "lastLine", (step.lines.end + 1).to_string());
    finish(element)
}

fn stage_element(stage: &PipelineStage, placement: &Placement, graph: &PipelineGraph, open: bool) -> DiagramElement {
    let indeterminate = !stage.gate.is_empty() || graph.edges.iter().any(|e| e.to_id == stage.id && e.is_broken);
    let mut element = node(
        &stage.id,
        STAGE_TYPE,
        placement.x,
        placement.y,
        placement.size.width,
        placement.size.height,
    );
    push(&mut element, "name", stage.name.clone());
    push(&mut element, "displayName", stage.label());
    push(&mut element, "kind", "stage");
    push(&mut element, "condition", stage.execution.condition.clone());
    push(&mut element, "indeterminate", flag(indeterminate));
    push(&mut element, "fromTemplate", flag(stage.is_from_template()));
    push(&mut element, "templatePath", stage.template.clone());
    push(&mut element, "enabled", flag(!stage.execution.is_disabled()));
    push(
        &mut element,
        "continuesOnError",
        flag(stage.execution.continues_on_error()),
    );
    push(&mut element, "triggerIsManual", flag(stage.trigger_is_manual));
    push(&mut element, "multiplicity", "1");
    push(&mut element, "pool", stage.pool.label());
    push(
        &mut element,
        "poolInherited",
        flag(stage.pool.origin == PoolOrigin::Pipeline),
    );
    push(&mut element, "jobCount", stage.jobs.len().to_string());
    push(&mut element, "expanded", flag(open));
    let (first, last) = if stage.is_implicit {
        (0, 0)
    } else {
        (stage.lines.start + 1, stage.lines.end + 1)
    };
    push(&mut element, "firstLine", first.to_string());
    push(&mut element, "lastLine", last.to_string());
    finish(element)
}

fn job_element(job: &PipelineJob, stage: &PipelineStage, placement: &Placement, open: bool) -> DiagramElement {
    let mut element = node(
        &job.id,
        JOB_TYPE,
        placement.x,
        placement.y,
        placement.size.width,
        placement.size.height,
    );
    element.parent = Some(stage.id.clone());
    push(&mut element, "name", job.name.clone());
    push(&mut element, "displayName", job.label());
    push(
        &mut element,
        "kind",
        if job.is_deployment { "deploymentJob" } else { "job" },
    );
    push(&mut element, "condition", job.execution.condition.clone());
    push(
        &mut element,
        "indeterminate",
        flag(!job.gate.is_empty() || !job.strategy.multiplicity_expression.is_empty()),
    );
    push(&mut element, "fromTemplate", flag(job.is_from_template()));
    push(&mut element, "templatePath", job.template.clone());
    push(&mut element, "enabled", flag(!job.execution.is_disabled()));
    push(
        &mut element,
        "continuesOnError",
        flag(job.execution.continues_on_error()),
    );
    push(&mut element, "multiplicity", job.strategy.multiplicity.to_string());
    push(&mut element, "pool", job.pool.label());
    push(&mut element, "poolInherited", flag(job.pool.is_inherited_by_a_job()));
    push(&mut element, "environment", job.environment.clone());
    push(
        &mut element,
        "strategy",
        if job.strategy.kind == StrategyKind::None {
            ""
        } else {
            job.strategy.kind.name()
        },
    );
    push(&mut element, "stepCount", job.steps.len().to_string());
    push(&mut element, "expanded", flag(open));
    let (first, last) = if job.is_implicit {
        (0, 0)
    } else {
        (job.lines.start + 1, job.lines.end + 1)
    };
    push(&mut element, "firstLine", first.to_string());
    push(&mut element, "lastLine", last.to_string());
    finish(element)
}

fn edges_of(graph: &PipelineGraph, arrangement: &Arrangement, parent_id: &str) -> Vec<DiagramElement> {
    graph
        .edges
        .iter()
        .map(|edge| {
            let at = arrangement.of(&edge.to_id);
            let mut element = DiagramElement::node(edge_id(edge), EDGE_TYPE);
            element.kind = ElementKind::Relation;
            element.x = at.map_or(0.0, |p| p.x);
            element.y = at.map_or(0.0, |p| p.y);
            element.source = (!edge.from_id.is_empty()).then(|| edge.from_id.clone());
            element.target = Some(edge.to_id.clone());
            element.parent = (!parent_id.is_empty()).then(|| parent_id.to_owned());
            push(&mut element, "name", edge.from_name.clone());
            push(&mut element, "kind", "edge");
            push(&mut element, "condition", edge.condition_text.clone());
            push(&mut element, "edgeCondition", edge.condition.name());
            push(&mut element, "implicitDependency", flag(edge.is_implicit));
            push(&mut element, "broken", flag(edge.is_broken));
            element
        })
        .collect()
}

fn unresolved_templates(
    model: &PipelineModel,
    arrangement: &Arrangement,
    metrics: &PipelineMetrics,
) -> Vec<DiagramElement> {
    let below = arrangement.size.height + metrics.vertical_gap;
    model
        .unresolved
        .iter()
        .enumerate()
        .map(|(index, unresolved)| {
            let reference = &unresolved.reference;
            let x = arrangement.of(&reference.owner_id).map_or(0.0, |p| p.x);
            let y = below + index as f64 * (metrics.stage_height + metrics.vertical_gap);
            let mut element = node(
                &format!("template:{}", reference.id),
                TEMPLATE_TYPE,
                x,
                y,
                metrics.stage_width,
                metrics.stage_height,
            );
            element.parent = (!reference.owner_id.is_empty()).then(|| reference.owner_id.clone());
            push(&mut element, "name", reference.path.clone());
            push(&mut element, "displayName", reference.reference());
            push(&mut element, "kind", "template");
            push(&mut element, "indeterminate", flag(true));
            push(&mut element, "enabled", flag(true));
            push(&mut element, "multiplicity", "1");
            push(&mut element, "reference", reference.reference());
            push(&mut element, "unresolvedReason", unresolved.explanation());
            push(&mut element, "firstLine", (reference.lines.start + 1).to_string());
            push(&mut element, "lastLine", (reference.lines.end + 1).to_string());
            finish(element)
        })
        .collect()
}
