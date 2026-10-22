//! Splicing one property, keeping every other byte: renames, `dependsOn` in its three forms, `enabled`, comments,
//! quoting, line endings, and the refusals of what is not written in this file (the standalone host's
//! `PipelineWriterTests`).

mod common;

use adp_diagram_azure_devops_pipeline::lines::{LineDocument, LineRange};
use adp_diagram_azure_devops_pipeline::model::{EditTarget, PipelineStage, PipelineStep};
use adp_diagram_azure_devops_pipeline::parser;
use adp_diagram_azure_devops_pipeline::writer::PipelineWriter;
use common::{all_fixtures, document, fixture, fixtures};

fn snapshot(document: &LineDocument) -> Vec<String> {
    document
        .lines()
        .iter()
        .map(|l| format!("{}{}", l.text, l.ending))
        .collect()
}

fn assert_only_these_lines_changed(before: &[String], after: &[String], start: usize, removed: usize, added: usize) {
    assert_eq!(before.len() - removed + added, after.len());
    for index in 0..start {
        assert_eq!(before[index], after[index], "line {index}");
    }
    for offset in 0..before.len() - start - removed {
        assert_eq!(before[start + removed + offset], after[start + added + offset]);
    }
}

fn stage_in(document: &LineDocument, name: &str) -> PipelineStage {
    parser::parse(document)
        .unwrap()
        .stages
        .into_iter()
        .find(|s| s.name == name)
        .unwrap()
}

fn steps_in(document: &LineDocument) -> Vec<PipelineStep> {
    parser::parse(document).unwrap().stages[0].jobs[0].steps.clone()
}

fn find(lines: &[String], text: &str) -> usize {
    lines.iter().position(|l| l.contains(text)).unwrap()
}

#[test]
fn a_rename_changes_the_one_line_holding_the_name() {
    let mut document = document("multi-stage.yml");
    let build = stage_in(&document, "Build");
    let before = snapshot(&document);
    let changed = PipelineWriter::new(&mut document)
        .set_display_name(&EditTarget::of_stage(&build), "Build everything")
        .unwrap();
    let after = snapshot(&document);
    assert!(changed);
    let line = find(&before, "displayName: Build the solution");
    assert!(after[line].contains("displayName: Build everything"));
    assert_only_these_lines_changed(&before, &after, line, 1, 1);
}

#[test]
fn a_new_property_on_a_tied_file_takes_crlf() {
    let mut document = document("edge-tied-endings.yml");
    let unnamed = parser::parse(&document).unwrap().steps().last().unwrap().clone();
    let before = snapshot(&document);
    let changed = PipelineWriter::new(&mut document)
        .set_display_name(&EditTarget::of_step(&unnamed), "Named")
        .unwrap();
    let after = snapshot(&document);
    assert!(changed);
    let line = find(&after, "displayName: Named");
    assert!(after[line].ends_with("\r\n"));
    assert_only_these_lines_changed(&before, &after, line, 0, 1);
}

#[test]
fn a_rename_keeps_the_indentation_the_file_was_using() {
    let mut document = document("edge-indentation.yml");
    let build = stage_in(&document, "Build");
    PipelineWriter::new(&mut document)
        .set_display_name(&EditTarget::of_stage(&build), "Built")
        .unwrap();
    assert!(document.text().contains("\n  displayName: Built\n"));
}

#[test]
fn a_rename_of_a_stage_does_not_find_the_name_of_one_of_its_jobs() {
    let mut document = document("multi-stage.yml");
    let build = stage_in(&document, "Build");
    PipelineWriter::new(&mut document)
        .set_display_name(&EditTarget::of_stage(&build), "Renamed")
        .unwrap();
    assert!(document.text().contains("displayName: Renamed"));
    assert!(document.text().contains("displayName: Compile"));
}

#[test]
fn a_rename_that_changes_nothing_writes_nothing() {
    let mut document = document("multi-stage.yml");
    let build = stage_in(&document, "Build");
    let before = snapshot(&document);
    let changed = PipelineWriter::new(&mut document)
        .set_display_name(&EditTarget::of_stage(&build), "Build the solution")
        .unwrap();
    assert!(!changed);
    assert_eq!(before, snapshot(&document));
}

#[test]
fn a_rename_keeps_a_trailing_comment_on_the_line_it_rewrites() {
    let mut document =
        LineDocument::parse("stages:\n  - stage: Build\n    displayName: Old   # why it is called that\n");
    let build = stage_in(&document, "Build");
    PipelineWriter::new(&mut document)
        .set_display_name(&EditTarget::of_stage(&build), "New")
        .unwrap();
    assert!(document.text().contains("displayName: New   # why it is called that"));
}

