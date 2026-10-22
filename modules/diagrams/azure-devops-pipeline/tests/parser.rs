//! Reading a pipeline: every fixture reads, every element points at its own declaration, the implicit stage and job,
//! deployment hooks, step kinds, `dependsOn`, merge keys, gates and templates (the standalone host's
//! `PipelineParserTests`).

mod common;

use std::collections::HashSet;

use adp_diagram_azure_devops_pipeline::lines::LineRange;
use adp_diagram_azure_devops_pipeline::model::{StepKind, StrategyKind, TemplateSlot};
use common::{all_fixtures, document, is_comment, parse, parse_text, parse_with_templates};

#[test]
fn every_fixture_parses() {
    for name in all_fixtures() {
        assert!(
            adp_diagram_azure_devops_pipeline::parser::parse(&document(&name)).is_ok(),
            "{name}"
        );
    }
}

#[test]
fn every_element_points_at_its_own_declaration() {
    for name in all_fixtures() {
        let document = document(&name);
        let model = adp_diagram_azure_devops_pipeline::parser::parse(&document).unwrap();
        let elements: Vec<(&str, &str, LineRange)> = model
            .stages
            .iter()
            .map(|s| ("stage", s.id.as_str(), s.lines))
            .chain(model.jobs().map(|j| ("job", j.id.as_str(), j.lines)))
            .chain(model.steps().map(|s| ("step", s.id.as_str(), s.lines)))
            .collect();
        for (kind, id, lines) in elements {
            assert!(lines.start < document.lines().len(), "{name}: {kind} {id}");
            assert!(
                lines.end >= lines.start && lines.end < document.lines().len(),
                "{name}: {kind} {id}"
            );
            let first = &document.lines()[lines.start];
            assert!(!first.is_blank(), "{name}: {kind} {id} starts on a blank line");
            assert!(!is_comment(&first.text), "{name}: {kind} {id} starts on a comment line");
        }
    }
}

#[test]
fn an_elements_range_contains_its_childrens() {
    for name in all_fixtures() {
        let model = parse(&name);
        for stage in &model.stages {
            for job in &stage.jobs {
                assert!(
                    job.lines.start >= stage.lines.start && job.lines.end <= stage.lines.end,
                    "{name}: job {} at {:?} escapes stage {} at {:?}",
                    job.id,
                    job.lines,
                    stage.id,
                    stage.lines
                );
                for step in &job.steps {
                    assert!(
                        step.lines.start >= job.lines.start && step.lines.end <= job.lines.end,
                        "{name}: step {} escapes job {}",
                        step.id,
                        job.id
                    );
                }
            }
        }
    }
}

#[test]
fn a_multi_stage_pipeline_reads_its_stages_in_order() {
    let model = parse("multi-stage.yml");
    let names: Vec<&str> = model.stages.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(
        vec!["Build", "Test", "DeployStaging", "DeployProduction", "Notify"],
        names
    );
    assert!(model.stages.iter().all(|s| !s.is_implicit));
}

#[test]
fn a_stages_declaration_points_at_the_line_holding_its_name() {
    let document = document("multi-stage.yml");
    let model = adp_diagram_azure_devops_pipeline::parser::parse(&document).unwrap();
    let test = model.stages.iter().find(|s| s.name == "Test").unwrap();
    assert!(document.line(test.lines.start).contains("stage: Test"));
    assert!(document.line(test.lines.end).contains("Integration tests"));
}

#[test]
fn a_stages_range_does_not_swallow_the_comment_introducing_the_next_stage() {
    let document = document("multi-stage.yml");
    let model = adp_diagram_azure_devops_pipeline::parser::parse(&document).unwrap();
    let build = model.stages.iter().find(|s| s.name == "Build").unwrap();
    let last = &document.lines()[build.lines.end];
    assert!(!last.is_blank());
    assert!(!is_comment(&last.text));
    assert!(!last.text.contains("stage: Test"));
}

