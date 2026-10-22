//! The splices of a bundle, a job and a pipeline: only the lines an edit concerns change, and a refusal changes
//! nothing (etalii.adp spec 030, task T081), as the standalone host's `BundleWriter.Tests.cs`, `JobWriter.Tests.cs` and
//! `PipelineWriter.Tests.cs` check them at `8b6ccfc`.

mod common;

use adp_diagram_databricks::lines::LineDocument;
use adp_diagram_databricks::model::{BundleModel, JobModel, PipelineModel};
use adp_diagram_databricks::writer::{self, Written};
use common::{document, model};

fn job(document: &LineDocument) -> JobModel {
    model(document).jobs.remove(0)
}

fn pipeline(document: &LineDocument) -> PipelineModel {
    model(document).pipelines.remove(0)
}

fn on_job(document: &mut LineDocument, f: impl FnOnce(&mut LineDocument, &JobModel) -> Written) -> Written {
    let it = job(document);
    f(document, &it)
}

fn on_pipeline(document: &mut LineDocument, f: impl FnOnce(&mut LineDocument, &PipelineModel) -> Written) -> Written {
    let it = pipeline(document);
    f(document, &it)
}

fn on_bundle(document: &mut LineDocument, f: impl FnOnce(&mut LineDocument, &BundleModel) -> Written) -> Written {
    let it = model(document).bundle;
    f(document, &it)
}

const YAML: &str = "resources:\r\n  pipelines:\r\n    bronze:\r\n      name: bronze\r\n      libraries:\r\n        - \
                    notebook:\r\n            path: transformations/bronze\r\n";

// ---- a job ---------------------------------------------------------------------------------------------------------

#[test]
fn connect_then_disconnect_on_a_task_with_no_dependencies_returns_the_bytes() {
    let mut document = document("job.yml");
    let original = document.text();
    on_job(&mut document, |d, it| writer::connect(d, it, "publish", "ingest", "")).unwrap();
    let midway = job(&document);
    assert_eq!(midway.tasks[0].depends_on[0].task_key, "publish");
    assert!(
        document
            .text()
            .contains("        - task_key: ingest\r\n          depends_on:\r\n            - task_key: publish\r\n")
    );
    writer::disconnect(&mut document, &midway, "publish", "ingest").unwrap();
    assert_eq!(document.text(), original);
}

#[test]
fn connect_appends_after_the_existing_entries_and_only_there() {
    let mut document = document("job.yml");
    let before = document.lines().len();
    on_job(&mut document, |d, it| {
        writer::connect(d, it, "alert", "refresh_dashboard", "")
    })
    .unwrap();
    assert_eq!(document.lines().len(), before + 1);
    let task = job(&document).tasks.remove(4);
    let keys: Vec<&str> = task.depends_on.iter().map(|d| d.task_key.as_str()).collect();
    assert_eq!(keys, ["publish", "alert"]);
}

#[test]
fn connect_with_an_outcome_writes_the_outcome_edge() {
    let mut document = document("job.yml");
    on_job(&mut document, |d, it| {
        writer::connect(d, it, "quality_gate", "refresh_dashboard", "false")
    })
    .unwrap();
    let task = job(&document).tasks.remove(4);
    let edge = task.depends_on.iter().find(|d| d.task_key == "quality_gate").unwrap();
    assert_eq!(edge.outcome, "false");
}

#[test]
fn connect_refusals_happen_before_any_splice() {
    let mut document = document("job.yml");
    let original = document.text();
    let job = job(&document);
    assert!(
        writer::connect(&mut document, &job, "quality_gate", "publish", "")
            .unwrap_err()
            .contains("already depends")
    );
    assert!(
        writer::connect(&mut document, &job, "ingest", "ingest", "")
            .unwrap_err()
            .contains("cannot depend on itself")
    );
    assert!(
        writer::connect(&mut document, &job, "ghost", "ingest", "")
            .unwrap_err()
            .contains("not there")
    );
    assert_eq!(document.text(), original);
}

#[test]
fn remove_task_takes_every_reference_to_it_and_their_emptied_keys() {
    let mut document = document("job.yml");
    let job = job(&document);
    assert_eq!(writer::references_to(&job, "quality_gate").len(), 2);
    writer::remove_task(&mut document, &job, "quality_gate").unwrap();
    let reparsed = self::job(&document);
    assert_eq!(reparsed.tasks.len(), 4);
    assert!(reparsed.tasks.iter().all(|t| t.key != "quality_gate"));
    assert!(
        reparsed
            .tasks
            .iter()
            .find(|t| t.key == "publish")
            .unwrap()
            .depends_on
            .is_empty()
    );
    assert!(
        reparsed
            .tasks
            .iter()
            .find(|t| t.key == "alert")
            .unwrap()
            .depends_on
            .is_empty()
    );
    assert!(!document.text().contains("quality_gate"));
}

