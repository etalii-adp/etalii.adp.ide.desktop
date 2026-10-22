//! Where a pipeline's stages and jobs are drawn, from what waits for what (etalii.adp spec 030, task T078).
//!
//! Ported from the standalone host's `src/diagrams/azure-devops-pipeline/backend/EtAlii.Adp.Diagram.AzureDevOpsPipeline/
//! PipelineLayout.cs` and `_Model/PipelineMetrics.cs`, `PipelinePlacement.cs` and `PipelineArrangement.cs` at `8b6ccfc`.
//! An Azure pipeline has no coordinates, so nothing is read from the file and nothing is written to it: each element's
//! column is the length of the longest chain of what it waits for (`Layering.LongestPath`, a back edge at zero), each
//! column filled top-down in declared order, and an open stage sized to hold its jobs laid out the same way.

use std::collections::{BTreeMap, HashMap, HashSet};

use adp_layout::CyclePolicy;
use adp_layout::layering::longest_path;

use crate::graph::{self, PipelineGraph};
use crate::model::PipelineModel;

/// The fixed pitch the layout arranges at (`PipelineMetrics`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PipelineMetrics {
    pub stage_width: f64,
    pub stage_height: f64,
    pub job_width: f64,
    pub job_height: f64,
    pub horizontal_gap: f64,
    pub vertical_gap: f64,
    pub padding: f64,
    pub header_height: f64,
}

impl Default for PipelineMetrics {
    fn default() -> Self {
        PipelineMetrics {
            stage_width: 220.0,
            stage_height: 88.0,
            job_width: 180.0,
            job_height: 56.0,
            horizontal_gap: 80.0,
            vertical_gap: 32.0,
            padding: 24.0,
            header_height: 40.0,
        }
    }
}

impl PipelineMetrics {
    pub fn stage(&self) -> Size {
        Size::new(self.stage_width, self.stage_height)
    }

    pub fn job(&self) -> Size {
        Size::new(self.job_width, self.job_height)
    }
}

/// How big an element is drawn (`PipelineSize`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

impl Size {
    pub const fn new(width: f64, height: f64) -> Self {
        Size { width, height }
    }
}

/// Where one element went: its top-left, its size, its column and its row in that column (`PipelinePlacement`).
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub size: Size,
    pub layer: i32,
    pub row: i32,
}

/// A laid-out level: every placement, in declared order, and the room it took (`PipelineArrangement`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Arrangement {
    pub placements: Vec<Placement>,
    pub size: Size,
}

impl Arrangement {
    pub fn of(&self, id: &str) -> Option<&Placement> {
        self.placements.iter().find(|p| p.id == id)
    }
}

/// Lays out one level: each element in the column of its longest chain of dependencies, sized from `sizes` or
/// `fallback`.
pub fn arrange(
    graph: &PipelineGraph,
    metrics: &PipelineMetrics,
    sizes: &HashMap<String, Size>,
    fallback: Size,
) -> Arrangement {
    if graph.node_ids.is_empty() {
        return Arrangement::default();
    }
    let size_of = |id: &str| sizes.get(id).copied().unwrap_or(fallback);
    let layers = longest_path(
        &graph.node_ids,
        graph
            .edges
            .iter()
            .filter(|e| !e.is_broken)
            .map(|e| (e.from_id.as_str(), e.to_id.as_str())),
        CyclePolicy::BackEdgeIsZero,
    );
    let mut layer_width: BTreeMap<i32, f64> = BTreeMap::new();
    for id in &graph.node_ids {
        let width = layer_width.entry(layers[id]).or_insert(0.0);
        *width = width.max(size_of(id).width);
    }
    let mut layer_x: BTreeMap<i32, f64> = BTreeMap::new();
    let mut x = 0.0;
    for (layer, width) in &layer_width {
        layer_x.insert(*layer, x);
        x += width + metrics.horizontal_gap;
    }
    let mut next_y: HashMap<i32, f64> = HashMap::new();
    let mut next_row: HashMap<i32, i32> = HashMap::new();
    let mut placements = Vec::with_capacity(graph.node_ids.len());
    for id in &graph.node_ids {
        let layer = layers[id];
        let size = size_of(id);
        let y = next_y.get(&layer).copied().unwrap_or(0.0);
        let row = next_row.get(&layer).copied().unwrap_or(0);
        placements.push(Placement {
            id: id.clone(),
            x: layer_x[&layer],
            y,
            size,
            layer,
            row,
        });
        next_y.insert(layer, y + size.height + metrics.vertical_gap);
        next_row.insert(layer, row + 1);
    }
    // The column furthest right, the first of equals (`MaxBy` over the columns' x).
    let mut last_layer = *layer_x.keys().next().expect("a level with an element has a column");
    for (layer, x) in &layer_x {
        if *x > layer_x[&last_layer] {
            last_layer = *layer;
        }
    }
    let height = placements
        .iter()
        .map(|p| p.y + p.size.height)
        .fold(f64::NEG_INFINITY, f64::max);
    Arrangement {
        placements,
        size: Size::new(layer_x[&last_layer] + layer_width[&last_layer], height),
    }
}

/// The whole pipeline: its stages, and inside each open one its jobs, an open job as tall as its steps.
pub fn arrange_pipeline(model: &PipelineModel, metrics: &PipelineMetrics, expanded: &HashSet<String>) -> Arrangement {
    let mut within: HashMap<String, Arrangement> = HashMap::new();
    let mut sizes: HashMap<String, Size> = HashMap::new();
    for stage in model.stages.iter().filter(|s| expanded.contains(&s.id)) {
        let job_sizes: HashMap<String, Size> = stage
            .jobs
            .iter()
            .filter(|j| expanded.contains(&j.id) && !j.steps.is_empty())
            .map(|j| {
                let count = j.steps.len() as f64;
                (
                    j.id.clone(),
                    Size::new(
                        metrics.job_width.max(metrics.job_width + 2.0 * metrics.padding),
                        metrics.header_height
                            + 2.0 * metrics.padding
                            + count * metrics.job_height
                            + (count - 1.0) * metrics.vertical_gap,
                    ),
                )
            })
            .collect();
        let jobs = arrange(&graph::of_jobs(stage), metrics, &job_sizes, metrics.job());
        sizes.insert(
            stage.id.clone(),
            Size::new(
                metrics.stage_width.max(jobs.size.width + 2.0 * metrics.padding),
                metrics
                    .stage_height
                    .max(jobs.size.height + metrics.header_height + 2.0 * metrics.padding),
            ),
        );
        within.insert(stage.id.clone(), jobs);
    }
    let stages = arrange(&graph::of_stages(model), metrics, &sizes, metrics.stage());
    let mut placements = stages.placements.clone();
    for stage in &stages.placements {
        if let Some(inside) = within.get(&stage.id) {
            placements.extend(inside.placements.iter().map(|job| Placement {
                x: stage.x + metrics.padding + job.x,
                y: stage.y + metrics.header_height + metrics.padding + job.y,
                ..job.clone()
            }));
        }
    }
    Arrangement {
        placements,
        size: stages.size,
    }
}
