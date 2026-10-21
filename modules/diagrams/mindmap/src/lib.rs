//! The mind map diagram's residue (etalii.adp spec 030, FR-018, FR-031 and FR-032, task T059): what the tool type
//! `freeplane/mindmap` needs beside its specification (`vendor/etalii.adp/definitions/diagrams/mindmap.dis`) and its
//! binding (`mindmap.fbl#mindmap`), as `mindmap.md` lists it. Ported from the standalone host's
//! `src/diagrams/mindmap/backend/EtAlii.Adp.Diagram.Mindmap` at `8b6ccfc`.
//!
//! - [`xml`] and [`document`]: the `.mm` format, read and written back byte for byte where nothing changed.
//! - [`plugin`]: the persistence plugin `net.etalii.adp.freeplane.mm` the binding names as its reader, which reads a
//!   body into nodes and plans every change as the splice the standalone host's commands and save came to.
//! - [`layout`]: the two-sided layout `net.etalii.adp.freeplane.mindmapLayout`.
//! - [`actions`]: what a node's menu does, sibling naming and the link picker included.
//! - [`links`]: a link stored relative to the map and picked relative to the project.
//! - [`validator`]: the map's findings, as the problems panel shows them.
//!
//! [`MindmapModule`] hands all of it to `adp-diagram` through its per-origin hook ([`register`]).

pub mod actions;
pub mod document;
pub mod layout;
pub mod links;
pub mod plugin;
pub mod validator;
pub mod xml;

use std::collections::HashMap;
use std::rc::Rc;

use adp_diagram::session::{ActionOutcome, ActionTarget, DiagramSession};
use adp_diagram::workspace::DiagramWorkspace;
use adp_diagram::{DiagramDocument, DiagramModule, DiagramTools};
use adp_disl::{DislDiagram, DislElement, IdSource};
use adp_fbl::plugins::PersistencePlugin;

use crate::actions::MindmapActions;
use crate::layout::{LayoutNode, MindmapMetrics};
use crate::plugin::{MindmapFreeplanePlugin, PLUGIN_ID};

/// The tool type this module serves.
pub const ORIGIN: &str = "freeplane/mindmap";

/// The extension of a mind map's body.
pub const EXTENSION: &str = ".mm";

/// The mind map's residue, as `adp-diagram` asks for it.
#[derive(Debug, Default)]
pub struct MindmapModule {
    actions: MindmapActions,
    metrics: MindmapMetrics,
}

impl MindmapModule {
    pub fn new() -> Self {
        Self::default()
    }

    /// The module with the layout's metrics `metrics`.
    pub fn with_metrics(metrics: MindmapMetrics) -> Self {
        MindmapModule {
            metrics,
            ..Self::default()
        }
    }

    pub fn actions(&self) -> &MindmapActions {
        &self.actions
    }
}

/// `tools` with the mind map's module added to its tool type.
pub fn register(tools: DiagramTools) -> DiagramTools {
    tools.with_module(ORIGIN, Rc::new(MindmapModule::new()))
}

impl DiagramModule for MindmapModule {
    fn persistence(&self, plugin_id: &str) -> Option<Rc<dyn PersistencePlugin>> {
        (plugin_id == PLUGIN_ID).then(|| Rc::new(MindmapFreeplanePlugin) as Rc<dyn PersistencePlugin>)
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

    /// The centre of every visible node's box, as the two-sided layout places it for this viewer's folds.
    fn positions(&self, document: &DiagramDocument) -> Option<HashMap<String, (f64, f64)>> {
        let diagram = &document.model().diagram;
        let root = central_topic(diagram)?;
        let tree = layout_tree(&root);
        let boxes = layout::compute(&tree, &self.metrics, &|id| self.actions.is_folded(document, id));
        Some(
            boxes
                .into_iter()
                .map(|(id, place)| (id, (place.center_x(), place.center_y())))
                .collect(),
        )
    }
}

/// The tree under `element`, as the layout reads it.
fn layout_tree(element: &DislElement) -> LayoutNode {
    let text = |name: &str| element.value_of(name).as_str().map(str::to_owned);
    LayoutNode {
        id: element.id(),
        text: text("text").unwrap_or_default(),
        position: text("position"),
        children: element.children().iter().map(layout_tree).collect(),
    }
}

/// The diagram's central topic: its one node without a parent.
pub fn central_topic(diagram: &DislDiagram) -> Option<DislElement> {
    diagram.nodes().into_iter().find(|n| n.parent().is_none())
}
