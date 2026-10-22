//! The .NET dependency graph's DISL definition, loaded once, and what is derived from it (etalii.adp spec 030, task
//! T082): the property rows of a project or a package, and the sentence a write is refused with, mapped onto the
//! standalone host's ids by its `x-dotnet` block.
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/dotnet-dependency-graph/backend/
//! EtAlii.Adp.Diagram.DotNetDependencyGraph/DotNetDefinition.cs`. The definition is the vendored
//! `definitions/diagrams/dotnet-dependency-graph.dis`, whose metamodel, forms, reasons and messages are the ones the
//! standalone host bundles; only its `persistence` block differs (a binding at a later stage, where the bundled copy
//! names the plugin), and nothing here reads it. The model is built from what the readers read, not through FBL: a
//! graph spans the solution, every project file it names, the props files above them and the NuGet cache. Everything is
//! read-only (`env.readOnly`): every row carries the reason its form item gives, and a write is refused with the
//! definition's `std.readOnly`.

use adp_cel::{Map, Value};
use adp_context::{PropertyEditor, PropertyRow};
use adp_disl::derivation::forms;
use adp_disl::{DislDiagram, DislEnv, DislLoader, DislPluginFunctions, DislSpecification, WireIdMap};

use crate::graph::{DependencyGraphModel, DependsOnKind};

/// The vendored definition.
const DEFINITION: &[u8] =
    include_bytes!("../../../../vendor/etalii.adp/definitions/diagrams/dotnet-dependency-graph.dis");

/// The definition and its wire ids.
#[derive(Debug, Clone)]
pub struct DotNetDefinition {
    pub specification: DislSpecification,
    pub ids: WireIdMap,
}

thread_local! {
    static LOADED: DotNetDefinition = DotNetDefinition::load();
}

fn list(values: &[String]) -> Value {
    Value::from(values.iter().map(|v| Value::from(v.as_str())).collect::<Vec<_>>())
}

impl DotNetDefinition {
    /// The definition, loaded once per thread.
    pub fn get() -> DotNetDefinition {
        LOADED.with(Clone::clone)
    }

    fn load() -> DotNetDefinition {
        let loaded = DislLoader::load_bytes(DEFINITION, &DislPluginFunctions::none());
        let specification = loaded.specification.unwrap_or_else(|| {
            panic!(
                "the vendored dotnet-dependency-graph.dis loads: {:?}",
                loaded.diagnostics
            )
        });
        let ids = WireIdMap::of(&specification, "x-dotnet");
        DotNetDefinition { specification, ids }
    }

    /// What `env` reads: nothing in this diagram can be edited.
    pub fn env() -> DislEnv {
        DislEnv {
            read_only: true,
            ..DislEnv::default()
        }
    }

    /// Why nothing here can be written: the sentence of `behavior.messages`' `std.readOnly`.
    pub fn refusal(&self) -> String {
        self.specification
            .root()
            .get("behavior")
            .and_then(|b| b.get("messages"))
            .and_then(|m| m.get("std.readOnly"))
            .and_then(|m| m.as_str())
            .expect("the definition words a std.readOnly to refuse a write with")
            .to_owned()
    }

    /// `graph` as the definition's model: a node per project and package, a relation per reference. A reference whose
    /// end the graph does not hold is kept with that end missing.
    pub fn model(&self, graph: &DependencyGraphModel) -> DislDiagram {
        let diagram = DislDiagram::new(&self.specification);
        for project in &graph.projects {
            let mut attributes = Map::default();
            attributes.insert("name", Value::from(project.name.as_str()));
            attributes.insert("path", Value::from(project.relative_path.as_str()));
            attributes.insert("targetFrameworks", list(&project.target_frameworks));
            if let Some(version) = &project.dotnet_version {
                attributes.insert("dotNetVersion", Value::from(version.as_str()));
            }
            if diagram.element_by_id(&project.id).is_none() {
                let _ = diagram.add_node("Project", &project.id, attributes);
            }
        }
        for package in &graph.packages {
            let mut attributes = Map::default();
            attributes.insert("name", Value::from(package.package_id.as_str()));
            attributes.insert("versions", list(&package.versions));
            attributes.insert("hasVersionConflict", Value::from(package.has_version_conflict));
            attributes.insert("dependentProjectCount", Value::from(package.dependent_project_count));
            attributes.insert("isAmbient", Value::from(package.is_ambient));
            if let Some(description) = &package.description {
                attributes.insert("description", Value::from(description.as_str()));
            }
            if diagram.element_by_id(&package.id).is_none() {
                let _ = diagram.add_node("Package", &package.id, attributes);
            }
        }
        for edge in &graph.edges {
            let kind = match edge.kind {
                DependsOnKind::Project => "ProjectReference",
                DependsOnKind::Package => "PackageReference",
            };
            let from = diagram.element_by_id(&edge.from);
            let to = diagram.element_by_id(&edge.to);
            let _ = diagram.add_relation(kind, &edge.id, from.as_ref(), to.as_ref(), Map::default());
        }
        diagram
    }

    /// The property rows of the element `element_id` names in `graph`: its form's; none for a reference or an id the
    /// graph does not hold.
    pub fn rows(&self, graph: &DependencyGraphModel, element_id: &str) -> Vec<PropertyRow> {
        let diagram = self.model(graph);
        let Some(element) = diagram.element_by_id(element_id) else {
            return Vec::new();
        };
        forms::derive(
            &self.specification,
            &element,
            &DotNetDefinition::env(),
            &self.ids,
            "inspector",
        )
        .into_iter()
        .map(|row| PropertyRow {
            id: row.id,
            label: row.label,
            value: row.value,
            editor: PropertyEditor::Line,
            read_only_reason: row.read_only_reason,
            group: row.group,
            candidates: Vec::new(),
        })
        .collect()
    }
}
