//! What decides whether and how an element runs: pools and their inheritance, conditions, timeouts, `enabled`,
//! manual triggers and strategies (the standalone host's `PipelineParserExecutionTests`).

mod common;

use adp_diagram_azure_devops_pipeline::model::{PipelineExecution, PoolOrigin, StrategyKind};
use common::{parse, parse_text};

/// The lines joined as a C# raw string literal joins them: no newline after the last.
fn text(lines: &[&str]) -> String {
    lines.join("\n")
}

#[test]
fn a_pipeline_level_pool_reaches_a_job_that_declares_none() {
    let model = parse("jobs-only.yml");
    let build = model.jobs().find(|j| j.name == "Build").unwrap();
    assert_eq!("ubuntu-latest", build.pool.vm_image);
    assert_eq!(PoolOrigin::Pipeline, build.pool.origin);
    assert!(build.pool.is_inherited_by_a_job());
}

#[test]
fn a_stages_pool_beats_the_pipelines() {
    let model = parse_text(&text(&[
        "pool:",
        "  vmImage: ubuntu-latest",
        "stages:",
        "  - stage: Build",
        "    pool:",
        "      vmImage: windows-latest",
        "    jobs:",
        "      - job: Compile",
        "        steps:",
        "          - script: echo build",
    ]));
    assert_eq!(1, model.stages.len());
    let stage = &model.stages[0];
    assert_eq!("windows-latest", stage.pool.vm_image);
    assert_eq!(PoolOrigin::Stage, stage.pool.origin);
    assert_eq!("windows-latest", stage.jobs[0].pool.vm_image);
    assert_eq!(PoolOrigin::Stage, stage.jobs[0].pool.origin);
}

#[test]
fn a_jobs_own_pool_beats_both() {
    let model = parse_text(&text(&[
        "pool:",
        "  vmImage: ubuntu-latest",
        "stages:",
        "  - stage: Build",
        "    pool:",
        "      vmImage: windows-latest",
        "    jobs:",
        "      - job: Compile",
        "        pool:",
        "          vmImage: macos-latest",
        "        steps:",
        "          - script: echo build",
    ]));
    let job = model.jobs().next().unwrap();
    assert_eq!("macos-latest", job.pool.vm_image);
    assert_eq!(PoolOrigin::Job, job.pool.origin);
    assert!(!job.pool.is_inherited_by_a_job());
}

#[test]
fn a_pipeline_with_no_pool_anywhere_says_so_rather_than_inventing_one() {
    let model = parse_text(
        "stages:\n  - stage: Build\n    jobs:\n      - job: Compile\n        steps:\n          - script: x\n",
    );
    let job = model.jobs().next().unwrap();
    assert!(!job.pool.is_declared());
    assert_eq!(PoolOrigin::None, job.pool.origin);
}

#[test]
fn a_bare_pool_name_reads_as_a_name() {
    let model = parse_text("pool: Default\njobs:\n  - job: A\n    steps:\n      - script: x\n");
    let pool = &model.jobs().next().unwrap().pool;
    assert_eq!("Default", pool.name);
    assert_eq!("", pool.vm_image);
    assert_eq!("Default", pool.label());
}

#[test]
fn a_pools_demands_are_carried() {
    let model = parse_text(
        "pool:\n  name: Default\n  demands:\n    - agent.os -equals Linux\n    - docker\njobs:\n  - job: A\n    steps:\n      - script: x\n",
    );
    assert_eq!(
        vec!["agent.os -equals Linux".to_owned(), "docker".to_owned()],
        model.jobs().next().unwrap().pool.demands
    );
}

#[test]
fn a_merged_in_pool_is_inherited() {
    let model = parse("edge-anchors.yml");
    let one = model.stages.iter().find(|s| s.name == "One").unwrap();
    assert_eq!("ubuntu-latest", one.pool.vm_image);
    assert_eq!(PoolOrigin::Stage, one.pool.origin);
}

#[test]
fn a_stages_condition_is_carried_verbatim() {
    let model = parse("multi-stage.yml");
    let staging = model.stages.iter().find(|s| s.name == "DeployStaging").unwrap();
    assert_eq!(
        "and(succeeded(), eq(variables['Build.SourceBranch'], 'refs/heads/main'))",
        staging.execution.condition
    );
    assert!(staging.execution.has_condition());
}

#[test]
fn an_expression_valued_condition_survives_unevaluated() {
    let model = parse("edge-expressions.yml");
    let report = model.stages.iter().find(|s| s.name == "Report").unwrap();
    assert_eq!(
        "eq(dependencies.Build.outputs['Compile.result'], 'Succeeded')",
        report.execution.condition
    );
}

