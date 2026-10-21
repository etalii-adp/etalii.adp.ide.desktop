//! The three readings of a file and the lines each part occupies (etalii.adp spec 030, task T081), as the standalone
//! host's `BundleParser.Tests.cs`, `JobParser.Tests.cs` and `PipelineParser.Tests.cs` read them at `8b6ccfc`.

mod common;

use adp_diagram_databricks::lines::{LineDocument, LineRange};
use adp_diagram_databricks::parser;
use common::{document, model};

#[test]
fn the_bundle_fixture_reads_its_name_includes_and_variables() {
    let bundle = model(&document("bundle.yml")).bundle;
    assert_eq!(bundle.name, "lakehouse-nightly");
    assert_eq!(bundle.includes.len(), 1);
    assert_eq!(bundle.includes[0].glob, "resources/*.yml");
    assert_eq!(bundle.includes[0].lines, LineRange::new(5, 5));
    assert_eq!(bundle.variables.len(), 2);
    assert_eq!(
        (
            bundle.variables[0].name.as_str(),
            bundle.variables[0].default.as_str(),
            bundle.variables[0].description.as_str()
        ),
        ("catalog", "lakehouse_dev", "The Unity Catalog written to.")
    );
    assert_eq!(
        (bundle.variables[1].name.as_str(), bundle.variables[1].default.as_str()),
        ("warehouse_id", "4b9b953939869799")
    );
    assert_eq!(bundle.variables[0].lines, LineRange::new(8, 10));
    assert_eq!(bundle.variables[1].lines, LineRange::new(11, 12));
}

#[test]
fn the_bundle_fixture_reads_its_modelled_resources_with_their_lines() {
    let bundle = model(&document("bundle.yml")).bundle;
    assert_eq!(bundle.resources.len(), 2);
    assert_eq!(
        (bundle.resources[0].kind.as_str(), bundle.resources[0].key.as_str()),
        ("jobs", "nightly_ingest")
    );
    assert_eq!(bundle.resources[0].lines, LineRange::new(20, 21));
    assert_eq!(
        (bundle.resources[1].kind.as_str(), bundle.resources[1].key.as_str()),
        ("pipelines", "bronze_to_gold")
    );
}

#[test]
fn unmodelled_constructs_land_as_unknown_nodes_not_nowhere() {
    let bundle = model(&document("bundle.yml")).bundle;
    let paths: Vec<(&str, LineRange)> = bundle
        .unknown_nodes
        .iter()
        .map(|n| (n.path.as_str(), n.lines))
        .collect();
    assert_eq!(
        paths,
        vec![
            ("sync", LineRange::new(14, 16)),
            ("resources.experiments", LineRange::new(25, 27))
        ]
    );
}

#[test]
fn targets_carry_mode_default_and_overrides() {
    let bundle = model(&document("bundle.yml")).bundle;
    assert_eq!(bundle.targets.len(), 2);
    let dev = &bundle.targets[0];
    assert_eq!(
        (dev.name.as_str(), dev.mode.as_str(), dev.is_default),
        ("dev", "development", true)
    );
    assert!(dev.overrides.is_empty());
    let prod = &bundle.targets[1];
    assert_eq!(
        (prod.name.as_str(), prod.mode.as_str(), prod.is_default),
        ("prod", "production", false)
    );
    assert_eq!(prod.overrides.len(), 1);
    assert_eq!(
        (prod.overrides[0].kind.as_str(), prod.overrides[0].key.as_str()),
        ("jobs", "nightly_ingest")
    );
}

#[test]
fn a_file_that_is_no_bundle_reads_as_the_empty_bundle() {
    let bundle = model(&document("pipeline.json")).bundle;
    assert_eq!(bundle.name, "");
    assert!(bundle.resources.is_empty() && bundle.targets.is_empty());
}

#[test]
fn the_job_fixture_reads_its_identity_schedule_and_cluster() {
    let jobs = model(&document("job.yml")).jobs;
    assert_eq!(jobs.len(), 1);
    let job = &jobs[0];
    assert_eq!(
        (job.key.as_str(), job.name.as_str()),
        ("nightly_ingest", "Nightly ingest")
    );
    assert_eq!(job.schedule, "0 0 1 * * ?");
    assert!(!job.continuous);
    assert_eq!(job.clusters.len(), 1);
    let cluster = &job.clusters[0];
    assert_eq!(
        (
            cluster.key.as_str(),
            cluster.spark_version.as_str(),
            cluster.node_type.as_str(),
            cluster.workers
        ),
        ("ingest_cluster", "15.4.x-scala2.12", "Standard_DS3_v2", 2)
    );
    assert_eq!(cluster.lines, LineRange::new(5, 9));
}

