//! The desktop host's window (etalii.adp spec 030, research R2, task T021): one native window per workspace, drawn
//! with egui on wgpu, exposed to screen readers through AccessKit, in VS Code's default theme following the system.
//!
//! The window is a library that the `adp-desktop` binary runs, so that the tests under `tests/` drive the shell as the
//! window does: the accessibility tree a frame hands the platform is read there (task T088).

mod a11y;
mod add_dialog;
mod atd_client;
mod canvas;
/// What a failed start leaves behind: a crash log and a message box.
pub mod crash;
mod debugger;
mod documents;
mod editor;
mod explorer;
mod hierarchy;
mod highlight;
mod icons;
mod markdown;
mod platform_open;
mod problems;
mod project_diagrams;
mod properties;
mod ribbon;
/// The recording painter, for `atd record` (spec 035, T051).
pub use canvas::recording;
pub mod shell;
mod table;
mod validation;
// The table's tests sit beside it, declared here so that the benchmark, which includes the table module, leaves them out.
#[cfg(test)]
mod screenshot;
#[cfg(test)]
#[path = "table/tests.rs"]
mod table_tests;
pub mod theme;

/// The capabilities of the desktop host as the shell assembles them (etalii.adp spec 035, T125, research R14): the
/// registry's own, and those the modules contribute from code that stays with them: the RDF module's, and the Helm
/// chart module's folder reader as `reader.helmChart`, the SPARQL module's query reader as `reader.sparql` and the Ansible
/// module's folder reader, handed the folder's listing, as `reader.ansibleProject`, and the Azure DevOps pipeline module's
/// reader and writer as `reader.azurePipelinesYaml`, and the .NET dependency graph module's solution reader as
/// `reader.msbuildSolution` with its `analysis.msbuildImports` (T132).
pub fn capabilities() -> adp_atd_runtime::Capabilities {
    adp_diagram_rdf::capabilities::contribute(adp_atd_runtime::Capabilities::desktop())
        .with_reader(
            "reader.helmChart",
            std::rc::Rc::new(adp_diagram_helm_chart::plugin::HelmChartFolderPlugin),
        )
        .with_reader(
            "reader.sparql",
            std::rc::Rc::new(adp_diagram_sparql::plugin::SparqlQueryPlugin),
        )
        .with_reader(
            "reader.ansibleProject",
            std::rc::Rc::new(adp_diagram_ansible_structure::plugin::AnsibleFolderPlugin::handed()),
        )
        .with_reader(
            adp_diagram_azure_devops_pipeline::plugin::CAPABILITY,
            std::rc::Rc::new(adp_diagram_azure_devops_pipeline::plugin::PipelineYamlPlugin),
        )
        .with_reader(
            adp_diagram_dotnet_dependency_graph::plugin::READER,
            std::rc::Rc::new(adp_diagram_dotnet_dependency_graph::plugin::SolutionPlugin),
        )
        .with_analysis(
            adp_diagram_dotnet_dependency_graph::plugin::IMPORTS,
            std::rc::Rc::new(MsbuildImports),
        )
}

/// `analysis.msbuildImports`, the .NET dependency graph module's project reader.
struct MsbuildImports;

impl adp_atd_runtime::capabilities::AnalysisCapability for MsbuildImports {
    fn analyse(&self, input: &serde_json::Value, _args: &serde_json::Value) -> Result<serde_json::Value, String> {
        adp_diagram_dotnet_dependency_graph::plugin::imports(input)
    }
}

/// A runtime host with every capability of the desktop host.
pub fn runtime_host() -> adp_atd_runtime::RuntimeHost {
    adp_atd_runtime::RuntimeHost {
        capabilities: capabilities(),
        ..adp_atd_runtime::RuntimeHost::default()
    }
}
