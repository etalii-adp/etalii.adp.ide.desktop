//! One open pipeline diagram, and what is asked of it (etalii.adp spec 030, task T078).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/azure-devops-pipeline/backend/EtAlii.Adp.Diagram.
//! AzureDevOpsPipeline/PipelineSession.cs` (nothing is dragged: a pipeline arranges itself from its dependencies),
//! `PipelineViewState.cs` and `PipelineConnectionView.cs` (which stages and jobs a view has open, never written),
//! `PipelineContextSourceResolver.cs` (what a selected element or arrow reads as) and `PipelineValidator.cs`, with the
//! action and property providers of [`crate::actions`] and [`crate::properties`].

use std::cell::RefCell;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use adp_context::{ActionGroup, PropertyRow};
use adp_diagram::session::{ActionOutcome, ActionTarget};
use adp_diagram::{
    DiagramDocuments, DiagramElement, DiagramProblem, DiagramViewport, DiagramWorkspace, ModuleView, ToolboxItem,
};
use adp_history::{Command, Composite, Outcome};

use crate::actions::{self, Run};
use crate::commands::{PipelineCommand, PipelineEdit};
use crate::graph;
use crate::mapper;
use crate::model::PipelineModel;
use crate::store::{self, PipelineDocument};
use crate::{properties, rules, toolbox};

/// What dragging an element answers: its place comes from what it waits for.
pub const PLACEMENT_REFUSAL: &str = "This diagram cannot be arranged by dragging.";

/// What dropping an element onto another answers.
pub const PARENT_REFUSAL: &str =
    "A pipeline arranges itself from its dependencies. Change what a stage waits for to change where it sits.";

/// What an action its target does not offer answers (the standalone host's `ContextService.ExecuteAction`).
pub const NOT_AVAILABLE: &str = "That action is not available for this item.";

/// One open view of a pipeline: its body, the project it was opened in, and what it has open.
#[derive(Debug)]
pub struct PipelineView {
    root_path: PathBuf,
    body_path: PathBuf,
    expanded: RefCell<HashSet<String>>,
}

impl PipelineView {
    pub fn new(root_path: &Path, body_path: &Path) -> Self {
        PipelineView {
            root_path: root_path.to_path_buf(),
            body_path: body_path.to_path_buf(),
            expanded: RefCell::new(HashSet::new()),
        }
    }

    pub fn body_path(&self) -> &Path {
        &self.body_path
    }

    /// Whether this view has the stage or job `id` open.
    pub fn is_expanded(&self, id: &str) -> bool {
        self.expanded.borrow().contains(id)
    }

    /// Opens the stage or job `id`, or closes it; whether it is open now.
    pub fn toggle(&self, id: &str) -> bool {
        let mut expanded = self.expanded.borrow_mut();
        if expanded.remove(id) {
            false
        } else {
            expanded.insert(id.to_owned());
            true
        }
    }

    fn document<'a>(&self, documents: &'a DiagramDocuments) -> Result<&'a PipelineDocument, String> {
        store::get(documents, &self.body_path).ok_or_else(|| adp_diagram::document::NOT_OPEN.to_owned())
    }

    /// The document when it could be read; else what an action on it answers (`DiagramActionProvider.Unreadable`).
    fn usable<'a>(&self, documents: &'a DiagramDocuments) -> Result<&'a PipelineDocument, String> {
        let document = self.document(documents)?;
        if !document.is_usable() {
            return Err(format!(
                "{} could not be read, so it cannot be edited.",
                file_name(&self.body_path)
            ));
        }
        Ok(document)
    }

    fn command(&self, edit: PipelineEdit) -> PipelineCommand {
        PipelineCommand::new(&self.root_path, &self.body_path, edit)
    }
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

