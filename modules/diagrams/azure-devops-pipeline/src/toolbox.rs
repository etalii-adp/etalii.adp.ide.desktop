//! The palette: one entry per thing that can be added, each naming the menu action its drop runs (etalii.adp spec 030,
//! task T078).
//!
//! Ported from the standalone host's `src/diagrams/azure-devops-pipeline/backend/EtAlii.Adp.Diagram.AzureDevOpsPipeline/
//! PipelineToolboxProvider.cs` at `8b6ccfc`: a drop is the same edit as the menu's, so it cannot go where the menu
//! would not.

use adp_diagram::ToolboxItem;

use crate::actions::{ADD_DEPLOYMENT_JOB, ADD_JOB, ADD_STAGE, ADD_STEP};

fn item(id: &str, label: &str, icon: &str, description: &str, action: &str) -> ToolboxItem {
    ToolboxItem {
        id: id.to_owned(),
        label: label.to_owned(),
        icon: icon.to_owned(),
        description: description.to_owned(),
        drop_action_id: action.to_owned(),
    }
}

/// The palette, in the standalone host's order.
pub fn items() -> Vec<ToolboxItem> {
    vec![
        item(
            "azure-pipeline.toolbox.stage",
            "Stage",
            "mdi-layers-plus",
            "Drop on a stage to add another after it. It arrives with a job and a step, so it runs.",
            ADD_STAGE,
        ),
        item(
            "azure-pipeline.toolbox.job",
            "Job",
            "mdi-plus-box-outline",
            "Drop on a stage to add a job to it.",
            ADD_JOB,
        ),
        item(
            "azure-pipeline.toolbox.deployment-job",
            "Deployment job",
            "mdi-rocket-launch-outline",
            "Drop on a stage to add a deployment job, with the environment and strategy one needs.",
            ADD_DEPLOYMENT_JOB,
        ),
        item(
            "azure-pipeline.toolbox.step",
            "Script step",
            "mdi-console-line",
            "Drop on a job to add a script step to it.",
            ADD_STEP,
        ),
    ]
}
