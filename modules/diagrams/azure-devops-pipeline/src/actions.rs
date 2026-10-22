//! What can be done to a stage, a job or a step, and the edit each action becomes (etalii.adp spec 030, task T078).
//!
//! Ported from the standalone host's `src/diagrams/azure-devops-pipeline/backend/EtAlii.Adp.Diagram.AzureDevOpsPipeline/
//! PipelineContextActionProvider.cs` at `8b6ccfc`, over its `DiagramActionProvider`. An action that would fail is not
//! offered: a stage that is the only one, a job that is its stage's last and a step whose job would be left empty
//! offer no Remove, and an element whose text is somewhere else offers nothing. Opening a stage or a job is the view's
//! own state, never written; a rename asks for the new text first; everything else runs at once.

use adp_context::{Action, ActionGroup, Prompt, Shortcut};

use crate::commands::{AddKind, PipelineEdit};
use crate::edits;
use crate::model::{Location, LocationKind, PipelineModel};

pub const RENAME: &str = "azure-pipeline.rename";
pub const TOGGLE_ENABLED: &str = "azure-pipeline.toggle-enabled";
pub const ADD_STAGE: &str = "azure-pipeline.add-stage";
pub const ADD_JOB: &str = "azure-pipeline.add-job";
pub const ADD_DEPLOYMENT_JOB: &str = "azure-pipeline.add-deployment-job";
pub const ADD_STEP: &str = "azure-pipeline.add-step";
pub const REMOVE: &str = "azure-pipeline.remove";
pub const MOVE_STEP_UP: &str = "azure-pipeline.move-step-up";
pub const MOVE_STEP_DOWN: &str = "azure-pipeline.move-step-down";
pub const CLEAR_DEPENDENCIES: &str = "azure-pipeline.clear-dependencies";
pub const TOGGLE_STAGE: &str = "azure-pipeline.toggle-stage";
pub const TOGGLE_JOB: &str = "azure-pipeline.toggle-job";

fn action(id: &str, label: &str, icon: &str) -> Action {
    Action::new(id, label).with_icon(icon)
}

/// Whether removing this element would leave a pipeline that still runs.
fn removable(model: &PipelineModel, location: &Location<'_>) -> bool {
    match location.kind {
        LocationKind::Stage => model.stages.iter().filter(|s| !s.is_implicit).count() > 1,
        LocationKind::Job => location.stage.jobs.len() > 1,
        LocationKind::Step => location.job.is_some_and(|j| j.steps.len() > 1),
    }
}

fn depends_on_declared(location: &Location<'_>) -> bool {
    match location.kind {
        LocationKind::Stage => location.stage.depends_on_declared,
        _ => location.job.is_some_and(|j| j.depends_on_declared),
    }
}

/// The position of a step in its job.
fn index_of(location: &Location<'_>) -> i64 {
    let (Some(job), Some(step)) = (location.job, location.step) else {
        return -1;
    };
    job.steps.iter().position(|s| s.id == step.id).map_or(-1, |i| i as i64)
}

/// What opening a stage or a job shows it by: the id kept open, the action, and its two labels; none where there is
/// nothing inside to show.
pub fn opener_of(location: &Location<'_>) -> Option<(String, &'static str, &'static str, &'static str)> {
    match location.kind {
        LocationKind::Stage if !location.stage.jobs.is_empty() => {
            Some((location.stage.id.clone(), TOGGLE_STAGE, "Show jobs", "Hide jobs"))
        }
        LocationKind::Job => {
            let job = location.job?;
            (!job.steps.is_empty()).then(|| (job.id.clone(), TOGGLE_JOB, "Show steps", "Hide steps"))
        }
        _ => None,
    }
}

