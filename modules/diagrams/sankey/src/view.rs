//! One open Sankey diagram and what is asked of it (etalii.adp spec 030, task T077).
//!
//! Ported from the standalone host at `8b6ccfc`: `SankeySession.cs` over the core's `DocumentDiagramSession` (a drag
//! is a reorder: nothing in a Sankey diagram has a position of its own, so a node dropped higher or lower in its column
//! takes its place among the others there, and where across it was dropped does not matter), and
//! `SankeyContextSourceResolver.cs` (what a selected node or flow reads as).

use std::path::{Path, PathBuf};

use adp_context::{ActionGroup, PropertyRow};
use adp_diagram::session::{ActionOutcome, ActionTarget};
use adp_diagram::{
    DiagramDocuments, DiagramElement, DiagramProblem, DiagramViewport, DiagramWorkspace, ModuleView, ToolboxItem,
};
use adp_disl::IdSource;
use adp_history::{Command, Composite, Outcome};

use crate::actions::{self, Run};
use crate::commands::{SankeyCommand, SankeyEdit};
use crate::layout::SankeyLayout;
use crate::mapper;
use crate::model::SankeyModel;
use crate::rules;
use crate::store::{self, SankeyDocument};

/// What dragging a node onto another answers: a node takes its place by its column.
pub const PARENT_REFUSAL: &str = "A node takes its place by being dragged up or down its column.";

/// What a drag of something that is not a drawn node answers.
pub const NOT_MOVABLE: &str = "That is not something this diagram can move.";

/// One open view of a `.skv` document.
#[derive(Debug, Clone)]
pub struct SankeyDiagramView {
    body_path: PathBuf,
}

impl SankeyDiagramView {
    pub fn new(body_path: &Path) -> Self {
        SankeyDiagramView {
            body_path: body_path.to_path_buf(),
        }
    }

    pub fn body_path(&self) -> &Path {
        &self.body_path
    }

    fn document<'a>(&self, documents: &'a DiagramDocuments) -> Result<&'a SankeyDocument, String> {
        store::get(documents, &self.body_path).ok_or_else(|| adp_diagram::document::NOT_OPEN.to_owned())
    }

    fn model<'a>(&self, documents: &'a DiagramDocuments) -> Option<&'a SankeyModel> {
        self.document(documents).ok().map(|document| document.model.as_ref())
    }

    /// The document, when it could be read; else the sentence an action on it answers.
    fn usable<'a>(&self, documents: &'a DiagramDocuments) -> Result<&'a SankeyDocument, String> {
        let document = self.document(documents)?;
        if !document.is_usable() {
            return Err(format!(
                "{} could not be read, so it cannot be edited.",
                self.body_path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            ));
        }
        Ok(document)
    }

    /// Runs action `action_id` on `target`, minting the id of a node an add makes from `ids`.
    fn run(
        &self,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        new_id: impl FnOnce() -> Result<String, String>,
    ) -> ActionOutcome {
        let document = match self.usable(workspace.documents()) {
            Ok(document) => document,
            Err(refusal) => return ActionOutcome::Refused(refusal),
        };
        let model = document.model.clone();
        match actions::run(&model, &self.body_path, target, action_id, new_id) {
            Run::Command(command) => match execute(workspace, &command) {
                Ok(()) => ActionOutcome::Done,
                Err(refusal) => ActionOutcome::Refused(refusal),
            },
            Run::Prompt(prompt) => ActionOutcome::Prompt(prompt),
            Run::Refused(refusal) => ActionOutcome::Refused(refusal),
            Run::Nothing => ActionOutcome::Done,
        }
    }
}

/// Runs `command` on the project's history.
fn execute(workspace: &mut DiagramWorkspace, command: &dyn Command<DiagramDocuments>) -> Result<(), String> {
    match workspace.execute_command(command) {
        Outcome::Done { .. } => Ok(()),
        Outcome::Refused(refusal) => Err(refusal.0),
    }
}

/// Where a node dropped with its middle at `centre_y` goes: before the first other node of its column whose middle is
/// lower, or after the last one; none when that is where it already is (`SankeySession.PlaceOf`).
pub fn place_of(layout: &SankeyLayout, id: &str, centre_y: f64) -> Option<(String, bool)> {
    let column = &layout.column_order[layout.columns[id]];
    let others: Vec<&String> = column.iter().filter(|other| *other != id).collect();
    if others.is_empty() {
        return None;
    }
    let index = others
        .iter()
        .filter(|other| layout.boxes[other.as_str()].centre_y() < centre_y)
        .count();
    if Some(index) == column.iter().position(|other| other == id) {
        return None;
    }
    Some(match others.get(index) {
        Some(anchor) => ((*anchor).clone(), false),
        None => ((*others.last().expect("not empty")).clone(), true),
    })
}