#[test]
fn a_jobs_only_file_yields_one_implicit_stage() {
    let model = parse("jobs-only.yml");
    assert_eq!(1, model.stages.len());
    let stage = &model.stages[0];
    assert!(stage.is_implicit);
    assert_eq!("", stage.name);
    let names: Vec<&str> = stage.jobs.iter().map(|j| j.name.as_str()).collect();
    assert_eq!(vec!["Build", "Test"], names);
    assert!(stage.jobs.iter().all(|j| !j.is_implicit));
}

#[test]
fn a_steps_only_file_yields_one_implicit_stage_around_one_implicit_job() {
    let model = parse("steps-only.yml");
    assert_eq!(1, model.stages.len());
    let stage = &model.stages[0];
    assert!(stage.is_implicit);
    assert_eq!(1, stage.jobs.len());
    let job = &stage.jobs[0];
    assert!(job.is_implicit);
    let labels: Vec<String> = job.steps.iter().map(|s| s.label()).collect();
    assert_eq!(vec!["Build", "Test"], labels);
}

#[test]
fn a_deployment_job_carries_its_environment_and_strategy() {
    let model = parse("multi-stage.yml");
    let staging = model.jobs().find(|j| j.name == "Staging").unwrap();
    assert!(staging.is_deployment);
    assert_eq!("staging", staging.environment);
    assert_eq!(StrategyKind::RunOnce, staging.strategy.kind);
    assert!(staging.strategy.is_deployment());
    assert_eq!(LineRange::new(52, 58), staging.strategy.lines);
}

#[test]
fn a_deployment_jobs_steps_come_from_its_lifecycle_hooks() {
    let model = parse("multi-stage.yml");
    let staging = model.jobs().find(|j| j.name == "Staging").unwrap();
    let kinds: Vec<StepKind> = staging.steps.iter().map(|s| s.kind).collect();
    assert_eq!(vec![StepKind::Download, StepKind::Script], kinds);
    assert!(staging.steps.iter().all(|s| s.hook == "deploy"));
}

#[test]
fn a_plain_jobs_steps_carry_no_hook() {
    let model = parse("multi-stage.yml");
    let compile = model.jobs().find(|j| j.name == "Compile").unwrap();
    assert!(compile.steps.iter().all(|s| s.hook.is_empty()));
}

#[test]
fn each_step_kind_is_read_from_the_key_it_leads_with() {
    let model = parse("multi-stage.yml");
    let compile = model.jobs().find(|j| j.name == "Compile").unwrap();
    let kinds: Vec<StepKind> = compile.steps.iter().map(|s| s.kind).collect();
    assert_eq!(vec![StepKind::Checkout, StepKind::Script, StepKind::Publish], kinds);
    assert_eq!("self", compile.steps[0].identifier);
}

#[test]
fn a_step_without_a_display_name_falls_back_to_its_kinds_identifying_value() {
    let model = parse("multi-stage.yml");
    let checkout = &model.jobs().find(|j| j.name == "Compile").unwrap().steps[0];
    assert_eq!("", checkout.display_name);
    assert_eq!("self", checkout.label());
}

#[test]
fn a_multi_line_script_does_not_become_a_multi_line_label() {
    let model = parse_text("steps:\n  - script: |\n      echo one\n      echo two\n");
    let step = model.steps().next().unwrap();
    assert_eq!("echo one", step.label());
}

#[test]
fn depends_on_as_a_scalar_and_as_a_list_both_read() {
    let model = parse("multi-stage.yml");
    assert_eq!(
        vec!["Build".to_owned()],
        model.stages.iter().find(|s| s.name == "Test").unwrap().depends_on
    );
    assert_eq!(
        vec!["DeployStaging".to_owned(), "DeployProduction".to_owned()],
        model.stages.iter().find(|s| s.name == "Notify").unwrap().depends_on
    );
}

#[test]
fn an_absent_depends_on_is_not_the_same_as_an_empty_one() {
    let model = parse("edge-anchors.yml");
    let one = model.stages.iter().find(|s| s.name == "One").unwrap();
    let two = model.stages.iter().find(|s| s.name == "Two").unwrap();
    assert!(!one.depends_on_declared);
    assert!(one.depends_on.is_empty());
    assert!(two.depends_on_declared);
    assert!(two.depends_on.is_empty());
}

#[test]
fn a_merge_key_does_not_become_a_stage_of_its_own() {
    let model = parse("edge-anchors.yml");
    let names: Vec<&str> = model.stages.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(vec!["One", "Two"], names);
}

