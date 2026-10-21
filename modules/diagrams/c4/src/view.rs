//! One open C4 diagram: which document, which view within it, and what is asked of it (etalii.adp spec 030, task
//! T061).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4/C4Session.cs` (the
//! view a registration names, else the document's first; a drag places an element in the view's sidecar and never
//! re-parents it), `C4ContextSourceResolver.cs` (what a selected element or relationship reads as, a relationship the
//! view elevated included) and `C4Validator.cs`, with the action and property providers of [`crate::actions`] and
//! [`crate::properties`]. Several views may share one document, which is the whole of "model once, view many": each
//! reads it from the project's documents, so an edit through any of them reaches all.

use std::path::{Path, PathBuf};

use adp_context::{ActionGroup, PropertyRow};
use adp_diagram::session::{ActionOutcome, ActionTarget};
use adp_diagram::{
    DiagramDocuments, DiagramElement, DiagramProblem, DiagramViewport, DiagramWorkspace, ModuleView, ToolboxItem,
};
use adp_history::{Command, Composite, Outcome};

use crate::actions;
use crate::commands::MoveC4Element;
use crate::layout;
use crate::mapper;
use crate::metrics::C4Metrics;
use crate::model::{C4Element, C4Relationship, C4View, C4Workspace, same};
use crate::properties;
use crate::rules;
use crate::store::{self, C4Document};
use crate::toolbox;

/// What dragging an element onto another answers: a C4 element's parent is what the document declares.
pub const PARENT_REFUSAL: &str = "Dragging changes where an element is drawn, not what contains it.";

/// What a move on a diagram whose registration names no view of its document answers.
pub const NO_VIEW: &str = "This diagram has no view to arrange.";

/// One open view of a C4 document.
#[derive(Debug, Clone)]
pub struct C4DiagramView {
    origin: String,
    body_path: PathBuf,
    view_key: Option<String>,
    metrics: C4Metrics,
}

impl C4DiagramView {
    /// A view of tool type `origin` of the body at `body_path`, showing the view `view_key` names, or the document's
    /// first when it names none.
    pub fn new(origin: &str, body_path: &Path, view_key: Option<&str>) -> Self {
        C4DiagramView {
            origin: origin.to_owned(),
            body_path: body_path.to_path_buf(),
            view_key: view_key.filter(|k| !k.is_empty()).map(str::to_owned),
            metrics: C4Metrics::default(),
        }
    }

    pub fn body_path(&self) -> &Path {
        &self.body_path
    }

    pub fn view_key(&self) -> Option<&str> {
        self.view_key.as_deref()
    }

    fn document<'a>(&self, documents: &'a DiagramDocuments) -> Result<&'a C4Document, String> {
        store::get(documents, &self.body_path).ok_or_else(|| adp_diagram::document::NOT_OPEN.to_owned())
    }

    /// The view this diagram shows: the one its registration names, or the document's first; none when the document
    /// declares no view, or none of that key.
    pub fn view_of<'a>(&self, workspace: &'a C4Workspace) -> Option<&'a C4View> {
        match &self.view_key {
            None => workspace.views.first(),
            Some(key) => workspace.find_view(key),
        }
    }

    /// The document, when it could be read; else the sentence an action on it answers.
    fn usable<'a>(&self, documents: &'a DiagramDocuments) -> Result<&'a C4Document, String> {
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

/// The element id an action's target names: an element, or the empty canvas as the diagram itself.
fn target_id(target: &ActionTarget) -> Option<&str> {
    match target {
        ActionTarget::Element(id) => Some(id),
        ActionTarget::Canvas { .. } => Some(""),
        ActionTarget::Connection { .. } => None,
    }
}

impl ModuleView for C4DiagramView {
    fn render(&self, documents: &DiagramDocuments, viewport: DiagramViewport) -> Result<Vec<DiagramElement>, String> {
        let document = self.document(documents)?;
        Ok(match self.view_of(&document.workspace) {
            Some(view) => mapper::visible(
                &document.workspace,
                view,
                viewport,
                &self.metrics,
                Some(&self.body_path),
            ),
            None => Vec::new(),
        })
    }

    fn describe(&self, documents: &DiagramDocuments, element_id: &str) -> Option<(Vec<String>, String)> {
        let workspace = &self.document(documents).ok()?.workspace;
        if let Some(element) = workspace.find(element_id) {
            return Some((path_of(workspace, element), element.name.clone()));
        }
        let relationship = relationship_drawn_as(workspace, element_id)?;
        Some((Vec::new(), relationship_text(workspace, &relationship)))
    }

    fn toolbox(&self) -> Vec<ToolboxItem> {
        toolbox::view_kind_of(&self.origin)
            .map(toolbox::items_for)
            .unwrap_or_default()
    }

    fn problems(&self, documents: &DiagramDocuments) -> Vec<DiagramProblem> {
        self.document(documents)
            .map(|document| rules::validate(&document.workspace))
            .unwrap_or_default()
    }

    fn actions(&self, documents: &DiagramDocuments, target: &ActionTarget) -> Vec<ActionGroup> {
        let (Ok(document), Some(id)) = (self.document(documents), target_id(target)) else {
            return Vec::new();
        };
        actions::discover(&document.workspace, document.is_usable(), id)
    }

