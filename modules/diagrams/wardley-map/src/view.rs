//! One open Wardley map, and what is asked of it (etalii.adp spec 030, task T070).
//!
//! Ported from the standalone host at `8b6ccfc`: `WardleySession.cs` (what is drawn is read with the identities of
//! the session's document; a drop moves the element to a position on the map's axes; a drag never re-parents),
//! `WardleyContextSourceResolver.cs` (what a selection reads as), `WardleyValidator.cs`, and the action, property and
//! toolbox providers of [`crate::actions`], [`crate::properties`] and [`crate::toolbox`]. Every edit is a command on
//! the project's history.

use std::path::{Path, PathBuf};

use adp_context::{ActionGroup, PropertyRow};
use adp_diagram::session::{ActionOutcome, ActionTarget};
use adp_diagram::{
    DiagramDocuments, DiagramElement, DiagramProblem, DiagramViewport, DiagramWorkspace, ModuleView, ToolboxItem,
};
use adp_history::{Command, Composite, Outcome};

use crate::actions::{self, Run, WardleySubject};
use crate::axis;
use crate::commands::MoveWardleyElement;
use crate::descriptions;
use crate::mapper;
use crate::properties;
use crate::rules;
use crate::store::{self, WardleyDocument};
use crate::toolbox;

/// What dragging a component onto another answers: where it sits is what a drag changes.
pub const PARENT_REFUSAL: &str = "Dragging a component changes where it sits on the map, not what contains it.";

/// One open view of a Wardley map.
#[derive(Debug, Clone)]
pub struct WardleyView {
    root: PathBuf,
    body_path: PathBuf,
}

impl WardleyView {
    /// The view of the map at `body_path` in the project at `root`.
    pub fn new(root: &Path, body_path: &Path) -> Self {
        WardleyView {
            root: root.to_path_buf(),
            body_path: body_path.to_path_buf(),
        }
    }

    pub fn body_path(&self) -> &Path {
        &self.body_path
    }

    fn document<'a>(&self, documents: &'a DiagramDocuments) -> Result<&'a WardleyDocument, String> {
        store::get(documents, &self.body_path).ok_or_else(|| adp_diagram::document::NOT_OPEN.to_owned())
    }

    fn subject<'a>(&'a self, documents: &'a DiagramDocuments, element_id: &'a str) -> Option<WardleySubject<'a>> {
        let document = self.document(documents).ok()?;
        Some(WardleySubject {
            body_path: &self.body_path,
            map: &document.map,
            identities: store::identities_of(documents, &self.body_path),
            element_id,
        })
    }

    fn usable(&self, documents: &DiagramDocuments) -> bool {
        self.document(documents).is_ok_and(WardleyDocument::is_usable)
    }

    /// What an edit of a map that could not be read answers.
    fn unreadable(&self) -> String {
        format!(
            "{} could not be read, so it cannot be edited.",
            self.body_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        )
    }
}

/// Runs `command` on the project's history.
fn execute(workspace: &mut DiagramWorkspace, command: &dyn Command<DiagramDocuments>) -> Result<(), String> {
    match workspace.execute_command(command) {
        Outcome::Done { .. } => Ok(()),
        Outcome::Refused(refusal) => Err(refusal.0),
    }
}

/// The element id an action's target names: an element, or the empty canvas as the map itself.
fn target_id(target: &ActionTarget) -> Option<&str> {
    match target {
        ActionTarget::Element(id) => Some(id),
        ActionTarget::Canvas { .. } => Some(""),
        ActionTarget::Connection { .. } => None,
    }
}

fn does_not_apply(action_id: &str) -> ActionOutcome {
    ActionOutcome::Refused(format!("'{action_id}' does not apply to this selection."))
}

impl ModuleView for WardleyView {
    fn render(&self, documents: &DiagramDocuments, viewport: DiagramViewport) -> Result<Vec<DiagramElement>, String> {
        let document = self.document(documents)?;
        Ok(mapper::visible(
            &document.map,
            store::identities_of(documents, &self.body_path),
            viewport,
        ))
    }

