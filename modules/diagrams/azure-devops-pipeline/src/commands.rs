//! Every edit of a pipeline, as a command on the project's history (etalii.adp spec 030, task T078).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/azure-devops-pipeline/backend/EtAlii.Adp.Diagram.
//! AzureDevOpsPipeline/Commands/` (one handler per command, each checking its own preconditions every time it runs) and
//! the core's `EtAlii.Adp.History/DocumentEditRun.cs`, `RestoreDocumentCommand.cs` and `HistoryStack.cs`. Each edit
//! names the command that reverses it, as the standalone handlers did: a rename its old display name, a remove the
//! exact lines it took out, an add the remove of what it made. The standalone host's history undid a change by running
//! the inverse recorded when it was made and redid it by running the change itself again, whatever either of them
//! answered in turn, so [`PipelineCommand`] keeps that pair: an undo runs the recorded inverse and its redo the
//! original edit.

use std::path::{Path, PathBuf};

use adp_diagram::DiagramDocuments;
use adp_history::{Applied, Command, Refusal};

use crate::blocks;
use crate::edits;
use crate::lines::LineDocument;
use crate::model::{EditTarget, LocationKind, PipelineModel};
use crate::store;
use crate::writer::PipelineWriter;

/// The refusal of an element that is not there.
pub const GONE: &str = "That element is no longer in this pipeline.";

/// What the add command is asked to create (`PipelineAddKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddKind {
    Stage,
    Job,
    DeploymentJob,
    Step,
}

/// One edit, as its standalone command said it.
#[derive(Debug, Clone, PartialEq)]
pub enum PipelineEdit {
    /// An element's `displayName`; empty removes it (`RenamePipelineElementCommand`).
    Rename { element_id: String, display_name: String },
    /// What an element waits for; not `declared` puts it back on the default (`SetPipelineDependenciesCommand`).
    SetDependencies {
        element_id: String,
        depends_on: Vec<String>,
        declared: bool,
    },
    /// Whether an element runs (`SetPipelineElementEnabledCommand`).
    SetEnabled { element_id: String, enabled: bool },
    /// A stage, job or step at the end of its list (`AddPipelineElementCommand`).
    Add {
        kind: AddKind,
        parent_id: String,
        name: String,
    },
    /// A stage, job or step removed (`RemovePipelineElementCommand`).
    Remove { element_id: String },
    /// Removed lines put back (`RestorePipelineLinesCommand`).
    RestoreLines { at_line: usize, lines: Vec<String> },
    /// A step moved within its job (`MovePipelineStepCommand`).
    MoveStep { step_id: String, to_index: i64 },
    /// The body's whole text put back (`RestoreDocumentCommand`), for an edit that names no inverse of its own.
    RestoreDocument { text: String },
}

/// What an edit did (`EditOutcome`).
#[derive(Debug, Clone, PartialEq)]
pub enum EditOutcome {
    Refused(String),
    /// It ran and the text says what it already said, so there is nothing to record.
    Unchanged,
    /// It changed the text; the edit that reverses it, when it names one.
    Applied(Option<PipelineEdit>),
}

/// One edit of one body, read from one workspace root.
#[derive(Debug, Clone, PartialEq)]
pub struct PipelineCommand {
    pub root_path: PathBuf,
    pub body_path: PathBuf,
    pub edit: PipelineEdit,
}

impl PipelineCommand {
    pub fn new(root_path: &Path, body_path: &Path, edit: PipelineEdit) -> Self {
        PipelineCommand {
            root_path: root_path.to_path_buf(),
            body_path: body_path.to_path_buf(),
            edit,
        }
    }

    fn with(&self, edit: PipelineEdit) -> Self {
        PipelineCommand { edit, ..self.clone() }
    }

    /// The edit run (`DocumentEditRun.Run`): the body loaded, the edit made on a copy of its lines and their reading,
    /// the copy written, and the inverse to record; the module's restore of the whole text where the edit names none.
    pub fn run(&self, documents: &mut DiagramDocuments) -> Result<Option<PipelineCommand>, String> {
        let document = store::get_or_load(documents, &self.root_path, &self.body_path);
        if let Some(reason) = &document.unreadable {
            return Err(format!(
                "{} could not be read, so nothing can be edited until it can: {reason}",
                file_name(&self.body_path)
            ));
        }
        let before = document.text.clone();
        let mut copy = LineDocument::parse(&before);
        let model = store::store(documents).model_of(&self.root_path, &self.body_path, &copy);
        let outcome = apply(documents, self, &mut copy, &model);
        match outcome {
            EditOutcome::Refused(reason) => Err(reason),
            EditOutcome::Unchanged => Ok(None),
            EditOutcome::Applied(inverse) => {
                store::save(documents, &self.root_path, &self.body_path, &copy.text())?;
                let inverse = inverse.unwrap_or(PipelineEdit::RestoreDocument { text: before });
                Ok(Some(self.with(inverse)))
            }
        }
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn label_of(edit: &PipelineEdit) -> &'static str {
    match edit {
        PipelineEdit::Rename { .. } => "Rename",
        PipelineEdit::SetDependencies { .. } => "Change dependencies",
        PipelineEdit::SetEnabled { .. } => "Change enabled",
        PipelineEdit::Add { .. } => "Add",
        PipelineEdit::Remove { .. } => "Remove",
        PipelineEdit::RestoreLines { .. } | PipelineEdit::RestoreDocument { .. } => "Restore",
        PipelineEdit::MoveStep { .. } => "Move step",
    }
}

impl Command<DiagramDocuments> for PipelineCommand {
    fn label(&self) -> String {
        label_of(&self.edit).to_owned()
    }

