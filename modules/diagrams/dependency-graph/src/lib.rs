//! The dependency graph's residue (etalii.adp spec 030, task T071, after the standalone host's
//! `src/diagrams/dependency-graph` at `8b6ccfc`): what the dependency graph's specification
//! (`definitions/diagrams/dependency-graph.dis`) cannot state, registered with `adp-diagram` as the
//! [`DiagramModule`] of the origin [`ORIGIN`].
//!
//! The standalone module already read and wrote `.dgr` bodies through the binding `dependency-graph.fbl` and derived
//! its palette, menus, rows, operations and findings from the specification; the desktop host does all of that through
//! its generic DISL and FBL path, and keeps here only what the module added in code:
//!
//! - [`ends`]: the CEL function `writtenEnd` of the plugin `net.etalii.adp.generic.dependencyEnds`, which reads a
//!   dependency's ends as written, so a dangling end and a self-dependency between ids no node holds are reported;
//! - [`rows`] and [`placement`]: the module's coordinates, the authored `x` across and 60 canvas units per row down,
//!   and what a gesture to a point writes (the coordinate as sent and the nearest row, both set as the standalone
//!   placement command sets them);
//! - [`findings`]: the findings named by the ids the file writes, in the tool's order and at the tool's places;
//! - no edit, whichever gesture or operation makes it, writes a node that depends on itself
//!   (`ConnectDependencyGraphElementsCommandHandler.cs`: "A node cannot depend on itself.").

pub mod ends;
pub mod findings;
pub mod placement;
pub mod rows;
pub mod theme;

use std::rc::Rc;

use adp_cel::Map;
use adp_diagram::{DiagramDocument, DiagramModule, DiagramProblem, DiagramTools, ModulePlacement};
use adp_disl::{DislChange, DislConstraintOptions, DislDiagram, DislElement, DislPluginFunctions};

/// What a dependency from a node to itself answers.
pub const SELF_DEPENDENCY: &str = "A node cannot depend on itself.";

/// The tool type this module serves.
pub const ORIGIN: &str = "generic/dependencies";

/// The dependency graph's own code beside its specification.
#[derive(Debug, Default, Clone, Copy)]
pub struct DependencyGraphModule;

/// The module, to register with `adp_diagram::DiagramTools::with_module` under [`ORIGIN`].
pub fn module() -> Rc<dyn DiagramModule> {
    Rc::new(DependencyGraphModule)
}

/// `tools` with the dependency graph's module registered.
pub fn register(tools: DiagramTools) -> DiagramTools {
    tools.with_module(ORIGIN, module())
}

impl DiagramModule for DependencyGraphModule {
    fn plugin_functions(&self) -> DislPluginFunctions {
        ends::plugin_functions()
    }

    fn position(&self, element: &DislElement) -> Option<(f64, f64)> {
        (!element.is_relation()).then(|| placement::position(element))
    }

    fn placement(&self, element: &DislElement, x: f64, y: f64) -> Option<Result<ModulePlacement, String>> {
        Some(placement::placement(element, x, y))
    }

    fn position_at(&self, x: f64, y: f64) -> Option<Map> {
        Some(placement::position_at(x, y))
    }

    /// A dependency whose two ends are one node refused, as the standalone connect command refused it.
    fn written(&self, _diagram: &DislDiagram, changes: Vec<DislChange>) -> Result<Vec<DislChange>, String> {
        if changes
            .iter()
            .any(|change| matches!(change, DislChange::Connect { source_id, target_id, .. } if source_id == target_id))
        {
            return Err(SELF_DEPENDENCY.to_owned());
        }
        Ok(changes)
    }

    fn problems(&self, document: &DiagramDocument, options: DislConstraintOptions) -> Option<Vec<DiagramProblem>> {
        Some(findings::judge(document, options))
    }
}
