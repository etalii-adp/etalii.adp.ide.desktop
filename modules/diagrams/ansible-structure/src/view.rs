//! One open Ansible structure diagram, and what is asked of it (etalii.adp spec 030, task T079).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/ansible-structure/backend/
//! EtAlii.Adp.Diagram.AnsibleStructure/AnsibleSession.cs` (a reposition stored in the registration as one undoable
//! step, an edge refused because it follows its ends, a reparenting move refused with the definition's sentence),
//! `AnsibleContextSourceResolver.cs` (a node reads as its file and name, a relation as its declaring file and its
//! directive), `AnsibleContextPropertyProvider.cs` (every row read-only, every write refused with the binding's
//! sentence) and `AnsibleValidator.cs`. The module registers no toolbox and no actions, which is its whole statement:
//! nothing can be added to a diagram that writes nothing but positions. Positions are in canvas units, a node placed by
//! its top-left corner.

use std::path::{Path, PathBuf};

use adp_context::{ActionGroup, PropertyRow};
use adp_diagram::session::{ActionOutcome, ActionTarget};
use adp_diagram::{
    DiagramDocuments, DiagramElement, DiagramProblem, DiagramViewport, DiagramWorkspace, ModuleView, ToolboxItem,
};
use adp_history::{Command, Composite, Outcome};

use crate::commands::SetRegistrationLayout;
use crate::definition::{AnsibleDefinition, text};
use crate::store::{self, AnsibleFolder};
use crate::{findings, mapper, registration_layout};

/// What a move of a relation answers: it has no position of its own, it follows its ends.
pub const EDGE_REFUSAL: &str = "That element is not something this diagram can move.";

/// What an action answers: this diagram offers none.
pub const NO_ACTION: &str = "That action is not available for this item.";

/// One open view of a registered Ansible folder.
#[derive(Debug, Clone)]
pub struct AnsibleView {
    root: PathBuf,
    registration: PathBuf,
}

impl AnsibleView {
    pub fn new(root: &Path, registration: &Path) -> Self {
        AnsibleView {
            root: root.to_path_buf(),
            registration: registration.to_path_buf(),
        }
    }

    pub fn registration(&self) -> &Path {
        &self.registration
    }

    fn folder<'a>(&self, documents: &'a DiagramDocuments) -> Result<&'a AnsibleFolder, String> {
        store::get(documents, &self.registration).ok_or_else(|| adp_diagram::document::NOT_OPEN.to_owned())
    }
}

fn segments(relative: &str) -> Vec<String> {
    relative
        .split(['/', '\\'])
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

impl ModuleView for AnsibleView {
    fn render(&self, documents: &DiagramDocuments, viewport: DiagramViewport) -> Result<Vec<DiagramElement>, String> {
        let folder = self.folder(documents)?;
        let stored = registration_layout::read(&folder.registration_text);
        Ok(mapper::visible(&folder.model.diagram, viewport, &stored))
    }

    fn describe(&self, documents: &DiagramDocuments, element_id: &str) -> Option<(Vec<String>, String)> {
        let folder = self.folder(documents).ok()?;
        let element = folder.model.diagram.element_by_id(element_id)?;
        if !element.is_relation() {
            return Some((
                segments(&text(&element.value_of("path"))),
                text(&element.value_of("name")),
            ));
        }
        // A relation answers as its declaring side, by its directive as the file spells it.
        let label = AnsibleDefinition::get().label(&element, "directive");
        let label = if label.ends_with(':') {
            label
        } else {
            format!("{label}:")
        };
        Some((
            segments(&text(&element.value_of("declaredIn"))),
            format!("{label} {}", text(&element.value_of("targetAsWritten"))),
        ))
    }

    fn toolbox(&self) -> Vec<ToolboxItem> {
        Vec::new()
    }

    fn problems(&self, documents: &DiagramDocuments) -> Vec<DiagramProblem> {
        let Ok(folder) = self.folder(documents) else {
            return Vec::new();
        };
        let definition = AnsibleDefinition::get();
        let problems = findings::problems(&folder.model.diagram, &definition.findings(&folder.model));
        findings::rebased(problems, &self.root, &folder.folder)
    }

    fn actions(&self, _documents: &DiagramDocuments, _target: &ActionTarget) -> Vec<ActionGroup> {
        Vec::new()
    }

    fn execute(&self, _workspace: &mut DiagramWorkspace, _target: &ActionTarget, _action_id: &str) -> ActionOutcome {
        ActionOutcome::Refused(NO_ACTION.to_owned())
    }

    fn answer(
        &self,
        _workspace: &mut DiagramWorkspace,
        _target: &ActionTarget,
        _action_id: &str,
        _value: &str,
    ) -> ActionOutcome {
        ActionOutcome::Refused(NO_ACTION.to_owned())
    }

    fn properties(&self, documents: &DiagramDocuments, element_id: &str) -> Vec<PropertyRow> {
        self.folder(documents)
            .map(|folder| AnsibleDefinition::get().rows(&folder.model.diagram, element_id))
            .unwrap_or_default()
    }

    /// Refused, always, with the binding's sentence.
    fn set_property(
        &self,
        _workspace: &mut DiagramWorkspace,
        _element_ids: &[&str],
        _property_id: &str,
        _value: &str,
    ) -> Result<(), String> {
        Err(AnsibleDefinition::get().write_refusal())
    }

    /// Each position stored in the registration's `layout:` block, as one step; a relation refused.
    fn move_elements(&self, workspace: &mut DiagramWorkspace, moves: &[(String, f64, f64)]) -> Result<(), String> {
        self.folder(workspace.documents())?;
        if moves.iter().any(|(id, _, _)| id.starts_with("edge:")) {
            return Err(EDGE_REFUSAL.to_owned());
        }
        let commands: Vec<Box<dyn Command<DiagramDocuments>>> = moves
            .iter()
            .map(|(id, x, y)| {
                Box::new(SetRegistrationLayout {
                    registration: self.registration.clone(),
                    element_id: id.clone(),
                    x: *x,
                    y: *y,
                }) as Box<dyn Command<DiagramDocuments>>
            })
            .collect();
        let outcome = match commands.len() {
            0 => return Ok(()),
            1 => workspace.execute_command(commands[0].as_ref()),
            n => workspace.execute_command(&Composite::new(format!("Move {n} elements"), commands)),
        };
        match outcome {
            Outcome::Done { .. } => Ok(()),
            Outcome::Refused(refusal) => Err(refusal.0),
        }
    }

    /// Refused, always: nothing nests by dragging (the definition's containment rule).
    fn move_element(
        &self,
        _workspace: &mut DiagramWorkspace,
        _element_id: &str,
        _new_parent_id: Option<&str>,
        _index: i64,
    ) -> Result<(), String> {
        Err(AnsibleDefinition::get().move_refusal())
    }
}