    /// Runs the edit; what reverses it is recorded with this edit as its redo.
    fn apply(&self, documents: &mut DiagramDocuments) -> Result<Applied<DiagramDocuments>, Refusal> {
        match self.run(documents).map_err(Refusal::new)? {
            None => Ok(Applied::final_change()),
            Some(inverse) => Ok(Applied::undoable(Undo {
                inverse,
                change: self.clone(),
            })),
        }
    }
}

/// The undo of a change: its recorded inverse, whose own redo is the change itself (`HistoryStack.UndoAsync`).
struct Undo {
    inverse: PipelineCommand,
    change: PipelineCommand,
}

/// The redo of a change: the change run again, whose undo is the inverse recorded the first time
/// (`HistoryStack.RedoAsync`).
struct Redo {
    inverse: PipelineCommand,
    change: PipelineCommand,
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

/// Applies `command`'s edit to `body`, whose reading is `model`, or says why not.
fn apply(
    documents: &mut DiagramDocuments,
    command: &PipelineCommand,
    body: &mut LineDocument,
    model: &PipelineModel,
) -> EditOutcome {
    match &command.edit {
        PipelineEdit::Rename {
            element_id,
            display_name,
        } => rename(body, model, element_id, display_name),
        PipelineEdit::SetDependencies {
            element_id,
            depends_on,
            declared,
        } => set_dependencies(body, model, element_id, depends_on, *declared),
        PipelineEdit::SetEnabled { element_id, enabled } => set_enabled(body, model, element_id, *enabled),
        PipelineEdit::Add { kind, parent_id, name } => add(body, model, *kind, parent_id, name),
        PipelineEdit::Remove { element_id } => remove(body, model, element_id),
        PipelineEdit::RestoreLines { at_line, lines } => restore_lines(documents, command, body, *at_line, lines),
        PipelineEdit::MoveStep { step_id, to_index } => move_step(body, model, step_id, *to_index),
        PipelineEdit::RestoreDocument { text } => {
            if body.text() == *text {
                EditOutcome::Unchanged
            } else {
                *body = LineDocument::parse(text);
                EditOutcome::Applied(None)
            }
        }
    }
}

/// The location of `id`, or the refusal: it is gone, or it is written somewhere else (`PipelineEditHandler.Refuses`).
fn editable<'a>(model: &'a PipelineModel, id: &str) -> Result<crate::model::Location<'a>, EditOutcome> {
    let Some(location) = edits::locate(model, id) else {
        return Err(EditOutcome::Refused(GONE.to_owned()));
    };
    let refusal = edits::refusal_for(&location);
    if refusal.is_empty() {
        Ok(location)
    } else {
        Err(EditOutcome::Refused(refusal))
    }
}

fn written(result: Result<bool, String>, inverse: PipelineEdit) -> EditOutcome {
    match result {
        Err(reason) => EditOutcome::Refused(reason),
        Ok(true) => EditOutcome::Applied(Some(inverse)),
        Ok(false) => EditOutcome::Unchanged,
    }
}

fn rename(body: &mut LineDocument, model: &PipelineModel, id: &str, display_name: &str) -> EditOutcome {
    let location = match editable(model, id) {
        Ok(location) => location,
        Err(refusal) => return refusal,
    };
    let previous = location.display_name().to_owned();
    written(
        PipelineWriter::new(body).set_display_name(&location.target(), display_name),
        PipelineEdit::Rename {
            element_id: id.to_owned(),
            display_name: previous,
        },
    )
}

