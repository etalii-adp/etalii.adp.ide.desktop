//! What the shell draws of each of the three readings: boxes at their places, frames, and the edges between them
//! (etalii.adp spec 030, task T081).
//!
//! Ported from the standalone host's `src/diagrams/databricks/backend/EtAlii.Adp.Diagram.Databricks/
//! DatabricksElementMapper.cs` and `api/databricks.proto` at `8b6ccfc`, with what `client/databricksModel.ts` made of
//! each payload: a box's kind, its label and the badges it wears. Positions come in already merged, computed layout
//! with the authored `layout:` block overlaid, and every position is a box's top-left corner. An edge naming an element
//! that does not exist still goes out, so the canvas can mark it and the rules can report it.

use std::collections::HashSet;

use adp_diagram::element::Rect;
use adp_diagram::{DiagramElement, DiagramViewport, ElementKind};

use crate::layout::Positions;
use crate::model::{BundleModel, JobCluster, JobModel, PipelineModel};

pub const TASK_TYPE: &str = "databricks/job+task";
pub const CLUSTER_TYPE: &str = "databricks/job+cluster";
pub const EDGE_TYPE: &str = "databricks/job+edge";
pub const BUNDLE_TYPE: &str = "databricks/bundle+bundle";
pub const RESOURCE_TYPE: &str = "databricks/bundle+resource";
pub const TARGET_TYPE: &str = "databricks/bundle+target";
pub const OVERRIDE_EDGE_TYPE: &str = "databricks/bundle+edge";
pub const PIPELINE_NODE_TYPE: &str = "databricks/pipeline+node";
pub const FLOW_EDGE_TYPE: &str = "databricks/pipeline+edge";

/// The size the canvas draws a box at (`NODE_WIDTH`, `NODE_HEIGHT` in `DatabricksCanvas.tsx`).
pub const NODE_WIDTH: f64 = 200.0;
pub const NODE_HEIGHT: f64 = 56.0;
/// The size the canvas draws a target frame at (`FRAME_WIDTH`, `FRAME_HEIGHT`).
pub const FRAME_WIDTH: f64 = 220.0;
pub const FRAME_HEIGHT: f64 = 120.0;

/// Whether an element type is one of the family's edges.
pub fn is_edge(element_type: &str) -> bool {
    matches!(element_type, EDGE_TYPE | OVERRIDE_EDGE_TYPE | FLOW_EDGE_TYPE)
}

fn attributes(pairs: &[(&str, String)]) -> Vec<(String, String)> {
    pairs.iter().map(|(n, v)| ((*n).to_owned(), v.clone())).collect()
}

fn flag(value: bool) -> String {
    if value { "true" } else { "false" }.to_owned()
}

/// A box at its place, drawn as `kind`, named `label`, wearing `badges`.
fn node(
    id: &str,
    element_type: &str,
    positions: &Positions,
    placed: &HashSet<String>,
    mut pairs: Vec<(&str, String)>,
) -> DiagramElement {
    let (x, y) = positions.get(id).copied().unwrap_or_default();
    let mut element = DiagramElement::node(id, element_type);
    element.x = x;
    element.y = y;
    element.placed = placed.contains(id);
    let (width, height) = if element_type == TARGET_TYPE {
        (FRAME_WIDTH, FRAME_HEIGHT)
    } else {
        (NODE_WIDTH, NODE_HEIGHT)
    };
    pairs.push(("width", width.to_string()));
    pairs.push(("height", height.to_string()));
    element.attributes = attributes(&pairs);
    element
}

/// An edge from `from` to `to`, which carries no position of its own.
fn edge(id: &str, element_type: &str, from: &str, to: &str, outcome: &str) -> DiagramElement {
    let mut element = DiagramElement::node(id, element_type);
    element.kind = ElementKind::Relation;
    element.source = Some(from.to_owned());
    element.target = Some(to.to_owned());
    let kind = match element_type {
        EDGE_TYPE => "depends",
        OVERRIDE_EDGE_TYPE => "override",
        _ => "flow",
    };
    element.attributes = attributes(&[("outcome", outcome.to_owned()), ("kind", kind.to_owned())]);
    element
}

fn badges(list: &[String]) -> String {
    list.join("\n")
}

