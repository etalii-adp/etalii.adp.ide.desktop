//! One open SPARQL query diagram, and what is asked of it (etalii.adp spec 030, task T084).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/sparql/backend/EtAlii.Adp.Diagram.Sparql/
//! SparqlSession.cs` (the stored positions overlaid by the layout itself, the rendering through the viewport, a
//! reposition stored in the registration as one undoable step, every unmovable surface refused with its own sentence,
//! a reparent refused), `SparqlContextSourceResolver.cs`, `SparqlContextPropertyProvider.cs`,
//! `SparqlContextActionProvider.cs` and `SparqlValidator.cs`. The module registers no toolbox and no actions: a
//! read-only diagram that offered a palette would be promising an edit it cannot make.

use std::path::{Path, PathBuf};

use adp_context::{ActionGroup, PropertyRow};
use adp_diagram::session::{ActionOutcome, ActionTarget};
use adp_diagram::{
    DiagramDocuments, DiagramElement, DiagramProblem, DiagramViewport, DiagramWorkspace, ModuleView, ToolboxItem,
};
use adp_history::{Command, Composite, Outcome};

use crate::commands::SetRegistrationLayout;
use crate::mapper::{HEADER_ID, TRUNCATION_ID};
use crate::rows::{NO_ACTIONS_REASON, READ_ONLY_REASON};
use crate::store::{self, OpenQuery};
use crate::{findings, layout, mapper, projection, registration_layout, rows};

/// What a reparent answers, whatever the element.
pub const PARENT_REFUSAL: &str = "A query's structure comes from its text, so nothing here can be moved into a different group. Dragging changes where an element sits on the canvas.";

/// What a move answers on a query opened without a registration.
pub const NO_REGISTRATION: &str = "This query was opened without a registration, so there is nowhere to store a position. Register the file to arrange it.";

/// Why `element_id` cannot carry an authored position, or none when it can.
pub fn refusal(element_id: &str) -> Option<&'static str> {
    if element_id.starts_with("anon:") {
        return Some(
            "That is an anonymous variable - written as a blank node, it has no name to key a stored position by, so it takes its computed place.",
        );
    }
    if element_id.starts_with("edge:") {
        return Some("An edge is drawn between its endpoints; move one of those instead.");
    }
    if element_id.starts_with("note:") {
        return Some("An annotation stays with what it constrains; move that instead.");
    }
    match element_id {
        HEADER_ID => {
            Some("The header band states the query's form and modifiers, and stays at the top of the diagram.")
        }
        TRUNCATION_ID => Some("That banner reports the truncation and is not something this diagram can move."),
        _ => None,
    }
}

/// One open view of a query.
#[derive(Debug, Clone)]
pub struct SparqlView {
    query: PathBuf,
    registration: Option<PathBuf>,
}

impl SparqlView {
    pub fn new(query: &Path, registration: Option<&Path>) -> Self {
        SparqlView {
            query: query.to_path_buf(),
            registration: registration.map(Path::to_path_buf),
        }
    }

    fn open<'a>(&self, documents: &'a DiagramDocuments) -> Result<&'a OpenQuery, String> {
        store::get(documents, &self.query).ok_or_else(|| adp_diagram::document::NOT_OPEN.to_owned())
    }
}

impl ModuleView for SparqlView {
    /// Through the viewport, always, with the positions the registration holds; a query that does not parse draws its
    /// header alone.
    fn render(&self, documents: &DiagramDocuments, viewport: DiagramViewport) -> Result<Vec<DiagramElement>, String> {
        let open = self.open(documents)?;
        let projected = projection::project(&open.model);
        let stored = self
            .registration
            .as_deref()
            .map(|r| registration_layout::read(store::registration_text(documents, r)))
            .unwrap_or_default();
        // Layout applies the stored positions itself, because a region anchor moves its whole frame.
        let laid_out = layout::compute(&projected, &stored);
        Ok(mapper::visible(&projected, &laid_out, viewport))
    }

    fn describe(&self, documents: &DiagramDocuments, element_id: &str) -> Option<(Vec<String>, String)> {
        let open = self.open(documents).ok()?;
        let text = rows::describe(open.usable_model(), element_id)?;
        Some((vec![text.clone()], text))
    }

    fn toolbox(&self) -> Vec<ToolboxItem> {
        Vec::new()
    }

    /// The validator's findings over the query as it was last read.
    fn problems(&self, documents: &DiagramDocuments) -> Vec<DiagramProblem> {
        match self.open(documents) {
            Ok(open) if open.readable => findings::validate(&open.text),
            _ => Vec::new(),
        }
    }

    fn actions(&self, _documents: &DiagramDocuments, _target: &ActionTarget) -> Vec<ActionGroup> {
        Vec::new()
    }

    fn execute(&self, _workspace: &mut DiagramWorkspace, _target: &ActionTarget, _action_id: &str) -> ActionOutcome {
        ActionOutcome::Refused(NO_ACTIONS_REASON.to_owned())
    }

    fn answer(
        &self,
        _workspace: &mut DiagramWorkspace,
        _target: &ActionTarget,
        _action_id: &str,
        _value: &str,
    ) -> ActionOutcome {
        ActionOutcome::Refused(NO_ACTIONS_REASON.to_owned())
    }

    fn properties(&self, documents: &DiagramDocuments, element_id: &str) -> Vec<PropertyRow> {
        match self.open(documents).ok().and_then(OpenQuery::usable_model) {
            Some(model) => rows::rows(&projection::project(model), element_id),
            None => Vec::new(),
        }
    }

    /// Refused, always, with the one reason every row gives.
    fn set_property(
        &self,
        _workspace: &mut DiagramWorkspace,
        _element_ids: &[&str],
        _property_id: &str,
        _value: &str,
    ) -> Result<(), String> {
        Err(READ_ONLY_REASON.to_owned())
    }

    /// Each position stored in the registration's `layout:` block, as one step; an unmovable surface refused with
    /// its own sentence.
    fn move_elements(&self, workspace: &mut DiagramWorkspace, moves: &[(String, f64, f64)]) -> Result<(), String> {
        let Some(registration) = &self.registration else {
            // Registering the file lifts this.
            return Err(NO_REGISTRATION.to_owned());
        };
        if let Some(reason) = moves.iter().find_map(|(id, _, _)| refusal(id)) {
            return Err(reason.to_owned());
        }
        let commands: Vec<Box<dyn Command<DiagramDocuments>>> = moves
            .iter()
            .map(|(id, x, y)| {
                Box::new(SetRegistrationLayout {
                    registration: registration.clone(),
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

    /// Refused, always: a query's structure comes from its text.
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
