//! One open agent activity diagram, and what is asked of it (etalii.adp spec 030, task T075).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/agent-activity-diagram/backend/EtAlii.Adp.Diagram.
//! AgentActivityDiagram/AadSession.cs` (a drop locks an element where it is let go, through the project's history; a
//! re-parent refused with its sentence), `AadContextSourceResolver.cs` (what a selected element, row, relation, the
//! diagram's switch or a group reads as) and `AadProviders.cs`, with the action and property providers of
//! [`crate::actions`] and [`crate::properties`]. Positions are centres in canvas units: a locked position is a centre.

use std::path::{Path, PathBuf};

use adp_context::{ActionGroup, PropertyRow};
use adp_diagram::session::{ActionOutcome, ActionTarget};
use adp_diagram::{
    DiagramDocuments, DiagramElement, DiagramProblem, DiagramViewport, DiagramWorkspace, ModuleView, ToolboxItem,
};
use adp_disl::IdSource;
use adp_history::{Command, Composite, Outcome};

use crate::actions::{self, PIN, Run, Subject, VIEW_ID, parse_group_target};
use crate::commands::{AadCommand, AadEdit};
use crate::definition;
use crate::mapper;
use crate::store::{self, AadDocument};
use crate::validator;

/// What dragging an element onto another answers.
pub const PARENT_REFUSAL: &str =
    "An element of this diagram has no parent to move it under; it is locked where it is dropped.";

/// What an action its target does not offer, or offers unavailable, answers.
pub const NOT_AVAILABLE: &str = "That action is not available for this item.";

/// One open view of an `.aad` document.
#[derive(Debug, Clone)]
pub struct AgentActivityView {
    body_path: PathBuf,
}

impl AgentActivityView {
    pub fn new(body_path: &Path) -> Self {
        AgentActivityView {
            body_path: body_path.to_path_buf(),
        }
    }

    pub fn body_path(&self) -> &Path {
        &self.body_path
    }

    fn document<'a>(&self, documents: &'a DiagramDocuments) -> Result<&'a AadDocument, String> {
        store::get(documents, &self.body_path).ok_or_else(|| adp_diagram::document::NOT_OPEN.to_owned())
    }

    /// The document, when it could be read; else what an action on it answers.
    fn usable<'a>(&self, documents: &'a DiagramDocuments) -> Result<&'a AadDocument, String> {
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

fn execute(workspace: &mut DiagramWorkspace, command: &dyn Command<DiagramDocuments>) -> Result<(), String> {
    match workspace.execute_command(command) {
        Outcome::Done { .. } => Ok(()),
        Outcome::Refused(refusal) => Err(refusal.0),
    }
}

