//! The Helm chart diagram's residue (etalii.adp spec 030, FR-018, FR-031 and FR-032, task T080): the tool type
//! `helm/chart`, whose subject is a folder, after the standalone host's
//! `src/diagrams/helm-chart/backend/EtAlii.Adp.Diagram.HelmChart` at `8b6ccfc`.
//!
//! The diagram is the chart folder its registration sits in: the `.adp` beside `Chart.yaml` is the whole of what ADP
//! contributes, and the standalone host routed it as its own body. The folder is read through the FBL library's folder
//! body with the binding `helm-chart.fbl#chart` and its persistence plugin `net.etalii.adp.helm.chartFolder`, the
//! module's own reading; the vendored `helm-chart.dis` is the model, the rows and the findings of that reading. The one
//! edit is a position, written into the registration's `layout:` block; nothing inside the chart is ever written. The
//! generic path opens no folder subject, so [`HelmModule`] [opens its documents
//! itself](adp_diagram::DiagramModule::opens_documents) and each open view is a [`view::HelmView`].
//!
//! - [`yaml`], [`model`], [`template_scan`], [`entries`] and [`reader`]: the chart as read.
//! - [`resolution`] and [`plugin`]: its dependencies matched, and its nodes and relations delivered as the plugin's
//!   elements.
//! - [`definition`]: the definition and binding, the model of a reading, the rows and the findings.
//! - [`layout`] and [`mapper`]: the banded layout and what the shell draws.
//! - [`registration_layout`] and [`commands`]: the stored positions and the command that stores one.
//! - [`findings`], [`store`] and [`view`]: the problems, the open folders, and one open view.
//!
//! The add dialog registers a folder for this tool type with a `.adp` and no body, as the standalone host did: the
//! module has no document factory, since there is no empty body to write.

pub mod commands;
pub mod definition;
pub mod entries;
pub mod findings;
pub mod layout;
pub mod mapper;
pub mod model;
pub mod plugin;
pub mod reader;
pub mod registration_layout;
pub mod resolution;
pub mod store;
pub mod template_scan;
pub mod view;
pub mod yaml;

use std::path::Path;
use std::rc::Rc;

use adp_diagram::workspace::{CANNOT_OPEN, OpenedDiagram};
use adp_diagram::{DiagramDocuments, DiagramModule, DiagramTools, ModuleView};

use crate::view::HelmView;

/// The tool type this module serves.
pub const ORIGIN: &str = "helm/chart";

/// The Helm chart module.
#[derive(Debug, Clone, Copy, Default)]
pub struct HelmModule;

/// The Helm chart module, as a tool type's module.
pub fn module() -> Rc<dyn DiagramModule> {
    Rc::new(HelmModule)
}

/// `tools` with the Helm chart module added to its tool type.
pub fn register(tools: DiagramTools) -> DiagramTools {
    tools.with_module(ORIGIN, module())
}

impl DiagramModule for HelmModule {
    fn opens_documents(&self) -> bool {
        true
    }

    /// Opens the registration at `path`: the diagram is the folder it sits in, and the registration is its own body
    /// (`HelmSessionFactory.Open`).
    fn open(
        &self,
        root: &Path,
        path: &Path,
        origin: &str,
        documents: &mut DiagramDocuments,
    ) -> Result<(OpenedDiagram, Rc<dyn ModuleView>), String> {
        let is_registration = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("adp"))
            || path.file_name().is_some_and(|n| n.eq_ignore_ascii_case(".adp"));
        if !is_registration || !path.is_file() || path.parent().is_none() {
            return Err(CANNOT_OPEN.to_owned());
        }
        store::get_or_load(documents, path);
        let view: Rc<dyn ModuleView> = Rc::new(HelmView::new(root, path));
        Ok((
            OpenedDiagram {
                origin: origin.to_owned(),
                body_path: path.to_path_buf(),
                registration_path: Some(path.to_path_buf()),
            },
            view,
        ))
    }
}
