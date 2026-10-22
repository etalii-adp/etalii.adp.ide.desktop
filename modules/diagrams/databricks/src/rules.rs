//! What is wrong with a Databricks file, each finding on the line it is about, with a sentence saying what to do
//! (etalii.adp spec 030, task T081).
//!
//! Ported from the standalone host's `src/diagrams/databricks/backend/EtAlii.Adp.Diagram.Databricks/
//! DatabricksRuleSet.cs` and `DatabricksValidator.cs` at `8b6ccfc`. Nothing here looks at a workspace, the disk or
//! another file, which is why the override rule stays quiet for a bundle with `include:` globs: a resource the
//! override names may be declared in an included file the rules cannot see. A file that will not parse is one
//! finding on the line the reader stopped on.

use std::collections::{HashMap, HashSet};

use adp_diagram::{DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity};

use crate::lines::{LineDocument, LineRange};
use crate::model::{BundleModel, DatabricksModel, JobModel, JobTask, PipelineModel};
use crate::parser;

pub const TASK_KEY_MISSING: &str = "databricks.task-key-missing";
pub const MISSING_TASK: &str = "databricks.missing-task";
pub const DANGLING_CLUSTER: &str = "databricks.dangling-cluster";
pub const DUPLICATE_TASK_KEY: &str = "databricks.duplicate-task-key";
pub const CYCLE: &str = "databricks.cycle";
pub const NO_DEFAULT_TARGET: &str = "databricks.no-default-target";
pub const MULTIPLE_DEFAULT_TARGETS: &str = "databricks.multiple-default-targets";
pub const OVERRIDE_OF_UNDECLARED: &str = "databricks.override-of-undeclared";
pub const NO_LIBRARIES: &str = "databricks.no-libraries";
pub const SCHEMA_WITHOUT_CATALOG: &str = "databricks.schema-without-catalog";
pub const UNPARSEABLE: &str = "databricks.unparseable";

fn problem(severity: DiagramProblemSeverity, message: String, rule_id: &str, line: usize) -> DiagramProblem {
    DiagramProblem {
        severity,
        message,
        rule_id: rule_id.to_owned(),
        location: Some(DiagramProblemLocation::Line(
            u32::try_from(line + 1).unwrap_or(u32::MAX),
        )),
    }
}

fn error(message: String, rule_id: &str, lines: LineRange) -> DiagramProblem {
    problem(DiagramProblemSeverity::Error, message, rule_id, lines.start)
}

fn warning(message: String, rule_id: &str, line: usize) -> DiagramProblem {
    problem(DiagramProblemSeverity::Warning, message, rule_id, line)
}

/// The findings of a file's text (`DatabricksValidator.ValidateAsync`).
pub fn validate(text: &str) -> Vec<DiagramProblem> {
    let outcome = parser::outcome(&LineDocument::parse(text));
    if let Some(reason) = outcome.unreadable {
        let line = outcome.problems.first().map_or(0, |(line, _)| *line);
        return vec![problem(
            DiagramProblemSeverity::Error,
            format!("This is not configuration that can be read: {reason}"),
            UNPARSEABLE,
            line,
        )];
    }
    judge(&outcome.model)
}

/// Every rule over `model` (`DatabricksRuleSet.Judge`).
pub fn judge(model: &DatabricksModel) -> Vec<DiagramProblem> {
    let mut problems = Vec::new();
    for job in &model.jobs {
        judge_job(job, &mut problems);
    }
    judge_bundle(&model.bundle, &mut problems);
    for pipeline in &model.pipelines {
        judge_pipeline(pipeline, &mut problems);
    }
    problems
}

