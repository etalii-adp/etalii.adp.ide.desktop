//! One open functional decomposition graph, and what the session asks of it (etalii.adp spec 030, task T072).
//!
//! Ported from the standalone host at `8b6ccfc`: the module's `FdgSession.cs` (a drag places an element by its
//! top-left, a connection follows its ends and is not moved, nothing has a parent to move under),
//! `FdgContextSourceResolver.cs` (what a selection reads as: an element's name, a Comment's first line, a
//! connection's name or its ends) and `FdgValidator.cs`, with the menus of [`crate::actions`], the rows of
//! [`crate::properties`] and the palette of [`crate::mapper`]. Every edit is a command on the project's history.

use std::path::{Path, PathBuf};

use adp_context::{ActionGroup, PropertyRow};
use adp_diagram::session::{ActionOutcome, ActionTarget};
use adp_diagram::{
    DiagramDocuments, DiagramElement, DiagramProblem, DiagramViewport, DiagramWorkspace, ModuleView, ToolboxItem,
};
use adp_disl::IdSource;
use adp_history::{Command, Composite, Outcome};

use crate::actions::{self, FdgTarget, Run};
use crate::commands::{FdgCommand, FdgEdit};
use crate::mapper;
use crate::properties;
use crate::relations;
use crate::rules;
use crate::store::{self, FdgDocument};

/// What a drag of something that is not an element answers.
pub const NOT_MOVABLE: &str = "That is not something this graph can move.";

/// What a drag onto another element answers: an element's parent is a connection.
pub const PARENT_REFUSAL: &str =
    "An element here has no parent to move it under; its parent is a connection, drawn from the parent to it.";

/// One open view of a `.fdg`.
#[derive(Debug, Clone)]
pub struct FdgView {
    body_path: PathBuf,
}

impl FdgView {
    pub fn new(body_path: &Path) -> Self {
        FdgView {
            body_path: body_path.to_path_buf(),
        }
    }

    pub fn body_path(&self) -> &Path {
        &self.body_path
    }

    fn document<'a>(&self, documents: &'a DiagramDocuments) -> Result<&'a FdgDocument, String> {
        store::get(documents, &self.body_path).ok_or_else(|| adp_diagram::document::NOT_OPEN.to_owned())
    }

    /// The document, when it can be edited; else the sentence an edit of it answers.
    fn usable<'a>(&self, documents: &'a DiagramDocuments) -> Result<&'a FdgDocument, String> {
        let document = self.document(documents)?;
        if let Some(reason) = &document.unreadable {
            return Err(format!(
                "{} could not be read, so nothing can be edited until it can: {reason}",
                self.body_path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            ));
        }
        Ok(document)
    }

    /// The relation a drag from `source`'s body to `target`'s draws, as the client's `relationOnto` infers it from
    /// the pair: the connect action of the one relation whose ends admit both, or why there is none.
    pub fn connect_action(&self, documents: &DiagramDocuments, source: &str, target: &str) -> Result<String, String> {
        let model = &self.document(documents)?.model;
        let (Some(from), Some(to)) = (model.element_of(source), model.element_of(target)) else {
            return Err(relations::NOTHING_CONNECTS.to_owned());
        };
        relations::onto(&from.element_type, &to.element_type)
            .map(|r| actions::connect_action_id(r.id))
            .ok_or_else(|| relations::NOTHING_CONNECTS.to_owned())
    }

    /// Runs `action_id` on `target`, `ids` giving the id of what an add or a connect makes.
    pub fn run(
        &self,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        ids: Option<&dyn IdSource>,
    ) -> ActionOutcome {
        let document = match self.usable(workspace.documents()) {
            Ok(document) => document,
            Err(refusal) => return ActionOutcome::Refused(refusal),
        };
        let Some(target) = fdg_target(target) else {
            return ActionOutcome::Refused(actions::CONNECT_BY_GESTURE.to_owned());
        };
        if !actions::offers(&document.model, &target, action_id) {
            return ActionOutcome::Refused(format!("'{action_id}' does not apply to this selection."));
        }
        let model = document.model.clone();
        let mut mint = |type_name: &str| match ids {
            Some(ids) => ids.next(type_name),
            None => Ok(short_guid()),
        };
        match actions::run(&model, &self.body_path, &target, action_id, &mut mint) {
            Run::Command(command) => outcome(execute(workspace, &command)),
            Run::Ask(prompt) => ActionOutcome::Prompt(prompt),
            Run::Refused(refusal) => ActionOutcome::Refused(refusal),
        }
    }
}

/// A new id: a ShortGuid, a version 4 GUID written as 25 characters of base 36, as the standalone host's
/// `ShortGuid.NewShortGuid()` mints them.
pub fn short_guid() -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut value = uuid::Uuid::new_v4().as_u128();
    let mut digits = [b'0'; 25];
    for digit in digits.iter_mut().rev() {
        *digit = DIGITS[(value % 36) as usize];
        value /= 36;
    }
    String::from_utf8(digits.to_vec()).expect("base 36 digits")
}

/// What an action's target names here.
fn fdg_target(target: &ActionTarget) -> Option<FdgTarget<'_>> {
    Some(match target {
        ActionTarget::Element(id) => FdgTarget::Item(id),
        ActionTarget::Canvas { x, y } => FdgTarget::Placement { x: *x, y: *y },
        ActionTarget::Connection {
            source: Some(source),
            target: Some(target),
        } => FdgTarget::Relation {
            from: source,
            to: target,
        },
        ActionTarget::Connection { .. } => return None,
    })
}

