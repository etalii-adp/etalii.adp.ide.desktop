//! Where each reading puts its boxes (etalii.adp spec 030, task T081), as the standalone host's
//! `DatabricksLayout.Tests.cs` checks it at `8b6ccfc`.

mod common;

use adp_diagram_databricks::layout;
use common::{document, model};

#[test]
fn the_job_graph_layers_by_longest_path_left_to_right() {
    let job = model(&document("job.yml")).jobs.remove(0);
    let positions = layout::job(&job);
    let at = |id: &str| positions[id];
    assert_eq!(at("task:ingest"), (0.0, 0.0));
    assert_eq!(at("task:quality_gate"), (260.0, 0.0));
    assert_eq!(at("task:publish"), (520.0, 0.0));
    assert_eq!(at("task:alert"), (520.0, 120.0));
    assert_eq!(at("task:refresh_dashboard"), (780.0, 0.0));
    // The cluster band sits beneath the deepest row, a gap below it.
    assert_eq!(at("cluster:ingest_cluster"), (0.0, 2.0 * 120.0 + 80.0));
}

#[test]
fn a_cycle_neither_hangs_nor_drops_its_members_and_a_missing_task_gets_a_stub() {
    let job = model(&document("findings-job.yml")).jobs.remove(0);
    let positions = layout::job(&job);
    assert_eq!(positions["task:loop_b"].0, 0.0);
    assert!(positions["task:loop_a"].0 > positions["task:loop_b"].0);
    assert!(positions.contains_key("task:nowhere"));
}

#[test]
fn the_bundle_lands_in_three_bands() {
    let bundle = model(&document("bundle.yml")).bundle;
    let positions = layout::bundle(&bundle);
    assert_eq!(positions["bundle"], (0.0, 0.0));
    assert_eq!(positions["resource:jobs/nightly_ingest"], (0.0, 180.0));
    assert_eq!(positions["resource:pipelines/bronze_to_gold"], (260.0, 180.0));
    assert_eq!(positions["unknown:sync"], (520.0, 180.0));
    assert_eq!(positions["unknown:resources.experiments"], (780.0, 180.0));
    assert_eq!(positions["target:dev"], (0.0, 400.0));
    assert_eq!(positions["target:prod"], (260.0, 400.0));
}

#[test]
fn the_pipeline_flows_sources_into_the_pipeline_into_its_target() {
    let pipeline = model(&document("pipeline.json")).pipelines.remove(0);
    let positions = layout::pipeline(&pipeline);
    assert_eq!(positions["library:transformations/bronze"], (0.0, 0.0));
    assert_eq!(positions["library:transformations/gold/**"], (0.0, 240.0));
    assert_eq!(positions["pipeline"], (320.0, 120.0));
    assert_eq!(positions["target"], (640.0, 120.0));
    assert_eq!(positions["compute"], (0.0, 440.0));
    assert_eq!(positions["notifications"], (320.0, 440.0));
}

#[test]
fn a_stored_position_overrides_only_an_element_that_is_there() {
    let pipeline = model(&document("pipeline.json")).pipelines.remove(0);
    let mut positions = layout::pipeline(&pipeline);
    let stored = [("pipeline".to_owned(), (1.5, -2.0)), ("gone".to_owned(), (9.0, 9.0))]
        .into_iter()
        .collect();
    let placed = layout::apply(&mut positions, &stored);
    assert_eq!(positions["pipeline"], (1.5, -2.0));
    assert!(!positions.contains_key("gone"));
    assert_eq!(placed.len(), 1);
}
