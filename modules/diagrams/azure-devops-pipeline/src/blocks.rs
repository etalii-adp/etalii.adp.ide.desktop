//! The YAML a newly added stage, job or step is written as (etalii.adp spec 030, task T078).
//!
//! Ported from the standalone host's `src/diagrams/azure-devops-pipeline/backend/EtAlii.Adp.Diagram.AzureDevOpsPipeline/
//! PipelineBlocks.cs` at `8b6ccfc`. Every block is the smallest thing Azure DevOps will run: a stage with a job, a job
//! with a step, a deployment job with its environment and strategy.

use std::collections::HashSet;

/// The placeholder step a new job gets, so the job is runnable the moment it exists.
pub const PLACEHOLDER_SCRIPT: &str = "echo Add your build steps here";

fn pad(count: usize) -> String {
    " ".repeat(count)
}

/// A stage, with one job and one step inside it; `indent` is the column of its dash.
pub fn stage(name: &str, indent: usize) -> Vec<String> {
    let mut lines = vec![
        format!("{}- stage: {name}", pad(indent)),
        format!("{}jobs:", pad(indent + 2)),
    ];
    lines.extend(job(&format!("{name}Job"), indent + 4));
    lines
}

/// A job, with one step inside it.
pub fn job(name: &str, indent: usize) -> Vec<String> {
    let mut lines = vec![
        format!("{}- job: {name}", pad(indent)),
        format!("{}steps:", pad(indent + 2)),
    ];
    lines.extend(step(PLACEHOLDER_SCRIPT, indent + 4));
    lines
}

/// A script step.
pub fn step(script: &str, indent: usize) -> Vec<String> {
    vec![format!("{}- script: {script}", pad(indent))]
}

/// A deployment job, with the environment and the `runOnce` strategy the schema requires of one.
pub fn deployment_job(name: &str, environment: &str, indent: usize) -> Vec<String> {
    let inner = pad(indent + 2);
    let mut lines = vec![
        format!("{}- deployment: {name}", pad(indent)),
        format!("{inner}environment: {environment}"),
        format!("{inner}strategy:"),
        format!("{inner}  runOnce:"),
        format!("{inner}    deploy:"),
        format!("{inner}      steps:"),
    ];
    lines.extend(step(PLACEHOLDER_SCRIPT, indent + 8));
    lines
}

/// A name nothing in `taken` already uses, case-insensitively: `wanted`, else `wanted` with 2, 3, ... after it.
pub fn unused_name<'a>(wanted: &str, taken: impl IntoIterator<Item = &'a str>) -> String {
    let used: HashSet<String> = taken.into_iter().map(str::to_lowercase).collect();
    if !used.contains(&wanted.to_lowercase()) {
        return wanted.to_owned();
    }
    (2..)
        .map(|suffix| format!("{wanted}{suffix}"))
        .find(|candidate| !used.contains(&candidate.to_lowercase()))
        .expect("some suffix is free")
}
