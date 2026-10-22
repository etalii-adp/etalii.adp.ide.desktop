//! One open diagram of the family, whichever reading it was opened under (etalii.adp spec 030, task T083).
//!
//! Ported from the standalone host's `RdfFamilySession.cs` at `8b6ccfc`, with its base `DocumentDiagramSession.cs`,
//! the family's `RdfContextSourceResolver.cs` and the context providers of [`crate::actions`] and
//! [`crate::properties`]. A drag never touches the RDF file: a position is stored in the registration's `layout:`
//! block, keyed by the element's id; without a registration there is nowhere to store one, and the reading refuses the
//! ids whose identity does not survive a reparse. The whole document is projected and laid out whatever the viewport
//! says, and only then filtered, so panning moves nothing; the layout is kept between renders and made again for
//! another model or other stored positions.

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_context::{ActionGroup, PropertyRow};
use adp_diagram::session::{ActionOutcome, ActionTarget};
use adp_diagram::{
    DiagramDocuments, DiagramElement, DiagramProblem, DiagramViewport, DiagramWorkspace, ModuleView, ToolboxItem,
};
use adp_history::{Command, Composite, Outcome};

use crate::commands::{RdfCommand, RdfEdit, SetRegistrationLayout};
use crate::family::{Run, Target};
use crate::model::RdfModel;
use crate::reading::{Positions, Reading};
use crate::store::{self, RdfDocument};
use crate::{actions, properties, registration_layout, selection};

/// What a drag answers on a diagram opened without a registration.
pub const NO_REGISTRATION: &str = "This diagram was opened without a registration, so there is nowhere to store a position. Register the file to arrange it.";

/// What an action its target does not offer answers (the standalone host's `ContextService.ExecuteAction`).
pub const NOT_AVAILABLE: &str = "That action is not available for this item.";

/// The whole document's projection and layout, and what they were made from.
struct LaidOut<P> {
    model: Rc<RdfModel>,
    stored: Positions,
    projection: P,
    positions: Positions,
}

/// One open view of a body under one reading.
pub struct RdfView<R: Reading> {
    reading: R,
    root_path: PathBuf,
    body_path: PathBuf,
    registration: Option<PathBuf>,
    laid_out: RefCell<Option<LaidOut<R::Projection>>>,
}

impl<R: Reading> std::fmt::Debug for RdfView<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RdfView")
            .field("origin", &self.reading.origin())
            .field("body_path", &self.body_path)
            .field("registration", &self.registration)
            .finish()
    }
}

impl<R: Reading> RdfView<R> {
    pub fn new(reading: R, root_path: &Path, body_path: &Path, registration: Option<&Path>) -> Self {
        RdfView {
            reading,
            root_path: root_path.to_path_buf(),
            body_path: body_path.to_path_buf(),
            registration: registration.map(Path::to_path_buf),
            laid_out: RefCell::new(None),
        }
    }

    pub fn reading(&self) -> &R {
        &self.reading
    }

    pub fn body_path(&self) -> &Path {
        &self.body_path
    }

    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    pub fn registration(&self) -> Option<&Path> {
        self.registration.as_deref()
    }

    fn document<'a>(&self, documents: &'a DiagramDocuments) -> Result<&'a RdfDocument, String> {
        store::get(documents, &self.body_path).ok_or_else(|| adp_diagram::document::NOT_OPEN.to_owned())
    }

    fn target<'a>(&'a self, element_id: &'a str) -> Target<'a> {
        Target {
            element_id,
            body_path: &self.body_path,
            origin: self.reading.origin(),
        }
    }

    /// The positions the registration stores, none without one.
    fn stored(&self) -> Positions {
        self.registration
            .as_deref()
            .and_then(|path| fs::read(path).ok())
            .map(|bytes| registration_layout::read(&String::from_utf8_lossy(&bytes)))
            .unwrap_or_default()
    }

    /// Runs `f` on the whole of `model`, projected and positioned, made again only for another model or other stored
    /// positions.
    fn with_laid_out<T>(&self, model: &Rc<RdfModel>, f: impl FnOnce(&R::Projection, &Positions) -> T) -> T {
        let stored = self.stored();
        let mut cache = self.laid_out.borrow_mut();
        let fresh = cache
            .as_ref()
            .is_some_and(|c| Rc::ptr_eq(&c.model, model) && c.stored == stored);
        if !fresh {
            let projection = self.reading.project(model);
            let positions = self.reading.positions(&projection, &stored);
            *cache = Some(LaidOut {
                model: model.clone(),
                stored,
                projection,
                positions,
            });
        }
        let laid_out = cache.as_ref().expect("laid out above");
        f(&laid_out.projection, &laid_out.positions)
    }

    fn command(&self, edit: RdfEdit) -> RdfCommand {
        RdfCommand::new(&self.body_path, edit)
    }

    /// What a run of an action does in the workspace.
    fn outcome(&self, workspace: &mut DiagramWorkspace, run: Run) -> ActionOutcome {
        match run {
            Run::Edit(edit) => match execute(workspace, &self.command(edit)) {
                Ok(()) => ActionOutcome::Done,
                Err(refusal) => ActionOutcome::Refused(refusal),
            },
            Run::Ask(prompt) => ActionOutcome::Prompt(prompt),
            Run::Refused(refusal) => ActionOutcome::Refused(refusal),
            Run::Done => ActionOutcome::Done,
        }
    }
}

