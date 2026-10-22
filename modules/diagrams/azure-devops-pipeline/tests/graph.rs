//! What waits for what: the stage default, `dependsOn: []`, parallel jobs, fan-out and fan-in, broken edges, cycles
//! and the conditions an edge carries (the standalone host's `PipelineGraphBuilderTests`).

mod common;

use adp_diagram_azure_devops_pipeline::graph::{self, EdgeCondition, PipelineGraph};
use common::{all_fixtures, parse, parse_text};

fn text(lines: &[&str]) -> String {
    lines.join("\n")
}

fn sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids
}

#[test]
fn a_stage_with_no_depends_on_waits_for_the_stage_before_it() {
    let model = parse_text(&text(&[
        "stages:",
        "  - stage: One",
        "    jobs:",
        "      - job: A",
        "        steps:",
        "          - script: x",
        "  - stage: Two",
        "    jobs:",
        "      - job: B",
        "        steps:",
        "          - script: x",
    ]));
    let graph = graph::of_stages(&model);
    assert_eq!(1, graph.edges.len());
    let edge = &graph.edges[0];
    assert_eq!("One", edge.from_id);
    assert_eq!("Two", edge.to_id);
    assert!(edge.is_implicit);
}

#[test]
fn the_first_stage_waits_for_nothing() {
    let graph = graph::of_stages(&parse("multi-stage.yml"));
    assert_eq!(0, graph.dependencies_of("Build").count());
}

#[test]
fn a_stage_with_an_empty_depends_on_waits_for_nothing() {
    let model = parse_text(&text(&[
        "stages:",
        "  - stage: One",
        "    jobs:",
        "      - job: A",
        "        steps:",
        "          - script: x",
        "  - stage: Two",
        "    dependsOn: []",
        "    jobs:",
        "      - job: B",
        "        steps:",
        "          - script: x",
    ]));
    assert!(graph::of_stages(&model).edges.is_empty());
}

#[test]
fn jobs_with_no_depends_on_run_in_parallel_rather_than_in_order() {
    let model = parse("multi-stage.yml");
    let stage = model.stages.iter().find(|s| s.name == "Test").unwrap();
    let graph = graph::of_jobs(stage);
    assert_eq!(vec!["Test/Unit", "Test/Integration"], graph.node_ids);
    assert!(graph.edges.is_empty());
}

#[test]
fn a_job_with_a_depends_on_waits_for_the_job_it_names() {
    let model = parse("jobs-only.yml");
    let graph = graph::of_jobs(&model.stages[0]);
    assert_eq!(1, graph.edges.len());
    let edge = &graph.edges[0];
    assert_eq!("stage-0/Build", edge.from_id);
    assert_eq!("stage-0/Test", edge.to_id);
    assert!(!edge.is_implicit);
}

#[test]
fn fan_out_is_represented() {
    let graph = graph::of_stages(&parse("multi-stage.yml"));
    let from_test: Vec<&str> = graph.dependents_of("Test").collect();
    assert_eq!(vec!["DeployStaging", "DeployProduction"], from_test);
}

#[test]
fn fan_in_is_represented() {
    let graph = graph::of_stages(&parse("multi-stage.yml"));
    let into_notify: Vec<&str> = graph.dependencies_of("Notify").collect();
    assert_eq!(vec!["DeployStaging", "DeployProduction"], into_notify);
}

#[test]
fn a_depends_on_naming_nothing_keeps_its_edge_and_marks_it_broken() {
    let graph = graph::of_stages(&parse("edge-broken-graph.yml"));
    let broken: Vec<_> = graph.broken_edges().collect();
    assert_eq!(1, broken.len());
    assert_eq!("Ghost", broken[0].to_id);
    assert_eq!("DoesNotExist", broken[0].from_name);
    assert_eq!("", broken[0].from_id);
}

#[test]
fn a_cycle_is_reported_naming_the_elements_in_it() {
    let graph = graph::of_stages(&parse("edge-broken-graph.yml"));
    assert_eq!(1, graph.cycles.len());
    assert_eq!(vec!["Left", "Right"], sorted(graph.cycles[0].clone()));
}

#[test]
fn a_cycle_does_not_stop_the_rest_of_the_graph_being_built() {
    let graph = graph::of_stages(&parse("edge-broken-graph.yml"));
    assert_eq!(vec!["Build", "Ghost", "Left", "Right"], graph.node_ids);
    assert!(!graph.edges.is_empty());
}

#[test]
fn a_stage_depending_on_itself_is_a_cycle_of_one() {
    let model = parse_text(
        "stages:\n  - stage: Loop\n    dependsOn: Loop\n    jobs:\n      - job: A\n        steps:\n          - script: x\n",
    );
    let graph = graph::of_stages(&model);
    assert_eq!(vec![vec!["Loop".to_owned()]], graph.cycles);
}