/// The job reading: the tasks, the stubs a dependency names and no task declares, the clusters and the edges.
pub fn job_elements(job: &JobModel, positions: &Positions, placed: &HashSet<String>) -> Vec<DiagramElement> {
    let mut elements = Vec::new();
    let declared: HashSet<&str> = job
        .tasks
        .iter()
        .filter(|t| !t.key.is_empty())
        .map(|t| t.key.as_str())
        .collect();
    for task in job.tasks.iter().filter(|t| !t.key.is_empty()) {
        let id = format!("task:{}", task.key);
        let mut worn = Vec::new();
        if !task.run_if.is_empty() {
            worn.push(task.run_if.clone());
        }
        worn.push(if task.cluster_key.is_empty() {
            "serverless".to_owned()
        } else {
            task.cluster_key.clone()
        });
        elements.push(node(
            &id,
            TASK_TYPE,
            positions,
            placed,
            vec![
                ("taskKey", task.key.clone()),
                ("taskType", task.task_type.clone()),
                ("source", task.source.clone()),
                ("clusterKey", task.cluster_key.clone()),
                ("runIf", task.run_if.clone()),
                ("unresolved", flag(false)),
                ("kind", task.task_type.clone()),
                ("label", task.key.clone()),
                ("badges", badges(&worn)),
            ],
        ));
        for dependency in &task.depends_on {
            elements.push(edge(
                &format!("edge:{}->{}", dependency.task_key, task.key),
                EDGE_TYPE,
                &format!("task:{}", dependency.task_key),
                &id,
                &dependency.outcome,
            ));
        }
    }
    let mut stubs: Vec<&str> = Vec::new();
    for dependency in job.tasks.iter().flat_map(|t| &t.depends_on) {
        if !declared.contains(dependency.task_key.as_str()) && !stubs.contains(&dependency.task_key.as_str()) {
            stubs.push(&dependency.task_key);
        }
    }
    for stub in stubs {
        elements.push(node(
            &format!("task:{stub}"),
            TASK_TYPE,
            positions,
            placed,
            vec![
                ("taskKey", stub.to_owned()),
                ("taskType", "other".to_owned()),
                ("source", String::new()),
                ("clusterKey", String::new()),
                ("runIf", String::new()),
                ("unresolved", flag(true)),
                ("kind", "other".to_owned()),
                ("label", stub.to_owned()),
                ("badges", "serverless".to_owned()),
            ],
        ));
    }
    for cluster in &job.clusters {
        elements.push(node(
            &format!("cluster:{}", cluster.key),
            CLUSTER_TYPE,
            positions,
            placed,
            vec![
                ("role", "compute".to_owned()),
                ("kind", "compute".to_owned()),
                ("label", cluster.key.clone()),
                ("badges", badges(&cluster_badges(cluster))),
            ],
        ));
    }
    elements
}

fn cluster_badges(cluster: &JobCluster) -> Vec<String> {
    let workers = if cluster.workers > 0 {
        format!("{} workers", cluster.workers)
    } else {
        String::new()
    };
    [cluster.spark_version.clone(), cluster.node_type.clone(), workers]
        .into_iter()
        .filter(|b| !b.is_empty())
        .collect()
}

/// The bundle reading: the bundle, its resources and targets, and the override edges.
pub fn bundle_elements(bundle: &BundleModel, positions: &Positions, placed: &HashSet<String>) -> Vec<DiagramElement> {
    let resource = |id: &str, element_type: &str, kind: &str, key: &str| {
        let kind_shown = if kind.is_empty() { "resource" } else { kind };
        let label = if key.is_empty() { kind } else { key };
        let worn = if element_type == BUNDLE_TYPE {
            String::new()
        } else {
            kind.to_owned()
        };
        node(
            id,
            element_type,
            positions,
            placed,
            vec![
                ("resourceKind", kind.to_owned()),
                ("key", key.to_owned()),
                ("kind", kind_shown.to_owned()),
                ("label", label.to_owned()),
                ("badges", worn),
            ],
        )
    };
    let mut elements = vec![resource("bundle", BUNDLE_TYPE, "bundle", &bundle.name)];
    for r in &bundle.resources {
        elements.push(resource(
            &format!("resource:{}/{}", r.kind, r.key),
            RESOURCE_TYPE,
            &r.kind,
            &r.key,
        ));
    }
    for unknown in &bundle.unknown_nodes {
        elements.push(resource(
            &format!("unknown:{}", unknown.path),
            RESOURCE_TYPE,
            &unknown.path,
            &unknown.key,
        ));
    }
    for target in &bundle.targets {
        let id = format!("target:{}", target.name);
        elements.push(node(
            &id,
            TARGET_TYPE,
            positions,
            placed,
            vec![
                ("mode", target.mode.clone()),
                ("isDefault", flag(target.is_default)),
                ("overrideCount", target.overrides.len().to_string()),
                ("label", target.name.clone()),
            ],
        ));
        for overridden in &target.overrides {
            elements.push(edge(
                &format!("override:{}/{}/{}", target.name, overridden.kind, overridden.key),
                OVERRIDE_EDGE_TYPE,
                &id,
                &format!("resource:{}/{}", overridden.kind, overridden.key),
                "",
            ));
        }
    }
    elements
}