    fn execute(&self, workspace: &mut DiagramWorkspace, target: &ActionTarget, action_id: &str) -> ActionOutcome {
        let document = match self.usable(workspace.documents()) {
            Ok(document) => document,
            Err(refusal) => return ActionOutcome::Refused(refusal),
        };
        let Some(id) = target_id(target) else {
            return ActionOutcome::Refused(format!("'{action_id}' does not apply to this selection."));
        };
        match actions::run(&document.workspace, id, action_id) {
            Ok(prompt) => ActionOutcome::Prompt(prompt),
            Err(refusal) => ActionOutcome::Refused(refusal),
        }
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
        let command = target_id(target)
            .and_then(|id| actions::commit(&document.workspace, &self.body_path, id, action_id, value));
        let Some(command) = command else {
            return ActionOutcome::Refused(format!("'{action_id}' does not apply to this selection."));
        };
        match execute(workspace, command.as_ref()) {
            Ok(()) => ActionOutcome::Done,
            Err(refusal) => ActionOutcome::Refused(refusal),
        }
    }

    fn properties(&self, documents: &DiagramDocuments, element_id: &str) -> Vec<PropertyRow> {
        self.document(documents)
            .map(|document| properties::describe(&document.workspace, element_id))
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
            commands.push(properties::change(
                &document.workspace,
                &self.body_path,
                id,
                property_id,
                value,
            )?);
        }
        match commands.len() {
            0 => Ok(()),
            1 => execute(workspace, commands[0].as_ref()),
            n => execute(workspace, &Composite::new(format!("Change {n} elements"), commands)),
        }
    }

    fn move_elements(&self, workspace: &mut DiagramWorkspace, moves: &[(String, f64, f64)]) -> Result<(), String> {
        let document = self.document(workspace.documents())?;
        let Some(view) = self.view_of(&document.workspace) else {
            return Err(NO_VIEW.to_owned());
        };
        let commands: Vec<Box<dyn Command<DiagramDocuments>>> = moves
            .iter()
            .map(|(id, x, y)| {
                Box::new(MoveC4Element {
                    body_path: self.body_path.clone(),
                    view_key: view.key.clone(),
                    element_id: id.clone(),
                    x: *x,
                    y: *y,
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

/// An element's chain of names from the top of the model, which is what identifies it to a person.
fn path_of(workspace: &C4Workspace, element: &C4Element) -> Vec<String> {
    let mut names = Vec::new();
    let mut current = Some(element);
    while let Some(element) = current {
        names.insert(
            0,
            if element.name.is_empty() {
                element.id.clone()
            } else {
                element.name.clone()
            },
        );
        current = element.parent_id.as_deref().and_then(|p| workspace.find(p));
    }
    names
}

/// The relationship a drawn relationship id names: the model's own, one a view elevated to its ends' nearest drawn
/// ancestors or drew between instances, or a dynamic view's interaction; returned with the drawn ends.
fn relationship_drawn_as(workspace: &C4Workspace, element_id: &str) -> Option<C4Relationship> {
    if let Some(exact) = workspace.relationship(element_id) {
        return Some(exact.clone());
    }
    let at = element_id.rfind('@')?;
    let arrow = element_id.find("->")?;
    if arrow == 0 || at < arrow + 2 {
        return None;
    }
    let line: u32 = element_id[at + 1..].parse().ok()?;
    let from = &element_id[..arrow];
    let to = &element_id[arrow + 2..at];
    let elevated = workspace.relationships.iter().find(|candidate| {
        candidate.line == line
            && (is_self_or_ancestor(workspace, from, &candidate.source_id)
                || is_instance_of(workspace, from, &candidate.source_id))
            && (is_self_or_ancestor(workspace, to, &candidate.destination_id)
                || is_instance_of(workspace, to, &candidate.destination_id))
    });
    if let Some(elevated) = elevated {
        return Some(C4Relationship {
            source_id: from.to_owned(),
            destination_id: to.to_owned(),
            ..elevated.clone()
        });
    }
    let interaction = workspace
        .views
        .iter()
        .flat_map(|view| &view.interactions)
        .find(|candidate| {
            candidate.line == line && same(&candidate.source_id, from) && same(&candidate.destination_id, to)
        })?;
    Some(C4Relationship {
        source_id: interaction.source_id.clone(),
        destination_id: interaction.destination_id.clone(),
        description: interaction.description.clone(),
        technology: String::new(),
        line: interaction.line,
    })
}

/// Whether `candidate` is an instance of `id`.
fn is_instance_of(workspace: &C4Workspace, candidate: &str, id: &str) -> bool {
    workspace
        .find(candidate)
        .and_then(|e| e.referenced_id.as_deref())
        .is_some_and(|referenced| same(referenced, id))
}

/// Whether `candidate` is `id` or one of the elements it sits inside.
fn is_self_or_ancestor(workspace: &C4Workspace, candidate: &str, id: &str) -> bool {
    let mut current = workspace.find(id);
    while let Some(element) = current {
        if same(&element.id, candidate) {
            return true;
        }
        current = element.parent_id.as_deref().and_then(|p| workspace.find(p));
    }
    false
}

/// An end's name as drawn: an instance goes by the element it instantiates.
fn name_of(workspace: &C4Workspace, id: &str) -> String {
    match workspace.find(id) {
        Some(element) => layout::displayed(workspace, element).name,
        None => id.to_owned(),
    }
}

fn relationship_text(workspace: &C4Workspace, relationship: &C4Relationship) -> String {
    let source = name_of(workspace, &relationship.source_id);
    let destination = name_of(workspace, &relationship.destination_id);
    if relationship.description.is_empty() {
        format!("{source} -> {destination}")
    } else {
        format!("{source} -> {destination}: {}", relationship.description)
    }
}
