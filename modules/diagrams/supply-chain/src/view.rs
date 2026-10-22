//! One open supply chain diagram, and what is asked of it (etalii.adp spec 030, task T076).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/supply-chain/backend/EtAlii.Adp.Diagram.SupplyChain/
//! SupplyChainSession.cs` (a drag is one move, a node placed where it was dropped and joining the group whose frame it
//! was dropped in, a group moved with every member; a re-parent refused with its sentence),
//! `SupplyChainContextSourceResolver.cs` (what a selected group, node or flow reads as) and `SupplyChainValidator.cs`,
//! with the action and property providers of [`crate::actions`] and [`crate::properties`]. Positions are in canvas
//! units: an element is drawn at its centre, and a move names the top-left the document holds, as the standalone
//! host's canvas sent it.

use std::path::{Path, PathBuf};

use adp_context::{ActionGroup, PropertyRow};
use adp_diagram::session::{ActionOutcome, ActionTarget};
use adp_diagram::{
    DiagramDocuments, DiagramElement, DiagramProblem, DiagramViewport, DiagramWorkspace, ModuleView, ToolboxItem,
};
use adp_disl::IdSource;
use adp_history::{Command, Composite, Outcome};

use crate::actions::{self, Run, Subject};
use crate::commands::{SupplyChainCommand, SupplyChainEdit};
use crate::mapper;
use crate::store::{self, SupplyChainDocument};
use crate::trace::SupplyChainTrace;
use crate::validator;
use crate::{ShortGuids, toolbox};

/// What dragging an element onto another answers: a node joins a group by being dropped inside its frame.
pub const PARENT_REFUSAL: &str =
    "A node joins a group by being dragged inside its frame, or through its Group property.";

/// One open view of a `.supply` document.
#[derive(Debug, Clone)]
pub struct SupplyChainView {
    body_path: PathBuf,
}

impl SupplyChainView {
    pub fn new(body_path: &Path) -> Self {
        SupplyChainView {
            body_path: body_path.to_path_buf(),
        }
    }

    pub fn body_path(&self) -> &Path {
        &self.body_path
    }

    fn document<'a>(&self, documents: &'a DiagramDocuments) -> Result<&'a SupplyChainDocument, String> {
        store::get(documents, &self.body_path).ok_or_else(|| adp_diagram::document::NOT_OPEN.to_owned())
    }

    /// The document, when it could be read; else what an action on it answers (`DiagramActionProvider.Unreadable`).
    fn usable<'a>(&self, documents: &'a DiagramDocuments) -> Result<&'a SupplyChainDocument, String> {
        let document = self.document(documents)?;
        if !document.is_usable() {
            return Err(format!(
                "{} could not be read, so it cannot be edited.",
                file_name(&self.body_path)
            ));
        }
        Ok(document)
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Runs `command` on the project's history.
fn execute(workspace: &mut DiagramWorkspace, command: &dyn Command<DiagramDocuments>) -> Result<(), String> {
    match workspace.execute_command(command) {
        Outcome::Done { .. } => Ok(()),
        Outcome::Refused(refusal) => Err(refusal.0),
    }
}

/// What an action's target names: an entry, a drop on the canvas, or a finished connect gesture.
fn subject_of(target: &ActionTarget) -> Option<Subject> {
    match target {
        ActionTarget::Element(id) => Some(Subject::Entry(id.clone())),
        ActionTarget::Canvas { x, y } => Some(Subject::Placement(*x, *y)),
        ActionTarget::Connection {
            source: Some(source),
            target: Some(target),
        } => Some(Subject::Relation(source.clone(), target.clone())),
        ActionTarget::Connection { .. } => None,
    }
}

/// What an action its target does not offer, or offers unavailable, answers (the standalone host's
/// `ContextService.ExecuteAction`).
pub const NOT_AVAILABLE: &str = "That action is not available for this item.";

/// Whether `action_id` is on `menu` and available.
fn offered(menu: &[ActionGroup], action_id: &str) -> bool {
    menu.iter()
        .flat_map(|g| &g.actions)
        .any(|a| a.id == action_id && a.is_available())
}

fn minting(ids: &dyn IdSource) -> impl Fn(&str) -> Result<String, String> + '_ {
    move |type_name| ids.next(type_name)
}

impl ModuleView for SupplyChainView {
    fn render(&self, documents: &DiagramDocuments, viewport: DiagramViewport) -> Result<Vec<DiagramElement>, String> {
        let document = self.document(documents)?;
        Ok(mapper::visible(&document.model, viewport, &SupplyChainTrace::none()))
    }

    fn describe(&self, documents: &DiagramDocuments, element_id: &str) -> Option<(Vec<String>, String)> {
        let model = &self.document(documents).ok()?.model;
        let text = if let Some(group) = model.group_of(element_id) {
            if group.name.is_empty() { &group.id } else { &group.name }.clone()
        } else if let Some(node) = model.node_of(element_id) {
            if node.name.is_empty() { &node.id } else { &node.name }.clone()
        } else {
            let flow = model.flow_of(element_id)?;
            if flow.product.is_empty() {
                format!("{} → {}", flow.from, flow.to)
            } else {
                flow.product.clone()
            }
        };
        Some((vec![text.clone()], text))
    }

    fn toolbox(&self) -> Vec<ToolboxItem> {
        toolbox::items()
    }

