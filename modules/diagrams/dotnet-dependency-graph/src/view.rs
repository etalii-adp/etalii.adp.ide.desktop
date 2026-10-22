//! One open .NET dependency graph, and what is asked of it (etalii.adp spec 030, task T082).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/dotnet-dependency-graph/backend/
//! EtAlii.Adp.Diagram.DotNetDependencyGraph/DotNetDependencyGraphSession.cs` (the whole graph whatever the viewport, a
//! reposition stored in the registration as one undoable step, a reference refused because it follows its ends, a
//! reparent refused), `DotNetContextSourceResolver.cs` (a project reads as its file and name, a package as its id, a
//! reference as its kind) and `DotNetContextPropertyProvider.cs` (every row read-only, every write refused, with the
//! row's reason or the definition's sentence). The module registers no toolbox, no actions and no validation: nothing
//! can be added to a diagram whose contents are derived, and a solution's failures travel with its graph without being
//! reported.

use std::path::{Path, PathBuf};

use adp_context::{ActionGroup, PropertyRow};
use adp_diagram::session::{ActionOutcome, ActionTarget};
use adp_diagram::{
    DiagramDocuments, DiagramElement, DiagramProblem, DiagramViewport, DiagramWorkspace, ModuleView, ToolboxItem,
};
use adp_history::{Command, Composite, Outcome};

use crate::commands::SetRegistrationLayout;
use crate::definition::DotNetDefinition;
use crate::graph::DependsOnKind;
use crate::store::{self, OpenSolution};
use crate::{mapper, registration_layout};

/// What a move of a reference answers: it has no position of its own, it follows its ends.
pub const EDGE_REFUSAL: &str = "That element is not something this diagram can move.";

/// What a reparent answers, whatever the element.
pub const REPARENT_REFUSAL: &str = "A .NET dependency graph is drawn from the solution's own files, so nothing on it can be moved from here. Change a reference in the project file, and the diagram will follow.";

/// What a move answers on a diagram opened without a registration.
pub const NO_REGISTRATION: &str =
    "This diagram was opened without a registration, so there is nowhere to store a position.";

/// What an action answers: this diagram offers none.
pub const NO_ACTION: &str = "That action is not available for this item.";

/// One open view of a solution.
#[derive(Debug, Clone)]
pub struct DotNetView {
    solution: PathBuf,
    registration: Option<PathBuf>,
}

impl DotNetView {
    pub fn new(solution: &Path, registration: Option<&Path>) -> Self {
        DotNetView {
            solution: solution.to_path_buf(),
            registration: registration.map(Path::to_path_buf),
        }
    }

    fn open<'a>(&self, documents: &'a DiagramDocuments) -> Result<&'a OpenSolution, String> {
        store::get(documents, &self.solution).ok_or_else(|| adp_diagram::document::NOT_OPEN.to_owned())
    }
}

fn segments(relative: &str) -> Vec<String> {
    relative
        .split(['/', '\\'])
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

impl ModuleView for DotNetView {
    /// The whole graph, whatever the viewport, with the positions the registration holds.
    fn render(&self, documents: &DiagramDocuments, _viewport: DiagramViewport) -> Result<Vec<DiagramElement>, String> {
        let open = self.open(documents)?;
        let stored = self
            .registration
            .as_deref()
            .map(|r| registration_layout::read(store::registration_text(documents, r)))
            .unwrap_or_default();
        Ok(mapper::elements(&open.model, &stored))
    }

    fn describe(&self, documents: &DiagramDocuments, element_id: &str) -> Option<(Vec<String>, String)> {
        let model = &self.open(documents).ok()?.model;
        if let Some(project) = model.projects.iter().find(|p| p.id == element_id) {
            return Some((segments(&project.relative_path), project.name.clone()));
        }
        if let Some(package) = model.packages.iter().find(|p| p.id == element_id) {
            return Some((vec![package.package_id.clone()], package.package_id.clone()));
        }
        model.edges.iter().find(|e| e.id == element_id).map(|edge| {
            let label = match edge.kind {
                DependsOnKind::Project => "ProjectReference",
                DependsOnKind::Package => "PackageReference",
            };
            (vec![edge.id.clone()], label.to_owned())
        })
    }

    fn toolbox(&self) -> Vec<ToolboxItem> {
        Vec::new()
    }

    fn problems(&self, _documents: &DiagramDocuments) -> Vec<DiagramProblem> {
        Vec::new()
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
        self.open(documents)
            .map(|open| DotNetDefinition::get().rows(&open.model, element_id))
            .unwrap_or_default()
    }

    /// Refused, always: with the row's read-only reason, as the standalone host's `ContextPropertyResolver` refused a
    /// write to a row its provider described as read-only; with the definition's `std.readOnly`, as the provider's
    /// `SetAsync` did, for a row the element does not have.
    fn set_property(
        &self,
        workspace: &mut DiagramWorkspace,
        element_ids: &[&str],
        property_id: &str,
        _value: &str,
    ) -> Result<(), String> {
        let reason = element_ids
            .iter()
            .flat_map(|id| self.properties(workspace.documents(), id))
            .find(|row| row.id == property_id && !row.read_only_reason.is_empty())
            .map(|row| row.read_only_reason);
        Err(reason.unwrap_or_else(|| DotNetDefinition::get().refusal()))
    }

    /// Each position stored in the registration's `layout:` block, as one step; a reference refused.
    fn move_elements(&self, workspace: &mut DiagramWorkspace, moves: &[(String, f64, f64)]) -> Result<(), String> {
        let Some(registration) = &self.registration else {
            return Err(NO_REGISTRATION.to_owned());
        };
        if moves.iter().any(|(id, _, _)| id.starts_with("depends:")) {
            return Err(EDGE_REFUSAL.to_owned());
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

    /// Refused, always: a project belongs to the solution that names it.
    fn move_element(
        &self,
        _workspace: &mut DiagramWorkspace,
        _element_id: &str,
        _new_parent_id: Option<&str>,
        _index: i64,
    ) -> Result<(), String> {
        Err(REPARENT_REFUSAL.to_owned())
    }
}
