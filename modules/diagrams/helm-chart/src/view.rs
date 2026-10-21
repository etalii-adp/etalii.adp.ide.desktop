//! One open Helm chart diagram, and what is asked of it (etalii.adp spec 030, task T080).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/helm-chart/backend/EtAlii.Adp.Diagram.HelmChart/
//! HelmSession.cs` (a reposition stored in the registration as one undoable step, an edge refused because it follows
//! its ends, a reparenting move refused), `HelmContextSourceResolver.cs` (a node reads as its own file and name, a
//! dependency as the file that declares it, a relation as its declaring side and what it does),
//! `HelmContextPropertyProvider.cs` (every row read-only, every write refused with the definition's `std.readOnly`)
//! and `HelmValidator.cs`. The module registers no toolbox and no actions, which is its whole statement: nothing can be
//! added to a diagram that mirrors a folder. Positions are in canvas units, a box placed by its top-left corner.

use std::path::{Path, PathBuf};

use adp_context::{ActionGroup, PropertyRow};
use adp_diagram::session::{ActionOutcome, ActionTarget};
use adp_diagram::{
    DiagramDocuments, DiagramElement, DiagramProblem, DiagramViewport, DiagramWorkspace, ModuleView, ToolboxItem,
};
use adp_history::{Command, Composite, Outcome};

use crate::commands::SetRegistrationLayout;
use crate::definition::{HelmDefinition, text};
use crate::store::{self, HelmFolder};
use crate::{findings, mapper, registration_layout};

/// What a move of a relation answers: it has no position of its own, it follows its ends.
pub const EDGE_REFUSAL: &str = "That element is not something this diagram can move.";

/// What a reparenting move answers: nothing in this diagram nests under a parent, its place is a position.
pub const REPARENT_REFUSAL: &str =
    "A chart element has no parent to move it under; dragging changes where it sits on the canvas.";

/// What an action answers: this diagram offers none.
pub const NO_ACTION: &str = "That action is not available for this item.";

/// One open view of a registered chart folder.
#[derive(Debug, Clone)]
pub struct HelmView {
    root: PathBuf,
    registration: PathBuf,
}

impl HelmView {
    pub fn new(root: &Path, registration: &Path) -> Self {
        HelmView {
            root: root.to_path_buf(),
            registration: registration.to_path_buf(),
        }
    }

    pub fn registration(&self) -> &Path {
        &self.registration
    }

    fn folder<'a>(&self, documents: &'a DiagramDocuments) -> Result<&'a HelmFolder, String> {
        store::get(documents, &self.registration).ok_or_else(|| adp_diagram::document::NOT_OPEN.to_owned())
    }
}

fn segments(relative: &str) -> Vec<String> {
    relative
        .split(['/', '\\'])
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

impl ModuleView for HelmView {
    fn render(&self, documents: &DiagramDocuments, viewport: DiagramViewport) -> Result<Vec<DiagramElement>, String> {
        let folder = self.folder(documents)?;
        let stored = registration_layout::read(&folder.registration_text);
        Ok(mapper::visible(&folder.model.diagram, viewport, &stored))
    }

    fn describe(&self, documents: &DiagramDocuments, element_id: &str) -> Option<(Vec<String>, String)> {
        let diagram = &self.folder(documents).ok()?.model.diagram;
        // The first node of the id: two dependencies of one effective name share one.
        if let Some(node) = diagram.nodes().into_iter().find(|n| n.id() == element_id) {
            let path = mapper::path_of(&node);
            let path = if path.is_empty() {
                declaring_file(diagram).to_owned()
            } else {
                path
            };
            return Some((segments(&path), mapper::name_of(&node)));
        }
        // A relation answers as its declaring side, by what it does.
        let relation = diagram.relations().into_iter().find(|r| r.id() == element_id)?;
        let source_path = relation
            .source()
            .and_then(|s| diagram.nodes().into_iter().find(|n| n.id() == s.id()))
            .map(|n| mapper::path_of(&n))
            .filter(|p| !p.is_empty());
        let declared_in = source_path.unwrap_or_else(|| declaring_file(diagram).to_owned());
        let verb = verb(&relation.type_name());
        let says = text(&relation.value_of("label"));
        let label = if says.is_empty() {
            verb.to_owned()
        } else {
            format!("{verb} {says}")
        };
        Some((segments(&declared_in), label))
    }

    fn toolbox(&self) -> Vec<ToolboxItem> {
        Vec::new()
    }

    fn problems(&self, documents: &DiagramDocuments) -> Vec<DiagramProblem> {
        let Ok(folder) = self.folder(documents) else {
            return Vec::new();
        };
        findings::rebased(problems_of(folder), &self.root, &folder.folder)
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
        self.folder(documents)
            .map(|folder| HelmDefinition::get().rows(&folder.model.diagram, element_id))
            .unwrap_or_default()
    }

    /// Refused, always, with the definition's `std.readOnly`.
    fn set_property(
        &self,
        _workspace: &mut DiagramWorkspace,
        _element_ids: &[&str],
        _property_id: &str,
        _value: &str,
    ) -> Result<(), String> {
        Err(HelmDefinition::get().write_refusal())
    }

    /// Each position stored in the registration's `layout:` block, as one step; a relation refused.
    fn move_elements(&self, workspace: &mut DiagramWorkspace, moves: &[(String, f64, f64)]) -> Result<(), String> {
        self.folder(workspace.documents())?;
        if moves.iter().any(|(id, _, _)| id.starts_with("edge:")) {
            return Err(EDGE_REFUSAL.to_owned());
        }
        let commands: Vec<Box<dyn Command<DiagramDocuments>>> = moves
            .iter()
            .map(|(id, x, y)| {
                Box::new(SetRegistrationLayout {
                    registration: self.registration.clone(),
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

    /// Refused, always: nothing in this diagram nests under a parent.
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

/// The findings of `folder`, located relative to the chart folder: for a registered folder that is not a chart, the one
/// warning `helm.not-a-chart`; otherwise the definition's, in the tool's order.
pub fn problems_of(folder: &HelmFolder) -> Vec<DiagramProblem> {
    if !folder.is_chart() {
        return findings::not_a_chart();
    }
    let definition = HelmDefinition::get();
    findings::problems(
        &folder.model.diagram,
        &definition.findings(&folder.model, &folder.reading.findings),
    )
}

/// Where dependencies are declared: `Chart.yaml`, or `requirements.yaml` for a legacy chart.
fn declaring_file(diagram: &adp_disl::DislDiagram) -> &'static str {
    let legacy = diagram
        .nodes_of_type("Chart")
        .first()
        .is_some_and(|chart| chart.value_of("legacy") == adp_cel::Value::Bool(true));
    if legacy { "requirements.yaml" } else { "Chart.yaml" }
}

/// What a relation of the definition's type does, as a reader says it.
fn verb(type_name: &str) -> &'static str {
    match type_name {
        "Declares" => "declares",
        "Resolves" => "resolves to",
        "Overrides" => "overrides",
        "Configures" => "configures",
        "Includes" => "includes",
        _ => "",
    }
}