#[test]
fn a_longer_cycle_is_reported_once_rather_than_once_per_way_in() {
    let mut lines = vec!["stages:"];
    for (name, depends) in [("A", "C"), ("B", "A"), ("C", "B")] {
        lines.extend([
            match name {
                "A" => "  - stage: A",
                "B" => "  - stage: B",
                _ => "  - stage: C",
            },
            match depends {
                "A" => "    dependsOn: A",
                "B" => "    dependsOn: B",
                _ => "    dependsOn: C",
            },
            "    jobs:",
            "      - job: J",
            "        steps:",
            "          - script: x",
        ]);
    }
    let graph = graph::of_stages(&parse_text(&text(&lines)));
    assert_eq!(1, graph.cycles.len());
    assert_eq!(vec!["A", "B", "C"], sorted(graph.cycles[0].clone()));
}

#[test]
fn an_acyclic_pipeline_reports_no_cycles() {
    assert!(graph::of_stages(&parse("multi-stage.yml")).cycles.is_empty());
}

#[test]
fn a_well_known_condition_says_which_outcome_it_waits_for() {
    for (condition, expected) in [
        ("", EdgeCondition::OnSuccess),
        ("succeeded()", EdgeCondition::OnSuccess),
        ("succeeded('Build')", EdgeCondition::OnSuccess),
        ("failed()", EdgeCondition::OnFailure),
        ("always()", EdgeCondition::Always),
        ("succeededOrFailed()", EdgeCondition::OnSuccessOrFailure),
        ("SucceededOrFailed()", EdgeCondition::OnSuccessOrFailure),
    ] {
        assert_eq!(expected, graph::classify(condition), "{condition}");
    }
}

#[test]
fn a_condition_this_module_will_not_interpret_is_custom_rather_than_guessed_at() {
    for condition in [
        "and(succeeded(), eq(variables['x'], 'y'))",
        "eq(dependencies.Build.outputs['Compile.result'], 'Succeeded')",
        "${{ parameters.deploy }}",
    ] {
        assert_eq!(EdgeCondition::Custom, graph::classify(condition), "{condition}");
    }
}

#[test]
fn a_true_difference_between_always_and_succeeded_or_failed_is_kept() {
    assert_ne!(graph::classify("always()"), graph::classify("succeededOrFailed()"));
}

#[test]
fn an_edge_into_a_conditional_stage_carries_the_condition_verbatim() {
    let graph = graph::of_stages(&parse("multi-stage.yml"));
    let edges: Vec<_> = graph.edges.iter().filter(|e| e.to_id == "Notify").collect();
    assert_eq!(2, edges.len());
    for edge in edges {
        assert!(edge.is_conditional());
        assert_eq!("always()", edge.condition_text);
        assert_eq!(EdgeCondition::Always, edge.condition);
    }
}

#[test]
fn an_edge_into_an_unconditional_stage_is_not_marked_conditional() {
    let graph = graph::of_stages(&parse("multi-stage.yml"));
    let edge = graph.edges.iter().find(|e| e.to_id == "Test").unwrap();
    assert!(!edge.is_conditional());
    assert_eq!(EdgeCondition::OnSuccess, edge.condition);
}

#[test]
fn a_depends_on_matches_by_name_not_by_id() {
    let model = parse_text(&text(&[
        "stages:",
        "  - stage: One",
        "    jobs:",
        "      - job: Alpha",
        "        steps:",
        "          - script: x",
        "      - job: Beta",
        "        dependsOn: Alpha",
        "        steps:",
        "          - script: x",
    ]));
    let graph = graph::of_jobs(&model.stages[0]);
    assert_eq!(1, graph.edges.len());
    assert_eq!("One/Alpha", graph.edges[0].from_id);
    assert_eq!("One/Beta", graph.edges[0].to_id);
}

#[test]
fn an_empty_level_is_an_empty_graph() {
    let model = parse_text("stages:\n  - stage: Empty\n");
    assert_eq!(PipelineGraph::default(), graph::of_jobs(&model.stages[0]));
}

#[test]
fn every_fixture_builds_a_graph_without_failing() {
    let names = all_fixtures();
    assert!(names.len() >= 10);
    let mut staged = 0;
    for name in names {
        let model = parse(&name);
        let graph = graph::of_stages(&model);
        assert_eq!(model.stages.len(), graph.node_ids.len());
        for stage in &model.stages {
            let _ = graph::of_jobs(stage);
        }
        staged += model.stages.len();
    }
    assert!(staged >= 15, "only {staged} stages were parsed");
}
