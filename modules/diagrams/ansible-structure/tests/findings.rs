//! The findings the definition derives, in the tool's order and words (etalii.adp spec 030, task T079): the
//! standalone host's `AnsibleFindings.Tests.cs` and `AnsibleRuleSet.Tests.cs` at `8b6ccfc`, with the same
//! expectations, the rules judged by the vendored definition as there.

mod common;

use adp_diagram::{DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity};
use adp_diagram_ansible_structure::definition::AnsibleDefinition;
use adp_diagram_ansible_structure::findings;
use adp_disl::{DislDiagram, DislFinding};
use common::model;

fn judge(fixture: &str) -> Vec<DiagramProblem> {
    let model = model(fixture);
    findings::problems(&model.diagram, &AnsibleDefinition::get().findings(&model))
}

fn of(rule: &str) -> Vec<DiagramProblem> {
    judge("broken").into_iter().filter(|p| p.rule_id == rule).collect()
}

fn location(problem: &DiagramProblem) -> (String, u32) {
    match &problem.location {
        Some(DiagramProblemLocation::File { relative_path, line }) => (relative_path.clone(), *line),
        other => panic!("not at a file: {other:?}"),
    }
}

fn finding(
    code: &str,
    constraint: &str,
    severity: &str,
    ids: &[String],
    line: Option<i32>,
    file: Option<&str>,
) -> DislFinding {
    DislFinding {
        code: code.to_owned(),
        constraint_id: constraint.to_owned(),
        severity: severity.to_owned(),
        message: code.to_owned(),
        element_ids: ids.to_vec(),
        line,
        file: file.map(str::to_owned),
        subject: None,
    }
}

fn broken() -> DislDiagram {
    model("broken").diagram
}

// ---- the order and the words -------------------------------------------------------------------------------------------

#[test]
fn findings_about_relations_are_in_relation_order_whichever_rule_raised_them() {
    let diagram = broken();
    let relations = diagram.relations();
    let (first, second) = (relations[0].id(), relations[1].id());
    let problems = findings::problems(
        &diagram,
        &[
            finding(
                "b.second",
                "b.second",
                "error",
                &[second],
                Some(3),
                Some("playbooks/deploy.yml"),
            ),
            finding(
                "a.first",
                "a.first",
                "error",
                &[first],
                Some(3),
                Some("playbooks/deploy.yml"),
            ),
        ],
    );
    let rules: Vec<&str> = problems.iter().map(|p| p.rule_id.as_str()).collect();
    assert_eq!(rules, ["a.first", "b.second"]);
}

#[test]
fn a_finding_about_a_node_keeps_its_place_between_runs_about_relations() {
    let diagram = broken();
    let relations = diagram.relations();
    let node = diagram.nodes()[0].id();
    let problems = findings::problems(
        &diagram,
        &[
            finding("one", "one", "error", &[relations[1].id()], Some(3), Some("x")),
            finding("two", "two", "error", &[node], Some(3), Some("x")),
            finding("three", "three", "error", &[relations[0].id()], Some(3), Some("x")),
        ],
    );
    let rules: Vec<&str> = problems.iter().map(|p| p.rule_id.as_str()).collect();
    assert_eq!(rules, ["one", "two", "three"]);
}

#[test]
fn a_file_that_does_not_parse_is_named_in_its_sentence_at_its_line() {
    let mut unparsed = finding(
        "ansible.unreadable-yaml",
        "std.unparseable",
        "error",
        &[],
        Some(3),
        Some("repeated.yml"),
    );
    unparsed.message = "Duplicate key hosts".into();
    let problems = findings::problems(&broken(), &[unparsed]);
    let [problem] = problems.as_slice() else {
        panic!("one problem")
    };
    assert_eq!(
        problem.message,
        "'repeated.yml' is not readable YAML: Duplicate key hosts"
    );
    assert_eq!(problem.severity, DiagramProblemSeverity::Error);
    assert_eq!(problem.rule_id, "ansible.unreadable-yaml");
    assert_eq!(location(problem), ("repeated.yml".into(), 3));
}

#[test]
fn a_file_that_does_not_parse_but_names_no_file_is_not_passed_on() {
    let unparsed = finding("ansible.unreadable-yaml", "std.unparseable", "error", &[], None, None);
    assert!(findings::problems(&broken(), &[unparsed]).is_empty());
}

#[test]
fn a_finding_without_a_line_is_at_its_file_at_line_zero() {
    let diagram = broken();
    let empty = finding(
        "ansible.empty-role",
        "emptyRole",
        "warning",
        &[diagram.nodes()[0].id()],
        None,
        Some("roles/hollow"),
    );
    let problems = findings::problems(&diagram, &[empty]);
    let [problem] = problems.as_slice() else {
        panic!("one problem")
    };
    assert_eq!(problem.severity, DiagramProblemSeverity::Warning);
    assert_eq!(location(problem), ("roles/hollow".into(), 0));
}