#[test]
fn tasks_carry_their_types_sources_outcomes_and_bindings() {
    let job = model(&document("job.yml")).jobs.remove(0);
    let keys: Vec<&str> = job.tasks.iter().map(|t| t.key.as_str()).collect();
    assert_eq!(
        keys,
        ["ingest", "quality_gate", "publish", "alert", "refresh_dashboard"]
    );
    let types: Vec<&str> = job.tasks.iter().map(|t| t.task_type.as_str()).collect();
    assert_eq!(types, ["notebook", "condition", "notebook", "python", "sql"]);
    assert_eq!(job.tasks[0].source, "notebooks/ingest");
    assert_eq!(job.tasks[3].source, "scripts/alert.py");
    assert_eq!(job.tasks[4].source, "9d3f2b1a");
    let publish = &job.tasks[2].depends_on;
    assert_eq!(
        (publish[0].task_key.as_str(), publish[0].outcome.as_str()),
        ("quality_gate", "true")
    );
    let alert = &job.tasks[3].depends_on;
    assert_eq!(
        (alert[0].task_key.as_str(), alert[0].outcome.as_str()),
        ("quality_gate", "false")
    );
    assert_eq!(job.tasks[3].run_if, "AT_LEAST_ONE_FAILED");
    assert_eq!(job.tasks[2].run_if, "");
    assert_eq!(job.tasks[0].cluster_key, "ingest_cluster");
    assert_eq!(job.tasks[1].cluster_key, "");
    for pair in job.tasks.windows(2) {
        assert!(pair[0].lines.end < pair[1].lines.start, "{pair:?}");
    }
}

#[test]
fn a_bare_settings_json_is_one_pipeline() {
    let pipelines = model(&document("pipeline.json")).pipelines;
    assert_eq!(pipelines.len(), 1);
    let pipeline = &pipelines[0];
    assert_eq!(pipeline.key, "");
    assert_eq!(
        (
            pipeline.name.as_str(),
            pipeline.catalog.as_str(),
            pipeline.schema.as_str(),
            pipeline.channel.as_str()
        ),
        ("bronze_to_gold", "lakehouse_dev", "gold", "CURRENT")
    );
    assert!(pipeline.serverless && !pipeline.continuous && pipeline.development);
    let libraries: Vec<(&str, &str)> = pipeline
        .libraries
        .iter()
        .map(|l| (l.kind.as_str(), l.path.as_str()))
        .collect();
    assert_eq!(
        libraries,
        [
            ("notebook", "transformations/bronze"),
            ("file", "transformations/silver.py"),
            ("glob", "transformations/gold/**")
        ]
    );
    let notification = &pipeline.notifications[0];
    assert_eq!(notification.recipients, ["data-eng@example.com"]);
    assert_eq!(notification.alerts, ["on-update-failure"]);
    assert_eq!(notification.lines, LineRange::new(14, 16));
    assert_eq!(pipeline.unknown_nodes.len(), 1);
    assert_eq!(pipeline.unknown_nodes[0].key, "edition");
    assert_eq!(pipeline.unknown_nodes[0].lines, LineRange::new(19, 19));
}

#[test]
fn a_keyed_declaration_in_a_bundle_reads_with_its_key() {
    let pipelines = model(&document("bundle.yml")).pipelines;
    assert_eq!(pipelines.len(), 1);
    assert_eq!(
        (pipelines[0].key.as_str(), pipelines[0].name.as_str()),
        ("bronze_to_gold", "Bronze to gold")
    );
    assert!(model(&document("job.yml")).pipelines.is_empty());
    assert!(model(&document("pipeline.json")).jobs.is_empty());
}

#[test]
fn a_text_that_is_not_yaml_reads_as_nothing_with_the_line_it_stopped_on() {
    let outcome = parser::outcome(&document("broken.yml"));
    assert!(outcome.unreadable.is_some());
    assert_eq!(outcome.model, Default::default());
    assert_eq!(outcome.problems.len(), 1);
}

#[test]
fn an_empty_file_declares_nothing_and_is_no_error() {
    let outcome = parser::outcome(&LineDocument::parse(""));
    assert!(outcome.unreadable.is_none());
    assert!(outcome.model.jobs.is_empty() && outcome.model.pipelines.is_empty());
}

#[test]
fn every_corpus_file_comes_back_byte_for_byte_untouched() {
    for name in [
        "bundle.yml",
        "job.yml",
        "pipeline.json",
        "crlf-line-endings.yml",
        "lf-line-endings.yml",
        "no-trailing-newline.yml",
        "tied-line-endings.yml",
    ] {
        assert_eq!(document(name).text(), common::text(name), "{name}");
    }
}
