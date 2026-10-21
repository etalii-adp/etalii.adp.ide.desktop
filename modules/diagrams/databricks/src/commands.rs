//! Every edit of a Databricks file and every stored position, as a command on the project's history (etalii.adp spec
//! 030, task T081).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/databricks/backend/EtAlii.Adp.Diagram.Databricks/
//! Commands/` (one handler per command, over `DatabricksEditHandler`), the core's `EtAlii.Adp.History/DocumentEditRun.cs`
//! and `RestoreDocumentCommand.cs`, and `EtAlii.Adp.Hierarchy/SetRegistrationLayoutCommand.cs`. An edit is made on a
//! copy of the body and its reading, the copy written, and its undo is the whole text as it was: the family's
//! operations touch several distant ranges at once, the documents are small, and a snapshot cannot be wrong. Every
//! edit a writer accepts is recorded, even one that leaves the text as it was, as the standalone host recorded it. A
//! stored position is written to the registration's `layout:` block and undone by putting the entry it replaced
//! back, or by removing it when there was none.
//!
//! The standalone host's history undid a change by running the inverse recorded when it was made and redid it by
//! running the change itself again, so [`Undo`] and [`Redo`] keep that pair.

use std::fs;
use std::path::{Path, PathBuf};

use adp_diagram::DiagramDocuments;
use adp_history::{Applied, Command, Refusal};

use crate::lines::LineDocument;
use crate::model::{DatabricksModel, JobModel, PipelineModel};
use crate::{parser, registration_layout, store, writer};

/// One edit of a body, as its standalone command said it.
#[derive(Debug, Clone, PartialEq)]
pub enum DatabricksEdit {
    /// A resource skeleton under `resources:` (`AddDatabricksResourceCommand`).
    AddResource { kind: String, key: String },
    /// The bundle's name (`SetDatabricksBundleNameCommand`).
    SetBundleName { name: String },
    /// A task after the job's last (`InsertDatabricksTaskCommand`); an empty job key is the file's first job.
    InsertTask {
        job_key: String,
        task_key: String,
        task_type: String,
        source: String,
    },
    /// A task and every reference to it (`RemoveDatabricksTaskCommand`).
    RemoveTask { job_key: String, task_key: String },
    /// `to_key` comes to wait for `from_key` (`ConnectDatabricksTasksCommand`).
    Connect {
        job_key: String,
        from_key: String,
        to_key: String,
        outcome: String,
    },
    /// One dependency removed (`DisconnectDatabricksTasksCommand`).
    Disconnect {
        job_key: String,
        from_key: String,
        to_key: String,
    },
    /// A task's key and every reference to it (`RenameDatabricksTaskCommand`).
    RenameTask {
        job_key: String,
        task_key: String,
        new_key: String,
    },
    /// A task's `run_if`; empty restores the default (`SetDatabricksRunIfCommand`).
    SetRunIf {
        job_key: String,
        task_key: String,
        run_if: String,
    },
    /// A task's cluster; empty is serverless (`SetDatabricksClusterCommand`).
    SetCluster {
        job_key: String,
        task_key: String,
        cluster_key: String,
    },
    /// A library after the pipeline's last (`InsertDatabricksLibraryCommand`); an empty pipeline key is the first.
    InsertLibrary {
        pipeline_key: String,
        kind: String,
        path: String,
    },
    /// One library removed (`RemoveDatabricksLibraryCommand`).
    RemoveLibrary {
        pipeline_key: String,
        kind: String,
        path: String,
    },
    /// One of the pipeline's own scalars (`SetDatabricksPipelineScalarCommand`).
    SetPipelineScalar {
        pipeline_key: String,
        key: String,
        value: String,
    },
}

fn label_of(edit: &DatabricksEdit) -> &'static str {
    match edit {
        DatabricksEdit::AddResource { .. } => "Add resource",
        DatabricksEdit::SetBundleName { .. } => "Rename bundle",
        DatabricksEdit::InsertTask { .. } => "Add task",
        DatabricksEdit::RemoveTask { .. } => "Remove task",
        DatabricksEdit::Connect { .. } => "Add dependency",
        DatabricksEdit::Disconnect { .. } => "Remove dependency",
        DatabricksEdit::RenameTask { .. } => "Rename task",
        DatabricksEdit::SetRunIf { .. } => "Set run if",
        DatabricksEdit::SetCluster { .. } => "Assign cluster",
        DatabricksEdit::InsertLibrary { .. } => "Add library",
        DatabricksEdit::RemoveLibrary { .. } => "Remove library",
        DatabricksEdit::SetPipelineScalar { .. } => "Change pipeline",
    }
}

/// The job an edit addresses: the named one, or the file's first.
fn job_of<'a>(model: &'a DatabricksModel, job_key: &str) -> Option<&'a JobModel> {
    if job_key.is_empty() {
        model.jobs.first()
    } else {
        model.jobs.iter().find(|j| j.key == job_key)
    }
}