#[test]
fn a_hash_inside_a_quoted_value_is_not_mistaken_for_a_comment() {
    let mut document = LineDocument::parse("stages:\n  - stage: Build\n    displayName: \"Build #2\"\n");
    let build = stage_in(&document, "Build");
    PipelineWriter::new(&mut document)
        .set_display_name(&EditTarget::of_stage(&build), "New")
        .unwrap();
    assert!(document.text().contains("displayName: New\n"));
    assert!(!document.text().contains("#2"));
}

#[test]
fn an_element_without_a_display_name_gains_one_without_disturbing_the_rest() {
    let mut document = document("multi-stage.yml");
    let staging = stage_in(&document, "DeployStaging");
    let before = snapshot(&document);
    let changed = PipelineWriter::new(&mut document)
        .set_display_name(&EditTarget::of_stage(&staging), "Deploy to staging")
        .unwrap();
    let after = snapshot(&document);
    assert!(changed);
    assert_only_these_lines_changed(&before, &after, staging.lines.start + 1, 0, 1);
    assert_eq!("    displayName: Deploy to staging\n", after[staging.lines.start + 1]);
}

#[test]
fn a_new_key_on_a_step_that_opens_with_a_literal_block_goes_after_the_whole_script() {
    let mut document = document("block-scalar-step.yml");
    let step = steps_in(&document)[0].clone();
    let before = snapshot(&document);
    let last = find(&before, "dotnet test");
    let changed = PipelineWriter::new(&mut document)
        .set_display_name(&EditTarget::of_step(&step), "Build and test")
        .unwrap();
    let after = snapshot(&document);
    assert!(changed);
    assert_only_these_lines_changed(&before, &after, last + 1, 0, 1);
    assert_eq!("            displayName: Build and test\n", after[last + 1]);
    assert_eq!("Build and test", steps_in(&document)[0].display_name);
}

#[test]
fn a_new_key_on_a_step_that_opens_with_a_folded_block_goes_after_the_whole_script_and_before_the_next_key() {
    let mut document = document("block-scalar-step.yml");
    let step = steps_in(&document)[1].clone();
    let before = snapshot(&document);
    let last = find(&before, "onto one line");
    let changed = PipelineWriter::new(&mut document)
        .set_enabled(&EditTarget::of_step(&step), "false")
        .unwrap();
    let after = snapshot(&document);
    assert!(changed);
    assert_only_these_lines_changed(&before, &after, last + 1, 0, 1);
    assert_eq!("            enabled: false\n", after[last + 1]);
    assert_eq!("            condition: always()\n", after[last + 2]);
}

#[test]
fn removing_a_display_name_takes_its_line_and_no_other() {
    let mut document = document("multi-stage.yml");
    let build = stage_in(&document, "Build");
    let before = snapshot(&document);
    let line = find(&before, "displayName: Build the solution");
    let changed = PipelineWriter::new(&mut document)
        .set_display_name(&EditTarget::of_stage(&build), "")
        .unwrap();
    assert!(changed);
    assert_only_these_lines_changed(&before, &snapshot(&document), line, 1, 0);
}

#[test]
fn a_stage_gaining_a_depends_on_it_never_had_inserts_lines_without_disturbing_comments() {
    let mut document = document("multi-stage.yml");
    let build = stage_in(&document, "Build");
    let before = snapshot(&document);
    let changed = PipelineWriter::new(&mut document)
        .set_depends_on(&EditTarget::of_stage(&build), &["Prepare".to_owned()])
        .unwrap();
    let after = snapshot(&document);
    assert!(changed);
    assert_only_these_lines_changed(&before, &after, build.lines.start + 1, 0, 1);
    assert_eq!("    dependsOn: Prepare\n", after[build.lines.start + 1]);
}

#[test]
fn a_depends_on_list_rewrite_leaves_the_comment_above_it_alone() {
    let mut document = document("multi-stage.yml");
    let notify = stage_in(&document, "Notify");
    let before = snapshot(&document);
    PipelineWriter::new(&mut document)
        .set_depends_on(&EditTarget::of_stage(&notify), &["DeployStaging".to_owned()])
        .unwrap();
    let after = snapshot(&document);
    assert!(after.iter().any(|l| l.contains("Fan-in: waits for both deployments.")));
    assert_eq!(before.len() - 3 + 1, after.len());
}

