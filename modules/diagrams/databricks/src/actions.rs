//! What can be done to a selected Databricks element, and the edit each action becomes (etalii.adp spec 030, task
//! T081).
//!
//! Ported from the standalone host's `src/diagrams/databricks/backend/EtAlii.Adp.Diagram.Databricks/
//! DatabricksContextActionProvider.cs` at `8b6ccfc`, over its `DiagramActionProvider`. A drop on the canvas names its
//! place (`new:{x},{y}`) and a finished connect gesture its two ends (`rel:{from}->{to}`). The simulated actions run,
//! deploy and update play on the canvas in the standalone host and never reach a file or the history; one that
//! reaches the module anyway completes and does nothing.

use adp_context::{Action, ActionGroup, Prompt, Shortcut};

use crate::commands::DatabricksEdit;
use crate::model::{DatabricksModel, JobModel, JobTask};
use crate::selection;
use crate::writer;

pub const RENAME_TASK: &str = "databricks.rename-task";
pub const SET_RUN_IF: &str = "databricks.set-run-if";
pub const ASSIGN_CLUSTER: &str = "databricks.assign-cluster";
pub const REMOVE_TASK: &str = "databricks.remove-task";
/// The prefix of a per-dependency disconnect: `databricks.disconnect:{fromKey}`.
pub const DISCONNECT_PREFIX: &str = "databricks.disconnect:";
pub const REMOVE_EDGE: &str = "databricks.remove-edge";
pub const CONNECT: &str = "databricks.connect";
/// Add a task at a placement: `databricks.add-task:{type}`.
pub const ADD_TASK_PREFIX: &str = "databricks.add-task:";
pub const RENAME_BUNDLE: &str = "databricks.rename-bundle";
/// Add a resource skeleton: `databricks.add-resource:{kind}`.
pub const ADD_RESOURCE_PREFIX: &str = "databricks.add-resource:";
/// Add a library to the pipeline: `databricks.add-library:{kind}`.
pub const ADD_LIBRARY_PREFIX: &str = "databricks.add-library:";
pub const REMOVE_LIBRARY: &str = "databricks.remove-library";
/// The marker every simulated action's id carries.
pub const SIMULATED_MARKER: &str = ".simulated.";
pub const SIMULATED_RUN: &str = "databricks.simulated.run-job";
pub const SIMULATED_DEPLOY: &str = "databricks.simulated.deploy";
pub const SIMULATED_UPDATE: &str = "databricks.simulated.pipeline-update";

/// What running an action does.
#[derive(Debug, Clone, PartialEq)]
pub enum Run {
    /// Nothing more: a simulation, or an action with nothing to do.
    Done,
    /// It asks first: a value, or to confirm what goes with it.
    Ask(Prompt),
    /// It edits the file at once.
    Edit(DatabricksEdit),
}

fn action(id: &str, label: &str, icon: &str) -> Action {
    Action::new(id, label).with_icon(icon)
}

fn group(actions: Vec<Action>) -> ActionGroup {
    ActionGroup { actions }
}

fn for_task(task: &JobTask) -> Vec<ActionGroup> {
    let mut edits = vec![
        action(RENAME_TASK, "Rename…", "mdi-pencil-outline").with_shortcut(Shortcut::key("F2")),
        action(SET_RUN_IF, "Set run if…", "mdi-filter-outline"),
        action(ASSIGN_CLUSTER, "Assign cluster…", "mdi-server"),
    ];
    // One disconnect per upstream edge, so the menu names what each one severs.
    edits.extend(task.depends_on.iter().map(|d| {
        action(
            &format!("{DISCONNECT_PREFIX}{}", d.task_key),
            &format!("Disconnect from '{}'", d.task_key),
            "mdi-vector-polyline-remove",
        )
    }));
    edits.push(action(REMOVE_TASK, "Remove", "mdi-delete-outline").with_shortcut(Shortcut::key("Delete")));
    vec![
        group(edits),
        group(vec![action(SIMULATED_RUN, "Run job (simulated)", "mdi-play-outline")]),
    ]
}

