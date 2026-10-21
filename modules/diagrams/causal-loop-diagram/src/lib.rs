//! The causal loop diagram's residue (etalii.adp spec 030, task T069, after the standalone host's
//! `src/diagrams/causal-loop-diagram` at `8b6ccfc`): what the specification `definitions/diagrams/causal-loop-diagram.dis`
//! and its binding `causal-loop-diagram.fbl#cld` cannot state, registered with `adp-diagram` as the [`DiagramModule`] of
//! the origin [`ORIGIN`].
//!
//! The standalone module had already handed its menus, rows, forms, findings and edits to that specification, and kept
//! in code what `causal-loop-diagram.md` lists under "What stays code". The desktop host reads, draws and edits the
//! `.cld` through the specification like every DISL and FBL diagram, and keeps here what is left of that code:
//!
//! - [`cycles`]: the plugin function `causalCycles` of `net.etalii.adp.systems.cldCycles`, the tool's cycle finder,
//!   whose order DISL's `diagram.cycles` does not give (`CausalLoopCycles`, `CycleFinder`);
//! - [`ring`] and [`metrics`]: the ring a document opens on, seated by a walk of the links, with the widths it is
//!   measured with; a position the registration stores wins for its variable (`CausalLoopLayout`,
//!   `CausalLoopMetrics`, and the overlay of `CausalLoopSession.Render`);
//! - [`self_organizing`]: Arrange diagram, the plugin layout `net.etalii.adp.systems.cld`, written as the
//!   registration's positions in one step (`SelfOrganizingLayout`, `ArrangeCausalLoopCommand`);
//! - [`unreadable`]: the four sentences for a statement the binding leaves unread (`CausalLoopUnreadable`);
//! - a new `layout:` block written with CRLF, as the standalone host's `RegistrationLayout` wrote it.
//!
//! Not ported: `CausalLoopNote`, the stand-in that reads a link's bare-word note and the last of two notes where the
//! binding's `note` slot takes the first quoted word (gap 2). It reads the body again after the binding and writes a
//! note over the word it read it from, and `adp-diagram` has no hook into a reading; such a note reads here as the
//! binding reads it. Every other stand-in of the standalone module (its element mapper, session, store, providers and
//! command records) is what `adp-diagram` does for every tool type.

pub mod cycles;
pub mod metrics;
pub mod reading;
pub mod ring;
pub mod self_organizing;
pub mod unreadable;

use std::collections::HashMap;
use std::rc::Rc;

use adp_diagram::session::{ActionOutcome, ActionTarget, DiagramSession};
use adp_diagram::workspace::DiagramWorkspace;
use adp_diagram::{DiagramChange, DiagramDocument, DiagramModule, ModulePlacement};
use adp_disl::{DislElement, DislPluginFunctions, DislReaderFinding, IdSource};

use crate::metrics::Metrics;
use crate::reading::Model;

/// The tool type this module serves.
pub const ORIGIN: &str = "systems/causal-loop-diagram";

/// What a new `.cld` is written with when one is added: the binding's template, the *Births beget births* starter in
/// CRLF (`causal-loop-diagram.fbl#cld`, finding B9), as the standalone host registered it with
/// `AddEmptyDocument(CausalLoopOrigin, CausalLoopBody.Template)`.
pub const TEMPLATE: &str = "causal-loop 1\r\n\r\nvariable population \"Population\"\r\nvariable births \"Births\"\r\n\r\nlink population -> births +\r\nlink births -> population +\r\n\r\nloop R1 \"Births beget births\" population births\r\n";

/// The wire id of Arrange diagram (`x-cld`).
pub const ARRANGE: &str = "causal-loop.arrange";

/// What Arrange diagram answers for a diagram opened without a registration.
pub const NO_REGISTRATION: &str = "This diagram was opened without a registration, so there is nowhere to store an arrangement. Register the file to arrange it.";

/// What Arrange diagram answers when the registration was deleted after the diagram was opened.
pub const REGISTRATION_GONE: &str = "The registration file is no longer there.";

/// What Arrange diagram answers for a document that could not be read.
pub const UNREADABLE: &str = "This causal loop diagram could not be read, so there is nothing to arrange.";

/// What Arrange diagram answers when the layout placed nothing.
pub const NOTHING_TO_ARRANGE: &str = "This diagram has no variables to arrange.";

/// What a drag of a link or a loop answers: the specification's `refusals.move` of both, as `CausalLoopSession`'s
/// placement refused them.
pub const ONLY_VARIABLES_MOVE: &str =
    "Only a variable can be moved; a link is drawn between its ends and a loop label among its members.";

/// The causal loop diagram's own code beside its specification.
#[derive(Debug, Default, Clone, Copy)]
pub struct CausalLoopModule {
    metrics: Metrics,
}

impl CausalLoopModule {
    /// The module with the layouts' metrics `metrics`.
    pub fn with_metrics(metrics: Metrics) -> Self {
        CausalLoopModule { metrics }
    }
}

