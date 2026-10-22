//! What is wrong with a pipeline, each finding on the element it is about (etalii.adp spec 030, task T078).
//!
//! Ported from the standalone host's `src/diagrams/azure-devops-pipeline/backend/EtAlii.Adp.Diagram.AzureDevOpsPipeline/
//! PipelineRuleSet.cs`, `PipelineRules.cs` and `PipelineValidator.cs` at `8b6ccfc`: a `dependsOn` naming nothing, a
//! cycle, no stage to start with, a stage nothing can reach, an element without a name, a template that cannot be
//! followed; and a text that is not YAML, on the line the reader stopped. The validator reads the text alone, without
//! following any template, as the standalone host's did.

use std::collections::{HashSet, VecDeque};
use std::path::Path;

use adp_diagram::{DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity};

use crate::graph::{self, PipelineGraph};
use crate::lines::LineDocument;
use crate::model::PipelineModel;
use crate::parser;

pub const DANGLING_DEPENDENCY: &str = "azure-pipeline.dangling-dependency";
pub const CYCLE: &str = "azure-pipeline.cycle";
pub const UNREACHABLE: &str = "azure-pipeline.unreachable";
pub const NO_STARTING_STAGE: &str = "azure-pipeline.no-starting-stage";
pub const UNNAMED: &str = "azure-pipeline.unnamed";
pub const TEMPLATE_NOT_FOLLOWED: &str = "azure-pipeline.template-not-followed";
pub const UNPARSEABLE: &str = "azure-pipeline.unparseable";

fn problem(severity: DiagramProblemSeverity, message: String, rule_id: &str, element: Option<&str>) -> DiagramProblem {
    DiagramProblem {
        severity,
        message,
        rule_id: rule_id.to_owned(),
        location: element.map(|id| DiagramProblemLocation::Element(id.to_owned())),
    }
}

/// The last segment of an element id: what a reader calls it.
fn name_of(element_id: &str) -> &str {
    element_id.rsplit('/').next().unwrap_or(element_id)
}

fn contains_expression(path: &str) -> bool {
    path.contains("${{") || path.contains("$(") || path.contains("$[")
}

/// Every rule over `model`, in the standalone host's order.
pub fn judge(model: &PipelineModel) -> Vec<DiagramProblem> {
    if model.stages.is_empty() {
        return Vec::new();
    }
    let mut problems = Vec::new();
    let stages = graph::of_stages(model);
    problems.extend(dangling(&stages, "stage"));
    problems.extend(cycles(&stages));
    problems.extend(no_starting_stage(&stages, model));
    for stage in &model.stages {
        let jobs = graph::of_jobs(stage);
        problems.extend(dangling(&jobs, "job"));
        problems.extend(cycles(&jobs));
    }
    problems.extend(unreachable(&stages, model));
    problems.extend(unnamed(model));
    problems.extend(unfollowable_templates(model));
    problems
}

fn dangling(graph: &PipelineGraph, kind: &str) -> Vec<DiagramProblem> {
    graph
        .broken_edges()
        .map(|edge| {
            problem(
                DiagramProblemSeverity::Error,
                format!(
                    "'{}' waits for a {kind} called '{}', which this pipeline does not have.",
                    name_of(&edge.to_id),
                    edge.from_name
                ),
                DANGLING_DEPENDENCY,
                Some(&edge.to_id),
            )
        })
        .collect()
}

fn cycles(graph: &PipelineGraph) -> Vec<DiagramProblem> {
    graph
        .cycles
        .iter()
        .map(|cycle| {
            let message = if cycle.len() == 1 {
                format!("'{}' waits for itself, so it can never run.", name_of(&cycle[0]))
            } else {
                let names: Vec<&str> = cycle.iter().map(|id| name_of(id)).collect();
                format!(
                    "These wait for each other and so none of them can run: {} -> {}.",
                    names.join(" -> "),
                    name_of(&cycle[0])
                )
            };
            problem(DiagramProblemSeverity::Error, message, CYCLE, Some(&cycle[0]))
        })
        .collect()
}