    fn problems(&self, documents: &DiagramDocuments) -> Vec<DiagramProblem> {
        self.document(documents)
            .map(|document| validator::problems(&document.model))
            .unwrap_or_default()
    }

    fn actions(&self, documents: &DiagramDocuments, target: &ActionTarget) -> Vec<ActionGroup> {
        match (self.document(documents), subject_of(target)) {
            (Ok(document), Some(subject)) => actions::discover(&document.model, &subject),
            _ => Vec::new(),
        }
    }

    fn execute(&self, workspace: &mut DiagramWorkspace, target: &ActionTarget, action_id: &str) -> ActionOutcome {
        self.execute_with_ids(workspace, target, action_id, &ShortGuids)
    }

    fn execute_with_ids(
        &self,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        ids: &dyn IdSource,
    ) -> ActionOutcome {
        let document = match self.usable(workspace.documents()) {
            Ok(document) => document,
            Err(refusal) => return ActionOutcome::Refused(refusal),
        };
        let Some(subject) = subject_of(target) else {
            return ActionOutcome::Refused("A flow is drawn from one node to another.".to_owned());
        };
        // Executing an action by id finds only the actions its target offers.
        if !offered(&actions::discover(&document.model, &subject), action_id) {
            return ActionOutcome::Refused(NOT_AVAILABLE.to_owned());
        }
        let model = document.model.clone();
        let run = actions::run(&model, &self.body_path, &subject, action_id, &minting(ids));
        match run {
            Err(refusal) => ActionOutcome::Refused(refusal),
            Ok(Run::Ask(prompt)) => ActionOutcome::Prompt(prompt),
            Ok(Run::Completed) => ActionOutcome::Done,
            Ok(Run::Command(command)) => match execute(workspace, &command) {
                Ok(()) => ActionOutcome::Done,
                Err(refusal) => ActionOutcome::Refused(refusal),
            },
        }
    }

    /// A double-click on a stage or a group renames it (`SupplyChainCanvas.tsx`: the `activate` gesture invokes its
    /// rename on the element types of its stages and its group).
    fn double_click(&self, documents: &DiagramDocuments, element_id: &str) -> Option<String> {
        let model = &self.document(documents).ok()?.model;
        (model.node_of(element_id).is_some() || model.group_of(element_id).is_some())
            .then(|| actions::RENAME.to_owned())
    }

    /// A right drag from one element to another draws a connection (`SupplyChainCanvas.tsx`, `connectOnRightDrag: true`).
    fn connects_on_right_drag(&self) -> bool {
        true
    }

    /// A flow drawn from one card to another is the menu's Add flow on the finished gesture (`onConnectionDrawn`).
    fn connect_action(
        &self,
        _documents: &DiagramDocuments,
        _source: &str,
        _target: &str,
    ) -> Option<Result<String, String>> {
        Some(Ok(actions::CONNECT.to_owned()))
    }

    fn answer(
        &self,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        value: &str,
    ) -> ActionOutcome {
        self.answer_with_ids(workspace, target, action_id, value, &ShortGuids)
    }

    fn answer_with_ids(
        &self,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        value: &str,
        ids: &dyn IdSource,
    ) -> ActionOutcome {
        let document = match self.usable(workspace.documents()) {
            Ok(document) => document,
            Err(refusal) => return ActionOutcome::Refused(refusal),
        };
        if let Err(refusal) = actions::validate(action_id, value) {
            return ActionOutcome::Refused(refusal);
        }
        let Some(subject) = subject_of(target) else {
            return ActionOutcome::Refused(format!("'{action_id}' does not apply to this selection."));
        };
        let model = document.model.clone();
        match actions::commit(&model, &self.body_path, &subject, action_id, value, &minting(ids)) {
            Err(refusal) => ActionOutcome::Refused(refusal),
            Ok(command) => match execute(workspace, &command) {
                Ok(()) => ActionOutcome::Done,
                Err(refusal) => ActionOutcome::Refused(refusal),
            },
        }
    }

    fn properties(&self, documents: &DiagramDocuments, element_id: &str) -> Vec<PropertyRow> {
        self.document(documents)
            .map(|document| crate::properties::describe(&document.model, element_id))
            .unwrap_or_default()
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
            commands.push(Box::new(crate::properties::change(
                &document.model,
                &self.body_path,
                id,
                property_id,
                value,
            )?));
        }
        match commands.len() {
            0 => Ok(()),
            1 => execute(workspace, commands[0].as_ref()),
            n => execute(workspace, &Composite::new(format!("Change {n} elements"), commands)),
        }
    }

    /// Each move names the top-left the entry's frame or card is put at: one node, a group with its members, or
    /// several of them as one edit.
    fn move_elements(&self, workspace: &mut DiagramWorkspace, moves: &[(String, f64, f64)]) -> Result<(), String> {
        self.usable(workspace.documents())?;
        let commands: Vec<Box<dyn Command<DiagramDocuments>>> = moves
            .iter()
            .map(|(id, x, y)| {
                Box::new(SupplyChainCommand::new(
                    &self.body_path,
                    SupplyChainEdit::Move {
                        entry_id: id.clone(),
                        x: *x,
                        y: *y,
                    },
                )) as Box<dyn Command<DiagramDocuments>>
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