    fn describe(&self, documents: &DiagramDocuments, element_id: &str) -> Option<(Vec<String>, String)> {
        let document = self.document(documents).ok()?;
        let description = descriptions::of(
            &document.map,
            store::identities_of(documents, &self.body_path),
            element_id,
        )?;
        Some((description.path, description.text))
    }

    fn toolbox(&self) -> Vec<ToolboxItem> {
        toolbox::items()
    }

    fn problems(&self, documents: &DiagramDocuments) -> Vec<DiagramProblem> {
        self.document(documents)
            .map(|document| rules::validate_document(&document.text, &self.root))
            .unwrap_or_default()
    }

    fn actions(&self, documents: &DiagramDocuments, target: &ActionTarget) -> Vec<ActionGroup> {
        let usable = self.usable(documents);
        target_id(target)
            .and_then(|id| self.subject(documents, id))
            .map(|subject| actions::offered(&subject, usable))
            .unwrap_or_default()
    }

    fn execute(&self, workspace: &mut DiagramWorkspace, target: &ActionTarget, action_id: &str) -> ActionOutcome {
        let usable = self.usable(workspace.documents());
        let run = match target_id(target).and_then(|id| self.subject(workspace.documents(), id)) {
            Some(subject) => actions::run(&subject, usable, action_id),
            None => return does_not_apply(action_id),
        };
        match run {
            Run::Prompt(prompt) => ActionOutcome::Prompt(prompt),
            Run::Refused(refusal) => ActionOutcome::Refused(refusal),
            Run::Command(command) => match execute(workspace, command.as_ref()) {
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
        if !self.usable(workspace.documents()) {
            return ActionOutcome::Refused(self.unreadable());
        }
        if let Err(refusal) = actions::validate(action_id, value) {
            return ActionOutcome::Refused(refusal);
        }
        let command = target_id(target)
            .and_then(|id| self.subject(workspace.documents(), id))
            .and_then(|subject| actions::command(&subject, action_id, value));
        let Some(command) = command else {
            return does_not_apply(action_id);
        };
        match execute(workspace, command.as_ref()) {
            Ok(()) => ActionOutcome::Done,
            Err(refusal) => ActionOutcome::Refused(refusal),
        }
    }

    fn properties(&self, documents: &DiagramDocuments, element_id: &str) -> Vec<PropertyRow> {
        self.subject(documents, element_id)
            .map(|subject| properties::describe(&subject))
            .unwrap_or_default()
    }

    fn set_property(
        &self,
        workspace: &mut DiagramWorkspace,
        element_ids: &[&str],
        property_id: &str,
        value: &str,
    ) -> Result<(), String> {
        if !self.usable(workspace.documents()) {
            return Err(self.unreadable());
        }
        let mut commands = Vec::with_capacity(element_ids.len());
        for id in element_ids {
            let subject = self
                .subject(workspace.documents(), id)
                .ok_or_else(|| adp_diagram::document::NOT_OPEN.to_owned())?;
            commands.push(properties::change(&subject, property_id, value)?);
        }
        match commands.len() {
            0 => Ok(()),
            1 => execute(workspace, commands[0].as_ref()),
            n => execute(workspace, &Composite::new(format!("Change {n} elements"), commands)),
        }
    }

    /// Places each element at its point in the map's 0..1 space, converted back into the document's axes through the
    /// one function that may; the command clamps it to the map.
    fn move_elements(&self, workspace: &mut DiagramWorkspace, moves: &[(String, f64, f64)]) -> Result<(), String> {
        self.document(workspace.documents())?;
        let commands: Vec<Box<dyn Command<DiagramDocuments>>> = moves
            .iter()
            .map(|(id, x, y)| {
                let coordinate = axis::to_coordinate(*x, *y);
                Box::new(MoveWardleyElement {
                    body_path: self.body_path.clone(),
                    element_id: id.clone(),
                    visibility: coordinate.visibility,
                    maturity: coordinate.maturity,
                }) as Box<dyn Command<DiagramDocuments>>
            })
            .collect();
        match commands.len() {
            0 => Ok(()),
            1 => execute(workspace, commands[0].as_ref()),
            n => execute(workspace, &Composite::new(format!("Move {n} elements"), commands)),
        }
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