fn set_dependencies(
    body: &mut LineDocument,
    model: &PipelineModel,
    id: &str,
    depends_on: &[String],
    declared: bool,
) -> EditOutcome {
    if edits::locate(model, id).is_some_and(|l| l.kind == LocationKind::Step) {
        return EditOutcome::Refused("Steps run in order, so a step has nothing to wait for.".to_owned());
    }
    let location = match editable(model, id) {
        Ok(location) => location,
        Err(refusal) => return refusal,
    };
    let names = edits::names_to_ids(model, &location);
    if let Some(unknown) = depends_on.iter().find(|n| !names.contains_key(&n.to_lowercase())) {
        return EditOutcome::Refused(format!("This pipeline has nothing called '{unknown}' to wait for."));
    }
    if edits::would_cycle(model, id, depends_on) {
        return EditOutcome::Refused(format!(
            "That would make {} wait for something that is already waiting for it.",
            location.label()
        ));
    }
    if location.kind == LocationKind::Stage && declared && edits::would_leave_no_starting_stage(model, id, depends_on) {
        return EditOutcome::Refused(
            "A pipeline must contain at least one stage with no dependencies, or it has nothing to start with."
                .to_owned(),
        );
    }
    let (previous, was_declared) = if location.kind == LocationKind::Stage {
        (location.stage.depends_on.clone(), location.stage.depends_on_declared)
    } else {
        let job = location.job.expect("a job's location has its job");
        (job.depends_on.clone(), job.depends_on_declared)
    };
    let target = location.target();
    let mut writer = PipelineWriter::new(body);
    let result = if declared {
        writer.set_depends_on(&target, depends_on)
    } else {
        writer.clear_depends_on(&target)
    };
    written(
        result,
        PipelineEdit::SetDependencies {
            element_id: id.to_owned(),
            depends_on: previous,
            declared: was_declared,
        },
    )
}

fn set_enabled(body: &mut LineDocument, model: &PipelineModel, id: &str, enabled: bool) -> EditOutcome {
    let location = match editable(model, id) {
        Ok(location) => location,
        Err(refusal) => return refusal,
    };
    let was_enabled = !location.execution().is_disabled();
    written(
        PipelineWriter::new(body).set_enabled(&location.target(), if enabled { "" } else { "false" }),
        PipelineEdit::SetEnabled {
            element_id: id.to_owned(),
            enabled: was_enabled,
        },
    )
}

/// What an add made, or why it did not (`PipelineAddOutcome`).
fn add(body: &mut LineDocument, model: &PipelineModel, kind: AddKind, parent_id: &str, name: &str) -> EditOutcome {
    let added = match kind {
        AddKind::Stage => add_stage(body, model, name),
        _ => add_within(body, model, kind, parent_id, name),
    };
    match added {
        Err(reason) => EditOutcome::Refused(reason),
        Ok(element_id) => EditOutcome::Applied(Some(PipelineEdit::Remove { element_id })),
    }
}

fn add_stage(body: &mut LineDocument, model: &PipelineModel, name: &str) -> Result<String, String> {
    let Some(last) = model.stages.iter().rfind(|s| !s.is_implicit && !s.is_from_template()) else {
        return Err("This pipeline has no stages block to add a stage to. Add one in the file first.".to_owned());
    };
    let name = blocks::unused_name(
        if name.is_empty() { "NewStage" } else { name },
        model.stages.iter().map(|s| s.name.as_str()),
    );
    let indent = body.lines()[last.lines.start].indent();
    PipelineWriter::new(body).insert_element(Some(&EditTarget::of_stage(last)), 0, &blocks::stage(&name, indent))?;
    Ok(name)
}

fn add_within(
    body: &mut LineDocument,
    model: &PipelineModel,
    kind: AddKind,
    parent_id: &str,
    name: &str,
) -> Result<String, String> {
    let Some(parent) = edits::locate(model, parent_id) else {
        return Err(GONE.to_owned());
    };
    let refusal = edits::refusal_for(&parent);
    if !refusal.is_empty() {
        return Err(refusal);
    }
    if kind == AddKind::Step {
        if parent.kind != LocationKind::Job {
            return Err("A step goes in a job.".to_owned());
        }
        let job = parent.job.expect("a job's location has its job");
        let Some(last) = job.steps.iter().rfind(|s| !s.is_from_template()) else {
            return Err("This job has no steps of its own to add one beside.".to_owned());
        };
        let indent = body.lines()[last.lines.start].indent();
        PipelineWriter::new(body).insert_element(
            Some(&EditTarget::of_step(last)),
            0,
            &blocks::step(blocks::PLACEHOLDER_SCRIPT, indent),
        )?;
        return Ok(format!("{}/step-{}", job.id, job.steps.len()));
    }
    if parent.kind != LocationKind::Stage {
        return Err("A job goes in a stage.".to_owned());
    }
    let stage = parent.stage;
    let Some(last) = stage.jobs.iter().rfind(|j| !j.is_from_template()) else {
        return Err("This stage has no jobs of its own to add one beside.".to_owned());
    };
    let name = blocks::unused_name(
        if name.is_empty() { "NewJob" } else { name },
        model.jobs().map(|j| j.name.as_str()),
    );
    let indent = body.lines()[last.lines.start].indent();
    let lines = if kind == AddKind::DeploymentJob {
        blocks::deployment_job(&name, &name.to_lowercase(), indent)
    } else {
        blocks::job(&name, indent)
    };
    PipelineWriter::new(body).insert_element(Some(&EditTarget::of_job(last)), 0, &lines)?;
    Ok(format!("{}/{name}", stage.id))
}

