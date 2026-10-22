//! One open Databricks diagram, and what is asked of it (etalii.adp spec 030, task T081).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/databricks/backend/EtAlii.Adp.Diagram.Databricks/
//! DatabricksSession.cs` (which of the family's three readings the diagram shows, the declaration its registration's
//! `resource:` header picks, and a drag stored in the registration's `layout:` block, never in the body),
//! `DatabricksContextSourceResolver.cs` and `DatabricksValidator.cs`, with the action and property providers of
//! [`crate::actions`] and [`crate::properties`].

use std::fs;
use std::path::{Path, PathBuf};

use adp_context::{ActionGroup, PropertyRow};
use adp_diagram::session::{ActionOutcome, ActionTarget};
use adp_diagram::{
    DiagramDocuments, DiagramElement, DiagramProblem, DiagramViewport, DiagramWorkspace, ModuleView, ToolboxItem,
};
use adp_history::{Command, Composite, Outcome};

use crate::actions::{self, Run};
use crate::commands::{DatabricksCommand, DatabricksEdit, LayoutCommand};
use crate::model::DatabricksModel;
use crate::store::{self, DatabricksDocument};
use crate::{layout, mapper, properties, registration_layout, rules, selection, toolbox};

/// What dropping an element onto another answers: nothing here nests under a parent.
pub const PARENT_REFUSAL: &str =
    "A Databricks element has no parent to move it under; dragging changes where it sits on the canvas.";

/// What dragging an edge answers: it follows its ends.
pub const NOT_MOVABLE: &str = "That element is not something this diagram can move.";

/// What a drag answers on a diagram opened without a registration.
pub const NO_REGISTRATION: &str =
    "This diagram was opened without a registration, so there is nowhere to store a position.";

/// What an action its target does not offer answers (the standalone host's `ContextService.ExecuteAction`).
pub const NOT_AVAILABLE: &str = "That action is not available for this item.";

/// One open view of a Databricks file: its body, the registration it was opened through, which reading it shows and
/// which declaration.
#[derive(Debug)]
pub struct DatabricksView {
    body_path: PathBuf,
    registration_path: Option<PathBuf>,
    /// `bundle`, `job` or `pipeline`.
    reading: String,
    /// The `resource:` header's key, or none for the file's first declaration.
    resource_key: Option<String>,
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn execute(workspace: &mut DiagramWorkspace, command: &dyn Command<DiagramDocuments>) -> Result<(), String> {
    match workspace.execute_command(command) {
        Outcome::Done { .. } => Ok(()),
        Outcome::Refused(refusal) => Err(refusal.0),
    }
}

fn offered(menu: &[ActionGroup], action_id: &str) -> bool {
    menu.iter()
        .flat_map(|g| &g.actions)
        .any(|a| a.id == action_id && a.is_available())
}

/// The id a target names: an element's own, a canvas point's placement, a finished gesture's relation.
fn id_of(target: &ActionTarget) -> String {
    match target {
        ActionTarget::Element(id) => id.clone(),
        ActionTarget::Canvas { x, y } => selection::placement_id(*x, *y),
        ActionTarget::Connection { source, target } => selection::relation_id(
            source.as_deref().unwrap_or_default(),
            target.as_deref().unwrap_or_default(),
        ),
    }
}

impl DatabricksView {
    pub fn new(
        body_path: &Path,
        registration_path: Option<&Path>,
        reading: &str,
        resource_key: Option<String>,
    ) -> Self {
        DatabricksView {
            body_path: body_path.to_path_buf(),
            registration_path: registration_path.map(Path::to_path_buf),
            reading: reading.to_owned(),
            resource_key,
        }
    }

    pub fn body_path(&self) -> &Path {
        &self.body_path
    }

    /// `bundle`, `job` or `pipeline`.
    pub fn reading(&self) -> &str {
        &self.reading
    }