fn judge_job(job: &JobModel, problems: &mut Vec<DiagramProblem>) {
    let keys: HashSet<&str> = job
        .tasks
        .iter()
        .map(|t| t.key.as_str())
        .filter(|k| !k.is_empty())
        .collect();
    for task in &job.tasks {
        if task.key.is_empty() {
            problems.push(error(
                "A task has no task_key. Give it one, or nothing can depend on it.".to_owned(),
                TASK_KEY_MISSING,
                task.lines,
            ));
        }
        for dependency in task.depends_on.iter().filter(|d| !keys.contains(d.task_key.as_str())) {
            problems.push(error(
                format!(
                    "'{}' depends on '{}', which is not a task in this job. Fix the key or remove the dependency.",
                    task.key, dependency.task_key
                ),
                MISSING_TASK,
                dependency.lines,
            ));
        }
        if !task.cluster_key.is_empty() && job.clusters.iter().all(|c| c.key != task.cluster_key) {
            problems.push(error(
                format!(
                    "'{}' runs on cluster '{}', which this job does not declare. Declare it under job_clusters, or \
                     clear the binding for serverless.",
                    task.key, task.cluster_key
                ),
                DANGLING_CLUSTER,
                task.lines,
            ));
        }
    }
    let mut groups: Vec<(&str, Vec<&JobTask>)> = Vec::new();
    for task in job.tasks.iter().filter(|t| !t.key.is_empty()) {
        match groups.iter_mut().find(|(k, _)| *k == task.key) {
            Some((_, members)) => members.push(task),
            None => groups.push((&task.key, vec![task])),
        }
    }
    for (key, members) in &groups {
        for task in members.iter().skip(1) {
            problems.push(error(
                format!("The task key '{key}' is used more than once. Every task needs its own."),
                DUPLICATE_TASK_KEY,
                task.lines,
            ));
        }
    }
    for member in cycle_members(job) {
        problems.push(error(
            format!(
                "'{}' is part of a dependency circle, so it can never run. Break the circle.",
                member.key
            ),
            CYCLE,
            member.lines,
        ));
    }
}

/// The tasks on a dependency circle: whatever cannot be removed by repeatedly removing tasks with no remaining
/// dependencies. A dependency on a task that does not exist does not hold a task back here.
fn cycle_members(job: &JobModel) -> Vec<&JobTask> {
    let mut order: Vec<&str> = Vec::new();
    let mut remaining: HashMap<&str, &JobTask> = HashMap::new();
    for task in &job.tasks {
        if !remaining.contains_key(task.key.as_str()) {
            remaining.insert(&task.key, task);
            order.push(&task.key);
        }
    }
    let mut removed = true;
    while removed {
        removed = false;
        for key in order.clone() {
            let Some(task) = remaining.get(key) else { continue };
            if task
                .depends_on
                .iter()
                .all(|d| !remaining.contains_key(d.task_key.as_str()))
            {
                remaining.remove(key);
                removed = true;
            }
        }
    }
    let mut members: Vec<&JobTask> = order.iter().filter_map(|k| remaining.get(k).copied()).collect();
    members.sort_by_key(|t| t.lines.start);
    members
}

fn judge_bundle(bundle: &BundleModel, problems: &mut Vec<DiagramProblem>) {
    let defaults: Vec<_> = bundle.targets.iter().filter(|t| t.is_default).collect();
    if let Some(first) = bundle.targets.first()
        && defaults.is_empty()
    {
        problems.push(warning(
            "No target says default: true, so every deploy must name one. Mark one target as the default.".to_owned(),
            NO_DEFAULT_TARGET,
            first.lines.start,
        ));
    }
    for target in defaults.iter().skip(1) {
        problems.push(error(
            format!(
                "'{}' is a second default target; only one target may be the default.",
                target.name
            ),
            MULTIPLE_DEFAULT_TARGETS,
            target.lines,
        ));
    }
    if bundle.includes.is_empty() {
        for target in &bundle.targets {
            for stray in target
                .overrides
                .iter()
                .filter(|o| !bundle.resources.iter().any(|r| r.kind == o.kind && r.key == o.key))
            {
                problems.push(warning(
                    format!(
                        "Target '{}' overrides {} '{}', which this bundle does not declare. Declare the resource or \
                         drop the override.",
                        target.name, stray.kind, stray.key
                    ),
                    OVERRIDE_OF_UNDECLARED,
                    stray.lines.start,
                ));
            }
        }
    }
}

fn named(pipeline: &PipelineModel) -> String {
    if pipeline.name.is_empty() {
        String::new()
    } else {
        format!(" '{}'", pipeline.name)
    }
}

fn judge_pipeline(pipeline: &PipelineModel, problems: &mut Vec<DiagramProblem>) {
    if pipeline.libraries.is_empty() {
        problems.push(error(
            format!(
                "The pipeline{} has no libraries, so it has nothing to run. Add at least one notebook, file or glob.",
                named(pipeline)
            ),
            NO_LIBRARIES,
            pipeline.lines,
        ));
    }
    if !pipeline.schema.is_empty() && pipeline.catalog.is_empty() {
        problems.push(warning(
            format!(
                "The pipeline{} sets a schema but no catalog, so the schema has nowhere to live. Set catalog too.",
                named(pipeline)
            ),
            SCHEMA_WITHOUT_CATALOG,
            pipeline.lines.start,
        ));
    }
}
