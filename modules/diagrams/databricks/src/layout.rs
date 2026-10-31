//! Where each of the three readings puts its boxes, before the registration's `layout:` block overlays the positions a
//! person dragged them to (etalii.adp spec 030, task T081).
//!
//! The algorithms are `layout.layeredColumns` and `layout.bands` in the capability registry (etalii.adp spec 067, T027);
//! this keeps what is the module's: which nodes each reading has and its constants. Ported from the standalone host's
//! `src/diagrams/databricks/backend/EtAlii.Adp.Diagram.Databricks/DatabricksBundleLayout.cs`, `DatabricksJobLayout.cs` and
//! `DatabricksPipelineLayout.cs`, and the core's `RegistrationLayout.Apply`, at `8b6ccfc`. Every position is a box's
//! top-left corner, in the module's own units, by the ids the mapper speaks.

use std::collections::{BTreeMap, HashMap, HashSet};

use adp_atd_runtime::capabilities::bands::{Band, place_bands};
use adp_atd_runtime::capabilities::layered_columns::{ColumnArguments, ColumnNode, place_nodes as place_columns};

use crate::model::{BundleModel, JobModel, PipelineModel};

/// Positions by element id.
pub type Positions = HashMap<String, (f64, f64)>;

const COLUMN_WIDTH: f64 = 260.0;
const BUNDLE_BAND_Y: f64 = 0.0;
const RESOURCE_BAND_Y: f64 = 180.0;
const TARGET_BAND_Y: f64 = 400.0;
const ROW_HEIGHT: f64 = 120.0;
const CLUSTER_BAND_GAP: f64 = 80.0;
const PIPELINE_COLUMN_WIDTH: f64 = 320.0;
const SATELLITE_BAND_GAP: f64 = 80.0;

/// The bundle reading's three bands: the bundle, the resources it declares (unknown kinds included), and the target
/// frames beneath. `layout.bands`, with the reading's own constants.
pub fn bundle(model: &BundleModel) -> Positions {
    let resources = model
        .resources
        .iter()
        .map(|resource| format!("resource:{}/{}", resource.kind, resource.key));
    let unknown = model
        .unknown_nodes
        .iter()
        .map(|unknown| format!("unknown:{}", unknown.path));
    let bands = [
        Band {
            y: BUNDLE_BAND_Y,
            members: vec!["bundle".to_owned()],
        },
        Band {
            y: RESOURCE_BAND_Y,
            members: resources.chain(unknown).collect(),
        },
        Band {
            y: TARGET_BAND_Y,
            members: model
                .targets
                .iter()
                .map(|target| format!("target:{}", target.name))
                .collect(),
        },
    ];
    positions_of(place_bands(&bands, COLUMN_WIDTH))
}

/// The job reading: longest-path layering left to right over the `depends_on` graph, an edge closing a cycle left out
/// of the layering; a stub a dependency names and no task declares one column left of the first task naming it; the
/// clusters in a band beneath. `layout.layeredColumns`, with the reading's own constants.
pub fn job(model: &JobModel) -> Positions {
    let tasks = || model.tasks.iter().filter(|t| !t.key.is_empty());
    let declared: HashSet<&str> = tasks().map(|t| t.key.as_str()).collect();
    // The first task of each key carries the dependencies the layering reads, as a dictionary keeps its first entry.
    let mut seen: HashSet<&str> = HashSet::new();
    let mut edges: Vec<(String, String)> = tasks()
        .filter(|t| seen.insert(&t.key))
        .flat_map(|t| {
            t.depends_on
                .iter()
                .filter(|d| declared.contains(d.task_key.as_str()))
                .map(|d| (format!("task:{}", d.task_key), format!("task:{}", t.key)))
        })
        .collect();

    let mut nodes: Vec<ColumnNode> = tasks()
        .map(|t| ColumnNode::layered(format!("task:{}", t.key)))
        .collect();
    let mut found: HashSet<&str> = HashSet::new();
    for task in tasks() {
        for dependency in &task.depends_on {
            if !declared.contains(dependency.task_key.as_str()) && found.insert(&dependency.task_key) {
                let stub = format!("task:{}", dependency.task_key);
                edges.push((stub.clone(), format!("task:{}", task.key)));
                nodes.push(ColumnNode {
                    placeholder: true,
                    ..ColumnNode::layered(stub)
                });
            }
        }
    }
    nodes.extend(model.clusters.iter().map(|cluster| ColumnNode {
        band: true,
        ..ColumnNode::layered(format!("cluster:{}", cluster.key))
    }));
    let arguments = ColumnArguments {
        column_width: COLUMN_WIDTH,
        row_height: ROW_HEIGHT,
        band_gap: CLUSTER_BAND_GAP,
        middle: false,
    };
    positions_of(place_columns(&nodes, &edges, &arguments))
}

/// The pipeline reading: the source libraries in a column flowing into the pipeline at their vertical middle and on to
/// its target, the satellites beneath. `layout.layeredColumns`, with the reading's own constants.
pub fn pipeline(model: &PipelineModel) -> Positions {
    let in_column = |id: String, column: i32| ColumnNode {
        column: Some(column),
        ..ColumnNode::layered(id)
    };
    let in_band = |id: &str| ColumnNode {
        band: true,
        ..ColumnNode::layered(id.to_owned())
    };
    let mut nodes: Vec<ColumnNode> = model
        .libraries
        .iter()
        .map(|library| in_column(format!("library:{}", library.path), 0))
        .collect();
    nodes.push(in_column("pipeline".to_owned(), 1));
    nodes.push(in_column("target".to_owned(), 2));
    nodes.push(in_band("compute"));
    if !model.notifications.is_empty() {
        nodes.push(in_band("notifications"));
    }
    let arguments = ColumnArguments {
        column_width: PIPELINE_COLUMN_WIDTH,
        row_height: ROW_HEIGHT,
        band_gap: SATELLITE_BAND_GAP,
        middle: true,
    };
    positions_of(place_columns(&nodes, &[], &arguments))
}

/// Corners by id, a repeated id at its later place.
fn positions_of(placed: Vec<(String, f64, f64)>) -> Positions {
    placed.into_iter().map(|(id, x, y)| (id, (x, y))).collect()
}

/// Stored positions overlaid on computed ones, element by element: a stored id with nothing computed for it has
/// nothing to override (`RegistrationLayout.Apply`).
pub fn apply(computed: &mut Positions, stored: &BTreeMap<String, (f64, f64)>) -> HashSet<String> {
    let mut placed = HashSet::new();
    for (id, position) in stored {
        if let Some(slot) = computed.get_mut(id) {
            *slot = *position;
            placed.insert(id.clone());
        }
    }
    placed
}
