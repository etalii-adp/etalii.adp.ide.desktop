//! What a tool type's module adds to the host where its specification and binding cannot say it (etalii.adp spec
//! 030, FR-018, FR-030 and FR-031; plan.md, "Project Structure": `modules/diagrams/*`).
//!
//! The standalone host at `8b6ccfc` built every diagram type as a module of its own (`src/diagrams/<type>/backend`),
//! registered through dependency injection: its document store, its `IContextActionProvider`, its layout. Here one
//! value per origin carries what is left of it, given to [`crate::DiagramTools::with_module`]: the persistence
//! plugin its binding names as its reader (FBL section 11), the actions it runs itself, the positions its layout
//! gives (the mind map's), the implementations of the specification's plugin CEL functions (DISL section 13.1.1),
//! the canvas coordinates of a coordinate system whose placement attributes are not plain numbers (section 5; the
//! timeline's `TimelineScale` and `TimelineRows`), the plugin layouts its `layout` actions name (section 10), the
//! attribute an inline rename writes where a computed label names none, the size a node is drawn at and what a resize
//! of it writes where its coordinates are not plain numbers, and the changes as its body writes them.
//! Every hook has the generic answer as its default, so a module implements only what it adds, and a tool type
//! without a module is drawn and edited from its specification alone.

use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use adp_cel::Map;
use adp_disl::{DislChange, DislDiagram, DislElement, DislPluginFunctions, DislSpecification, IdSource};
use adp_fbl::plugins::PersistencePlugin;

use crate::document::DiagramDocument;
use crate::session::{ActionOutcome, ActionTarget, DiagramSession};
use crate::sizing::{Extent, Resize};
use crate::workspace::DiagramWorkspace;

/// Where a gesture puts an element, as a module's coordinates say: the attributes to set, and the bounds before and
/// after that the specification's placement constraints judge (DISL section 8.4, `oldBounds` and `newBounds`).
#[derive(Debug, Clone, Default)]
pub struct ModulePlacement {
    /// The attributes the move writes; one already holding its value is left out, so it is not written again.
    pub attributes: Map,
    pub old_bounds: Map,
    pub new_bounds: Map,
}

/// A tool type's own code, beside its specification. Each method's default is the session's generic behaviour.
/// The residue of one tool type: what its module implements beside its specification. Each method's default is the
/// session's generic behaviour.
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

    /// The host's implementations of the CEL functions the specification's plugins declare.
    fn plugin_functions(&self) -> DislPluginFunctions {
        DislPluginFunctions::none()
    }

    /// Where `element` is drawn, in the canvas units of the specification's coordinate system; none to read its
    /// placement attributes as plain numbers.
    fn position(&self, _element: &DislElement) -> Option<(f64, f64)> {
        None
    }

    /// What a gesture that puts `element` at (`x`, `y`) in canvas units writes, or why it may not; none to write the
    /// point into the placement attributes as numbers.
    fn placement(&self, _element: &DislElement, _x: f64, _y: f64) -> Option<Result<ModulePlacement, String>> {
        None
    }

    /// `element`'s stored size, in the units of [`DiagramModule::position`] or in canvas units; none to read it from
    /// the attributes its notation binds (DISL sections 5.8 and 6.9).
    fn extent(&self, _element: &DislElement) -> Option<Extent> {
        None
    }

    /// What a resize of `element` to `resize` writes, or why it may not; none to write it into the attributes its
    /// notation binds.
    fn resize(&self, _element: &DislElement, _resize: &Resize) -> Option<Result<ModulePlacement, String>> {
        None
    }

    /// The `position` an operation or a tool run at the canvas point (`x`, `y`) reads (DISL section 12.3): none for
    /// the point itself, as a map of `x` and `y`.
    fn position_at(&self, _x: f64, _y: f64) -> Option<Map> {
        None
    }

    /// Runs the plugin layout `plugin` (a layout algorithm `plugin:<plugin>`, DISL section 10) over `diagram`: the
    /// changes it makes, as one step, or why it cannot; none when the module does not implement it.
    fn layout(
        &self,
        _specification: &DislSpecification,
        _plugin: &str,
        _diagram: &DislDiagram,
    ) -> Option<Result<Vec<DislChange>, String>> {
        None
    }

    /// The attribute an inline edit of `element`'s label `label` writes when the label is computed and binds none
    /// (DISL section 6.12): none for the specification's answer alone.
    fn label_attribute(&self, _element: &DislElement, _label: &str) -> Option<String> {
        None
    }

    /// The changes of a transaction on `diagram` as the body writes them, before they are planned, or why the body
    /// may not hold them: where the binding's rules tell an element's type from what it holds, a retype is the
    /// attribute change beside it, and the module leaves it out; a module refuses what no edit may make, whichever
    /// gesture, form or operation made it. The changes as they are by default.
    fn written(&self, _diagram: &DislDiagram, changes: Vec<DislChange>) -> Result<Vec<DislChange>, String> {
        Ok(changes)
    }
}

impl fmt::Debug for dyn DiagramModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DiagramModule")
    }
}