#[test]
fn a_depends_on_of_several_names_is_written_as_a_block_list() {
    let mut document = document("multi-stage.yml");
    let test = stage_in(&document, "Test");
    PipelineWriter::new(&mut document)
        .set_depends_on(
            &EditTarget::of_stage(&test),
            &["Build".to_owned(), "Prepare".to_owned()],
        )
        .unwrap();
    assert!(
        document
            .text()
            .contains("    dependsOn:\n      - Build\n      - Prepare\n")
    );
}

#[test]
fn a_depends_on_of_nothing_is_written_as_an_empty_list_rather_than_removed() {
    let mut document = document("multi-stage.yml");
    let test = stage_in(&document, "Test");
    PipelineWriter::new(&mut document)
        .set_depends_on(&EditTarget::of_stage(&test), &[])
        .unwrap();
    assert!(document.text().contains("    dependsOn: []\n"));
    let reread = stage_in(&document, "Test");
    assert!(reread.depends_on.is_empty());
    assert!(reread.depends_on_declared);
}

#[test]
fn clearing_a_depends_on_puts_the_element_back_on_the_default() {
    let mut document = document("multi-stage.yml");
    let test = stage_in(&document, "Test");
    let changed = PipelineWriter::new(&mut document)
        .clear_depends_on(&EditTarget::of_stage(&test))
        .unwrap();
    assert!(changed);
    assert!(!stage_in(&document, "Test").depends_on_declared);
}

#[test]
fn an_edited_file_is_otherwise_byte_for_byte_what_it_was() {
    let original = fixture("edge-comments.yml");
    let mut document = LineDocument::parse(&original);
    let stage = parser::parse(&document).unwrap().stages[0].clone();
    PipelineWriter::new(&mut document)
        .set_display_name(&EditTarget::of_stage(&stage), "Renamed")
        .unwrap();
    let before = snapshot(&LineDocument::parse(&original));
    let after = snapshot(&document);
    let inserted = stage.lines.start + 1;
    assert_eq!("    displayName: Renamed\n", after[inserted]);
    assert_only_these_lines_changed(&before, &after, inserted, 0, 1);
}

#[test]
fn an_edit_to_an_anchored_file_leaves_the_anchors_as_written() {
    let mut document = document("edge-anchors.yml");
    let two = stage_in(&document, "Two");
    PipelineWriter::new(&mut document)
        .set_display_name(&EditTarget::of_stage(&two), "The second one")
        .unwrap();
    assert!(document.text().contains(".defaults: &defaults"));
    assert_eq!(2, document.text().matches("<<: *defaults").count());
}

#[test]
fn an_edit_to_a_crlf_file_does_not_introduce_a_lone_newline() {
    let mut document = document("edge-crlf.yml");
    let step = parser::parse(&document).unwrap().steps().next().unwrap().clone();
    PipelineWriter::new(&mut document)
        .set_enabled(&EditTarget::of_step(&step), "false")
        .unwrap();
    let lines = document.lines();
    assert!(lines[..lines.len() - 1].iter().all(|l| l.ending == "\r\n"));
}

const SMALL: &str = "stages:\n  - stage: Build\n    jobs:\n      - job: A\n        steps:\n          - script: x\n";

#[test]
fn a_value_that_would_change_meaning_unquoted_is_quoted() {
    let mut document = LineDocument::parse(SMALL);
    let build = stage_in(&document, "Build");
    PipelineWriter::new(&mut document)
        .set_display_name(&EditTarget::of_stage(&build), "Build: everything")
        .unwrap();
    assert!(document.text().contains("displayName: \"Build: everything\""));
    assert_eq!("Build: everything", stage_in(&document, "Build").display_name);
}

#[test]
fn an_ordinary_value_is_not_quoted() {
    let mut document = LineDocument::parse(SMALL);
    let build = stage_in(&document, "Build");
    PipelineWriter::new(&mut document)
        .set_display_name(&EditTarget::of_stage(&build), "Build the solution")
        .unwrap();
    assert!(document.text().contains("displayName: Build the solution\n"));
}

#[test]
fn an_expression_value_is_written_exactly_as_given() {
    let mut document = LineDocument::parse(SMALL);
    let build = stage_in(&document, "Build");
    PipelineWriter::new(&mut document)
        .set_display_name(&EditTarget::of_stage(&build), "${{ parameters.title }}")
        .unwrap();
    assert!(document.text().contains("displayName: ${{ parameters.title }}\n"));
}

