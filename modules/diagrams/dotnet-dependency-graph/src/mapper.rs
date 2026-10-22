//! Turns a derived graph and its layout into what the shell draws (etalii.adp spec 030, task T082), ported from the
//! standalone host's `DependencyElementMapper.cs` at `8b6ccfc`, whose protobuf payload (`DependencyElementPayload`) is
//! carried here as the element's attributes. The projects, then the packages, then the references, in the graph's
//! order. A stored position wins element by element; a stored id the graph does not hold is not applied. A reference
//! has no position of its own: it carries the origin and the canvas draws it between its two boxes. Every box is
//! drawn 220 by 56 (`DotNetDependencyGraphCanvas.tsx`), placed by its top-left.

use std::collections::BTreeMap;

use adp_diagram::{DiagramElement, ElementKind};

use crate::graph::{DependencyGraphModel, DependsOnKind};
use crate::layout;

/// The element kinds this tool type puts on the wire.
pub const PROJECT_TYPE: &str = "dotnet/dependency-graph+project";
pub const PACKAGE_TYPE: &str = "dotnet/dependency-graph+package";
pub const EDGE_TYPE: &str = "dotnet/dependency-graph+edge";

/// How wide and tall a box is drawn: every node here is one shape.
pub const NODE_WIDTH: f64 = 220.0;
pub const NODE_HEIGHT: f64 = 56.0;

/// Every element of `graph`, arranged: the positions in `stored` overlaying the computed ones.
pub fn elements(graph: &DependencyGraphModel, stored: &BTreeMap<String, (f64, f64)>) -> Vec<DiagramElement> {
    let computed = layout::compute(graph);
    let position = |id: &str| {
        stored
            .get(id)
            .or_else(|| computed.get(id))
            .copied()
            .unwrap_or((0.0, 0.0))
    };
    let size = || {
        [
            ("width".to_owned(), NODE_WIDTH.to_string()),
            ("height".to_owned(), NODE_HEIGHT.to_string()),
        ]
    };
    let mut elements = Vec::with_capacity(graph.projects.len() + graph.packages.len() + graph.edges.len());
    for project in &graph.projects {
        let (x, y) = position(&project.id);
        let mut attributes = vec![
            ("name".to_owned(), project.name.clone()),
            ("kind".to_owned(), "Project".to_owned()),
            ("projectRelativePath".to_owned(), project.relative_path.clone()),
            ("targetFrameworks".to_owned(), project.target_frameworks.join(", ")),
        ];
        attributes.extend(size());
        elements.push(DiagramElement {
            id: project.id.clone(),
            x,
            y,
            placed: stored.contains_key(&project.id),
            element_type: PROJECT_TYPE.to_owned(),
            kind: ElementKind::Node,
            parent: None,
            source: None,
            target: None,
            attributes,
        });
    }
    for package in &graph.packages {
        let (x, y) = position(&package.id);
        let mut attributes = vec![
            ("name".to_owned(), package.package_id.clone()),
            ("kind".to_owned(), "Package".to_owned()),
            ("versions".to_owned(), package.versions.join(", ")),
            (
                "hasVersionConflict".to_owned(),
                package.has_version_conflict.to_string(),
            ),
            (
                "dependentProjectCount".to_owned(),
                package.dependent_project_count.to_string(),
            ),
            ("isAmbient".to_owned(), package.is_ambient.to_string()),
        ];
        attributes.extend(size());
        elements.push(DiagramElement {
            id: package.id.clone(),
            x,
            y,
            placed: stored.contains_key(&package.id),
            element_type: PACKAGE_TYPE.to_owned(),
            kind: ElementKind::Node,
            parent: None,
            source: None,
            target: None,
            attributes,
        });
    }
    for edge in &graph.edges {
        let kind = match edge.kind {
            DependsOnKind::Project => "ProjectReference",
            DependsOnKind::Package => "PackageReference",
        };
        elements.push(DiagramElement {
            id: edge.id.clone(),
            x: 0.0,
            y: 0.0,
            placed: false,
            element_type: EDGE_TYPE.to_owned(),
            kind: ElementKind::Relation,
            parent: None,
            source: Some(edge.from.clone()),
            target: Some(edge.to.clone()),
            attributes: vec![("kind".to_owned(), kind.to_owned())],
        });
    }
    elements
}