/// What a selection of `element_id` reads as: its chain of names and its text, or an arrow's "waits for"
/// (`PipelineContextSourceResolver.Find`).
pub fn find(model: &PipelineModel, element_id: &str) -> Option<(Vec<String>, String)> {
    for stage in &model.stages {
        if stage.id == element_id {
            return Some((vec![stage.label().to_owned()], stage.label().to_owned()));
        }
        for job in &stage.jobs {
            if job.id == element_id {
                return Some((
                    vec![stage.label().to_owned(), job.label().to_owned()],
                    job.label().to_owned(),
                ));
            }
            if let Some(step) = job.steps.iter().find(|s| s.id == element_id) {
                return Some((
                    vec![stage.label().to_owned(), job.label().to_owned(), step.label()],
                    step.label(),
                ));
            }
        }
    }
    if let Some(arrow) = graph::of_stages(model)
        .edges
        .into_iter()
        .find(|e| mapper::edge_id(e) == element_id)
    {
        let waiting = model.stages.iter().find(|s| s.id == arrow.to_id)?;
        let waited_for = model
            .stages
            .iter()
            .find(|s| s.id == arrow.from_id)
            .map_or(arrow.from_name.clone(), |s| s.label().to_owned());
        return Some((
            vec![waiting.label().to_owned()],
            format!("{} waits for {waited_for}", waiting.label()),
        ));
    }
    for stage in &model.stages {
        let Some(arrow) = graph::of_jobs(stage)
            .edges
            .into_iter()
            .find(|e| mapper::edge_id(e) == element_id)
        else {
            continue;
        };
        let waiting = stage.jobs.iter().find(|j| j.id == arrow.to_id)?;
        let waited_for = stage
            .jobs
            .iter()
            .find(|j| j.id == arrow.from_id)
            .map_or(arrow.from_name.clone(), |j| j.label().to_owned());
        return Some((
            vec![stage.label().to_owned(), waiting.label().to_owned()],
            format!("{} waits for {waited_for}", waiting.label()),
        ));
    }
    None
}

impl ModuleView for PipelineView {
    fn render(&self, documents: &DiagramDocuments, _viewport: DiagramViewport) -> Result<Vec<DiagramElement>, String> {
        let document = self.document(documents)?;
        Ok(mapper::visible(&document.model, &self.expanded.borrow()))
    }

    fn describe(&self, documents: &DiagramDocuments, element_id: &str) -> Option<(Vec<String>, String)> {
        let document = self.document(documents).ok()?;
        if !document.is_usable() {
            return None;
        }
        find(&document.model, element_id)
    }

    fn toolbox(&self) -> Vec<ToolboxItem> {
        toolbox::items()
    }

    fn problems(&self, documents: &DiagramDocuments) -> Vec<DiagramProblem> {
        self.document(documents)
            .map(|document| rules::validate(&document.text))
            .unwrap_or_default()
    }

    fn actions(&self, documents: &DiagramDocuments, target: &ActionTarget) -> Vec<ActionGroup> {
        let (Ok(document), ActionTarget::Element(id)) = (self.document(documents), target) else {
            return Vec::new();
        };
        if !document.is_usable() {
            return Vec::new();
        }
        actions::discover(&document.model, id, &|open| self.is_expanded(open))
    }

    fn execute(&self, workspace: &mut DiagramWorkspace, target: &ActionTarget, action_id: &str) -> ActionOutcome {
        if !offered(&self.actions(workspace.documents(), target), action_id) {
            return ActionOutcome::Refused(NOT_AVAILABLE.to_owned());
        }
        let ActionTarget::Element(id) = target else {
            return ActionOutcome::Refused(NOT_AVAILABLE.to_owned());
        };
        let model = match self.usable(workspace.documents()) {
            Ok(document) => document.model.clone(),
            Err(refusal) => return ActionOutcome::Refused(refusal),
        };
        match actions::run(&model, id, action_id) {
            Err(refusal) => ActionOutcome::Refused(refusal),
            Ok(Run::Toggle(id)) => {
                self.toggle(&id);
                ActionOutcome::Done
            }
            Ok(Run::Ask(prompt)) => ActionOutcome::Prompt(prompt),
            Ok(Run::Edit(edit)) => match execute(workspace, &self.command(edit)) {
                Ok(()) => ActionOutcome::Done,
                Err(refusal) => ActionOutcome::Refused(refusal),
            },
        }
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
        let ActionTarget::Element(id) = target else {
            return ActionOutcome::Refused(format!("'{action_id}' does not apply to this selection."));
        };
        match actions::commit(&model, id, action_id, value) {
            Err(refusal) => ActionOutcome::Refused(refusal),
            Ok(edit) => match execute(workspace, &self.command(edit)) {
                Ok(()) => ActionOutcome::Done,
                Err(refusal) => ActionOutcome::Refused(refusal),
            },
        }
    }

    fn properties(&self, documents: &DiagramDocuments, element_id: &str) -> Vec<PropertyRow> {
        match self.document(documents) {
            Ok(document) if document.is_usable() => properties::describe(&document.model, element_id),
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

    fn move_elements(&self, _workspace: &mut DiagramWorkspace, _moves: &[(String, f64, f64)]) -> Result<(), String> {
        Err(PLACEMENT_REFUSAL.to_owned())
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
