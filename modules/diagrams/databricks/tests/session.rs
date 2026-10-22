//! An open Databricks diagram: what each of the three readings draws of the lakehouse example, the menus, prompts and
//! rows each element offers, edits written to the body and undone byte for byte, and a drag stored in the
//! registration's `layout:` block and never in the body (etalii.adp spec 030, task T081; the standalone host's
//! `DatabricksSession.Tests.cs`, `DatabricksContextActionProvider.Tests.cs`, `DatabricksContextPropertyProvider.Tests.cs`
//! and `LakehouseExample.Tests.cs` at `8b6ccfc`).

mod common;

use std::fs;
use std::path::Path;

use adp_context::Prompt;
use adp_diagram::{ActionOutcome, ActionTarget, DiagramElement, ElementKind};
use adp_diagram_databricks::actions::{ADD_TASK_PREFIX, CONNECT, REMOVE_TASK, RENAME_TASK, SIMULATED_RUN};
use adp_diagram_databricks::mapper::{OVERRIDE_EDGE_TYPE, TARGET_TYPE, TASK_TYPE};
use adp_diagram_databricks::view::{NOT_MOVABLE, PARENT_REFUSAL};
use adp_diagram_databricks::{JOB_ORIGIN, NOT_REGISTERED, empty_document, key_of, resource_header};
use common::Lakehouse;

/// No ids: no action of this family makes an element whose id is not its key.
struct NoIds;

impl adp_disl::IdSource for NoIds {
    fn next(&self, type_name: &str) -> Result<String, String> {
        Err(format!("no id for a {type_name}"))
    }
}

const JOB: &str = "resources/nightly-ingest.adp";
const JOB_BODY: &str = "resources/nightly-ingest.yml";

fn element<'a>(elements: &'a [DiagramElement], id: &str) -> &'a DiagramElement {
    elements
        .iter()
        .find(|e| e.id == id)
        .unwrap_or_else(|| panic!("{id} is drawn"))
}

fn element_id(id: &str) -> ActionTarget {
    ActionTarget::Element(id.to_owned())
}

#[test]
fn the_bundle_reading_draws_the_bundle_its_resources_targets_and_overrides() {
    let mut lakehouse = Lakehouse::new();
    let session = lakehouse.open("databricks.adp");
    let elements = session.render(lakehouse.workspace.documents()).unwrap();
    let ids: Vec<&str> = elements.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "bundle",
            "resource:pipelines/bronze_to_gold",
            "target:dev",
            "target:prod",
            "override:prod/pipelines/bronze_to_gold",
            "override:prod/jobs/nightly_ingest"
        ]
    );
    // The positions the registration's layout block stores overlay the computed bands.
    let bundle = element(&elements, "bundle");
    assert_eq!((bundle.x, bundle.y, bundle.placed), (-68.665, 289.917, true));
    assert_eq!(bundle.attribute("label"), "lakehouse");
    let dev = element(&elements, "target:dev");
    assert_eq!((dev.x, dev.y, dev.placed), (0.0, 400.0, false));
    assert_eq!(dev.element_type, TARGET_TYPE);
    assert_eq!(
        (dev.attribute("mode"), dev.attribute("isDefault")),
        ("development", "true")
    );
    let stray = element(&elements, "override:prod/jobs/nightly_ingest");
    assert_eq!(stray.element_type, OVERRIDE_EDGE_TYPE);
    assert_eq!(stray.kind, ElementKind::Relation);
    assert_eq!(stray.target.as_deref(), Some("resource:jobs/nightly_ingest"));
}

#[test]
fn the_pipeline_reading_shows_the_declaration_its_resource_header_names() {
    let mut lakehouse = Lakehouse::new();
    let session = lakehouse.open("databricks.pipeline.adp");
    let elements = session.render(lakehouse.workspace.documents()).unwrap();
    let pipeline = element(&elements, "pipeline");
    assert_eq!(pipeline.attribute("label"), "Bronze to gold");
    assert_eq!(pipeline.attribute("badges"), "serverless\nCURRENT");
    assert_eq!((pipeline.x, pipeline.y), (320.0, 120.0));
    assert_eq!(element(&elements, "target").attribute("label"), "${var.catalog}.gold");
    assert_eq!(
        element(&elements, "notifications").attribute("label"),
        "data-eng@vrenken.eu"
    );
    assert_eq!(
        element(&elements, "library:transformations/gold/**").attribute("badges"),
        "glob"
    );
    assert!(elements.iter().any(|e| e.id == "flow:pipeline->target"));
}

