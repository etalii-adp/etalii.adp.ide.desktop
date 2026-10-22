//! Following templates: what each contributes and where, and every reason one is not followed (the standalone host's
//! `PipelineTemplatesTests`).

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use adp_diagram_azure_devops_pipeline::lines::LineDocument;
use adp_diagram_azure_devops_pipeline::model::{PipelineModel, StepKind, UnresolvedReason};
use adp_diagram_azure_devops_pipeline::parser;
use adp_diagram_azure_devops_pipeline::templates::PipelineTemplates;
use common::{fixture, parse_text};

/// A workspace inside a folder of its own, so a path can lead out of it into the folder around it.
struct Workspace {
    outer: tempfile::TempDir,
}

impl Workspace {
    fn new() -> Workspace {
        let outer = tempfile::tempdir().unwrap();
        fs::create_dir_all(outer.path().join("workspace")).unwrap();
        Workspace { outer }
    }

    fn root(&self) -> PathBuf {
        self.outer.path().join("workspace")
    }

    fn write(&self, relative: &str, content: &str) -> PathBuf {
        let full = self.root().join(relative);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(&full, content).unwrap();
        full
    }

    fn copy_fixture(&self, name: &str, relative: &str) {
        self.write(relative, &fixture(name));
    }

    fn parse(&self, relative: &str) -> PipelineModel {
        self.parse_with(&PipelineTemplates::new(&self.root()), &self.root().join(relative))
    }

    fn parse_with(&self, resolver: &PipelineTemplates, path: &Path) -> PipelineModel {
        let document = LineDocument::parse(&fs::read_to_string(path).unwrap());
        parser::parse_with(&document, resolver, path).unwrap()
    }

    fn templates_example(&self) -> PipelineModel {
        self.copy_fixture("templates.yml", "azure-pipelines.yml");
        self.copy_fixture("templates/build-jobs.yml", "templates/build-jobs.yml");
        self.copy_fixture("templates/test-steps.yml", "templates/test-steps.yml");
        self.parse("azure-pipelines.yml")
    }
}

#[test]
fn a_jobs_template_contributes_its_jobs_to_the_stage_that_referenced_it() {
    let model = Workspace::new().templates_example();
    let build = model.stages.iter().find(|s| s.name == "Build").unwrap();
    assert_eq!(1, build.jobs.len());
    let compile = &build.jobs[0];
    assert_eq!("Compile", compile.name);
    assert_eq!("Build/Compile", compile.id);
    assert_eq!("templates/build-jobs.yml", compile.template);
    assert!(compile.is_from_template());
}

#[test]
fn a_stage_that_authored_its_own_jobs_does_not_mark_them_as_template_sourced() {
    let model = Workspace::new().templates_example();
    let unit = model.jobs().find(|j| j.name == "Unit").unwrap();
    assert_eq!("", unit.template);
    assert!(!unit.is_from_template());
}

#[test]
fn a_steps_template_contributes_its_steps_where_it_was_referenced() {
    let model = Workspace::new().templates_example();
    let unit = model.jobs().find(|j| j.name == "Unit").unwrap();
    let kinds: Vec<StepKind> = unit.steps.iter().map(|s| s.kind).collect();
    assert_eq!(vec![StepKind::Template, StepKind::Script], kinds);
    assert_eq!("Test from the template", unit.steps[1].label());
    assert_eq!("templates/test-steps.yml", unit.steps[1].template);
}

#[test]
fn a_steps_templates_contributions_get_their_own_places_in_the_job() {
    let model = Workspace::new().templates_example();
    let ids: Vec<&str> = model.steps().map(|s| s.id.as_str()).collect();
    let distinct: std::collections::HashSet<&str> = ids.iter().copied().collect();
    assert_eq!(ids.len(), distinct.len());
}

#[test]
fn a_template_in_another_repository_is_reported_rather_than_followed() {
    let model = Workspace::new().templates_example();
    assert_eq!(1, model.unresolved.len());
    let unresolved = &model.unresolved[0];
    assert_eq!(UnresolvedReason::OtherRepository, unresolved.reason);
    assert_eq!("templates/deploy-stages.yml", unresolved.reference.path);
    assert!(unresolved.explanation().contains("shared"));
}

#[test]
fn a_template_path_leaving_the_workspace_is_refused() {
    let workspace = Workspace::new();
    workspace.write("azure-pipelines.yml", "stages:\n  - template: ../outside/steal.yml\n");
    let outside = workspace.outer.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("steal.yml"), "stages:\n  - stage: Stolen\n").unwrap();
    let model = workspace.parse("azure-pipelines.yml");
    assert!(model.stages.is_empty());
    assert_eq!(1, model.unresolved.len());
    assert_eq!(UnresolvedReason::OutsideWorkspace, model.unresolved[0].reason);
}

#[test]
fn an_absolute_windows_path_is_refused() {
    let workspace = Workspace::new();
    workspace.write("azure-pipelines.yml", "stages:\n  - template: C:/Windows/win.ini\n");
    let model = workspace.parse("azure-pipelines.yml");
    assert_eq!(1, model.unresolved.len());
    assert_eq!(UnresolvedReason::OutsideWorkspace, model.unresolved[0].reason);
}

#[test]
fn a_rooted_path_resolves_against_the_workspace_root() {
    let workspace = Workspace::new();
    workspace.write(
        "nested/azure-pipelines.yml",
        "stages:\n  - template: /shared/stages.yml\n",
    );
    workspace.write(
        "shared/stages.yml",
        "stages:\n  - stage: FromRoot\n    jobs:\n      - job: A\n        steps:\n          - script: x\n",
    );
    let model = workspace.parse("nested/azure-pipelines.yml");
    assert!(model.unresolved.is_empty(), "{:?}", model.unresolved);
    assert_eq!(1, model.stages.len());
    assert_eq!("FromRoot", model.stages[0].name);
}