    fn document<'a>(&self, documents: &'a DiagramDocuments) -> Result<&'a DatabricksDocument, String> {
        store::get(documents, &self.body_path).ok_or_else(|| adp_diagram::document::NOT_OPEN.to_owned())
    }

    /// The document when it could be read; else what an action on it answers.
    fn usable<'a>(&self, documents: &'a DiagramDocuments) -> Result<&'a DatabricksDocument, String> {
        let document = self.document(documents)?;
        if !document.is_usable() {
            return Err(format!(
                "{} could not be read, so it cannot be edited.",
                file_name(&self.body_path)
            ));
        }
        Ok(document)
    }

    fn command(&self, edit: DatabricksEdit) -> DatabricksCommand {
        DatabricksCommand::new(&self.body_path, edit)
    }

    /// The declaration this view shows: the one the `resource:` header names, or the file's first.
    fn pick<'a, T>(&self, candidates: &'a [T], key: impl Fn(&T) -> &str) -> Option<&'a T> {
        match &self.resource_key {
            Some(wanted) => candidates.iter().find(|c| key(c) == wanted),
            None => candidates.first(),
        }
    }

    /// The reading this view shows of `model`, with the registration's stored positions overlaid.
    pub fn elements(&self, model: &DatabricksModel) -> Vec<DiagramElement> {
        let stored = self
            .registration_path
            .as_deref()
            .and_then(|path| fs::read(path).ok())
            .map(|bytes| registration_layout::read(&String::from_utf8_lossy(&bytes)))
            .unwrap_or_default();
        match self.reading.as_str() {
            "job" => {
                let Some(job) = self.pick(&model.jobs, |j| &j.key) else {
                    return Vec::new();
                };
                let mut positions = layout::job(job);
                let placed = layout::apply(&mut positions, &stored);
                mapper::job_elements(job, &positions, &placed)
            }
            "pipeline" => {
                let Some(pipeline) = self.pick(&model.pipelines, |p| &p.key) else {
                    return Vec::new();
                };
                let mut positions = layout::pipeline(pipeline);
                let placed = layout::apply(&mut positions, &stored);
                mapper::pipeline_elements(pipeline, &positions, &placed)
            }
            _ => {
                let mut positions = layout::bundle(&model.bundle);
                let placed = layout::apply(&mut positions, &stored);
                mapper::bundle_elements(&model.bundle, &positions, &placed)
            }
        }
    }
}

impl ModuleView for DatabricksView {
    fn render(&self, documents: &DiagramDocuments, viewport: DiagramViewport) -> Result<Vec<DiagramElement>, String> {
        let document = self.document(documents)?;
        Ok(mapper::visible(self.elements(&document.model), viewport))
    }

    fn describe(&self, documents: &DiagramDocuments, element_id: &str) -> Option<(Vec<String>, String)> {
        let document = self.document(documents).ok()?;
        let text = selection::describe(&document.model, element_id)?;
        Some((vec![text.clone()], text))
    }

    fn toolbox(&self) -> Vec<ToolboxItem> {
        toolbox::items(&self.reading)
    }

    fn problems(&self, documents: &DiagramDocuments) -> Vec<DiagramProblem> {
        self.document(documents)
            .map(|document| rules::validate(&document.text))
            .unwrap_or_default()
    }

    fn actions(&self, documents: &DiagramDocuments, target: &ActionTarget) -> Vec<ActionGroup> {
        match self.document(documents) {
            // A file that does not parse withholds every edit.
            Ok(document) if document.is_usable() => actions::discover(&document.model, &id_of(target)),
            _ => Vec::new(),
        }
    }