#[test]
fn the_job_reading_draws_its_tasks_outcome_edges_and_cluster() {
    let mut lakehouse = Lakehouse::new();
    let session = lakehouse.open(JOB);
    let elements = session.render(lakehouse.workspace.documents()).unwrap();
    let land = element(&elements, "task:land_raw");
    assert_eq!((land.x, land.y, land.placed), (-24.539, 93.149, true));
    assert_eq!(land.element_type, TASK_TYPE);
    assert_eq!(land.attribute("badges"), "ingest_cluster");
    let alert = element(&elements, "task:alert_on_empty");
    assert_eq!(alert.attribute("badges"), "AT_LEAST_ONE_FAILED\nserverless");
    assert_eq!(alert.attribute("kind"), "python");
    assert_eq!((alert.x, alert.y), (520.0, 120.0));
    let edge = element(&elements, "edge:quality_gate->publish");
    assert_eq!(edge.attribute("outcome"), "true");
    assert_eq!(edge.source.as_deref(), Some("task:quality_gate"));
    assert_eq!(
        element(&elements, "cluster:ingest_cluster").attribute("badges"),
        "15.4.x-scala2.12\nStandard_DS3_v2\n2 workers"
    );
    // A selection reads as the task's key, an edge as its two ends.
    let documents = lakehouse.workspace.documents();
    assert_eq!(session.select(documents, "task:publish").unwrap().text, "publish");
    assert_eq!(
        session.select(documents, "edge:land_raw->quality_gate").unwrap().text,
        "land_raw → quality_gate"
    );
    assert!(session.select(documents, "task:nowhere").is_none());
}

#[test]
fn a_rename_asks_first_and_rewrites_every_reference_and_an_undo_puts_the_bytes_back() {
    let mut lakehouse = Lakehouse::new();
    let session = lakehouse.open(JOB);
    let original = lakehouse.read(JOB_BODY);
    let target = element_id("task:quality_gate");
    let ids = NoIds;
    match session.execute(&mut lakehouse.workspace, &target, RENAME_TASK, &ids) {
        ActionOutcome::Prompt(Prompt::Input { initial_value, .. }) => assert_eq!(initial_value, "quality_gate"),
        other => panic!("{other:?}"),
    }
    assert_eq!(
        session.answer(&mut lakehouse.workspace, &target, RENAME_TASK, " gate "),
        ActionOutcome::Done
    );
    let written = lakehouse.read(JOB_BODY);
    assert!(!written.contains("quality_gate"));
    assert_eq!(written.matches("task_key: gate").count(), 3);
    assert!(lakehouse.workspace.undo().is_done());
    assert_eq!(lakehouse.read(JOB_BODY), original);
    assert!(lakehouse.workspace.redo().is_done());
    assert_eq!(lakehouse.read(JOB_BODY), written);
}

#[test]
fn removing_a_task_with_dependencies_asks_to_confirm_and_one_without_does_not() {
    let mut lakehouse = Lakehouse::new();
    let session = lakehouse.open(JOB);
    let ids = NoIds;
    let gate = element_id("task:quality_gate");
    match session.execute(&mut lakehouse.workspace, &gate, REMOVE_TASK, &ids) {
        ActionOutcome::Prompt(Prompt::Confirm { message, .. }) => assert_eq!(
            message,
            "Removing this task also removes the 3 dependencies touching it."
        ),
        other => panic!("{other:?}"),
    }
    assert_eq!(
        session.answer(&mut lakehouse.workspace, &gate, REMOVE_TASK, ""),
        ActionOutcome::Done
    );
    assert!(!lakehouse.read(JOB_BODY).contains("quality_gate"));
    // A dropped task has no edges, so it goes at once.
    let drop = ActionTarget::Canvas { x: 600.0, y: 400.0 };
    let add = format!("{ADD_TASK_PREFIX}python");
    assert_eq!(
        session.execute(&mut lakehouse.workspace, &drop, &add, &ids),
        ActionOutcome::Done
    );
    assert!(lakehouse.read(JOB_BODY).ends_with(
        "        - task_key: python_1\n          spark_python_task:\n            python_file: scripts/python_1.py\n"
    ));
    assert_eq!(
        session.execute(
            &mut lakehouse.workspace,
            &element_id("task:python_1"),
            REMOVE_TASK,
            &ids
        ),
        ActionOutcome::Done
    );
    assert!(!lakehouse.read(JOB_BODY).contains("python_1"));
}