fn pipeline_node(
    id: &str,
    positions: &Positions,
    placed: &HashSet<String>,
    role: &str,
    label: &str,
    worn: &[String],
) -> DiagramElement {
    node(
        id,
        PIPELINE_NODE_TYPE,
        positions,
        placed,
        vec![
            ("role", role.to_owned()),
            ("kind", role.to_owned()),
            ("label", label.to_owned()),
            ("badges", badges(worn)),
        ],
    )
}

/// The pipeline reading: the sources into the pipeline into its target, the satellites beneath.
pub fn pipeline_elements(
    pipeline: &PipelineModel,
    positions: &Positions,
    placed: &HashSet<String>,
) -> Vec<DiagramElement> {
    let mut elements = Vec::new();
    for library in &pipeline.libraries {
        let id = format!("library:{}", library.path);
        elements.push(pipeline_node(
            &id,
            positions,
            placed,
            "source",
            &library.path,
            std::slice::from_ref(&library.kind),
        ));
        elements.push(edge(
            &format!("flow:{}->pipeline", library.path),
            FLOW_EDGE_TYPE,
            &id,
            "pipeline",
            "",
        ));
    }
    let mut worn = Vec::new();
    if pipeline.serverless {
        worn.push("serverless".to_owned());
    }
    if pipeline.continuous {
        worn.push("continuous".to_owned());
    }
    if pipeline.development {
        worn.push("development".to_owned());
    }
    if !pipeline.channel.is_empty() {
        worn.push(pipeline.channel.clone());
    }
    elements.push(pipeline_node(
        "pipeline",
        positions,
        placed,
        "pipeline",
        &pipeline.name,
        &worn,
    ));
    let target = match (pipeline.catalog.is_empty(), pipeline.schema.is_empty()) {
        (false, false) => format!("{}.{}", pipeline.catalog, pipeline.schema),
        (false, true) => pipeline.catalog.clone(),
        _ => pipeline.schema.clone(),
    };
    elements.push(pipeline_node("target", positions, placed, "target", &target, &[]));
    elements.push(edge("flow:pipeline->target", FLOW_EDGE_TYPE, "pipeline", "target", ""));
    elements.push(pipeline_node(
        "compute",
        positions,
        placed,
        "compute",
        if pipeline.serverless { "serverless" } else { "cluster" },
        &[],
    ));
    if !pipeline.notifications.is_empty() {
        let recipients: Vec<&str> = pipeline
            .notifications
            .iter()
            .flat_map(|n| n.recipients.iter().map(String::as_str))
            .collect();
        elements.push(pipeline_node(
            "notifications",
            positions,
            placed,
            "notifications",
            &recipients.join(", "),
            &[],
        ));
    }
    elements
}

/// Whether a box overlaps the viewport, at the size the canvas draws it.
fn intersects(element: &DiagramElement, viewport: DiagramViewport) -> bool {
    let (width, height) = if element.element_type == TARGET_TYPE {
        (FRAME_WIDTH, FRAME_HEIGHT)
    } else {
        (NODE_WIDTH, NODE_HEIGHT)
    };
    viewport.intersects(Rect::new(element.x, element.y, width, height))
}

/// The elements a viewport can see (`DatabricksElementMapper.Visible`): every box that intersects it, and one hop
/// exactly, the far end of an edge with one end already in view, so an edge is never drawn to a box that was culled.
pub fn visible(elements: Vec<DiagramElement>, viewport: DiagramViewport) -> Vec<DiagramElement> {
    let in_view: HashSet<String> = elements
        .iter()
        .filter(|e| !is_edge(&e.element_type) && intersects(e, viewport))
        .map(|e| e.id.clone())
        .collect();
    let mut reached = in_view.clone();
    for element in elements.iter().filter(|e| is_edge(&e.element_type)) {
        let (from, to) = ends_of(element);
        if in_view.contains(from) {
            reached.insert(to.to_owned());
        } else if in_view.contains(to) {
            reached.insert(from.to_owned());
        }
    }
    elements
        .into_iter()
        .filter(|e| {
            if is_edge(&e.element_type) {
                let (from, to) = ends_of(e);
                reached.contains(from) && reached.contains(to)
            } else {
                reached.contains(&e.id)
            }
        })
        .collect()
}

fn ends_of(edge: &DiagramElement) -> (&str, &str) {
    (
        edge.source.as_deref().unwrap_or_default(),
        edge.target.as_deref().unwrap_or_default(),
    )
}