/// The menu of the element, placement or gesture `id` (`Discover`).
pub fn discover(model: &DatabricksModel, id: &str) -> Vec<ActionGroup> {
    if let Some((_, task)) = selection::task_of(model, id) {
        return for_task(task);
    }
    if selection::edge_of(id).is_some() {
        return vec![group(vec![
            action(REMOVE_EDGE, "Remove dependency", "mdi-vector-polyline-remove")
                .with_shortcut(Shortcut::key("Delete")),
        ])];
    }
    if selection::is_bundle(id) {
        return vec![
            group(vec![
                action(RENAME_BUNDLE, "Rename…", "mdi-pencil-outline").with_shortcut(Shortcut::key("F2")),
            ]),
            group(vec![
                action(
                    &format!("{ADD_RESOURCE_PREFIX}jobs"),
                    "Add job resource…",
                    "mdi-transit-connection-horizontal",
                ),
                action(
                    &format!("{ADD_RESOURCE_PREFIX}pipelines"),
                    "Add pipeline resource…",
                    "mdi-pipe",
                ),
            ]),
        ];
    }
    if selection::target_of(model, id).is_some() {
        return vec![group(vec![action(
            SIMULATED_DEPLOY,
            "Deploy here (simulated)",
            "mdi-rocket-launch-outline",
        )])];
    }
    if selection::is_pipeline_node(id) {
        return vec![
            group(vec![
                action(
                    &format!("{ADD_LIBRARY_PREFIX}notebook"),
                    "Add notebook library…",
                    "mdi-notebook-outline",
                ),
                action(
                    &format!("{ADD_LIBRARY_PREFIX}file"),
                    "Add file library…",
                    "mdi-file-code-outline",
                ),
            ]),
            group(vec![action(
                SIMULATED_UPDATE,
                "Start update (simulated)",
                "mdi-play-outline",
            )]),
        ];
    }
    if selection::library_of(model, id).is_some() {
        return vec![group(vec![
            action(REMOVE_LIBRARY, "Remove library", "mdi-delete-outline").with_shortcut(Shortcut::key("Delete")),
        ])];
    }
    if selection::is_placement(id) {
        // Empty canvas offers what can be dropped or added there.
        return vec![group(vec![
            action(
                &format!("{ADD_TASK_PREFIX}notebook"),
                "Add notebook task here",
                "mdi-notebook-outline",
            ),
            action(
                &format!("{ADD_TASK_PREFIX}python"),
                "Add Python task here",
                "mdi-language-python",
            ),
            action(
                &format!("{ADD_TASK_PREFIX}condition"),
                "Add condition task here",
                "mdi-call-split",
            ),
        ])];
    }
    if selection::relation_of(id).is_some() {
        return vec![group(vec![action(CONNECT, "Depend on", "mdi-ray-start-arrow")])];
    }
    Vec::new()
}

fn input(title: &str, icon: &str, label: &str, initial_value: &str, confirm_label: &str) -> Run {
    Run::Ask(Prompt::Input {
        title: title.to_owned(),
        icon: icon.to_owned(),
        label: label.to_owned(),
        initial_value: initial_value.to_owned(),
        confirm_label: confirm_label.to_owned(),
    })
}

/// The job whose task carries `task_key`; empty for the file's first.
fn job_key_for(model: &DatabricksModel, task_key: &str) -> String {
    model
        .jobs
        .iter()
        .find(|j| j.tasks.iter().any(|t| t.key == task_key))
        .map(|j| j.key.clone())
        .unwrap_or_default()
}

/// An element id's task key: `task:{key}`, or the id as given.
fn key_of(id: &str) -> &str {
    id.strip_prefix("task:").unwrap_or(id)
}

/// A key no task in the job carries yet: `{type}_1`, counting up.
fn fresh_key(job: &JobModel, task_type: &str) -> String {
    (1..)
        .map(|counter| format!("{task_type}_{counter}"))
        .find(|candidate| job.tasks.iter().all(|t| t.key != *candidate))
        .expect("a key is free")
}

fn source_for(task_type: &str, key: &str) -> String {
    match task_type {
        "notebook" => format!("notebooks/{key}"),
        "python" => format!("scripts/{key}.py"),
        _ => key.to_owned(),
    }
}