#[test]
fn a_location_is_rebased_onto_the_project_when_the_folder_is_not_its_root() {
    let problems = vec![DiagramProblem {
        severity: DiagramProblemSeverity::Error,
        message: String::new(),
        rule_id: String::new(),
        location: Some(DiagramProblemLocation::File {
            relative_path: "site.yml".into(),
            line: 2,
        }),
    }];
    let root = std::path::Path::new("/projects/infra");
    let rebased = findings::rebased(problems, root, &root.join("ansible").join("prod"));
    assert_eq!(location(&rebased[0]), ("ansible/prod/site.yml".into(), 2));
}

// ---- the rules ---------------------------------------------------------------------------------------------------------

#[test]
fn the_best_practice_project_validates_clean() {
    assert_eq!(judge("infrastructure"), Vec::new());
}

#[test]
fn an_unrecognised_layout_produces_nothing_at_all() {
    assert_eq!(judge("unconventional"), Vec::new());
}

#[test]
fn a_root_ini_inventory_resolves_its_plays_hosts() {
    assert_eq!(judge("ini-inventory"), Vec::new());
}

#[test]
fn a_role_with_no_folder_is_an_error_at_the_playbook_that_named_it() {
    let problems = of("ansible.role-missing");
    let from_playbook: Vec<&DiagramProblem> = problems
        .iter()
        .filter(|p| p.message.contains("'absent-role'"))
        .collect();
    assert_eq!(from_playbook.len(), 1);
    assert_eq!(from_playbook[0].severity, DiagramProblemSeverity::Error);
    let (path, line) = location(from_playbook[0]);
    assert_eq!(path, "playbooks/deploy.yml");
    assert!(line > 0);
}

#[test]
fn a_role_missing_from_a_meta_dependency_is_the_same_rule_at_the_meta_file() {
    let problems = of("ansible.role-missing");
    let from_meta = problems.iter().find(|p| p.message.contains("'base'")).unwrap();
    assert!(from_meta.message.to_lowercase().contains("dependency"));
    assert_eq!(location(from_meta).0, "roles/web/meta/main.yml");
}

#[test]
fn a_dangling_import_playbook_is_an_error() {
    let problems = of("ansible.dangling-import");
    let import = problems
        .iter()
        .find(|p| p.message.contains("does-not-exist.yml"))
        .unwrap();
    assert_eq!(import.severity, DiagramProblemSeverity::Error);
    assert!(import.message.contains("imported"));
    assert_eq!(location(import).0, "site.yml");
}

#[test]
fn a_dangling_include_tasks_is_the_same_rule_at_the_task_file() {
    let problems = of("ansible.dangling-import");
    let include = problems.iter().find(|p| p.message.contains("'tls.yml'")).unwrap();
    assert!(include.message.contains("included"));
    assert_eq!(location(include).0, "roles/web/tasks/main.yml");
}

#[test]
fn a_hosts_pattern_no_inventory_defines_is_a_warning() {
    let problems = of("ansible.unmatched-hosts");
    let [unmatched] = problems.as_slice() else {
        panic!("one: {problems:?}")
    };
    assert_eq!(unmatched.severity, DiagramProblemSeverity::Warning);
    assert!(unmatched.message.contains("'cache'"));
    assert_eq!(location(unmatched).0, "playbooks/deploy.yml");
}

#[test]
fn an_unparsable_file_is_an_error_in_the_parsers_own_words() {
    let problems = of("ansible.unreadable-yaml");
    let [unreadable] = problems.as_slice() else {
        panic!("one: {problems:?}")
    };
    assert_eq!(unreadable.severity, DiagramProblemSeverity::Error);
    let (path, line) = location(unreadable);
    assert_eq!(path, "unparsable.yml");
    assert!(line > 0);
    assert!(!unreadable.message.contains("Line:"));
}

#[test]
fn an_empty_role_is_a_warning_at_the_role_folder() {
    let problems = of("ansible.empty-role");
    let [hollow] = problems.as_slice() else {
        panic!("one: {problems:?}")
    };
    assert_eq!(hollow.severity, DiagramProblemSeverity::Warning);
    assert!(hollow.message.contains("'hollow'"));
    assert_eq!(location(hollow), ("roles/hollow".into(), 0));
}

#[test]
fn an_expression_target_is_never_reported_as_missing() {
    let problems = judge("broken");
    assert!(
        !problems
            .iter()
            .any(|p| p.message.contains("{{") || p.message.contains("role_name"))
    );
}

#[test]
fn the_broken_fixture_fires_each_rule_and_nothing_else() {
    let mut rules: Vec<String> = judge("broken").into_iter().map(|p| p.rule_id).collect();
    rules.sort();
    rules.dedup();
    assert_eq!(
        rules,
        [
            "ansible.dangling-import",
            "ansible.empty-role",
            "ansible.role-missing",
            "ansible.unmatched-hosts",
            "ansible.unreadable-yaml",
        ]
    );
}