#[test]
fn a_connect_gesture_adds_the_dependency_and_a_second_is_refused() {
    let mut lakehouse = Lakehouse::new();
    let session = lakehouse.open(JOB);
    let ids = NoIds;
    let documents = lakehouse.workspace.documents();
    let view = session.module_view().unwrap().clone();
    assert_eq!(
        view.connect_action(documents, "task:land_raw", "task:refresh_dashboard"),
        Some(Ok(CONNECT.to_owned()))
    );
    assert!(matches!(
        view.connect_action(documents, "cluster:ingest_cluster", "task:publish"),
        Some(Err(_))
    ));
    let gesture = ActionTarget::Connection {
        source: Some("task:land_raw".to_owned()),
        target: Some("task:refresh_dashboard".to_owned()),
    };
    assert_eq!(
        session.execute(&mut lakehouse.workspace, &gesture, CONNECT, &ids),
        ActionOutcome::Done
    );
    assert!(
        lakehouse
            .read(JOB_BODY)
            .contains("            - task_key: publish\n            - task_key: land_raw\n          sql_task:\n")
    );
    assert!(matches!(
        session.execute(&mut lakehouse.workspace, &gesture, CONNECT, &ids),
        ActionOutcome::Refused(reason) if reason.contains("already depends")
    ));
}

#[test]
fn a_drag_is_stored_in_the_registration_never_in_the_body_and_undone_byte_for_byte() {
    let mut lakehouse = Lakehouse::new();
    let session = lakehouse.open(JOB);
    let (body, registration) = (lakehouse.read(JOB_BODY), lakehouse.read(JOB));
    session
        .move_elements(&mut lakehouse.workspace, &[("task:publish".to_owned(), 12.5, -30.0)])
        .unwrap();
    assert_eq!(lakehouse.read(JOB_BODY), body);
    assert_eq!(
        lakehouse.read(JOB),
        "databricks/job\nbody: nightly-ingest.yml\nlayout:\r\n  cluster:ingest_cluster: -20.533 261.907\r\n  \
         task:land_raw: -24.539 93.149\r\n  task:publish: 12.5 -30\r\n"
    );
    let publish = session
        .render(lakehouse.workspace.documents())
        .unwrap()
        .into_iter()
        .find(|e| e.id == "task:publish")
        .unwrap();
    assert_eq!((publish.x, publish.y, publish.placed), (12.5, -30.0, true));
    // A second drag of the same box undoes to the first position, not to nothing.
    session
        .move_elements(&mut lakehouse.workspace, &[("task:publish".to_owned(), 40.0, 40.0)])
        .unwrap();
    assert!(lakehouse.workspace.undo().is_done());
    assert!(lakehouse.read(JOB).contains("  task:publish: 12.5 -30\r\n"));
    assert!(lakehouse.workspace.undo().is_done());
    // The block is rewritten in the registration's own conventions only where it changes: the old entries keep their
    // numbers, and the endings of the lines the block replaced become CRLF.
    assert_eq!(
        lakehouse.read(JOB).replace("\r\n", "\n"),
        registration.replace("\r\n", "\n")
    );
    assert_eq!(
        session.move_elements(
            &mut lakehouse.workspace,
            &[("edge:land_raw->quality_gate".to_owned(), 0.0, 0.0)]
        ),
        Err(NOT_MOVABLE.to_owned())
    );
    assert_eq!(
        session.move_element(&mut lakehouse.workspace, "task:publish", Some("task:land_raw"), -1),
        Err(PARENT_REFUSAL.to_owned())
    );
}

#[test]
fn a_property_row_edits_the_file_and_a_read_only_one_refuses_with_its_reason() {
    let mut lakehouse = Lakehouse::new();
    let session = lakehouse.open(JOB);
    let rows = session.properties(lakehouse.workspace.documents(), &["task:alert_on_empty"]);
    let labels: Vec<&str> = rows.iter().map(|r| r.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Key",
            "Type",
            "Source",
            "Run if",
            "Cluster",
            "Workspace connection",
            "Last simulated run"
        ]
    );
    session
        .set_property(
            &mut lakehouse.workspace,
            &["task:alert_on_empty"],
            "databricks.run-if",
            "",
        )
        .unwrap();
    assert!(!lakehouse.read(JOB_BODY).contains("run_if"));
    assert_eq!(
        session.set_property(&mut lakehouse.workspace, &["task:publish"], "databricks.task-key", "x"),
        Err("Rename through the context menu, so every depends_on reference follows the key.".to_owned())
    );
    assert!(
        session
            .set_property(
                &mut lakehouse.workspace,
                &["task:publish"],
                "databricks.cluster",
                "warp_core"
            )
            .unwrap_err()
            .contains("no cluster named 'warp_core'")
    );
}