#[test]
fn rename_task_rewrites_every_reference_in_one_operation() {
    let mut document = document("job.yml");
    let before = document.lines().len();
    on_job(&mut document, |d, it| {
        writer::rename_task(d, it, "quality_gate", "gate")
    })
    .unwrap();
    assert_eq!(document.lines().len(), before);
    let reparsed = job(&document);
    let publish = &reparsed.tasks[2].depends_on[0];
    assert_eq!((publish.task_key.as_str(), publish.outcome.as_str()), ("gate", "true"));
    let alert = &reparsed.tasks[3].depends_on[0];
    assert_eq!((alert.task_key.as_str(), alert.outcome.as_str()), ("gate", "false"));
    assert!(!document.text().contains("quality_gate"));
}

#[test]
fn rename_task_to_an_existing_key_is_refused() {
    let mut document = document("job.yml");
    let original = document.text();
    assert!(
        on_job(&mut document, |d, it| writer::rename_task(d, it, "ingest", "publish"))
            .unwrap_err()
            .contains("already there")
    );
    assert_eq!(document.text(), original);
}

#[test]
fn insert_task_appends_after_the_last_task_copying_the_file_style() {
    let mut document = document("job.yml");
    let original = document.text();
    on_job(&mut document, |d, it| {
        writer::insert_task(d, it, "cleanup", "notebook", "notebooks/cleanup")
    })
    .unwrap();
    assert!(document.text().starts_with(&original));
    assert!(document.text().ends_with(
        "        - task_key: cleanup\r\n          notebook_task:\r\n            notebook_path: notebooks/cleanup\r\n"
    ));
    let last = job(&document).tasks.pop().unwrap();
    assert_eq!(
        (last.key.as_str(), last.task_type.as_str(), last.source.as_str()),
        ("cleanup", "notebook", "notebooks/cleanup")
    );
}

#[test]
fn insert_task_refusals_cover_duplicates_and_unknown_types() {
    let mut document = document("job.yml");
    let original = document.text();
    let job = job(&document);
    assert!(
        writer::insert_task(&mut document, &job, "ingest", "notebook", "x")
            .unwrap_err()
            .contains("already there")
    );
    assert!(
        writer::insert_task(&mut document, &job, "fresh", "shiny", "x")
            .unwrap_err()
            .contains("no 'shiny' task type")
    );
    assert_eq!(document.text(), original);
}

#[test]
fn set_run_if_adds_rewrites_and_removes_the_one_line() {
    let mut document = document("job.yml");
    let original = document.text();
    on_job(&mut document, |d, it| {
        writer::set_run_if(d, it, "publish", "AT_LEAST_ONE_SUCCESS")
    })
    .unwrap();
    assert_eq!(job(&document).tasks[2].run_if, "AT_LEAST_ONE_SUCCESS");
    on_job(&mut document, |d, it| writer::set_run_if(d, it, "publish", "ALL_DONE")).unwrap();
    assert_eq!(job(&document).tasks[2].run_if, "ALL_DONE");
    on_job(&mut document, |d, it| writer::set_run_if(d, it, "publish", "")).unwrap();
    assert_eq!(document.text(), original);
}

#[test]
fn set_cluster_refuses_a_cluster_the_job_does_not_declare() {
    let mut document = document("job.yml");
    assert!(
        on_job(&mut document, |d, it| writer::set_cluster(d, it, "ingest", "warp_core"))
            .unwrap_err()
            .contains("no cluster named 'warp_core'")
    );
    on_job(&mut document, |d, it| {
        writer::set_cluster(d, it, "publish", "ingest_cluster")
    })
    .unwrap();
    assert_eq!(job(&document).tasks[2].cluster_key, "ingest_cluster");
}

// ---- a pipeline ----------------------------------------------------------------------------------------------------

#[test]
fn insert_library_in_yaml_appends_a_dash_entry() {
    let mut document = LineDocument::parse(YAML);
    on_pipeline(&mut document, |d, it| {
        writer::insert_library(d, it, "file", "transformations/silver.py")
    })
    .unwrap();
    let libraries: Vec<(String, String)> = pipeline(&document)
        .libraries
        .into_iter()
        .map(|l| (l.kind, l.path))
        .collect();
    assert_eq!(
        libraries,
        [
            ("notebook".to_owned(), "transformations/bronze".to_owned()),
            ("file".to_owned(), "transformations/silver.py".to_owned())
        ]
    );
}

