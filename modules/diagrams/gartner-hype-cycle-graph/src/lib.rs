//! The Gartner hype cycle graph's residue (etalii.adp spec 030, task T073, after the standalone host's
//! `src/diagrams/gartner-hype-cycle-graph` at `8b6ccfc`): what its specification
//! (`definitions/diagrams/gartner-hype-cycle-graph.dis`) cannot state, registered with `adp-diagram` as the
//! [`DiagramModule`] of the origin [`ORIGIN`].
//!
//! The standalone module read and wrote `.ghg` bodies through the binding `gartner-hype-cycle-graph.fbl` and its own
//! commands; the desktop host reads and writes them through the specification and the same binding, like every other
//! DISL and FBL diagram, and keeps here only what the module's code added:
//!
//! - [`scale`]: the time axis, four canvas units per step of the diagram's unit from 1900-01, and rows 56 units
//!   apart (`GhgScale`), with where each element is drawn and what a move or a resize writes;
//! - [`phases`]: the phase boundaries, spread, moved, rescaled and kept a month apart (`GhgPhases`);
//! - [`edits`]: what the standalone module's commands wrote where the specification's generic edit writes otherwise;
//! - [`arrangement`]: the rows Arrange diagram gives (`GhgArrangement`, the layout `rowArrange`);
//! - [`compact`]: where the Compact viewpoint draws every element, the layout `rowPacked`;
//! - [`ruler`]: the labels of the time ruler under the canvas, on a scale uniform in months.

pub mod arrangement;
pub mod compact;
pub mod edits;
pub mod phases;
pub mod placement;
pub mod reading;
pub mod ruler;
pub mod scale;

use std::rc::Rc;

use adp_cel::Map;

use adp_diagram::session::{ActionOutcome, ActionTarget};
use adp_diagram::{DiagramModule, DiagramSession, DiagramWorkspace, Extent, ModulePlacement, Resize};
use adp_disl::{DislChange, DislDiagram, DislElement, DislSpecification, IdSource};

use crate::scale::TimeUnit;

/// The tool type this module serves.
pub const ORIGIN: &str = "gartner/hypecycle-graph";

/// A new, empty `.ghg` body, as the standalone host's Add dialog wrote it (`GhgParser.EmptyDocument("\r\n")`,
/// `ServiceCollection.AddGartnerHypeCycleGraph.cs`): the header and both sections written flow-empty, with CRLF.
pub const EMPTY_DOCUMENT: &str = "gartner-hypecycle-graph: 1\r\ntrends: []\r\ninfluences: []\r\n";

/// The hype cycle graph's own code beside its specification.
#[derive(Debug, Default, Clone, Copy)]
pub struct HypeCycleModule;

/// The module, to register with `adp_diagram::DiagramTools::with_module` under [`ORIGIN`].
pub fn module() -> Rc<dyn DiagramModule> {
    Rc::new(HypeCycleModule)
}

impl DiagramModule for HypeCycleModule {
    /// A trend's and a note's top-left corner, a trigger's centre, in canvas units.
    fn position(&self, element: &DislElement) -> Option<(f64, f64)> {
        placement::position(element)
    }

    fn placement(&self, element: &DislElement, x: f64, y: f64) -> Option<Result<ModulePlacement, String>> {
        placement::placement(element, x, y)
    }

    fn extent(&self, element: &DislElement) -> Option<Extent> {
        placement::extent(element)
    }

    fn resize(&self, element: &DislElement, resize: &Resize) -> Option<Result<ModulePlacement, String>> {
        placement::resize(element, resize)
    }

    /// A point on the canvas as an addition there reads it, in a diagram of months: the month index and the row.
    fn position_at(&self, x: f64, y: f64) -> Option<Map> {
        Some(placement::position_at(x, y, TimeUnit::Month))
    }

    /// A point on the canvas of `diagram` as an addition there reads it, in its time unit: the month index and the
    /// row, both fractional (`GhgDefinition.Position`).
    fn position_in(&self, diagram: &DislDiagram, x: f64, y: f64) -> Option<Map> {
        Some(placement::position_at(x, y, reading::unit_of(diagram)))
    }

    fn execute(
        &self,
        session: &DiagramSession,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        ids: &dyn IdSource,
    ) -> Option<ActionOutcome> {
        edits::execute(session, workspace, target, action_id, ids)
    }

    fn layout(
        &self,
        _specification: &DislSpecification,
        plugin: &str,
        diagram: &DislDiagram,
    ) -> Option<Result<Vec<DislChange>, String>> {
        (plugin == arrangement::ALGORITHM).then(|| arrangement::arrange(diagram))
    }

    fn written(&self, diagram: &DislDiagram, changes: Vec<DislChange>) -> Result<Vec<DislChange>, String> {
        edits::written(diagram, changes)
    }
}