/// The menu of the element `element_id`, with `is_open` saying whether an element is open in the view.
pub fn discover(model: &PipelineModel, element_id: &str, is_open: &dyn Fn(&str) -> bool) -> Vec<ActionGroup> {
    let Some(location) = edits::locate(model, element_id) else {
        return Vec::new();
    };
    if !location.target().is_editable() {
        return Vec::new();
    }
    let disabled = location.execution().is_disabled();
    let mut edits_group = vec![
        action(RENAME, "Rename…", "mdi-pencil-outline").with_shortcut(Shortcut::key("F2")),
        action(
            TOGGLE_ENABLED,
            if disabled { "Enable" } else { "Disable" },
            if disabled {
                "mdi-play-circle-outline"
            } else {
                "mdi-pause-circle-outline"
            },
        ),
    ];
    if location.kind != LocationKind::Step && depends_on_declared(&location) {
        edits_group.push(action(
            CLEAR_DEPENDENCIES,
            "Use the default order",
            "mdi-arrow-decision-outline",
        ));
    }
    if removable(model, &location) {
        edits_group.push(action(REMOVE, "Remove", "mdi-delete-outline").with_shortcut(Shortcut::key("Delete")));
    }
    if let Some((id, toggle, show, hide)) = opener_of(&location) {
        let open = is_open(&id);
        edits_group.insert(
            0,
            action(
                toggle,
                if open { hide } else { show },
                if open {
                    "mdi-unfold-less-horizontal"
                } else {
                    "mdi-unfold-more-horizontal"
                },
            )
            .with_shortcut(Shortcut::key(" ")),
        );
    }
    let additions = match location.kind {
        LocationKind::Stage => vec![
            action(ADD_STAGE, "Add stage", "mdi-layers-plus"),
            action(ADD_JOB, "Add job", "mdi-plus-box-outline"),
            action(ADD_DEPLOYMENT_JOB, "Add deployment job", "mdi-rocket-launch-outline"),
        ],
        LocationKind::Job => vec![action(ADD_STEP, "Add step", "mdi-plus-box-outline")],
        LocationKind::Step => Vec::new(),
    };
    let mut ordering = Vec::new();
    if location.kind == LocationKind::Step
        && let Some(job) = location.job
        && !job.steps.iter().any(|s| s.is_from_template())
    {
        let index = index_of(&location);
        if index > 0 {
            ordering.push(
                action(MOVE_STEP_UP, "Move up", "mdi-arrow-up").with_shortcut(Shortcut::key("ArrowUp").with_alt()),
            );
        }
        if index < job.steps.len() as i64 - 1 {
            ordering.push(
                action(MOVE_STEP_DOWN, "Move down", "mdi-arrow-down")
                    .with_shortcut(Shortcut::key("ArrowDown").with_alt()),
            );
        }
    }
    [edits_group, additions, ordering]
        .into_iter()
        .filter(|actions| !actions.is_empty())
        .map(|actions| ActionGroup { actions })
        .collect()
}

/// What running an action asks for.
#[derive(Debug, Clone, PartialEq)]
pub enum Run {
    /// Open or close the element with this id in the view.
    Toggle(String),
    /// Ask first.
    Ask(Prompt),
    /// The edit to run.
    Edit(PipelineEdit),
}

/// What action `action_id` on `element_id` does (`RunAsync`).
pub fn run(model: &PipelineModel, element_id: &str, action_id: &str) -> Result<Run, String> {
    let Some(location) = edits::locate(model, element_id) else {
        return Err(crate::commands::GONE.to_owned());
    };
    if action_id == TOGGLE_STAGE {
        return Ok(Run::Toggle(location.stage.id.clone()));
    }
    if action_id == TOGGLE_JOB {
        return location
            .job
            .map(|j| Run::Toggle(j.id.clone()))
            .ok_or_else(|| format!("'{action_id}' does not apply to this selection."));
    }
    if action_id == RENAME {
        return Ok(Run::Ask(Prompt::Input {
            title: "Rename".to_owned(),
            icon: "mdi-pencil-outline".to_owned(),
            label: "Display name".to_owned(),
            initial_value: location.display_name().to_owned(),
            confirm_label: "Rename".to_owned(),
        }));
    }
    edit_for(&location, action_id, "")
        .map(Run::Edit)
        .ok_or_else(|| format!("'{action_id}' does not apply to this selection."))
}

/// The edit an answer of `value` to action `action_id` on `element_id` commits (`CommitCommand`).
pub fn commit(model: &PipelineModel, element_id: &str, action_id: &str, value: &str) -> Result<PipelineEdit, String> {
    edits::locate(model, element_id)
        .and_then(|location| edit_for(&location, action_id, value))
        .ok_or_else(|| format!("'{action_id}' does not apply to this selection."))
}

fn edit_for(location: &Location<'_>, action_id: &str, value: &str) -> Option<PipelineEdit> {
    let id = location.id().to_owned();
    Some(match action_id {
        RENAME => PipelineEdit::Rename {
            element_id: id,
            display_name: value.to_owned(),
        },
        TOGGLE_ENABLED => PipelineEdit::SetEnabled {
            element_id: id,
            enabled: location.execution().is_disabled(),
        },
        CLEAR_DEPENDENCIES => PipelineEdit::SetDependencies {
            element_id: id,
            depends_on: Vec::new(),
            declared: false,
        },
        REMOVE => PipelineEdit::Remove { element_id: id },
        ADD_STAGE => PipelineEdit::Add {
            kind: AddKind::Stage,
            parent_id: String::new(),
            name: String::new(),
        },
        ADD_JOB => PipelineEdit::Add {
            kind: AddKind::Job,
            parent_id: id,
            name: String::new(),
        },
        ADD_DEPLOYMENT_JOB => PipelineEdit::Add {
            kind: AddKind::DeploymentJob,
            parent_id: id,
            name: String::new(),
        },
        ADD_STEP => PipelineEdit::Add {
            kind: AddKind::Step,
            parent_id: id,
            name: String::new(),
        },
        MOVE_STEP_UP => PipelineEdit::MoveStep {
            step_id: id,
            to_index: index_of(location) - 1,
        },
        MOVE_STEP_DOWN => PipelineEdit::MoveStep {
            step_id: id,
            to_index: index_of(location) + 1,
        },
        _ => return None,
    })
}