/// The module, to register with `adp_diagram::DiagramTools::with_module` under [`ORIGIN`].
pub fn module() -> Rc<dyn DiagramModule> {
    Rc::new(CausalLoopModule::default())
}

/// Where each variable is drawn, by element id, as the centre of its box: on the ring, else where the registration
/// stores it (the standalone host's `CausalLoopSession.Render`, whose stored positions are centres).
pub fn positions(document: &DiagramDocument, metrics: &Metrics) -> HashMap<String, (f64, f64)> {
    let model = Model::of(&document.model().diagram);
    let mut positions: HashMap<String, (f64, f64)> = ring::compute(&model, metrics)
        .into_iter()
        .map(|(name, rect)| (format!("variable:{name}"), (rect.center_x(), rect.center_y())))
        .collect();
    if let Some(layout) = document.registration().and_then(|r| r.layout.as_ref()) {
        for entry in &layout.entries {
            let mut parts = entry.value.split_whitespace().map(str::parse::<f64>);
            if let (Some(Ok(x)), Some(Ok(y)), None) = (parts.next(), parts.next(), parts.next())
                && let Some(position) = positions.get_mut(&entry.key)
            {
                *position = (x, y);
            }
        }
    }
    positions
}

impl CausalLoopModule {
    /// Arrange diagram: the self-organizing layout's centres stored as the registration's positions, in model order,
    /// as one command.
    fn arrange(&self, session: &DiagramSession, workspace: &mut DiagramWorkspace) -> Option<ActionOutcome> {
        let document = workspace.documents().get(session.body_path())?;
        if document.model().is_unreadable {
            return Some(ActionOutcome::Refused(UNREADABLE.to_owned()));
        }
        let model = Model::of(&document.model().diagram);
        // Fewer than two variables: the specification's own reason, from its menu.
        if model.variables.len() < 2 {
            return None;
        }
        let Some(registration) = document.registration_path() else {
            return Some(ActionOutcome::Refused(NO_REGISTRATION.to_owned()));
        };
        if !registration.exists() {
            return Some(ActionOutcome::Refused(REGISTRATION_GONE.to_owned()));
        }
        let boxes = match self_organizing::compute(
            &model,
            &self.metrics,
            self_organizing::DEFAULT_BUDGET,
            self_organizing::SEPARATION_ROUNDS,
        ) {
            Ok(boxes) => boxes,
            Err(refusal) => return Some(ActionOutcome::Refused(refusal)),
        };
        if boxes.is_empty() {
            return Some(ActionOutcome::Refused(NOTHING_TO_ARRANGE.to_owned()));
        }
        let changes: Vec<DiagramChange> = model
            .variables
            .iter()
            .filter_map(|variable| {
                let rect = boxes.get(&variable.name)?;
                Some(DiagramChange::Place {
                    id: variable.element_id(),
                    x: rect.center_x(),
                    y: rect.center_y(),
                })
            })
            .collect();
        let outcome = document
            .plan("Arrange diagram", &changes)
            .and_then(|edit| workspace.execute(edit));
        Some(match outcome {
            Ok(()) => ActionOutcome::Done,
            Err(refusal) => ActionOutcome::Refused(refusal),
        })
    }
}

impl DiagramModule for CausalLoopModule {
    fn plugin_functions(&self) -> DislPluginFunctions {
        cycles::plugin_functions()
    }

    /// Only a variable has a position of its own to store; a loop sits among its members (`movable: false`).
    fn placement(&self, element: &DislElement, _x: f64, _y: f64) -> Option<Result<ModulePlacement, String>> {
        (!element.is_a("Variable")).then(|| Err(ONLY_VARIABLES_MOVE.to_owned()))
    }

    fn positions(&self, document: &DiagramDocument) -> Option<HashMap<String, (f64, f64)>> {
        Some(positions(document, &self.metrics))
    }

    fn execute(
        &self,
        session: &DiagramSession,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        _ids: &dyn IdSource,
    ) -> Option<ActionOutcome> {
        (action_id == ARRANGE && matches!(target, ActionTarget::Canvas { .. }))
            .then(|| self.arrange(session, workspace))
            .flatten()
    }

    fn created_block_ending(&self) -> Option<String> {
        Some("\r\n".to_owned())
    }

    fn reader_findings(&self, document: &DiagramDocument) -> Vec<DislReaderFinding> {
        let body = document.body_bytes();
        document
            .model()
            .unreadable
            .iter()
            .map(|element| {
                let span = element.own_span;
                let statement = String::from_utf8_lossy(&body[span.start.min(body.len())..span.end.min(body.len())]);
                let statement = statement.trim_end_matches(['\r', '\n']);
                DislReaderFinding::unreadable(&unreadable::sentence_for(statement), Some(element.line as i32), None)
            })
            .collect()
    }
}
