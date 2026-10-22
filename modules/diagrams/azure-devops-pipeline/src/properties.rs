//! What a selected stage, job or step shows in the property grid, and the edit a changed row becomes (etalii.adp spec
//! 030, task T078).
//!
//! Ported from the standalone host's `src/diagrams/azure-devops-pipeline/backend/EtAlii.Adp.Diagram.AzureDevOpsPipeline/
//! PipelineContextPropertyProvider.cs` at `8b6ccfc`. Almost everything is shown and not editable, with the reason;
//! only the display name, what an element waits for and whether it runs are written from here. A key the file does
//! not carry has no row, so a key declared empty and one absent stay apart.

use adp_context::{PropertyEditor, PropertyRow};

use crate::commands::{GONE, PipelineEdit};
use crate::edits;
use crate::graph;
use crate::model::{Location, LocationKind, PipelineExecution, PipelineModel, PipelinePool, PoolOrigin, StrategyKind};

pub const DISPLAY_NAME: &str = "azure-pipeline.display-name";
pub const DEPENDS_ON: &str = "azure-pipeline.depends-on";
pub const ENABLED: &str = "azure-pipeline.enabled";

pub const IDENTITY_GROUP: &str = "Identity";
pub const ORDERING_GROUP: &str = "Ordering";
pub const EXECUTION_GROUP: &str = "Execution";

/// The candidate meaning "wait for nothing": `dependsOn: []`.
pub const NOTHING_CANDIDATE: &str = "(nothing)";
/// The candidate meaning "the schema's default": no `dependsOn` key.
pub const DEFAULT_CANDIDATE: &str = "(the default order)";

const EDIT_IT_IN_THE_FILE: &str = "This is edited in the pipeline file. A wrong value here would break the build.";

fn row(id: &str, label: &str, value: &str, editor: PropertyEditor, reason: &str, group: &str) -> PropertyRow {
    PropertyRow {
        read_only_reason: reason.to_owned(),
        group: group.to_owned(),
        ..PropertyRow::new(id, label, editor, value)
    }
}

fn read_only(id: &str, label: &str, value: &str, group: &str) -> PropertyRow {
    row(id, label, value, PropertyEditor::Line, EDIT_IT_IN_THE_FILE, group)
}

fn editable(id: &str, label: &str, value: &str, group: &str) -> PropertyRow {
    row(id, label, value, PropertyEditor::Line, "", group)
}

fn is_expression(value: &str) -> bool {
    value.contains("${{") || value.contains("$(") || value.contains("$[")
}

/// The rows of `element_id`, or none when it names nothing.
pub fn describe(model: &PipelineModel, element_id: &str) -> Vec<PropertyRow> {
    let Some(location) = edits::locate(model, element_id) else {
        return Vec::new();
    };
    let rows = match location.kind {
        LocationKind::Stage => stage_rows(model, &location),
        LocationKind::Job => job_rows(&location),
        LocationKind::Step => step_rows(&location),
    };
    let target = location.target();
    if target.is_editable() {
        rows
    } else {
        let reason = target.read_only_reason();
        rows.into_iter()
            .map(|r| PropertyRow {
                read_only_reason: reason.clone(),
                ..r
            })
            .collect()
    }
}

/// The edit setting row `property_id` of `element_id` to `value` writes (`Change`), or why it is refused. A row that
/// is shown read-only is refused with its reason, as the standalone host's property resolver refused it.
pub fn change(model: &PipelineModel, element_id: &str, property_id: &str, value: &str) -> Result<PipelineEdit, String> {
    let Some(location) = edits::locate(model, element_id) else {
        return Err(GONE.to_owned());
    };
    let rows = describe(model, element_id);
    let Some(shown) = rows.iter().find(|r| r.id == property_id) else {
        return Err(format!("'{property_id}' is not a property of what is selected."));
    };
    if !shown.read_only_reason.is_empty() {
        return Err(shown.read_only_reason.clone());
    }
    let id = location.id().to_owned();
    match property_id {
        DISPLAY_NAME => Ok(PipelineEdit::Rename {
            element_id: id,
            display_name: value.trim().to_owned(),
        }),
        ENABLED => Ok(PipelineEdit::SetEnabled {
            element_id: id,
            enabled: value.trim().eq_ignore_ascii_case("true"),
        }),
        DEPENDS_ON if location.kind != LocationKind::Step => {
            let chosen = value.trim();
            Ok(match chosen {
                DEFAULT_CANDIDATE => PipelineEdit::SetDependencies {
                    element_id: id,
                    depends_on: Vec::new(),
                    declared: false,
                },
                NOTHING_CANDIDATE | "" => PipelineEdit::SetDependencies {
                    element_id: id,
                    depends_on: Vec::new(),
                    declared: true,
                },
                _ => PipelineEdit::SetDependencies {
                    element_id: id,
                    depends_on: vec![chosen.to_owned()],
                    declared: true,
                },
            })
        }
        _ => Err(format!("'{property_id}' cannot be set on this element.")),
    }
}