#[test]
fn a_simulated_action_completes_and_touches_nothing() {
    let mut lakehouse = Lakehouse::new();
    let session = lakehouse.open(JOB);
    let body = lakehouse.read(JOB_BODY);
    let ids = NoIds;
    assert_eq!(
        session.execute(
            &mut lakehouse.workspace,
            &element_id("task:publish"),
            SIMULATED_RUN,
            &ids
        ),
        ActionOutcome::Done
    );
    assert_eq!(lakehouse.read(JOB_BODY), body);
    assert!(!lakehouse.workspace.undo().is_done());
}

#[test]
fn two_readings_of_one_file_share_its_document() {
    let mut lakehouse = Lakehouse::new();
    let bundle = lakehouse.open("databricks.adp");
    let pipeline = lakehouse.open("databricks.pipeline.adp");
    pipeline
        .set_property(
            &mut lakehouse.workspace,
            &["pipeline"],
            "databricks.pipeline:name",
            "Bronze: gold",
        )
        .unwrap();
    assert!(
        lakehouse
            .read("databricks.yml")
            .contains("      name: \"Bronze: gold\"\n")
    );
    // The bundle view reads the same document, which now holds the new name.
    bundle
        .set_property(&mut lakehouse.workspace, &["bundle"], "databricks.bundle-name", "core")
        .unwrap();
    let text = lakehouse.read("databricks.yml");
    assert!(text.contains("  name: core\n") && text.contains("      name: \"Bronze: gold\"\n"));
}

#[test]
fn a_body_that_does_not_parse_withholds_every_edit() {
    let mut lakehouse = Lakehouse::new();
    fs::write(lakehouse.folder.path().join(JOB_BODY), "resources: [unclosed\n").unwrap();
    let session = lakehouse.open(JOB);
    let documents = lakehouse.workspace.documents();
    assert!(session.render(documents).unwrap().is_empty());
    let view = session.module_view().unwrap().clone();
    assert!(view.actions(documents, &element_id("task:publish")).is_empty());
    assert_eq!(view.problems(documents).len(), 1);
}

#[test]
fn a_yml_no_registration_names_is_not_claimed() {
    let mut lakehouse = Lakehouse::new();
    fs::remove_file(lakehouse.folder.path().join(JOB)).unwrap();
    let opened = lakehouse.workspace.open(&mut lakehouse.tools, Path::new(JOB_BODY));
    assert!(matches!(opened, Err(reason) if reason == NOT_REGISTERED || reason.contains("cannot be opened")));
}

#[test]
fn a_new_diagram_starts_from_a_body_that_opens_reads_and_judges_clean() {
    assert_eq!(key_of("Nightly Ingest.job"), "nightly_ingest_job");
    assert_eq!(key_of("--"), "untitled");
    assert_eq!(
        empty_document(JOB_ORIGIN, "Nightly Ingest"),
        "resources:\r\n  jobs:\r\n    nightly_ingest:\r\n      name: Nightly Ingest\r\n      tasks:\r\n        - task_key: \
         main\r\n          notebook_task:\r\n            notebook_path: notebooks/nightly_ingest\r\n"
    );
    assert_eq!(
        empty_document("databricks/pipeline", "gold"),
        "{\r\n  \"name\": \"gold\",\r\n  \"libraries\": [\r\n    { \"notebook\": { \"path\": \"transformations/gold\" } \
         }\r\n  ]\r\n}\r\n"
    );
    assert_eq!(
        empty_document("databricks/bundle", "Lake"),
        "bundle:\r\n  name: lake\r\ntargets:\r\n  dev:\r\n    mode: development\r\n    default: true\r\n"
    );
    for origin in adp_diagram_databricks::ORIGINS {
        assert!(adp_diagram_databricks::rules::validate(&empty_document(origin, "fresh")).is_empty());
    }
    assert_eq!(
        resource_header("databricks/pipeline\r\nbody: databricks.yml\r\nresource: bronze_to_gold\r\n").as_deref(),
        Some("bronze_to_gold")
    );
    assert_eq!(resource_header("databricks/job\nlayout:\n  resource: x\n"), None);
}