fn no_starting_stage(graph: &PipelineGraph, model: &PipelineModel) -> Vec<DiagramProblem> {
    if model
        .stages
        .iter()
        .any(|stage| graph.dependencies_of(&stage.id).next().is_none())
    {
        return Vec::new();
    }
    vec![problem(
        DiagramProblemSeverity::Error,
        "Every stage waits for another one, so this pipeline has nothing to start with. At least one stage must depend on nothing.".to_owned(),
        NO_STARTING_STAGE,
        None,
    )]
}

fn unreachable(graph: &PipelineGraph, model: &PipelineModel) -> Vec<DiagramProblem> {
    let mut reachable: HashSet<&str> = HashSet::new();
    let mut pending: VecDeque<&str> = model
        .stages
        .iter()
        .filter(|stage| graph.edges.iter().all(|edge| edge.to_id != stage.id))
        .map(|stage| stage.id.as_str())
        .collect();
    reachable.extend(pending.iter().copied());
    while let Some(next) = pending.pop_front() {
        for dependent in graph.dependents_of(next) {
            if reachable.insert(dependent) {
                pending.push_back(dependent);
            }
        }
    }
    model
        .stages
        .iter()
        .filter(|stage| !reachable.contains(stage.id.as_str()))
        .filter(|stage| !graph.cycles.iter().any(|cycle| cycle.contains(&stage.id)))
        .filter(|stage| graph.broken_edges().all(|edge| edge.to_id != stage.id))
        .map(|stage| {
            problem(
                DiagramProblemSeverity::Warning,
                format!(
                    "Nothing can reach '{}': what it waits for can never finish, so it will never run.",
                    stage.label()
                ),
                UNREACHABLE,
                Some(&stage.id),
            )
        })
        .collect()
}

fn unnamed(model: &PipelineModel) -> Vec<DiagramProblem> {
    let mut problems = Vec::new();
    for stage in model
        .stages
        .iter()
        .filter(|s| !s.is_implicit && s.name.is_empty() && s.display_name.is_empty())
    {
        problems.push(problem(
            DiagramProblemSeverity::Warning,
            "This stage has no name, so nothing can depend on it and it cannot be picked out of a run log.".to_owned(),
            UNNAMED,
            Some(&stage.id),
        ));
    }
    for job in model
        .jobs()
        .filter(|j| !j.is_implicit && j.name.is_empty() && j.display_name.is_empty())
    {
        problems.push(problem(
            DiagramProblemSeverity::Warning,
            "This job has no name, so nothing can depend on it and it cannot be picked out of a run log.".to_owned(),
            UNNAMED,
            Some(&job.id),
        ));
    }
    problems
}

fn unfollowable_templates(model: &PipelineModel) -> Vec<DiagramProblem> {
    model
        .templates
        .iter()
        .filter(|t| !t.resource.is_empty() || contains_expression(&t.path))
        .map(|t| {
            let message = if t.resource.is_empty() {
                format!(
                    "'{}' has a path decided at compile time, so what it contributes is not shown here.",
                    t.reference()
                )
            } else {
                format!(
                    "'{}' comes from another repository, so what it contributes is not shown here.",
                    t.reference()
                )
            };
            problem(
                DiagramProblemSeverity::Warning,
                message,
                TEMPLATE_NOT_FOLLOWED,
                Some(&format!("template:{}", t.id)),
            )
        })
        .collect()
}

/// The findings of a pipeline's text (`PipelineValidator.ValidateAsync`): the reader's sentence on its line for a text
/// that is not YAML, else every rule over what it reads, no template followed.
pub fn validate(text: &str) -> Vec<DiagramProblem> {
    let outcome = parser::outcome(&LineDocument::parse(text), None, Path::new(""));
    if let Some(first) = outcome.problems.first()
        && !outcome.is_usable()
    {
        return vec![DiagramProblem {
            severity: DiagramProblemSeverity::Error,
            message: format!("This is not YAML that can be read: {}", first.message),
            rule_id: UNPARSEABLE.to_owned(),
            location: Some(DiagramProblemLocation::Line(first.line as u32 + 1)),
        }];
    }
    judge(&outcome.model)
}