/// What running `action_id` on `id` does (`RunAsync`), or why it cannot.
pub fn run(model: &DatabricksModel, id: &str, action_id: &str) -> Result<Run, String> {
    if action_id.contains(SIMULATED_MARKER) {
        // The canvas plays the show; a simulated action that reaches the module completes and touches nothing.
        return Ok(Run::Done);
    }
    let task = selection::task_of(model, id);
    if let Some((job, task)) = task {
        match action_id {
            RENAME_TASK => return Ok(input("Rename task", "mdi-pencil-outline", "Key", &task.key, "Rename")),
            SET_RUN_IF => {
                return Ok(input(
                    "Set run if",
                    "mdi-filter-outline",
                    "Run if (empty = ALL_SUCCESS)",
                    &task.run_if,
                    "Set",
                ));
            }
            ASSIGN_CLUSTER => {
                return Ok(input(
                    "Assign cluster",
                    "mdi-server",
                    "Cluster key (empty = serverless)",
                    &task.cluster_key,
                    "Assign",
                ));
            }
            REMOVE_TASK => {
                // The action says how many edges go with it, before it runs; an untouched task needs no ceremony.
                let going = task.depends_on.len() + writer::references_to(job, &task.key).len();
                let remove = DatabricksEdit::RemoveTask {
                    job_key: job.key.clone(),
                    task_key: task.key.clone(),
                };
                if going == 0 {
                    return Ok(Run::Edit(remove));
                }
                return Ok(Run::Ask(Prompt::Confirm {
                    title: "Remove task".to_owned(),
                    icon: "mdi-delete-outline".to_owned(),
                    message: if going == 1 {
                        "Removing this task also removes the 1 dependency touching it.".to_owned()
                    } else {
                        format!("Removing this task also removes the {going} dependencies touching it.")
                    },
                    confirm_label: "Remove".to_owned(),
                    danger: true,
                }));
            }
            _ => {}
        }
        if let Some(from_key) = action_id.strip_prefix(DISCONNECT_PREFIX) {
            return Ok(Run::Edit(DatabricksEdit::Disconnect {
                job_key: job.key.clone(),
                from_key: from_key.to_owned(),
                to_key: task.key.clone(),
            }));
        }
    }
    if action_id == REMOVE_EDGE
        && let Some((from_key, to_key)) = selection::edge_of(id)
    {
        return Ok(Run::Edit(DatabricksEdit::Disconnect {
            job_key: job_key_for(model, to_key),
            from_key: from_key.to_owned(),
            to_key: to_key.to_owned(),
        }));
    }
    if action_id == CONNECT
        && let Some((from, to)) = selection::relation_of(id)
    {
        // This family creates tasks by drop, not by a relation released on empty canvas.
        if selection::is_placement(from) || selection::is_placement(to) {
            return Err("Drop the dependency on a task; a dependency needs both of its ends.".to_owned());
        }
        let (from_key, to_key) = (key_of(from), key_of(to));
        return Ok(Run::Edit(DatabricksEdit::Connect {
            job_key: job_key_for(model, to_key),
            from_key: from_key.to_owned(),
            to_key: to_key.to_owned(),
            outcome: String::new(),
        }));
    }
    if let Some(task_type) = action_id.strip_prefix(ADD_TASK_PREFIX) {
        if selection::is_placement(id) {
            // The gesture already said everything an add needs, so nothing is asked and the task gets a fresh key.
            let Some(job) = model.jobs.first() else {
                return Err("This file declares no job to add a task to.".to_owned());
            };
            let key = fresh_key(job, task_type);
            return Ok(Run::Edit(DatabricksEdit::InsertTask {
                job_key: job.key.clone(),
                source: source_for(task_type, &key),
                task_key: key,
                task_type: task_type.to_owned(),
            }));
        }
        return Ok(input("Add task", "mdi-plus", "Task key", "", "Add"));
    }
    if action_id == RENAME_BUNDLE && selection::is_bundle(id) {
        return Ok(input(
            "Rename bundle",
            "mdi-pencil-outline",
            "Name",
            &model.bundle.name,
            "Rename",
        ));
    }
    if action_id.starts_with(ADD_RESOURCE_PREFIX) {
        return Ok(input("Add resource", "mdi-plus", "Key", "", "Add"));
    }
    if action_id.starts_with(ADD_LIBRARY_PREFIX) {
        return Ok(input("Add library", "mdi-plus", "Path", "", "Add"));
    }
    if action_id == REMOVE_LIBRARY
        && let Some((pipeline, library)) = selection::library_of(model, id)
    {
        return Ok(Run::Edit(DatabricksEdit::RemoveLibrary {
            pipeline_key: pipeline.key.clone(),
            kind: library.kind.clone(),
            path: library.path.clone(),
        }));
    }
    Ok(Run::Done)
}

/// The edit that commits `value` for `action_id` on `id`, asked for earlier as input or a confirmation
/// (`CommitCommand`), or none when the action does not apply.
pub fn commit(model: &DatabricksModel, id: &str, action_id: &str, value: &str) -> Option<DatabricksEdit> {
    let trimmed = value.trim().to_owned();
    if let Some(kind) = action_id.strip_prefix(ADD_RESOURCE_PREFIX) {
        return Some(DatabricksEdit::AddResource {
            kind: kind.to_owned(),
            key: trimmed,
        });
    }
    if let Some(kind) = action_id.strip_prefix(ADD_LIBRARY_PREFIX) {
        return Some(DatabricksEdit::InsertLibrary {
            pipeline_key: model.pipelines.first().map(|p| p.key.clone()).unwrap_or_default(),
            kind: kind.to_owned(),
            path: trimmed,
        });
    }
    if let Some(task_type) = action_id.strip_prefix(ADD_TASK_PREFIX) {
        let job = model.jobs.first()?;
        return Some(DatabricksEdit::InsertTask {
            job_key: job.key.clone(),
            source: source_for(task_type, &trimmed),
            task_key: trimmed,
            task_type: task_type.to_owned(),
        });
    }
    let task = selection::task_of(model, id);
    match (action_id, task) {
        (RENAME_TASK, Some((job, task))) => Some(DatabricksEdit::RenameTask {
            job_key: job.key.clone(),
            task_key: task.key.clone(),
            new_key: trimmed,
        }),
        (SET_RUN_IF, Some((job, task))) => Some(DatabricksEdit::SetRunIf {
            job_key: job.key.clone(),
            task_key: task.key.clone(),
            run_if: trimmed,
        }),
        (ASSIGN_CLUSTER, Some((job, task))) => Some(DatabricksEdit::SetCluster {
            job_key: job.key.clone(),
            task_key: task.key.clone(),
            cluster_key: trimmed,
        }),
        (REMOVE_TASK, Some((job, task))) => Some(DatabricksEdit::RemoveTask {
            job_key: job.key.clone(),
            task_key: task.key.clone(),
        }),
        (RENAME_BUNDLE, _) => Some(DatabricksEdit::SetBundleName { name: trimmed }),
        _ => None,
    }
}