impl ModuleView for SankeyDiagramView {
    fn render(&self, documents: &DiagramDocuments, viewport: DiagramViewport) -> Result<Vec<DiagramElement>, String> {
        Ok(mapper::visible(&self.document(documents)?.model, viewport))
    }

    fn describe(&self, documents: &DiagramDocuments, element_id: &str) -> Option<(Vec<String>, String)> {
        let model = self.model(documents)?;
        if let Some(node) = model.node_of(element_id) {
            return Some((vec![node.label().to_owned()], node.label().to_owned()));
        }
        let flow = model.flow_of(element_id)?;
        let label = |id: &str| model.node_of(id).map_or(id.to_owned(), |node| node.label().to_owned());
        let text = format!("{} → {}", label(&flow.from), label(&flow.to));
        Some((vec![text.clone()], text))
    }

    fn toolbox(&self) -> Vec<ToolboxItem> {
        actions::toolbox()
    }

    fn problems(&self, documents: &DiagramDocuments) -> Vec<DiagramProblem> {
        self.model(documents).map(rules::problems).unwrap_or_default()
    }

    fn actions(&self, documents: &DiagramDocuments, target: &ActionTarget) -> Vec<ActionGroup> {
        self.model(documents)
            .map(|model| actions::discover(model, target))
            .unwrap_or_default()
    }

    fn execute(&self, workspace: &mut DiagramWorkspace, target: &ActionTarget, action_id: &str) -> ActionOutcome {
        self.run(workspace, target, action_id, || Ok(short_guid()))
    }

    fn execute_with_ids(
        &self,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        ids: &dyn IdSource,
    ) -> ActionOutcome {
        self.run(workspace, target, action_id, || ids.next("node"))
    }

    /// A right drag from one element to another draws a connection (`SankeyCanvas.tsx`, `connectOnRightDrag: true`).
    fn connects_on_right_drag(&self) -> bool {
        true
    }

    /// A connect gesture between two nodes is Add flow (`onConnectionDrawn`); what it may not join, it refuses.
    fn connect_action(
        &self,
        documents: &DiagramDocuments,
        source: &str,
        target: &str,
    ) -> Option<Result<String, String>> {
        let _ = (documents, source, target);
        Some(Ok(actions::CONNECT.to_owned()))
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
        let model = document.model.clone();
        match actions::commit(&model, &self.body_path, target, action_id, value) {
            Ok(command) => match execute(workspace, &command) {
                Ok(()) => ActionOutcome::Done,
                Err(refusal) => ActionOutcome::Refused(refusal),
            },
            Err(refusal) => ActionOutcome::Refused(refusal),
        }
    }

    fn properties(&self, documents: &DiagramDocuments, element_id: &str) -> Vec<PropertyRow> {
        self.model(documents)
            .map(|model| actions::properties(model, element_id))
            .unwrap_or_default()
    }

    fn set_property(
        &self,
        workspace: &mut DiagramWorkspace,
        element_ids: &[&str],
        property_id: &str,
        value: &str,
    ) -> Result<(), String> {
        self.usable(workspace.documents())?;
        let mut commands: Vec<Box<dyn Command<DiagramDocuments>>> = Vec::new();
        for id in element_ids {
            if let Some(command) = actions::change(&self.body_path, id, property_id, value)? {
                commands.push(Box::new(command));
            }
        }
        match commands.len() {
            0 => Ok(()),
            1 => execute(workspace, commands[0].as_ref()),
            n => execute(workspace, &Composite::new(format!("Change {n} elements"), commands)),
        }
    }

    /// Each node to its place in its column, read from where its top left was dropped, one move after another.
    fn move_elements(&self, workspace: &mut DiagramWorkspace, moves: &[(String, f64, f64)]) -> Result<(), String> {
        for (id, _, y) in moves {
            let model = self.document(workspace.documents())?.model.clone();
            let layout = SankeyLayout::of(&model);
            let Some(bar) = layout.boxes.get(id) else {
                return Err(NOT_MOVABLE.to_owned());
            };
            // Dropped back where it was: nothing to write, and nothing to refuse.
            if let Some((anchor_id, after)) = place_of(&layout, id, y + bar.height / 2.0) {
                let command = SankeyCommand::new(
                    &self.body_path,
                    SankeyEdit::MoveNode {
                        node_id: id.clone(),
                        anchor_id,
                        after,
                    },
                );
                execute(workspace, &command)?;
            }
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

/// A new node's id where nothing hands one out: a ShortGuid, a version 4 GUID as 25 characters of base 36, as the
/// standalone host's `ShortGuid.NewShortGuid()` writes it.
fn short_guid() -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut value = uuid::Uuid::new_v4().as_u128();
    let mut digits = [b'0'; 25];
    for digit in digits.iter_mut().rev() {
        *digit = DIGITS[(value % 36) as usize];
        value /= 36;
    }
    String::from_utf8(digits.to_vec()).expect("base 36 digits")
}