#[test]
fn a_jobs_timeout_and_continue_on_error_are_carried() {
    let model = parse("multi-stage.yml");
    let integration = model.jobs().find(|j| j.name == "Integration").unwrap();
    assert_eq!("30", integration.execution.timeout_in_minutes);
    assert_eq!("true", integration.execution.continue_on_error);
    assert!(integration.execution.continues_on_error());
}

#[test]
fn a_stage_with_none_of_these_carries_the_default() {
    let model = parse("multi-stage.yml");
    let build = model.stages.iter().find(|s| s.name == "Build").unwrap();
    assert_eq!(PipelineExecution::default(), build.execution);
    assert!(!build.execution.has_condition());
}

#[test]
fn a_disabled_step_is_known_to_be_disabled() {
    let model = parse_text("steps:\n  - script: echo x\n    enabled: false\n");
    assert!(model.steps().next().unwrap().execution.is_disabled());
}

#[test]
fn an_expression_valued_enabled_is_not_treated_as_disabled() {
    let model = parse_text("steps:\n  - script: echo x\n    enabled: $(runIt)\n");
    let step = model.steps().next().unwrap();
    assert_eq!("$(runIt)", step.execution.enabled);
    assert!(!step.execution.is_disabled());
}

#[test]
fn a_manually_triggered_stage_is_known_to_wait_for_a_person() {
    let model = parse("multi-stage.yml");
    assert!(
        model
            .stages
            .iter()
            .find(|s| s.name == "DeployProduction")
            .unwrap()
            .trigger_is_manual
    );
    assert!(
        !model
            .stages
            .iter()
            .find(|s| s.name == "Build")
            .unwrap()
            .trigger_is_manual
    );
}

#[test]
fn a_repository_trigger_is_not_a_manual_stage_trigger() {
    let model = parse_text(
        "trigger:\n  - main\nstages:\n  - stage: Build\n    jobs:\n      - job: A\n        steps:\n          - script: x\n",
    );
    assert!(!model.stages[0].trigger_is_manual);
}

#[test]
fn an_unskippable_stage_carries_its_is_skippable() {
    let model = parse_text(
        "stages:\n  - stage: Audit\n    isSkippable: false\n    jobs:\n      - job: A\n        steps:\n          - script: x\n",
    );
    assert_eq!("false", model.stages[0].is_skippable);
}

#[test]
fn a_matrix_strategy_multiplies_the_job_by_its_entries() {
    let model = parse_text(&text(&[
        "jobs:",
        "  - job: Test",
        "    strategy:",
        "      matrix:",
        "        linux:",
        "          image: ubuntu-latest",
        "        windows:",
        "          image: windows-latest",
        "        mac:",
        "          image: macos-latest",
        "    steps:",
        "      - script: dotnet test",
    ]));
    let job = model.jobs().next().unwrap();
    assert_eq!(StrategyKind::Matrix, job.strategy.kind);
    assert_eq!(3, job.strategy.multiplicity);
    assert!(job.strategy.is_multiplied());
    assert_eq!("", job.strategy.multiplicity_expression);
}

#[test]
fn a_parallel_strategy_multiplies_the_job_by_its_count() {
    let model = parse_text("jobs:\n  - job: Test\n    strategy:\n      parallel: 5\n    steps:\n      - script: x\n");
    let job = model.jobs().next().unwrap();
    assert_eq!(StrategyKind::Parallel, job.strategy.kind);
    assert_eq!(5, job.strategy.multiplicity);
}

#[test]
fn a_parallel_count_given_as_a_variable_admits_it_is_not_known_yet() {
    let model =
        parse_text("jobs:\n  - job: Test\n    strategy:\n      parallel: $(slices)\n    steps:\n      - script: x\n");
    let job = model.jobs().next().unwrap();
    assert_eq!(1, job.strategy.multiplicity);
    assert_eq!("$(slices)", job.strategy.multiplicity_expression);
    assert!(job.strategy.is_multiplied());
}

#[test]
fn a_deployment_strategy_multiplies_nothing() {
    let model = parse("multi-stage.yml");
    let staging = model.jobs().find(|j| j.name == "Staging").unwrap();
    assert_eq!(1, staging.strategy.multiplicity);
    assert!(!staging.strategy.is_multiplied());
}

#[test]
fn a_job_with_no_strategy_runs_once() {
    let model = parse("multi-stage.yml");
    let compile = model.jobs().find(|j| j.name == "Compile").unwrap();
    assert_eq!(StrategyKind::None, compile.strategy.kind);
    assert_eq!(1, compile.strategy.multiplicity);
    assert!(!compile.strategy.is_multiplied());
}
