//! Where each of the three readings puts its boxes, before the registration's `layout:` block overlays the positions a
//! person dragged them to (etalii.adp spec 030, task T081).
//!
//! Ported from the standalone host's `src/diagrams/databricks/backend/EtAlii.Adp.Diagram.Databricks/
//! DatabricksBundleLayout.cs`, `DatabricksJobLayout.cs` and `DatabricksPipelineLayout.cs`, and the core's
//! `RegistrationLayout.Apply`, at `8b6ccfc`. Every position is a box's top-left corner, in the module's own units, by
//! the ids the mapper speaks.

use std::collections::{BTreeMap, HashMap, HashSet};

use adp_layout::CyclePolicy;
use adp_layout::layering::longest_path;

use crate::model::{BundleModel, JobModel, PipelineModel};

/// Positions by element id.
pub type Positions = HashMap<String, (f64, f64)>;

const COLUMN_WIDTH: f64 = 260.0;
const BUNDLE_BAND_Y: f64 = 0.0;
const RESOURCE_BAND_Y: f64 = 180.0;
const TARGET_BAND_Y: f64 = 400.0;
const ROW_HEIGHT: f64 = 120.0;
const CLUSTER_BAND_GAP: f64 = 80.0;
const SOURCE_COLUMN_X: f64 = 0.0;
const PIPELINE_COLUMN_X: f64 = 320.0;
const TARGET_COLUMN_X: f64 = 640.0;
const SATELLITE_BAND_GAP: f64 = 80.0;

/// The bundle reading's three bands: the bundle, the resources it declares (unknown kinds included), and the target
/// frames beneath.
pub fn bundle(model: &BundleModel) -> Positions {
    let mut positions = Positions::new();
    positions.insert("bundle".to_owned(), (0.0, BUNDLE_BAND_Y));
    let mut column = 0.0;
    for resource in &model.resources {
        positions.insert(
            format!("resource:{}/{}", resource.kind, resource.key),
            (column * COLUMN_WIDTH, RESOURCE_BAND_Y),
        );
        column += 1.0;
    }
    for unknown in &model.unknown_nodes {
        positions.insert(
            format!("unknown:{}", unknown.path),
            (column * COLUMN_WIDTH, RESOURCE_BAND_Y),
        );
        column += 1.0;
    }
    for (index, target) in model.targets.iter().enumerate() {
        positions.insert(
            format!("target:{}", target.name),
            (index as f64 * COLUMN_WIDTH, TARGET_BAND_Y),
        );
    }
    positions
}

/// The job reading: longest-path layering left to right over the `depends_on` graph, an edge closing a cycle left out
/// of the layering; a stub a dependency names and no task declares one column left of the first task naming it; the
/// clusters in a band beneath.
pub fn job(model: &JobModel) -> Positions {
    let mut keys: Vec<&str> = Vec::new();
    let mut declared: HashSet<&str> = HashSet::new();
    for task in model.tasks.iter().filter(|t| !t.key.is_empty()) {
        if declared.insert(&task.key) {
            keys.push(&task.key);
        }
    }
    // The first task of each key carries the dependencies the layering reads, as a dictionary keeps its first entry.
    let mut seen: HashSet<&str> = HashSet::new();
    let edges: Vec<(&str, &str)> = model
        .tasks
        .iter()
        .filter(|t| !t.key.is_empty() && seen.insert(&t.key))
        .flat_map(|t| t.depends_on.iter().map(move |d| (d.task_key.as_str(), t.key.as_str())))
        .collect();
    let layers = longest_path(&keys, edges, CyclePolicy::BackEdgeIgnored);
    let layer_of = |key: &str| layers.get(key).copied().unwrap_or(0);

    let mut stubs: Vec<(&str, i32)> = Vec::new();
    let mut found: HashSet<&str> = HashSet::new();
    for task in model.tasks.iter().filter(|t| !t.key.is_empty()) {
        for dependency in &task.depends_on {
            if !declared.contains(dependency.task_key.as_str()) && found.insert(&dependency.task_key) {
                stubs.push((&dependency.task_key, layer_of(&task.key)));
            }
        }
    }

    let mut positions = Positions::new();
    let mut rows: BTreeMap<i32, i32> = BTreeMap::new();
    let mut deepest = 0;
    let mut place = |positions: &mut Positions, id: String, layer: i32| {
        let row = rows.get(&layer).copied().unwrap_or(0);
        rows.insert(layer, row + 1);
        deepest = deepest.max(row);
        positions.insert(id, (f64::from(layer) * COLUMN_WIDTH, f64::from(row) * ROW_HEIGHT));
    };
    for task in model.tasks.iter().filter(|t| !t.key.is_empty()) {
        place(&mut positions, format!("task:{}", task.key), layer_of(&task.key));
    }
    for (stub, dependent_layer) in stubs {
        place(&mut positions, format!("task:{stub}"), (dependent_layer - 1).max(0));
    }
    let cluster_y = f64::from(deepest + 1) * ROW_HEIGHT + CLUSTER_BAND_GAP;
    for (index, cluster) in model.clusters.iter().enumerate() {
        positions.insert(
            format!("cluster:{}", cluster.key),
            (index as f64 * COLUMN_WIDTH, cluster_y),
        );
    }
    positions
}

/// The pipeline reading: the source libraries in a column flowing into the pipeline at their vertical middle and on to
/// its target, the satellites beneath.
pub fn pipeline(model: &PipelineModel) -> Positions {
    let mut positions = Positions::new();
    for (index, library) in model.libraries.iter().enumerate() {
        positions.insert(
            format!("library:{}", library.path),
            (SOURCE_COLUMN_X, index as f64 * ROW_HEIGHT),
        );
    }
    let count = model.libraries.len();
    let middle = count.saturating_sub(1) as f64 * ROW_HEIGHT / 2.0;
    positions.insert("pipeline".to_owned(), (PIPELINE_COLUMN_X, middle));
    positions.insert("target".to_owned(), (TARGET_COLUMN_X, middle));
    let satellite_y = count.max(1) as f64 * ROW_HEIGHT + SATELLITE_BAND_GAP;
    positions.insert("compute".to_owned(), (SOURCE_COLUMN_X, satellite_y));
    if !model.notifications.is_empty() {
        positions.insert("notifications".to_owned(), (PIPELINE_COLUMN_X, satellite_y));
    }
    positions
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