    fn execute(&self, workspace: &mut DiagramWorkspace, target: &ActionTarget, action_id: &str) -> ActionOutcome {
        let model = match self.usable(workspace.documents()) {
            Ok(document) => document.model.clone(),
            Err(refusal) => return ActionOutcome::Refused(refusal),
        };
        let id = id_of(target);
        if !offered(&actions::discover(&model, &id), action_id) {
            return ActionOutcome::Refused(NOT_AVAILABLE.to_owned());
        }
        match actions::run(&model, &id, action_id) {
            Err(refusal) => ActionOutcome::Refused(refusal),
            Ok(Run::Done) => ActionOutcome::Done,
            Ok(Run::Ask(prompt)) => ActionOutcome::Prompt(prompt),
            Ok(Run::Edit(edit)) => match execute(workspace, &self.command(edit)) {
                Ok(()) => ActionOutcome::Done,
                Err(refusal) => ActionOutcome::Refused(refusal),
            },
        }
    }

    fn connect_action(
        &self,
        documents: &DiagramDocuments,
        source: &str,
        target: &str,
    ) -> Option<Result<String, String>> {
        // The dependency gesture is the job canvas's alone, and starts on a task; it may land on anything drawn.
        let _ = target;
        if self.reading != "job" {
            return None;
        }
        let document = self.document(documents).ok()?;
        if selection::task_of(&document.model, source).is_none() {
            return Some(Err(
                "A dependency starts on a task: drag from a task's side to the task that waits for it.".to_owned(),
            ));
        }
        Some(Ok(actions::CONNECT.to_owned()))
    }

    fn answer(
        &self,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        value: &str,
    ) -> ActionOutcome {
        let model = match self.usable(workspace.documents()) {
            Ok(document) => document.model.clone(),
            Err(refusal) => return ActionOutcome::Refused(refusal),
        };
        match actions::commit(&model, &id_of(target), action_id, value) {
            None => ActionOutcome::Refused(format!("'{action_id}' does not apply to this selection.")),
            Some(edit) => match execute(workspace, &self.command(edit)) {
                Ok(()) => ActionOutcome::Done,
                Err(refusal) => ActionOutcome::Refused(refusal),
            },
        }
    }

    fn properties(&self, documents: &DiagramDocuments, element_id: &str) -> Vec<PropertyRow> {
        match self.document(documents) {
            Ok(document) => properties::describe(&document.model, element_id),
            _ => Vec::new(),
        }
    }

    fn set_property(
        &self,
        workspace: &mut DiagramWorkspace,
        element_ids: &[&str],
        property_id: &str,
        value: &str,
    ) -> Result<(), String> {
        let document = self.usable(workspace.documents())?;
        let mut commands: Vec<Box<dyn Command<DiagramDocuments>>> = Vec::with_capacity(element_ids.len());
        for id in element_ids {
            let edit = properties::change(&document.model, id, property_id, value)?;
            commands.push(Box::new(self.command(edit)));
        }
        match commands.len() {
            0 => Ok(()),
            1 => execute(workspace, commands[0].as_ref()),
            n => execute(workspace, &Composite::new(format!("Change {n} elements"), commands)),
        }
    }

    /// Stores each authored position in the registration's `layout:` block, one undoable command per element, and
    /// never writes the body (`DatabricksSession.PlacementOf`).
    fn move_elements(&self, workspace: &mut DiagramWorkspace, moves: &[(String, f64, f64)]) -> Result<(), String> {
        let Some(registration_path) = &self.registration_path else {
            return Err(NO_REGISTRATION.to_owned());
        };
        for (id, x, y) in moves {
            if id.starts_with("edge:") || id.starts_with("override:") || id.starts_with("flow:") {
                // An edge has no position of its own: it follows its ends.
                return Err(NOT_MOVABLE.to_owned());
            }
            let command = LayoutCommand {
                registration_path: registration_path.clone(),
                body_path: self.body_path.clone(),
                element_id: id.clone(),
                x: *x,
                y: *y,
            };
            execute(workspace, &command)?;
        }
        Ok(())
    }

    fn move_element(
        &self,
        _workspace: &mut DiagramWorkspace,
        _element_id: &str,
        _new_parent_id: Option<&str>,
        _index: i64,
    ) -> Result<(), String> {
        Err(PARENT_REFUSAL.to_owned())
    }
}