fn outcome(result: Result<(), String>) -> ActionOutcome {
    match result {
        Ok(()) => ActionOutcome::Done,
        Err(refusal) => ActionOutcome::Refused(refusal),
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

impl ModuleView for AgentActivityView {
    fn render(&self, documents: &DiagramDocuments, _viewport: DiagramViewport) -> Result<Vec<DiagramElement>, String> {
        // Nothing is culled by the viewport: the layout needs every element to place any of them.
        Ok(mapper::visible(self.document(documents)?))
    }

    fn describe(&self, documents: &DiagramDocuments, element_id: &str) -> Option<(Vec<String>, String)> {
        let model = &self.document(documents).ok()?.model;
        let named = |text: &str, id: &str| {
            if text.is_empty() {
                id.to_owned()
            } else {
                text.to_owned()
            }
        };
        let text = if element_id == VIEW_ID {
            "Diagram".to_owned()
        } else if let Some(element) = model.element(element_id) {
            named(&element.name, &element.id)
        } else if let Some(row) = model.elements.iter().flat_map(|e| &e.rows).find(|r| r.id == element_id) {
            named(&row.title, &row.id)
        } else if let Some(relation) = model.relations.iter().find(|r| r.id == element_id) {
            let name = |id: &str| {
                model
                    .element(id)
                    .map_or_else(|| id.to_owned(), |e| named(&e.name, &e.id))
            };
            format!("{} → {}", name(&relation.from), name(&relation.to))
        } else {
            let (owner, _) = parse_group_target(element_id)?;
            model.element(owner)?;
            "Group".to_owned()
        };
        Some((vec![text.clone()], text))
    }

    fn toolbox(&self) -> Vec<ToolboxItem> {
        definition::get().definition.toolbox().to_vec()
    }

    fn problems(&self, documents: &DiagramDocuments) -> Vec<DiagramProblem> {
        self.document(documents)
            .map(|document| validator::validate(&document.body))
            .unwrap_or_default()
    }

    fn actions(&self, documents: &DiagramDocuments, target: &ActionTarget) -> Vec<ActionGroup> {
        match (self.document(documents), subject_of(target)) {
            (Ok(document), Some(subject)) => actions::discover(document, &subject),
            _ => Vec::new(),
        }
    }

    fn execute(&self, workspace: &mut DiagramWorkspace, target: &ActionTarget, action_id: &str) -> ActionOutcome {
        self.execute_with_ids(workspace, target, action_id, &adp_disl::behavior::ids::short_guids())
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
            return ActionOutcome::Refused("A relation is drawn from one element to another.".to_owned());
        };
        // Executing an action by id finds only the actions its target offers.
        if !actions::is_offered(&actions::discover(document, &subject), action_id) {
            return ActionOutcome::Refused(NOT_AVAILABLE.to_owned());
        }
        // Lock position: the element is locked where the layout drew it, which the core knows here.
        if action_id == PIN
            && let Subject::Entry(id) = &subject
        {
            let Some(drawn) = mapper::visible(document).into_iter().find(|e| e.id == *id) else {
                return ActionOutcome::Refused(definition::GONE.to_owned());
            };
            let command = AadCommand::new(
                &self.body_path,
                AadEdit::Pin {
                    element_id: id.clone(),
                    x: drawn.x,
                    y: drawn.y,
                },
            );
            return outcome(execute(workspace, &command));
        }
        let mint = |type_name: &str| ids.next(type_name);
        match actions::run(document, &self.body_path, &subject, action_id, &mint) {
            Err(refusal) => ActionOutcome::Refused(refusal),
            Ok(Run::Ask(prompt)) => ActionOutcome::Prompt(prompt),
            Ok(Run::Completed) => ActionOutcome::Done,
            Ok(Run::Command(command)) => outcome(execute(workspace, &command)),
        }
    }

    /// A right drag from one element's body to another draws a relation (`aadDefinition.ts`, `extras`:
    /// `connectOnRightDrag: true`): the vendored `agent-activity-diagram.dis` declares no pointer binding, so the
    /// module says it.
    fn connects_on_right_drag(&self) -> bool {
        true
    }

    /// A line drawn from one element to another is the relation the pair allows, whichever way it was drawn.
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
        self.answer_with_ids(
            workspace,
            target,
            action_id,
            value,
            &adp_disl::behavior::ids::short_guids(),
        )
    }

    fn answer_with_ids(
        &self,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        value: &str,
        _ids: &dyn IdSource,
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
        match actions::commit(document, &self.body_path, &subject, action_id, value) {
            Err(refusal) => ActionOutcome::Refused(refusal),
            Ok(command) => outcome(execute(workspace, &command)),
        }
    }

    fn properties(&self, documents: &DiagramDocuments, element_id: &str) -> Vec<PropertyRow> {
        self.document(documents)
            .map(|document| crate::properties::describe(document, element_id))
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
                document,
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

    /// A drag locks each element where it was let go, its centre, through the project's history.
    fn move_elements(&self, workspace: &mut DiagramWorkspace, moves: &[(String, f64, f64)]) -> Result<(), String> {
        self.usable(workspace.documents())?;
        let commands: Vec<Box<dyn Command<DiagramDocuments>>> = moves
            .iter()
            .map(|(id, x, y)| {
                Box::new(AadCommand::new(
                    &self.body_path,
                    AadEdit::Pin {
                        element_id: id.clone(),
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