#[test]
fn insert_library_in_json_keeps_the_comma_discipline() {
    let mut document = document("pipeline.json");
    on_pipeline(&mut document, |d, it| {
        writer::insert_library(d, it, "notebook", "transformations/platinum")
    })
    .unwrap();
    assert!(document.text().contains(
        "    { \"glob\": { \"include\": \"transformations/gold/**\" } },\r\n    { \"notebook\": { \"path\": \
         \"transformations/platinum\" } }\r\n  ],"
    ));
    let reparsed = pipeline(&document);
    assert_eq!(reparsed.libraries.len(), 4);
}

#[test]
fn remove_library_in_json_of_the_last_entry_strips_the_new_lasts_comma() {
    let mut document = document("pipeline.json");
    on_pipeline(&mut document, |d, it| {
        writer::remove_library(d, it, "glob", "transformations/gold/**")
    })
    .unwrap();
    let reparsed = pipeline(&document);
    assert_eq!(reparsed.libraries.len(), 2);
    assert_eq!(reparsed.libraries[1].kind, "file");
    assert!(
        document
            .text()
            .contains("{ \"file\": { \"path\": \"transformations/silver.py\" } }\r\n  ],")
    );
}

#[test]
fn remove_library_of_the_only_entry_is_refused() {
    let mut document = LineDocument::parse(YAML);
    assert!(
        on_pipeline(&mut document, |d, it| writer::remove_library(
            d,
            it,
            "notebook",
            "transformations/bronze"
        ))
        .unwrap_err()
        .contains("at least one library")
    );
    assert_eq!(document.text(), YAML);
}

#[test]
fn set_scalar_in_json_keeps_quoting_and_the_trailing_comma() {
    let mut document = document("pipeline.json");
    let before = document.lines().len();
    on_pipeline(&mut document, |d, it| {
        writer::set_scalar(d, it, "catalog", "lakehouse_prod")
    })
    .unwrap();
    assert_eq!(document.lines().len(), before);
    assert_eq!(pipeline(&document).catalog, "lakehouse_prod");
    assert!(document.text().contains("\"catalog\": \"lakehouse_prod\",\r\n"));
    on_pipeline(&mut document, |d, it| writer::set_scalar(d, it, "name", "a \"b\"")).unwrap();
    assert!(document.text().contains("\"name\": \"a \\\"b\\\"\",\r\n"));
    assert!(
        on_pipeline(&mut document, |d, it| writer::set_scalar(d, it, "target", "x"))
            .unwrap_err()
            .contains("no \"target\"")
    );
}

#[test]
fn set_scalar_in_yaml_adds_a_missing_key_and_quotes_where_it_must() {
    let mut document = LineDocument::parse(YAML);
    on_pipeline(&mut document, |d, it| {
        writer::set_scalar(d, it, "catalog", "lakehouse_dev")
    })
    .unwrap();
    assert_eq!(pipeline(&document).catalog, "lakehouse_dev");
    assert!(
        document
            .text()
            .contains("    bronze:\r\n      catalog: lakehouse_dev\r\n")
    );
    on_pipeline(&mut document, |d, it| writer::set_scalar(d, it, "schema", "gold #2")).unwrap();
    assert!(document.text().contains("      schema: \"gold #2\"\r\n"));
}

#[test]
fn set_scalar_clearing_the_name_is_refused() {
    let mut document = document("pipeline.json");
    let original = document.text();
    assert!(
        on_pipeline(&mut document, |d, it| writer::set_scalar(d, it, "name", ""))
            .unwrap_err()
            .contains("needs a name")
    );
    assert_eq!(document.text(), original);
}

#[test]
fn insert_library_refusals_cover_kind_path_and_duplicates() {
    let mut document = document("pipeline.json");
    let original = document.text();
    let pipeline = pipeline(&document);
    assert!(
        writer::insert_library(&mut document, &pipeline, "wheel", "x")
            .unwrap_err()
            .contains("no 'wheel' library kind")
    );
    assert!(
        writer::insert_library(&mut document, &pipeline, "file", "")
            .unwrap_err()
            .contains("needs a path")
    );
    assert!(
        writer::insert_library(&mut document, &pipeline, "file", "transformations/silver.py")
            .unwrap_err()
            .contains("already there")
    );
    assert_eq!(document.text(), original);
}

// ---- a bundle ------------------------------------------------------------------------------------------------------