/// The `dependsOn` row: a picker of what this could wait for, read-only where it waits for several things or there
/// is nothing to pick.
fn depends_on_row(label: &str, current: &[String], candidates: Vec<String>) -> PropertyRow {
    if current.len() > 1 {
        return row(
            DEPENDS_ON,
            label,
            &current.join(", "),
            PropertyEditor::Line,
            "This waits for several things, which is edited in the pipeline file.",
            ORDERING_GROUP,
        );
    }
    let reason = if candidates.is_empty() {
        "There is nothing before this for it to wait for."
    } else {
        ""
    };
    let mut choices = candidates;
    choices.push(NOTHING_CANDIDATE.to_owned());
    choices.push(DEFAULT_CANDIDATE.to_owned());
    PropertyRow {
        candidates: choices,
        ..row(
            DEPENDS_ON,
            label,
            current.first().map_or(NOTHING_CANDIDATE, String::as_str),
            PropertyEditor::Choice,
            reason,
            ORDERING_GROUP,
        )
    }
}

fn add_declared(
    rows: &mut Vec<PropertyRow>,
    execution: &PipelineExecution,
    key: &str,
    id: &str,
    label: &str,
    value: &str,
    group: &str,
) {
    if execution.has(key) {
        rows.push(read_only(id, label, value, group));
    }
}

fn add_if_present(rows: &mut Vec<PropertyRow>, id: &str, label: &str, value: &str, group: &str) {
    if !value.is_empty() {
        rows.push(read_only(id, label, value, group));
    }
}

fn add_pool(rows: &mut Vec<PropertyRow>, pool: &PipelinePool) {
    if !pool.is_declared() {
        return;
    }
    let reason = match pool.origin {
        PoolOrigin::Pipeline => "Set for the whole pipeline, at the top of the file.",
        PoolOrigin::Stage => "Set on the stage, not here.",
        _ => EDIT_IT_IN_THE_FILE,
    };
    rows.push(row(
        "azure-pipeline.pool",
        "Pool",
        pool.label(),
        PropertyEditor::Line,
        reason,
        EXECUTION_GROUP,
    ));
}

fn add_enabled(rows: &mut Vec<PropertyRow>, execution: &PipelineExecution) {
    if !execution.has(PipelineExecution::ENABLED_KEY) {
        return;
    }
    rows.push(row(
        ENABLED,
        "Enabled",
        if execution.is_disabled() { "false" } else { "true" },
        PropertyEditor::Toggle,
        if is_expression(&execution.enabled) {
            "This is decided by an expression when the pipeline runs."
        } else {
            ""
        },
        EXECUTION_GROUP,
    ));
}

fn stage_rows(model: &PipelineModel, location: &Location<'_>) -> Vec<PropertyRow> {
    let stage = location.stage;
    let mut rows = vec![
        read_only("azure-pipeline.name", "Name", &stage.name, IDENTITY_GROUP),
        editable(DISPLAY_NAME, "Display name", &stage.display_name, IDENTITY_GROUP),
        read_only(
            "azure-pipeline.job-count",
            "Jobs",
            &stage.jobs.len().to_string(),
            IDENTITY_GROUP,
        ),
    ];
    let graph = graph::of_stages(model);
    let waits_for: Vec<String> = graph
        .edges
        .iter()
        .filter(|e| e.to_id == stage.id)
        .map(|e| e.from_name.clone())
        .collect();
    let earlier: Vec<String> = model
        .stages
        .iter()
        .take_while(|s| s.id != stage.id)
        .filter(|s| !s.name.is_empty())
        .map(|s| s.name.clone())
        .collect();
    rows.push(depends_on_row(
        if stage.depends_on_declared {
            "Depends on"
        } else {
            "Depends on (by default)"
        },
        &waits_for,
        earlier,
    ));
    add_declared(
        &mut rows,
        &stage.execution,
        PipelineExecution::CONDITION_KEY,
        "azure-pipeline.condition",
        "Condition",
        &stage.execution.condition,
        ORDERING_GROUP,
    );
    if stage.trigger_is_manual {
        rows.push(read_only("azure-pipeline.trigger", "Trigger", "manual", ORDERING_GROUP));
    }
    add_if_present(
        &mut rows,
        "azure-pipeline.is-skippable",
        "Skippable",
        &stage.is_skippable,
        ORDERING_GROUP,
    );
    add_pool(&mut rows, &stage.pool);
    add_enabled(&mut rows, &stage.execution);
    rows
}

