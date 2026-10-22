//! The agent behavior model's residue (etalii.adp spec 030, FR-018, FR-031 and FR-032, task T074): what the tool type
//! `etalii/agent-behavior-modelling` needs beside its specification
//! (`vendor/etalii.adp/definitions/diagrams/agent-behavior-modelling.dis`) and its binding
//! (`agent-behavior-modelling.fbl#abm`, vendored from the standalone host's module folder), as
//! `agent-behavior-modelling.md` lists it. Ported from the standalone host's
//! `src/diagrams/agent-behavior-modelling/backend/EtAlii.Adp.Diagram.AgentBehaviorModelling` at `8b6ccfc`.
//!
//! The standalone module read and wrote its Markdown through that binding, whose reader is the persistence plugin
//! `net.etalii.adp.etalii.abmMarkdown`, and derived its palette, menus, rows, operations and findings from the
//! specification; the desktop host does that through its generic DISL and FBL path, and keeps here what the module
//! added in code:
//!
//! - [`parser`], [`writer`] and [`plugin`]: the Markdown read and written as the module always wrote it, a change
//!   planned as the lines it touches;
//! - [`layout`] and [`registration`]: the tidy tree, the row heights an author dragged kept in the registration's
//!   `layout:` block, and what a drag and Arrange diagram write there;
//! - [`actions`]: what the module ran itself rather than through the specification: a node added and then renamed in
//!   place, a drop placed under the nearest node above with room, a node moved one place earlier or later, Arrange
//!   diagram, and a drag that moves a row and may reorder it;
//! - the findings: the specification's, then the reader's `abm.no-behavior`.

pub mod actions;
pub mod kinds;
pub mod layout;
pub mod lines;
pub mod model;
pub mod parser;
pub mod plugin;
pub mod registration;
pub mod writer;

use std::collections::HashMap;
use std::rc::Rc;

use adp_diagram::session::{ActionOutcome, ActionTarget, DiagramSession};
use adp_diagram::workspace::DiagramWorkspace;
use adp_diagram::{DiagramChange, DiagramDocument, DiagramModule, DiagramProblem, DiagramTools, DocumentEdit};
use adp_disl::{DislConstraintOptions, IdSource};
use adp_fbl::plugins::PersistencePlugin;

use crate::actions::AbmActions;
use crate::plugin::{AbmMarkdownPlugin, PLUGIN_ID};

/// The tool type this module serves.
pub const ORIGIN: &str = "etalii/agent-behavior-modelling";

/// The extension of a behavior model's body.
pub const EXTENSION: &str = ".md";

/// The text of a new behavior model named `base_name`, as the standalone host's Add wrote it
/// (`ServiceCollection.AddAgentBehaviorModelling.cs`: `AbmWriter.EmptyDocument(baseName, "\r\n")`).
pub fn empty_document(base_name: &str) -> String {
    writer::empty_document(base_name, "\r\n")
}

/// The agent behavior model's residue, as `adp-diagram` asks for it.
#[derive(Debug, Default)]
pub struct AbmModule {
    actions: AbmActions,
}

/// The module, to register with `adp_diagram::DiagramTools::with_module` under [`ORIGIN`].
pub fn module() -> Rc<dyn DiagramModule> {
    Rc::new(AbmModule::default())
}

/// `tools` with the agent behavior model's module registered.
pub fn register(tools: DiagramTools) -> DiagramTools {
    tools.with_module(ORIGIN, module())
}

impl DiagramModule for AbmModule {
    fn persistence(&self, plugin_id: &str) -> Option<Rc<dyn PersistencePlugin>> {
        (plugin_id == PLUGIN_ID).then(|| Rc::new(AbmMarkdownPlugin) as Rc<dyn PersistencePlugin>)
    }

    fn execute(
        &self,
        session: &DiagramSession,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        ids: &dyn IdSource,
    ) -> Option<ActionOutcome> {
        self.actions.execute(session, workspace, target, action_id, ids)
    }

    fn answer(
        &self,
        session: &DiagramSession,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        value: &str,
    ) -> Option<ActionOutcome> {
        self.actions.answer(session, workspace, target, action_id, value)
    }

    /// The centre of every node, as the tidy tree places it with the rows the registration stores.
    fn positions(&self, document: &DiagramDocument) -> Option<HashMap<String, (f64, f64)>> {
        let model = plugin::read_model(document.body_bytes());
        let stored = registration::read(document.registration_bytes().unwrap_or_default());
        Some(
            layout::arrange(&model, &stored)
                .into_iter()
                .map(|(id, top_left)| (id, layout::centre(top_left)))
                .collect(),
        )
    }

    fn set_property(
        &self,
        document: &DiagramDocument,
        element_id: &str,
        property_id: &str,
        value: &str,
    ) -> Option<Result<Vec<DiagramChange>, String>> {
        Some(actions::set_property(document, element_id, property_id, value))
    }

    fn move_elements(
        &self,
        document: &DiagramDocument,
        moves: &[(String, f64, f64)],
    ) -> Option<Result<DocumentEdit, String>> {
        Some(actions::drag(document, moves))
    }

    fn problems(&self, document: &DiagramDocument, options: DislConstraintOptions) -> Option<Vec<DiagramProblem>> {
        Some(actions::problems(document, options))
    }
}