#[test]
fn a_template_path_built_from_a_parameter_is_reported_rather_than_guessed_at() {
    let workspace = Workspace::new();
    workspace.write(
        "azure-pipelines.yml",
        "stages:\n  - template: templates/${{ parameters.which }}.yml\n",
    );
    let model = workspace.parse("azure-pipelines.yml");
    assert_eq!(1, model.unresolved.len());
    assert_eq!(UnresolvedReason::ParameterDependent, model.unresolved[0].reason);
    assert!(model.unresolved[0].explanation().contains("parameter"));
}

#[test]
fn a_missing_template_is_reported_as_missing() {
    let workspace = Workspace::new();
    workspace.write("azure-pipelines.yml", "stages:\n  - template: templates/gone.yml\n");
    let model = workspace.parse("azure-pipelines.yml");
    assert_eq!(1, model.unresolved.len());
    assert_eq!(UnresolvedReason::NotFound, model.unresolved[0].reason);
}

#[test]
fn a_template_that_includes_itself_is_stopped_rather_than_followed_forever() {
    let workspace = Workspace::new();
    workspace.write("azure-pipelines.yml", "stages:\n  - template: templates/loop.yml\n");
    workspace.write("templates/loop.yml", "stages:\n  - template: loop.yml\n");
    let model = workspace.parse("azure-pipelines.yml");
    assert!(model.unresolved.iter().any(|u| u.reason == UnresolvedReason::Cyclic));
}

#[test]
fn an_unparseable_template_is_reported_without_taking_the_diagram_down() {
    let workspace = Workspace::new();
    workspace.write(
        "azure-pipelines.yml",
        "stages:\n  - stage: Build\n    jobs:\n      - template: templates/broken.yml\n",
    );
    workspace.write(
        "templates/broken.yml",
        "jobs:\n  - job: A\n   steps:\n      - script: x\n     bad: [indent\n",
    );
    let model = workspace.parse("azure-pipelines.yml");
    assert_eq!(1, model.stages.len());
    assert_eq!("Build", model.stages[0].name);
    assert_eq!(1, model.unresolved.len());
    assert_eq!(UnresolvedReason::Unreadable, model.unresolved[0].reason);
}

#[test]
fn an_extending_file_takes_its_shape_from_the_template() {
    let workspace = Workspace::new();
    workspace.copy_fixture("extends.yml", "azure-pipelines.yml");
    workspace.copy_fixture("templates/pipeline.yml", "templates/pipeline.yml");
    let model = workspace.parse("azure-pipelines.yml");
    let extends = model.extends.as_ref().expect("an extends");
    assert_eq!(
        vec!["buildConfiguration".to_owned(), "deployTo".to_owned()],
        extends.parameter_names
    );
    assert_eq!(1, model.stages.len());
    assert_eq!("Build", model.stages[0].name);
    assert_eq!("templates/pipeline.yml", model.stages[0].template);
    assert!(model.steps().all(|s| s.is_from_template()));
}

#[test]
fn a_template_shared_by_two_pipelines_is_read_once() {
    let workspace = Workspace::new();
    let one = workspace.write("one.yml", "jobs:\n  - template: templates/shared.yml\n");
    let two = workspace.write("two.yml", "jobs:\n  - template: templates/shared.yml\n");
    let shared = workspace.write(
        "templates/shared.yml",
        "jobs:\n  - job: Before\n    steps:\n      - script: x\n",
    );
    let resolver = PipelineTemplates::new(&workspace.root());
    let first = workspace.parse_with(&resolver, &one);
    fs::write(&shared, "jobs:\n  - job: After\n    steps:\n      - script: x\n").unwrap();
    let second = workspace.parse_with(&resolver, &two);
    assert_eq!("Before", first.jobs().next().unwrap().name);
    assert_eq!("Before", second.jobs().next().unwrap().name);
    assert!(second.unresolved.is_empty());
}

#[test]
fn a_template_deleted_from_disk_is_reported_missing_rather_than_served_from_the_cache() {
    let workspace = Workspace::new();
    let one = workspace.write("one.yml", "jobs:\n  - template: templates/shared.yml\n");
    let shared = workspace.write(
        "templates/shared.yml",
        "jobs:\n  - job: Shared\n    steps:\n      - script: x\n",
    );
    let resolver = PipelineTemplates::new(&workspace.root());
    workspace.parse_with(&resolver, &one);
    fs::remove_file(&shared).unwrap();
    let model = workspace.parse_with(&resolver, &one);
    assert_eq!(1, model.unresolved.len());
    assert_eq!(UnresolvedReason::NotFound, model.unresolved[0].reason);
}

#[test]
fn forgetting_what_was_read_makes_the_next_read_go_back_to_disk() {
    let workspace = Workspace::new();
    let path = workspace.write("one.yml", "jobs:\n  - template: templates/shared.yml\n");
    let shared = workspace.write(
        "templates/shared.yml",
        "jobs:\n  - job: Before\n    steps:\n      - script: x\n",
    );
    let resolver = PipelineTemplates::new(&workspace.root());
    workspace.parse_with(&resolver, &path);
    fs::write(&shared, "jobs:\n  - job: After\n    steps:\n      - script: x\n").unwrap();
    resolver.forget();
    let model = workspace.parse_with(&resolver, &path);
    assert_eq!("After", model.jobs().next().unwrap().name);
}

#[test]
fn without_a_resolver_a_template_is_neither_followed_nor_reported_as_unresolved() {
    let model = parse_text("stages:\n  - template: templates/anything.yml\n");
    assert_eq!(1, model.templates.len());
    assert!(model.unresolved.is_empty());
}