#[test]
fn an_edit_to_a_job_stays_inside_that_job() {
    let mut document = document("multi-stage.yml");
    let model = parser::parse(&document).unwrap();
    let integration = model.jobs().find(|j| j.name == "Integration").unwrap().clone();
    let before = snapshot(&document);
    PipelineWriter::new(&mut document)
        .set_enabled(&EditTarget::of_job(&integration), "false")
        .unwrap();
    let after = snapshot(&document);
    assert_only_these_lines_changed(&before, &after, integration.lines.start + 1, 0, 1);
    let reread = parser::parse(&document).unwrap();
    assert!(
        !reread
            .jobs()
            .find(|j| j.name == "Unit")
            .unwrap()
            .execution
            .is_disabled()
    );
    assert!(
        reread
            .jobs()
            .find(|j| j.name == "Integration")
            .unwrap()
            .execution
            .is_disabled()
    );
}

#[test]
fn an_edit_to_a_step_stays_inside_that_step() {
    let mut document = document("multi-stage.yml");
    let compile = parser::parse(&document)
        .unwrap()
        .jobs()
        .find(|j| j.name == "Compile")
        .unwrap()
        .clone();
    let before = snapshot(&document);
    PipelineWriter::new(&mut document)
        .set_enabled(&EditTarget::of_step(&compile.steps[1]), "false")
        .unwrap();
    let after = snapshot(&document);
    assert_only_these_lines_changed(&before, &after, compile.steps[1].lines.start + 1, 0, 1);
    let reread = parser::parse(&document).unwrap();
    let disabled: Vec<bool> = reread
        .jobs()
        .find(|j| j.name == "Compile")
        .unwrap()
        .steps
        .iter()
        .map(|s| s.execution.is_disabled())
        .collect();
    assert_eq!(vec![false, true, false], disabled);
}

#[test]
fn an_edit_outside_the_document_is_refused() {
    let mut document = LineDocument::parse("stages:\n  - stage: Build\n");
    let target = EditTarget {
        id: "x".to_owned(),
        lines: LineRange::new(0, 40),
        is_implicit: false,
        template: String::new(),
    };
    assert!(
        PipelineWriter::new(&mut document)
            .set_display_name(&target, "x")
            .is_err()
    );
}

#[test]
fn an_implicit_stage_is_refused_rather_than_edited_somewhere_else() {
    let mut document = document("jobs-only.yml");
    let stage = parser::parse(&document).unwrap().stages[0].clone();
    let before = snapshot(&document);
    assert!(stage.is_implicit);
    let error = PipelineWriter::new(&mut document)
        .set_display_name(&EditTarget::of_stage(&stage), "Everything")
        .unwrap_err();
    assert!(error.contains("nothing here to edit"), "{error}");
    assert_eq!(before, snapshot(&document));
}

#[test]
fn an_element_from_a_template_is_refused_and_says_where_to_edit_it_instead() {
    let target = EditTarget {
        id: "Build/Compile".to_owned(),
        lines: LineRange::new(0, 1),
        is_implicit: false,
        template: "templates/build-jobs.yml".to_owned(),
    };
    let mut document = LineDocument::parse("jobs:\n  - job: Compile\n");
    let error = PipelineWriter::new(&mut document)
        .set_display_name(&target, "Renamed")
        .unwrap_err();
    assert!(error.contains("templates/build-jobs.yml"), "{error}");
}

#[test]
fn every_editable_stage_in_the_corpus_still_parses_after_a_rename() {
    let names = all_fixtures();
    assert!(names.len() >= 10, "only {} fixtures were found", names.len());
    let mut renamed = 0;
    for name in names {
        let mut document = LineDocument::parse(&std::fs::read_to_string(fixtures().join(&name)).unwrap());
        let Some((index, stage)) = parser::parse(&document)
            .unwrap()
            .stages
            .into_iter()
            .enumerate()
            .find(|(_, s)| EditTarget::of_stage(s).is_editable())
        else {
            continue;
        };
        PipelineWriter::new(&mut document)
            .set_display_name(&EditTarget::of_stage(&stage), "Edited by a test")
            .unwrap();
        assert_eq!(
            "Edited by a test",
            parser::parse(&document).unwrap().stages[index].display_name,
            "{name}"
        );
        renamed += 1;
    }
    assert!(renamed >= 5, "only {renamed} fixtures reached the rename");
}