/// Why this element must stay, or empty when it may go: a pipeline keeps a stage, a stage a job, a job a step, and a
/// stage that waits for nothing. The standalone handler's last check asked `WouldLeaveNoStartingStage` with no names,
/// which answers no whatever the stages, so it never refused and is not asked here.
fn must_stay(model: &PipelineModel, location: &crate::model::Location<'_>) -> String {
    match location.kind {
        LocationKind::Stage if model.stages.iter().filter(|s| !s.is_implicit).count() <= 1 => {
            "A pipeline needs at least one stage.".to_owned()
        }
        LocationKind::Job if location.stage.jobs.len() <= 1 => {
            format!("{} needs at least one job.", location.stage.label())
        }
        LocationKind::Step if location.job.is_some_and(|j| j.steps.len() <= 1) => format!(
            "{} needs at least one step.",
            location.job.expect("a step's location has its job").label()
        ),
        _ => String::new(),
    }
}

fn remove(body: &mut LineDocument, model: &PipelineModel, id: &str) -> EditOutcome {
    let location = match editable(model, id) {
        Ok(location) => location,
        Err(refusal) => return refusal,
    };
    let reason = must_stay(model, &location);
    if !reason.is_empty() {
        return EditOutcome::Refused(reason);
    }
    let target = location.target();
    match PipelineWriter::new(body).remove_element(&target) {
        Err(reason) => EditOutcome::Refused(reason),
        Ok(removed) => EditOutcome::Applied(Some(PipelineEdit::RestoreLines {
            at_line: target.lines.start,
            lines: removed,
        })),
    }
}

/// The name a restored block declares: what follows the first colon of its first line.
fn id_of(lines: &[String]) -> String {
    let first = lines
        .first()
        .map(|l| l.trim_start_matches([' ', '-']))
        .unwrap_or_default();
    match first.find(':') {
        Some(colon) => first[colon + 1..].trim().to_owned(),
        None => String::new(),
    }
}

fn restore_lines(
    documents: &mut DiagramDocuments,
    command: &PipelineCommand,
    body: &mut LineDocument,
    at_line: usize,
    lines: &[String],
) -> EditOutcome {
    if at_line > body.lines().len() {
        return EditOutcome::Refused("This pipeline has changed too much since then to put that back.".to_owned());
    }
    if let Err(reason) = PipelineWriter::new(body).insert_element(None, at_line.saturating_sub(1), lines) {
        return EditOutcome::Refused(reason);
    }
    let restored = store::store(documents).model_of(&command.root_path, &command.body_path, body);
    match edits::locate(&restored, &id_of(lines)) {
        None => EditOutcome::Applied(None),
        Some(location) => EditOutcome::Applied(Some(PipelineEdit::Remove {
            element_id: location.id().to_owned(),
        })),
    }
}

fn move_step(body: &mut LineDocument, model: &PipelineModel, step_id: &str, to_index: i64) -> EditOutcome {
    let location = match edits::locate(model, step_id) {
        Some(location) if location.kind == LocationKind::Step => location,
        _ => return EditOutcome::Refused("That step is no longer in this pipeline.".to_owned()),
    };
    let refusal = edits::refusal_for(&location);
    if !refusal.is_empty() {
        return EditOutcome::Refused(refusal);
    }
    let job = location.job.expect("a step's location has its job");
    let steps = &job.steps;
    let Some(from) = steps.iter().position(|s| s.id == step_id) else {
        return EditOutcome::Unchanged;
    };
    let to = to_index.clamp(0, steps.len() as i64 - 1) as usize;
    if from == to {
        return EditOutcome::Unchanged;
    }
    if steps.iter().any(|s| s.is_from_template()) {
        return EditOutcome::Refused(
            "Some of this job's steps come from a template, so their order cannot be changed here.".to_owned(),
        );
    }
    let after = if to > from {
        Some(&steps[to])
    } else if to == 0 {
        None
    } else {
        Some(&steps[to - 1])
    };
    let list_key_line = steps[0].lines.start.saturating_sub(1);
    let after = after.map(EditTarget::of_step);
    written(
        PipelineWriter::new(body).move_element(&location.target(), after.as_ref(), list_key_line),
        PipelineEdit::MoveStep {
            step_id: format!("{}/step-{to}", job.id),
            to_index: from as i64,
        },
    )
}
