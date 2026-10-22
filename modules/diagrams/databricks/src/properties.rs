//! The property rows of a selected Databricks element, and the edit a changed row becomes (etalii.adp spec 030, task
//! T081).
//!
//! Ported from the standalone host's `src/diagrams/databricks/backend/EtAlii.Adp.Diagram.Databricks/
//! DatabricksContextPropertyProvider.cs` at `8b6ccfc`. Two read-only rows appear on every selection by design: the
//! workspace connection placeholder and the last simulated run, which only the canvas narrates. A row that cannot be
//! edited refuses with its reason before the module is asked, as the standalone host's property resolver refused it.

use adp_context::{PropertyEditor, PropertyRow};

use crate::commands::DatabricksEdit;
use crate::model::DatabricksModel;
use crate::selection;

pub const TASK_KEY: &str = "databricks.task-key";
pub const TASK_TYPE: &str = "databricks.task-type";
pub const TASK_SOURCE: &str = "databricks.task-source";
pub const RUN_IF: &str = "databricks.run-if";
pub const CLUSTER: &str = "databricks.cluster";
pub const BUNDLE_NAME: &str = "databricks.bundle-name";
/// A pipeline scalar: `databricks.pipeline:{key}`.
pub const PIPELINE_SCALAR_PREFIX: &str = "databricks.pipeline:";
pub const WORKSPACE: &str = "databricks.workspace";
pub const LAST_SIMULATED_RUN: &str = "databricks.last-simulated-run";

const IDENTITY: &str = "Identity";
const EXECUTION: &str = "Execution";
const WORKSPACE_GROUP: &str = "Workspace";
const RENAME_VIA_MENU: &str = "Rename through the context menu, so every depends_on reference follows the key.";
const TYPE_IS_THE_FILE: &str = "The type is which *_task mapping the file carries; change it in the file.";
const WORKSPACE_PLACEHOLDER: &str = "Not connected — a placeholder for the real workspace connection.";
const SIMULATION_IS_CLIENT_SIDE: &str = "Simulated runs play on the canvas and never touch the file.";

fn row(id: &str, label: &str, value: &str, group: &str, read_only: &str) -> PropertyRow {
    let mut row = PropertyRow::new(id, label, PropertyEditor::Line, value);
    row.group = group.to_owned();
    row.read_only_reason = read_only.to_owned();
    row
}

/// The two rows every selection wears.
fn placeholders() -> [PropertyRow; 2] {
    [
        row(
            WORKSPACE,
            "Workspace connection",
            "Not connected",
            WORKSPACE_GROUP,
            WORKSPACE_PLACEHOLDER,
        ),
        row(
            LAST_SIMULATED_RUN,
            "Last simulated run",
            "",
            WORKSPACE_GROUP,
            SIMULATION_IS_CLIENT_SIDE,
        ),
    ]
}

/// The rows of the element `id` (`Describe`).
pub fn describe(model: &DatabricksModel, id: &str) -> Vec<PropertyRow> {
    let mut rows = if let Some((_, task)) = selection::task_of(model, id) {
        vec![
            row(TASK_KEY, "Key", &task.key, IDENTITY, RENAME_VIA_MENU),
            row(TASK_TYPE, "Type", &task.task_type, IDENTITY, TYPE_IS_THE_FILE),
            row(TASK_SOURCE, "Source", &task.source, IDENTITY, TYPE_IS_THE_FILE),
            row(RUN_IF, "Run if", &task.run_if, EXECUTION, ""),
            row(CLUSTER, "Cluster", &task.cluster_key, EXECUTION, ""),
        ]
    } else if selection::is_bundle(id) {
        vec![row(BUNDLE_NAME, "Name", &model.bundle.name, IDENTITY, "")]
    } else if let Some(pipeline) = model.pipelines.first().filter(|_| selection::is_pipeline_node(id)) {
        let scalar = |key: &str| format!("{PIPELINE_SCALAR_PREFIX}{key}");
        vec![
            row(&scalar("name"), "Name", &pipeline.name, IDENTITY, ""),
            row(&scalar("catalog"), "Catalog", &pipeline.catalog, IDENTITY, ""),
            row(&scalar("schema"), "Schema", &pipeline.schema, IDENTITY, ""),
            row(&scalar("channel"), "Channel", &pipeline.channel, EXECUTION, ""),
        ]
    } else if let Some(frame) = selection::target_of(model, id) {
        vec![
            row(
                TASK_KEY,
                "Target",
                &frame.name,
                IDENTITY,
                "The target's key is the file's structure.",
            ),
            row(TASK_TYPE, "Mode", &frame.mode, IDENTITY, "Change the mode in the file."),
        ]
    } else {
        return Vec::new();
    };
    rows.extend(placeholders());
    rows
}

/// The edit that sets property `property_id` of `id` to `value` (`ContextPropertyResolver.SetAsync`, `Change`), or why
/// not.
pub fn change(model: &DatabricksModel, id: &str, property_id: &str, value: &str) -> Result<DatabricksEdit, String> {
    let Some(described) = describe(model, id).into_iter().find(|r| r.id == property_id) else {
        return Err(format!("'{property_id}' is not a property of what is selected."));
    };
    if described.is_read_only() {
        return Err(described.read_only_reason);
    }
    let trimmed = value.trim().to_owned();
    let refused = || format!("'{property_id}' cannot be edited on this selection.");
    if let Some(key) = property_id.strip_prefix(PIPELINE_SCALAR_PREFIX)
        && selection::is_pipeline_node(id)
    {
        let pipeline = model.pipelines.first().ok_or_else(refused)?;
        return Ok(DatabricksEdit::SetPipelineScalar {
            pipeline_key: pipeline.key.clone(),
            key: key.to_owned(),
            value: trimmed,
        });
    }
    let task = selection::task_of(model, id);
    match (property_id, task) {
        (RUN_IF, Some((job, task))) => Ok(DatabricksEdit::SetRunIf {
            job_key: job.key.clone(),
            task_key: task.key.clone(),
            run_if: trimmed,
        }),
        (CLUSTER, Some((job, task))) => Ok(DatabricksEdit::SetCluster {
            job_key: job.key.clone(),
            task_key: task.key.clone(),
            cluster_key: trimmed,
        }),
        (BUNDLE_NAME, _) if selection::is_bundle(id) => Ok(DatabricksEdit::SetBundleName { name: trimmed }),
        _ => Err(refused()),
    }
}
