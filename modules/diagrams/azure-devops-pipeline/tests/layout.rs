//! Where stages and jobs are drawn: one column per depth of what they wait for, rows in declared order, open stages
//! holding their jobs, and the same picture for the same file (the standalone host's `PipelineLayoutTests`).

mod common;

use std::collections::{HashMap, HashSet};

use adp_diagram_azure_devops_pipeline::graph::{self, PipelineGraph};
use adp_diagram_azure_devops_pipeline::layout::{self, Arrangement, PipelineMetrics};
use adp_diagram_azure_devops_pipeline::model::{EditTarget, PipelineModel};
use adp_diagram_azure_devops_pipeline::parser;
use adp_diagram_azure_devops_pipeline::writer::PipelineWriter;
use common::{all_fixtures, document, fixture, parse, parse_text};

const METRICS: PipelineMetrics = PipelineMetrics {
    stage_width: 100.0,
    stage_height: 50.0,
    job_width: 60.0,
    job_height: 20.0,
    horizontal_gap: 10.0,
    vertical_gap: 5.0,
    padding: 10.0,
    header_height: 20.0,
};

fn arrange_stages(model: &PipelineModel) -> Arrangement {
    layout::arrange(&graph::of_stages(model), &METRICS, &HashMap::new(), METRICS.stage())
}

fn expanded(ids: &[&str]) -> HashSet<String> {
    ids.iter().map(|id| (*id).to_owned()).collect()
}

/// Stages named as `A`, `B:A` (waits for A), `C:A, B` or `D:` (waits for nothing).
fn pipeline(stages: &[&str]) -> PipelineModel {
    let mut text = "stages:\n".to_owned();
    for stage in stages {
        let (name, depends) = match stage.split_once(':') {
            Some((name, depends)) => (name, Some(depends)),
            None => (*stage, None),
        };
        text.push_str(&format!("  - stage: {name}\n"));
        match depends {
            Some(d) if !d.is_empty() => text.push_str(&format!("    dependsOn: [{d}]\n")),
            Some(_) => text.push_str("    dependsOn: []\n"),
            None => {}
        }
        text.push_str("    jobs:\n      - job: J\n        steps:\n          - script: x\n");
    }
    parse_text(&text)
}

fn positions(arrangement: &Arrangement) -> Vec<(String, f64, f64)> {
    arrangement
        .placements
        .iter()
        .map(|p| (p.id.clone(), p.x, p.y))
        .collect()
}

#[test]
fn a_sequential_pipeline_runs_left_to_right_one_stage_per_column() {
    let arrangement = arrange_stages(&pipeline(&["A", "B", "C"]));
    let layers: Vec<i32> = arrangement.placements.iter().map(|p| p.layer).collect();
    let rows: Vec<i32> = arrangement.placements.iter().map(|p| p.row).collect();
    assert_eq!(vec![0, 1, 2], layers);
    assert_eq!(vec![0, 0, 0], rows);
}

#[test]
fn stages_that_can_run_at_the_same_time_share_a_column() {
    let arrangement = arrange_stages(&pipeline(&["A:", "B:A", "C:A"]));
    assert_eq!(0, arrangement.of("A").unwrap().layer);
    assert_eq!(1, arrangement.of("B").unwrap().layer);
    assert_eq!(1, arrangement.of("C").unwrap().layer);
    assert_eq!(0, arrangement.of("B").unwrap().row);
    assert_eq!(1, arrangement.of("C").unwrap().row);
}

#[test]
fn a_layer_is_the_longest_depth_not_the_shortest() {
    let arrangement = arrange_stages(&pipeline(&["A:", "B:A", "C:B", "D:A, C"]));
    assert_eq!(3, arrangement.of("D").unwrap().layer);
}

#[test]
fn every_arrow_points_forward() {
    let mut shapes: Vec<String> = all_fixtures().iter().map(|name| fixture(name)).collect();
    // The diamond: the one shape on which longest and shortest depth disagree.
    let diamond = pipeline(&["A:", "B:A", "C:B", "D:A, C"]);
    let mut models: Vec<PipelineModel> = shapes.drain(..).map(|text| parse_text(&text)).collect();
    models.push(diamond);
    for model in models {
        let graph = graph::of_stages(&model);
        if !graph.cycles.is_empty() {
            continue;
        }
        let arrangement = layout::arrange(&graph, &METRICS, &HashMap::new(), METRICS.stage());
        for edge in graph.edges.iter().filter(|e| !e.is_broken) {
            assert!(
                arrangement.of(&edge.from_id).unwrap().layer < arrangement.of(&edge.to_id).unwrap().layer,
                "{} -> {} does not point forward",
                edge.from_id,
                edge.to_id
            );
        }
    }
}

#[test]
fn the_same_file_always_produces_the_same_picture() {
    let first = arrange_stages(&parse("multi-stage.yml"));
    let second = arrange_stages(&parse("multi-stage.yml"));
    assert_eq!(positions(&first), positions(&second));
}

#[test]
fn renaming_a_step_moves_nothing() {
    let mut document = document("multi-stage.yml");
    let before = arrange_stages(&parser::parse(&document).unwrap());
    let step = parser::parse(&document)
        .unwrap()
        .jobs()
        .find(|j| j.name == "Compile")
        .unwrap()
        .steps[1]
        .clone();
    PipelineWriter::new(&mut document)
        .set_display_name(&EditTarget::of_step(&step), "Renamed entirely")
        .unwrap();
    let after = arrange_stages(&parser::parse(&document).unwrap());
    assert_eq!(positions(&before), positions(&after));
}