/// Runs `command` on the project's history.
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

/// Runs `commands` as one step: one command, or a composite named `label` of several.
fn execute_all(workspace: &mut DiagramWorkspace, label: &str, commands: Vec<FdgCommand>) -> Result<(), String> {
    match commands.len() {
        0 => Ok(()),
        1 => execute(workspace, &commands[0]),
        n => {
            let parts = commands
                .into_iter()
                .map(|c| Box::new(c) as Box<dyn Command<DiagramDocuments>>)
                .collect();
            execute(workspace, &Composite::new(format!("{label} {n} elements"), parts))
        }
    }
}

/// The first line of a Comment's text, trimmed, or `otherwise` when it is empty.
fn first_line_of(text: &str, otherwise: &str) -> String {
    let first = text
        .split('\n')
        .next()
        .unwrap_or_default()
        .trim_end_matches('\r')
        .trim();
    if first.is_empty() {
        otherwise.to_owned()
    } else {
        first.to_owned()
    }
}

impl ModuleView for FdgView {
    fn render(&self, documents: &DiagramDocuments, viewport: DiagramViewport) -> Result<Vec<DiagramElement>, String> {
        Ok(mapper::visible(&self.document(documents)?.model, viewport))
    }

    fn describe(&self, documents: &DiagramDocuments, element_id: &str) -> Option<(Vec<String>, String)> {
        let model = &self.document(documents).ok()?.model;
        let text = if let Some(element) = model.element_of(element_id) {
            if element.is_comment() {
                first_line_of(&element.text, "Comment")
            } else if element.name.is_empty() {
                element.id.clone()
            } else {
                element.name.clone()
            }
        } else {
            let connection = model.connection_of(element_id)?;
            if connection.name.is_empty() {
                format!("{} → {}", connection.from, connection.to)
            } else {
                connection.name.clone()
            }
        };
        Some((vec![text.clone()], text))
    }

    fn toolbox(&self) -> Vec<ToolboxItem> {
        mapper::toolbox()
    }

    fn problems(&self, documents: &DiagramDocuments) -> Vec<DiagramProblem> {
        self.document(documents)
            .map(|document| rules::problems_of(&document.text))
            .unwrap_or_default()
    }

    fn actions(&self, documents: &DiagramDocuments, target: &ActionTarget) -> Vec<ActionGroup> {
        match (self.document(documents), fdg_target(target)) {
            (Ok(document), Some(target)) => actions::discover(&document.model, &target),
            _ => Vec::new(),
        }
    }

    fn execute(&self, workspace: &mut DiagramWorkspace, target: &ActionTarget, action_id: &str) -> ActionOutcome {
        self.run(workspace, target, action_id, None)
    }

    fn execute_with_ids(
        &self,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        ids: &dyn IdSource,
    ) -> ActionOutcome {
        self.run(workspace, target, action_id, Some(ids))
    }

    /// A right drag from one element to another draws a connection (`FdgCanvas.tsx`, `connectOnRightDrag: true`).
    fn connects_on_right_drag(&self) -> bool {
        true
    }

    fn connect_action(
        &self,
        documents: &DiagramDocuments,
        source: &str,
        target: &str,
    ) -> Option<Result<String, String>> {
        Some(FdgView::connect_action(self, documents, source, target))
    }

    fn answer(
        &self,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        value: &str,
    ) -> ActionOutcome {
        let document = match self.usable(workspace.documents()) {
            Ok(document) => document,
            Err(refusal) => return ActionOutcome::Refused(refusal),
        };
        let command = match target {
            ActionTarget::Element(id) => actions::commit(&document.model, &self.body_path, id, action_id, value),
            _ => None,
        };
        match command {
            Some(command) => outcome(execute(workspace, &command)),
            None => ActionOutcome::Refused(format!("'{action_id}' does not apply to this selection.")),
        }
    }

    fn properties(&self, documents: &DiagramDocuments, element_id: &str) -> Vec<PropertyRow> {
        self.document(documents)
            .map(|document| properties::describe(&document.model, element_id))
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
        let mut commands = Vec::with_capacity(element_ids.len());
        for id in element_ids {
            // A set reaches only a row the element describes, as the standalone host's context service let it.
            if !properties::describe(&document.model, id)
                .iter()
                .any(|r| r.id == property_id)
            {
                return Err(format!("'{property_id}' cannot be edited on this selection."));
            }
            commands.push(properties::change(
                &document.model,
                &self.body_path,
                id,
                property_id,
                value,
            )?);
        }
        execute_all(workspace, "Change", commands)
    }

    fn move_elements(&self, workspace: &mut DiagramWorkspace, moves: &[(String, f64, f64)]) -> Result<(), String> {
        let document = self.document(workspace.documents())?;
        let mut commands = Vec::with_capacity(moves.len());
        for (id, x, y) in moves {
            if document.model.element_of(id).is_none() {
                return Err(NOT_MOVABLE.to_owned());
            }
            commands.push(FdgCommand::new(
                &self.body_path,
                FdgEdit::Place {
                    element_id: id.clone(),
                    x: *x,
                    y: *y,
                },
            ));
        }
        execute_all(workspace, "Move", commands)
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
