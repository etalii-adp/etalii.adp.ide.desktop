//! The palette of each of the family's three diagrams: entries that name the add action a drop runs, so a drop and a
//! menu click are the same edit (etalii.adp spec 030, task T081).
//!
//! Ported from the standalone host's `src/diagrams/databricks/backend/EtAlii.Adp.Diagram.Databricks/
//! DatabricksToolboxProvider.cs` at `8b6ccfc`.

use adp_diagram::ToolboxItem;

use crate::actions::{ADD_LIBRARY_PREFIX, ADD_RESOURCE_PREFIX, ADD_TASK_PREFIX};

fn item(id: &str, label: &str, icon: &str, description: &str, action: String) -> ToolboxItem {
    ToolboxItem {
        id: id.to_owned(),
        label: label.to_owned(),
        icon: icon.to_owned(),
        description: description.to_owned(),
        drop_action_id: action,
    }
}

/// The palette of the diagram type `reading` (`bundle`, `job` or `pipeline`), in the standalone host's order.
pub fn items(reading: &str) -> Vec<ToolboxItem> {
    match reading {
        "job" => vec![
            item(
                "databricks.toolbox.notebook-task",
                "Notebook task",
                "mdi-notebook-outline",
                "Runs a notebook. Drop on the canvas where it should sit.",
                format!("{ADD_TASK_PREFIX}notebook"),
            ),
            item(
                "databricks.toolbox.python-task",
                "Python task",
                "mdi-language-python",
                "Runs a Python file. Drop on the canvas where it should sit.",
                format!("{ADD_TASK_PREFIX}python"),
            ),
            item(
                "databricks.toolbox.condition-task",
                "Condition task",
                "mdi-call-split",
                "Branches on a condition; downstream edges follow its true or false outcome.",
                format!("{ADD_TASK_PREFIX}condition"),
            ),
        ],
        "pipeline" => vec![
            item(
                "databricks.toolbox.notebook-library",
                "Notebook library",
                "mdi-notebook-outline",
                "A notebook the pipeline's transformations come from.",
                format!("{ADD_LIBRARY_PREFIX}notebook"),
            ),
            item(
                "databricks.toolbox.file-library",
                "File library",
                "mdi-file-code-outline",
                "A source file the pipeline's transformations come from.",
                format!("{ADD_LIBRARY_PREFIX}file"),
            ),
        ],
        _ => vec![
            item(
                "databricks.toolbox.job-resource",
                "Job",
                "mdi-transit-connection-horizontal",
                "A job resource skeleton, complete with a starter task.",
                format!("{ADD_RESOURCE_PREFIX}jobs"),
            ),
            item(
                "databricks.toolbox.pipeline-resource",
                "Pipeline",
                "mdi-pipe",
                "A pipeline resource skeleton, complete with a starter library.",
                format!("{ADD_RESOURCE_PREFIX}pipelines"),
            ),
        ],
    }
}