#[test]
fn columns_are_spaced_by_their_width_and_the_gap() {
    let arrangement = arrange_stages(&pipeline(&["A", "B", "C"]));
    assert_eq!(0.0, arrangement.of("A").unwrap().x);
    assert_eq!(110.0, arrangement.of("B").unwrap().x);
    assert_eq!(220.0, arrangement.of("C").unwrap().x);
}

#[test]
fn rows_are_spaced_by_their_height_and_the_gap() {
    let arrangement = arrange_stages(&pipeline(&["A:", "B:A", "C:A"]));
    assert_eq!(0.0, arrangement.of("B").unwrap().y);
    assert_eq!(55.0, arrangement.of("C").unwrap().y);
}

#[test]
fn a_cycle_is_laid_out_rather_than_hanging_the_layout() {
    let model = parse("edge-broken-graph.yml");
    let arrangement = arrange_stages(&model);
    assert_eq!(model.stages.len(), arrangement.placements.len());
    assert!(
        arrangement
            .placements
            .iter()
            .all(|p| p.layer >= 0 && p.layer as usize <= model.stages.len())
    );
}

#[test]
fn a_broken_edge_does_not_drag_its_stage_into_a_column_of_its_own() {
    let arrangement = arrange_stages(&parse("edge-broken-graph.yml"));
    assert_eq!(0, arrangement.of("Ghost").unwrap().layer);
}

#[test]
fn an_empty_pipeline_arranges_to_nothing() {
    let arrangement = layout::arrange(&PipelineGraph::default(), &METRICS, &HashMap::new(), METRICS.stage());
    assert_eq!(Arrangement::default(), arrangement);
}

#[test]
fn an_expanded_stage_holds_its_jobs_inside() {
    let arrangement = layout::arrange_pipeline(&parse("multi-stage.yml"), &METRICS, &expanded(&["Test"]));
    let stage = arrangement.of("Test").unwrap();
    for job in ["Test/Unit", "Test/Integration"] {
        let placed = arrangement.of(job).unwrap();
        assert!(placed.x >= stage.x && placed.x <= stage.x + stage.size.width - placed.size.width);
        assert!(placed.y >= stage.y && placed.y <= stage.y + stage.size.height - placed.size.height);
    }
}

#[test]
fn an_expanded_stage_grows_to_fit_its_jobs() {
    let arrangement = layout::arrange_pipeline(&parse("multi-stage.yml"), &METRICS, &expanded(&["Test"]));
    assert!(arrangement.of("Test").unwrap().size.height > METRICS.stage_height);
    assert_eq!(METRICS.stage_height, arrangement.of("Build").unwrap().size.height);
}

#[test]
fn jobs_inside_a_stage_follow_their_own_dependency_graph() {
    let arrangement = layout::arrange_pipeline(&parse("multi-stage.yml"), &METRICS, &expanded(&["Test"]));
    assert_eq!(
        arrangement.of("Test/Unit").unwrap().x,
        arrangement.of("Test/Integration").unwrap().x
    );
    assert_ne!(
        arrangement.of("Test/Unit").unwrap().y,
        arrangement.of("Test/Integration").unwrap().y
    );
}

#[test]
fn jobs_that_wait_for_each_other_sit_in_different_columns_inside_the_stage() {
    let arrangement = layout::arrange_pipeline(&parse("jobs-only.yml"), &METRICS, &expanded(&["stage-0"]));
    assert!(arrangement.of("stage-0/Test").unwrap().x > arrangement.of("stage-0/Build").unwrap().x);
}

#[test]
fn a_collapsed_pipeline_places_only_its_stages() {
    let model = parse("multi-stage.yml");
    let arrangement = layout::arrange_pipeline(&model, &METRICS, &HashSet::new());
    assert_eq!(model.stages.len(), arrangement.placements.len());
    assert!(arrangement.of("Test/Unit").is_none());
}

#[test]
fn a_stage_beside_an_expanded_one_does_not_overlap_it() {
    let arrangement = layout::arrange_pipeline(&parse("multi-stage.yml"), &METRICS, &expanded(&["Test"]));
    let test = arrangement.of("Test").unwrap();
    let staging = arrangement.of("DeployStaging").unwrap();
    assert!(staging.x >= test.x + test.size.width);
}

#[test]
fn a_large_stage_lays_out_without_complaint() {
    let jobs: String = (0..256)
        .map(|index| format!("  - job: J{index}\n    steps:\n      - script: x\n"))
        .collect();
    let model = parse_text(&format!("jobs:\n{jobs}"));
    let arrangement = layout::arrange_pipeline(&model, &METRICS, &expanded(&["stage-0"]));
    assert_eq!(257, arrangement.placements.len());
    assert_eq!(
        256,
        arrangement
            .placements
            .iter()
            .filter(|p| p.id.starts_with("stage-0/"))
            .count()
    );
    assert_eq!(255, arrangement.of("stage-0/J255").unwrap().row);
}

#[test]
fn a_deep_pipeline_does_not_overflow_the_stack() {
    let stages: String = (0..500)
        .map(|index| format!("  - stage: S{index}\n    jobs:\n      - job: J\n        steps:\n          - script: x\n"))
        .collect();
    let model = parse_text(&format!("stages:\n{stages}"));
    assert_eq!(499, arrange_stages(&model).of("S499").unwrap().layer);
}