#[test]
fn add_resource_skeleton_lands_after_its_kinds_last_sibling() {
    let mut document = document("bundle.yml");
    on_bundle(&mut document, |d, it| {
        writer::add_resource_skeleton(d, it, "jobs", "weekly_rollup")
    })
    .unwrap();
    let reparsed = model(&document);
    let resources: Vec<(&str, &str)> = reparsed
        .bundle
        .resources
        .iter()
        .map(|r| (r.kind.as_str(), r.key.as_str()))
        .collect();
    assert_eq!(
        resources,
        [
            ("jobs", "nightly_ingest"),
            ("jobs", "weekly_rollup"),
            ("pipelines", "bronze_to_gold")
        ]
    );
    let job = reparsed.jobs.iter().find(|j| j.key == "weekly_rollup").unwrap();
    assert_eq!(job.tasks.len(), 1);
}

#[test]
fn add_resource_skeleton_on_a_file_without_resources_creates_the_section() {
    let mut document = LineDocument::parse("bundle:\r\n  name: fresh\r\n");
    on_bundle(&mut document, |d, it| {
        writer::add_resource_skeleton(d, it, "pipelines", "first")
    })
    .unwrap();
    assert_eq!(
        document.text(),
        "bundle:\r\n  name: fresh\r\nresources:\r\n  pipelines:\r\n    first:\r\n      name: first\r\n      \
         libraries:\r\n        - notebook:\r\n            path: transformations/first\r\n"
    );
    let pipeline = pipeline(&document);
    assert_eq!(pipeline.key, "first");
    assert_eq!(pipeline.libraries.len(), 1);
}

#[test]
fn add_resource_skeleton_refusals_happen_before_any_splice() {
    let mut document = document("bundle.yml");
    let original = document.text();
    let bundle = model(&document).bundle;
    assert!(
        writer::add_resource_skeleton(&mut document, &bundle, "jobs", "nightly_ingest")
            .unwrap_err()
            .contains("already there")
    );
    assert!(
        writer::add_resource_skeleton(&mut document, &bundle, "experiments", "fresh")
            .unwrap_err()
            .contains("no 'experiments' resource kind")
    );
    assert!(
        writer::add_resource_skeleton(&mut document, &bundle, "jobs", "")
            .unwrap_err()
            .contains("needs a key")
    );
    assert_eq!(document.text(), original);
}

#[test]
fn set_name_rewrites_the_one_line_and_quotes_where_it_must() {
    let mut document = document("bundle.yml");
    let before = document.lines().len();
    writer::set_bundle_name(&mut document, "lakehouse-hourly").unwrap();
    assert_eq!(document.lines().len(), before);
    assert_eq!(model(&document).bundle.name, "lakehouse-hourly");
    writer::set_bundle_name(&mut document, "lake: house").unwrap();
    assert!(document.text().contains("bundle:\r\n  name: \"lake: house\"\r\n"));
    assert!(
        writer::set_bundle_name(&mut document, "")
            .unwrap_err()
            .contains("needs a name")
    );
}

#[test]
fn an_inserted_line_takes_the_files_own_ending() {
    for (name, ending) in [
        ("crlf-line-endings.yml", "\r\n"),
        ("lf-line-endings.yml", "\n"),
        ("tied-line-endings.yml", "\r\n"),
    ] {
        let mut document = document(name);
        on_bundle(&mut document, |d, it| {
            writer::add_resource_skeleton(d, it, "jobs", "fresh")
        })
        .unwrap();
        assert!(
            document.text().contains(&format!("    fresh:{ending}")),
            "{name}: {:?}",
            document.text()
        );
    }
}

#[test]
fn an_append_to_a_file_without_a_trailing_newline_keeps_it_without_one() {
    let mut document = document("no-trailing-newline.yml");
    on_bundle(&mut document, |d, it| {
        writer::add_resource_skeleton(d, it, "jobs", "fresh")
    })
    .unwrap();
    assert!(document.text().ends_with("notebook_path: notebooks/fresh"));
}

#[test]
fn quote_leaves_an_ordinary_value_alone() {
    assert_eq!(writer::quote("lakehouse"), "lakehouse");
    assert_eq!(writer::quote(""), "\"\"");
    assert_eq!(writer::quote("a: b"), "\"a: b\"");
    assert_eq!(writer::quote("-x"), "\"-x\"");
    assert_eq!(writer::quote("say \"hi\" #1"), "\"say \\\"hi\\\" #1\"");
    assert_eq!(writer::quote("{x}"), "\"{x}\"");
}