/// The pipeline an edit addresses: the named one, or the file's first.
fn pipeline_of<'a>(model: &'a DatabricksModel, pipeline_key: &str) -> Option<&'a PipelineModel> {
    if pipeline_key.is_empty() {
        model.pipelines.first()
    } else {
        model.pipelines.iter().find(|p| p.key == pipeline_key)
    }
}

const NO_JOB: &str = "There is no job in this file any more.";
const NO_PIPELINE: &str = "There is no pipeline in this file any more.";

/// Applies `edit` to `body`, whose reading is `model`, or says why not.
pub fn apply(body: &mut LineDocument, model: &DatabricksModel, edit: &DatabricksEdit) -> Result<(), String> {
    let job = |key: &str| job_of(model, key).ok_or_else(|| NO_JOB.to_owned());
    let pipeline = |key: &str| pipeline_of(model, key).ok_or_else(|| NO_PIPELINE.to_owned());
    match edit {
        DatabricksEdit::AddResource { kind, key } => writer::add_resource_skeleton(body, &model.bundle, kind, key),
        DatabricksEdit::SetBundleName { name } => writer::set_bundle_name(body, name),
        DatabricksEdit::InsertTask {
            job_key,
            task_key,
            task_type,
            source,
        } => {
            let job = job_of(model, job_key).ok_or("There is no job in this file to add a task to.")?;
            writer::insert_task(body, job, task_key, task_type, source)
        }
        DatabricksEdit::RemoveTask { job_key, task_key } => writer::remove_task(body, job(job_key)?, task_key),
        DatabricksEdit::Connect {
            job_key,
            from_key,
            to_key,
            outcome,
        } => writer::connect(body, job(job_key)?, from_key, to_key, outcome),
        DatabricksEdit::Disconnect {
            job_key,
            from_key,
            to_key,
        } => writer::disconnect(body, job(job_key)?, from_key, to_key),
        DatabricksEdit::RenameTask {
            job_key,
            task_key,
            new_key,
        } => writer::rename_task(body, job(job_key)?, task_key, new_key),
        DatabricksEdit::SetRunIf {
            job_key,
            task_key,
            run_if,
        } => writer::set_run_if(body, job(job_key)?, task_key, run_if),
        DatabricksEdit::SetCluster {
            job_key,
            task_key,
            cluster_key,
        } => writer::set_cluster(body, job(job_key)?, task_key, cluster_key),
        DatabricksEdit::InsertLibrary {
            pipeline_key,
            kind,
            path,
        } => writer::insert_library(body, pipeline(pipeline_key)?, kind, path),
        DatabricksEdit::RemoveLibrary {
            pipeline_key,
            kind,
            path,
        } => writer::remove_library(body, pipeline(pipeline_key)?, kind, path),
        DatabricksEdit::SetPipelineScalar {
            pipeline_key,
            key,
            value,
        } => writer::set_scalar(body, pipeline(pipeline_key)?, key, value),
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// One edit of one body.
#[derive(Debug, Clone, PartialEq)]
pub struct DatabricksCommand {
    pub body_path: PathBuf,
    pub edit: DatabricksEdit,
}

impl DatabricksCommand {
    pub fn new(body_path: &Path, edit: DatabricksEdit) -> Self {
        DatabricksCommand {
            body_path: body_path.to_path_buf(),
            edit,
        }
    }

    /// The edit run (`DocumentEditRun.Run`): the body loaded, the edit made on a copy of its lines and their reading,
    /// the copy written; the text it had before, to restore on undo.
    pub fn run(&self, documents: &mut DiagramDocuments) -> Result<String, String> {
        let document = store::get_or_load(documents, &self.body_path);
        if let Some(reason) = &document.unreadable {
            return Err(format!(
                "{} could not be read, so nothing can be edited until it can: {reason}",
                file_name(&self.body_path)
            ));
        }
        let before = document.text.clone();
        let mut copy = LineDocument::parse(&before);
        let model = parser::outcome(&copy).model;
        apply(&mut copy, &model, &self.edit)?;
        store::save(documents, &self.body_path, &copy.text())?;
        Ok(before)
    }
}

impl Command<DiagramDocuments> for DatabricksCommand {
    fn label(&self) -> String {
        label_of(&self.edit).to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        let before = self.run(documents).map_err(Refusal::new)?;
        Ok(Applied::undoable(Undo {
            inverse: Inverse::Restore {
                body_path: self.body_path.clone(),
                text: before,
            },
            change: Change::Edit(self.clone()),
        }))
    }
}

/// Stores one element's authored position in a registration's `layout:` block (`SetRegistrationLayoutCommand`).
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutCommand {
    pub registration_path: PathBuf,
    pub body_path: PathBuf,
    pub element_id: String,
    pub x: f64,
    pub y: f64,
}

/// The refusal of a position stored in a registration that is gone.
pub const REGISTRATION_GONE: &str = "The registration file is no longer there.";

fn write_registration(
    documents: &mut DiagramDocuments,
    registration_path: &Path,
    body_path: &Path,
    text: &str,
) -> Result<(), String> {
    adp_documents::text::write_atomically(registration_path, text.as_bytes())
        .map_err(|e| format!("Could not store the position: {e}"))?;
    store::store(documents).wrote_registration(registration_path, text.as_bytes());
    documents.mark_changed(body_path);
    Ok(())
}

fn read_registration(path: &Path) -> Result<String, String> {
    if !path.is_file() {
        return Err(REGISTRATION_GONE.to_owned());
    }
    fs::read(path)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .map_err(|e| format!("Could not store the position: {e}"))
}

impl LayoutCommand {
    /// Writes the position; the one it replaced, none when the element had none.
    fn run(&self, documents: &mut DiagramDocuments) -> Result<Option<(f64, f64)>, String> {
        let text = read_registration(&self.registration_path)?;
        let (written, previous) = registration_layout::set_position(&text, &self.element_id, (self.x, self.y));
        write_registration(documents, &self.registration_path, &self.body_path, &written)?;
        Ok(previous)
    }
}

impl Command<DiagramDocuments> for LayoutCommand {
    fn label(&self) -> String {
        "Move".to_owned()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        let previous = self.run(documents).map_err(Refusal::new)?;
        // The inverse restores what was there before: the prior entry, or its absence.
        let inverse = match previous {
            Some((x, y)) => Inverse::Place(LayoutCommand { x, y, ..self.clone() }),
            None => Inverse::Unplace {
                registration_path: self.registration_path.clone(),
                body_path: self.body_path.clone(),
                element_id: self.element_id.clone(),
            },
        };
        Ok(Applied::undoable(Undo {
            inverse,
            change: Change::Place(self.clone()),
        }))
    }
}

/// What reverses a change.
#[derive(Debug, Clone)]
enum Inverse {
    /// The body's whole text put back (`RestoreDocumentCommand`).
    Restore { body_path: PathBuf, text: String },
    /// A prior position stored again.
    Place(LayoutCommand),
    /// A position removed (`RemoveRegistrationLayoutCommand`).
    Unplace {
        registration_path: PathBuf,
        body_path: PathBuf,
        element_id: String,
    },
}

impl Inverse {
    fn run(&self, documents: &mut DiagramDocuments) -> Result<(), String> {
        match self {
            Inverse::Restore { body_path, text } => {
                store::write(documents, body_path, text).map_err(|e| format!("Could not restore the file: {e}"))
            }
            Inverse::Place(command) => command.run(documents).map(|_| ()),
            Inverse::Unplace {
                registration_path,
                body_path,
                element_id,
            } => {
                let text = read_registration(registration_path)?;
                if !registration_layout::read(&text).contains_key(element_id) {
                    return Err("That element has no stored position any more.".to_owned());
                }
                match registration_layout::remove_position(&text, element_id) {
                    Some(written) => write_registration(documents, registration_path, body_path, &written),
                    None => Ok(()),
                }
            }
        }
    }
}

/// A change recorded on the history.
#[derive(Debug, Clone)]
enum Change {
    Edit(DatabricksCommand),
    Place(LayoutCommand),
}

impl Change {
    fn label(&self) -> String {
        match self {
            Change::Edit(command) => command.label(),
            Change::Place(command) => command.label(),
        }
    }

    fn run(&self, documents: &mut DiagramDocuments) -> Result<(), String> {
        match self {
            Change::Edit(command) => command.run(documents).map(|_| ()),
            Change::Place(command) => command.run(documents).map(|_| ()),
        }
    }
}

/// The undo of a change: its recorded inverse, whose own redo is the change itself (`HistoryStack.UndoAsync`).
struct Undo {
    inverse: Inverse,
    change: Change,
}

/// The redo of a change: the change run again, whose undo is the inverse recorded the first time
/// (`HistoryStack.RedoAsync`).
struct Redo {
    inverse: Inverse,
    change: Change,
}

impl Command<DiagramDocuments> for Undo {
    fn label(&self) -> String {
        self.change.label()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        self.inverse.run(documents).map_err(Refusal::new)?;
        Ok(Applied::undoable(Redo {
            inverse: self.inverse.clone(),
            change: self.change.clone(),
        }))
    }
}

impl Command<DiagramDocuments> for Redo {
    fn label(&self) -> String {
        self.change.label()
    }

    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        self.change.run(documents).map_err(Refusal::new)?;
        Ok(Applied::undoable(Undo {
            inverse: self.inverse.clone(),
            change: self.change.clone(),
        }))
    }
}