/// The element id the standalone host's canvas named a target by: an element by its id, a point of the canvas as a
/// placement (`new:{x},{y}`), and a finished connect gesture as a relation (`rel:{from}->{to}`).
pub fn element_id_of(target: &ActionTarget) -> String {
    match target {
        ActionTarget::Element(id) => id.clone(),
        ActionTarget::Canvas { x, y } => selection::placement_id(*x, *y),
        ActionTarget::Connection { source, target } => format!(
            "rel:{}->{}",
            source.as_deref().unwrap_or_default(),
            target.as_deref().unwrap_or_default()
        ),
    }
}

fn execute(workspace: &mut DiagramWorkspace, command: &dyn Command<DiagramDocuments>) -> Result<(), String> {
    match workspace.execute_command(command) {
        Outcome::Done { .. } => Ok(()),
        Outcome::Refused(refusal) => Err(refusal.0),
    }
}

impl<R: Reading + 'static> ModuleView for RdfView<R> {
    fn render(&self, documents: &DiagramDocuments, viewport: DiagramViewport) -> Result<Vec<DiagramElement>, String> {
        let model = self.document(documents)?.model.clone();
        Ok(self.with_laid_out(&model, |projection, positions| {
            let in_view = self.reading.in_view(projection, positions, viewport);
            self.reading.elements(&in_view, positions)
        }))
    }

    fn describe(&self, documents: &DiagramDocuments, element_id: &str) -> Option<(Vec<String>, String)> {
        let document = self.document(documents).ok()?;
        let text = actions::describe(&document.model, element_id)?;
        Some((vec![text.clone()], text))
    }

    fn toolbox(&self) -> Vec<ToolboxItem> {
        self.reading.toolbox()
    }

    fn problems(&self, documents: &DiagramDocuments) -> Vec<DiagramProblem> {
        self.document(documents)
            .map(|document| self.reading.problems(document, self.registration.as_deref()))
            .unwrap_or_default()
    }

    fn actions(&self, documents: &DiagramDocuments, target: &ActionTarget) -> Vec<ActionGroup> {
        let Ok(document) = self.document(documents) else {
            return Vec::new();
        };
        let element_id = element_id_of(target);
        actions::discover(document, &self.target(&element_id))
    }

    fn execute(&self, workspace: &mut DiagramWorkspace, target: &ActionTarget, action_id: &str) -> ActionOutcome {
        let element_id = element_id_of(target);
        let run = match self.document(workspace.documents()) {
            Ok(document) => {
                let target = self.target(&element_id);
                // An action its target does not offer, or offers unavailable, does nothing (`ContextService.ExecuteAction`).
                let offered = actions::discover(document, &target)
                    .iter()
                    .flat_map(|g| &g.actions)
                    .any(|a| a.id == action_id && a.is_available());
                if offered {
                    actions::execute(document, &target, action_id)
                } else {
                    Run::Refused(NOT_AVAILABLE.to_owned())
                }
            }
            Err(refusal) => Run::Refused(refusal),
        };
        self.outcome(workspace, run)
    }

    fn connect_action(
        &self,
        documents: &DiagramDocuments,
        source: &str,
        target: &str,
    ) -> Option<Result<String, String>> {
        let _ = (documents, source, target);
        self.reading.connect_action().map(|action| Ok(action.to_owned()))
    }

    fn answer(
        &self,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        value: &str,
    ) -> ActionOutcome {
        let element_id = element_id_of(target);
        let edit = match self.document(workspace.documents()) {
            Ok(document) => actions::commit(document, &self.target(&element_id), action_id, value),
            Err(refusal) => Err(refusal),
        };
        match edit {
            Ok(edit) => self.outcome(workspace, Run::Edit(edit)),
            Err(refusal) => ActionOutcome::Refused(refusal),
        }
    }

    fn properties(&self, documents: &DiagramDocuments, element_id: &str) -> Vec<PropertyRow> {
        match self.document(documents) {
            Ok(document) => properties::describe(document, &self.target(element_id)),
            Err(_) => Vec::new(),
        }
    }

    fn set_property(
        &self,
        workspace: &mut DiagramWorkspace,
        element_ids: &[&str],
        property_id: &str,
        value: &str,
    ) -> Result<(), String> {
        let document = self.document(workspace.documents())?;
        let mut commands: Vec<Box<dyn Command<DiagramDocuments>>> = Vec::with_capacity(element_ids.len());
        for id in element_ids {
            let edit = properties::command_for(document, &self.target(id), property_id, value)?;
            commands.push(Box::new(self.command(edit)));
        }
        match commands.len() {
            0 => Ok(()),
            1 => execute(workspace, commands[0].as_ref()),
            n => execute(workspace, &Composite::new(format!("Change {n} elements"), commands)),
        }
    }

    fn move_elements(&self, workspace: &mut DiagramWorkspace, moves: &[(String, f64, f64)]) -> Result<(), String> {
        self.document(workspace.documents())?;
        let Some(registration) = &self.registration else {
            return Err(NO_REGISTRATION.to_owned());
        };
        let mut commands: Vec<Box<dyn Command<DiagramDocuments>>> = Vec::with_capacity(moves.len());
        for (id, x, y) in moves {
            if let Some(refusal) = self.reading.refusal(id) {
                return Err(refusal);
            }
            commands.push(Box::new(SetRegistrationLayout {
                registration: registration.clone(),
                element_id: id.clone(),
                x: *x,
                y: *y,
            }));
        }
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
        Err(self.reading.parent_refusal().to_owned())
    }
}