fn job_rows(location: &Location<'_>) -> Vec<PropertyRow> {
    let job = location.job.expect("a job's location has its job");
    let mut rows = vec![
        read_only("azure-pipeline.name", "Name", &job.name, IDENTITY_GROUP),
        editable(DISPLAY_NAME, "Display name", &job.display_name, IDENTITY_GROUP),
        read_only(
            "azure-pipeline.kind",
            "Kind",
            if job.is_deployment { "Deployment job" } else { "Job" },
            IDENTITY_GROUP,
        ),
    ];
    let siblings: Vec<String> = location
        .stage
        .jobs
        .iter()
        .filter(|j| j.id != job.id && !j.name.is_empty())
        .map(|j| j.name.clone())
        .collect();
    rows.push(depends_on_row(
        if job.depends_on_declared {
            "Depends on"
        } else {
            "Depends on (by default)"
        },
        &job.depends_on,
        siblings,
    ));
    add_declared(
        &mut rows,
        &job.execution,
        PipelineExecution::CONDITION_KEY,
        "azure-pipeline.condition",
        "Condition",
        &job.execution.condition,
        ORDERING_GROUP,
    );
    add_pool(&mut rows, &job.pool);
    add_if_present(
        &mut rows,
        "azure-pipeline.environment",
        "Environment",
        &job.environment,
        EXECUTION_GROUP,
    );
    if job.strategy.kind != StrategyKind::None {
        rows.push(read_only(
            "azure-pipeline.strategy",
            "Strategy",
            job.strategy.kind.name(),
            EXECUTION_GROUP,
        ));
        if job.strategy.is_multiplied() {
            let runs = if job.strategy.multiplicity_expression.is_empty() {
                format!("{} times", job.strategy.multiplicity)
            } else {
                format!(
                    "{} times - decided when the pipeline runs",
                    job.strategy.multiplicity_expression
                )
            };
            rows.push(read_only("azure-pipeline.multiplicity", "Runs", &runs, EXECUTION_GROUP));
        }
    }
    add_declared(
        &mut rows,
        &job.execution,
        PipelineExecution::TIMEOUT_KEY,
        "azure-pipeline.timeout",
        "Timeout (minutes)",
        &job.execution.timeout_in_minutes,
        EXECUTION_GROUP,
    );
    add_declared(
        &mut rows,
        &job.execution,
        PipelineExecution::CONTINUE_ON_ERROR_KEY,
        "azure-pipeline.continue-on-error",
        "Continue on error",
        &job.execution.continue_on_error,
        EXECUTION_GROUP,
    );
    add_enabled(&mut rows, &job.execution);
    rows
}

fn step_rows(location: &Location<'_>) -> Vec<PropertyRow> {
    let step = location.step.expect("a step's location has its step");
    let mut rows = vec![
        read_only("azure-pipeline.kind", "Kind", step.kind.name(), IDENTITY_GROUP),
        editable(DISPLAY_NAME, "Display name", &step.display_name, IDENTITY_GROUP),
    ];
    if !step.identifier.is_empty() {
        rows.push(row(
            "azure-pipeline.identifier",
            step.kind.name(),
            &step.identifier,
            PropertyEditor::Text,
            EDIT_IT_IN_THE_FILE,
            IDENTITY_GROUP,
        ));
    }
    add_if_present(
        &mut rows,
        "azure-pipeline.hook",
        "Lifecycle hook",
        &step.hook,
        IDENTITY_GROUP,
    );
    add_declared(
        &mut rows,
        &step.execution,
        PipelineExecution::CONDITION_KEY,
        "azure-pipeline.condition",
        "Condition",
        &step.execution.condition,
        ORDERING_GROUP,
    );
    add_declared(
        &mut rows,
        &step.execution,
        PipelineExecution::TIMEOUT_KEY,
        "azure-pipeline.timeout",
        "Timeout (minutes)",
        &step.execution.timeout_in_minutes,
        EXECUTION_GROUP,
    );
    add_declared(
        &mut rows,
        &step.execution,
        PipelineExecution::CONTINUE_ON_ERROR_KEY,
        "azure-pipeline.continue-on-error",
        "Continue on error",
        &step.execution.continue_on_error,
        EXECUTION_GROUP,
    );
    add_enabled(&mut rows, &step.execution);
    rows
}
