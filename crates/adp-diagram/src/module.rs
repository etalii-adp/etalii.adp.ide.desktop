//! What a tool type's module adds to the host where its specification and binding cannot say it (etalii.adp spec
//! 030, FR-018 and FR-031; plan.md, "Project Structure": `modules/diagrams/*`): the persistence plugin its binding
//! names as its reader, the actions it runs itself, and the positions its layout gives.
//!
//! The standalone host at `8b6ccfc` registered each module's residue through dependency injection: its document store,
//! its `IContextActionProvider` and its layout (`src/diagrams/mindmap/backend/EtAlii.Adp.Diagram.Mindmap`, for
//! example). Here one value per origin carries it, given to [`crate::DiagramTools::with_module`], and every hook is
//! optional: a tool type without a module is drawn and edited from its specification alone.

use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use adp_disl::IdSource;
use adp_fbl::plugins::PersistencePlugin;

use crate::document::DiagramDocument;
use crate::session::{ActionOutcome, ActionTarget, DiagramSession};
use crate::workspace::DiagramWorkspace;

/// The residue of one tool type: what its module implements beside its specification.
pub trait DiagramModule {
    /// The persistence plugin with id `plugin_id` (FBL section 11), when the module provides it.
    fn persistence(&self, plugin_id: &str) -> Option<Rc<dyn PersistencePlugin>> {
        let _ = plugin_id;
        None
    }

    /// Runs action `action_id` of `target`'s menu itself, or none to leave it to the specification's behaviour.
    fn execute(
        &self,
        session: &DiagramSession,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        ids: &dyn IdSource,
    ) -> Option<ActionOutcome> {
        let _ = (session, workspace, target, action_id, ids);
        None
    }

    /// Answers the prompt action `action_id` of `target` raised, or none to leave it to the session.
    fn answer(
        &self,
        session: &DiagramSession,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        value: &str,
    ) -> Option<ActionOutcome> {
        let _ = (session, workspace, target, action_id, value);
        None
    }

    /// Where the module's layout puts each element, by id, in the diagram's units; none to keep the registration's.
    fn positions(&self, document: &DiagramDocument) -> Option<HashMap<String, (f64, f64)>> {
        let _ = document;
        None
    }
}

impl fmt::Debug for dyn DiagramModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DiagramModule")
    }
}