#[test]
fn an_unindented_sequence_reads_like_an_indented_one() {
    let model = parse("edge-indentation.yml");
    let names: Vec<&str> = model.stages.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(vec!["Build", "Test"], names);
    assert_eq!(
        "Build",
        model.jobs().find(|j| j.name == "Compile").unwrap().steps[0].label()
    );
}

#[test]
fn a_stage_inside_a_compile_time_conditional_still_appears() {
    let model = parse("edge-expressions.yml");
    let deploy = model
        .stages
        .iter()
        .find(|s| s.name == "Deploy")
        .expect("the gated stage appears");
    assert!(deploy.gate.contains("if parameters.deploy"));
    let names: Vec<&str> = model.stages.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(vec!["Build", "Deploy", "Report"], names);
}

#[test]
fn an_expression_valued_property_survives_verbatim() {
    let model = parse("edge-expressions.yml");
    let build = model.stages.iter().find(|s| s.name == "Build").unwrap();
    assert_eq!("${{ format('Build {0}', parameters.deploy) }}", build.display_name);
}

#[test]
fn a_stage_level_template_is_recorded_rather_than_dropped() {
    let model = parse("templates.yml");
    let found: Vec<_> = model
        .templates
        .iter()
        .filter(|t| t.slot == TemplateSlot::Stages)
        .collect();
    assert_eq!(1, found.len());
    let template = found[0];
    assert_eq!("templates/deploy-stages.yml", template.path);
    assert_eq!("shared", template.resource);
    assert_eq!("templates/deploy-stages.yml@shared", template.reference());
    assert_eq!(vec!["environment".to_owned()], template.parameter_names);
}

#[test]
fn a_template_that_names_a_template_gives_every_reference_an_id_of_its_own() {
    let model = parse_with_templates("template-in-template.yml");
    let found: Vec<String> = model.templates.iter().map(|t| format!("{} {}", t.id, t.path)).collect();
    assert_eq!(
        vec![
            "template-0 templates/nested-jobs.yml",
            "template-1 test-steps.yml",
            "template-2 templates/test-steps.yml"
        ],
        found
    );
}

#[test]
fn a_job_level_template_remembers_the_stage_it_sits_in() {
    let model = parse("templates.yml");
    let found: Vec<_> = model
        .templates
        .iter()
        .filter(|t| t.slot == TemplateSlot::Jobs)
        .collect();
    assert_eq!(1, found.len());
    assert_eq!("templates/build-jobs.yml", found[0].path);
    assert_eq!("", found[0].resource);
    assert_eq!("Build", found[0].owner_id);
}

#[test]
fn a_step_level_template_is_a_step_of_kind_template() {
    let model = parse("templates.yml");
    let step = model.steps().find(|s| s.kind == StepKind::Template).unwrap();
    assert_eq!("templates/test-steps.yml", step.identifier);
    assert_eq!("templates/test-steps.yml", step.label());
}

#[test]
fn an_extending_file_declares_no_stages_of_its_own() {
    let model = parse("extends.yml");
    assert!(model.stages.is_empty());
    let extends = model.extends.expect("an extends");
    assert_eq!("templates/pipeline.yml", extends.path);
    assert_eq!(
        vec!["buildConfiguration".to_owned(), "deployTo".to_owned()],
        extends.parameter_names
    );
}

#[test]
fn element_ids_are_unique_within_a_document() {
    let model = parse("multi-stage.yml");
    let ids: Vec<&str> = model
        .stages
        .iter()
        .map(|s| s.id.as_str())
        .chain(model.jobs().map(|j| j.id.as_str()))
        .chain(model.steps().map(|s| s.id.as_str()))
        .collect();
    let distinct: HashSet<&str> = ids.iter().copied().collect();
    assert_eq!(ids.len(), distinct.len());
}

#[test]
fn a_document_with_no_stages_jobs_or_steps_parses_to_nothing_rather_than_throwing() {
    let model = parse_text("variables:\n  buildConfiguration: Release\n");
    assert!(model.stages.is_empty());
    assert!(model.extends.is_none());
}
