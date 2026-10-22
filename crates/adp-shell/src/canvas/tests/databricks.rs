//! A Databricks job on the canvas with its module (etalii.adp spec 030, task T081): the lakehouse example's job
//! resource, opened through its registration from the parity fixtures, drawn on the module's layered layout with the
//! positions its `layout:` block stores, a task dragged (which writes the registration, never the body), a dependency
//! drawn from a task's side, and both undone through the project's history.

use std::fs;
use std::path::Path;

use adp_scene::Point;
use eframe::egui::{self, PointerButton};

use super::{Harness, texts_of};

fn nightly_ingest() -> Harness {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/fixtures/databricks/lakehouse/resources");
    Harness::open_from(
        &folder,
        &["nightly-ingest.adp", "nightly-ingest.yml"],
        "nightly-ingest.adp",
        "nightly-ingest.yml",
    )
}

#[test]
fn a_job_is_drawn_on_its_layout_and_a_drag_and_a_dependency_are_written_and_undone() {
    let mut harness = nightly_ingest();
    assert!(harness.tab.session.module_view().is_some());
    harness.settle();

    let texts = texts_of(harness.tab.scene().expect("a scene"));
    for label in [
        "land_raw",
        "quality_gate",
        "alert_on_empty",
        "ingest_cluster",
        "AT_LEAST_ONE_FAILED · serverless",
        "15.4.x-scala2.12 · Standard_DS3_v2 · 2 workers",
        "true",
        "false",
    ] {
        assert!(texts.contains(&label), "{label} is drawn: {texts:?}");
    }
    // The stored position overlays the computed layout; the rest sit in the columns of what they wait for.
    let land = harness.tab.bounds_of("task:land_raw").expect("land_raw is drawn");
    assert_eq!(
        (land.x, land.y, land.width, land.height),
        (-24.539, 93.149, 200.0, 56.0)
    );
    let gate = harness.tab.bounds_of("task:quality_gate").unwrap();
    let publish = harness.tab.bounds_of("task:publish").unwrap();
    assert!(gate.right() < publish.x, "{gate:?} {publish:?}");

    // A drag writes the box's top-left into the registration's layout block and leaves the body alone.
    let registration = harness.root.join("nightly-ingest.adp");
    let stored = fs::read_to_string(&registration).unwrap();
    let from = harness.screen_of("task:publish");
    harness.drag(from, from + egui::vec2(40.0, 30.0), PointerButton::Primary, true);
    harness.settle();
    assert_eq!(harness.written(), harness.original);
    let moved = fs::read_to_string(&registration).unwrap();
    assert!(moved.contains("  task:publish: "), "{moved}");

    // A dependency drawn from the right side of a task to another task makes it wait for the first.
    let land = harness.tab.bounds_of("task:land_raw").unwrap();
    let side = harness
        .tab
        .view
        .to_screen(harness.tab.surface, Point::new(land.right(), land.centre().y));
    let to = harness.screen_of("task:refresh_dashboard");
    harness.drag(side, to, PointerButton::Primary, true);
    harness.settle();
    assert!(
        harness
            .written()
            .contains("            - task_key: publish\n            - task_key: land_raw\n"),
        "{}",
        harness.written()
    );

    // Two undos take the dependency and the drag back, byte for byte.
    for _ in 0..2 {
        assert!(harness.workspace().undo().is_done());
    }
    assert_eq!(harness.written(), harness.original);
    // The headers keep their own endings; the block, written anew, is in CRLF (`AdpFileWriter.NewLine`).
    assert_eq!(
        stored,
        "databricks/job\nbody: nightly-ingest.yml\nlayout:\n  cluster:ingest_cluster: -20.533 261.907\n  task:land_raw: -24.539 93.149\n"
    );
    assert_eq!(
        fs::read_to_string(&registration).unwrap(),
        "databricks/job\nbody: nightly-ingest.yml\nlayout:\r\n  cluster:ingest_cluster: -20.533 261.907\r\n  \
         task:land_raw: -24.539 93.149\r\n"
    );
    harness.tab.changed();
    harness.settle();

    harness.screenshot(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/screenshots/databricks-job.png"));
}
