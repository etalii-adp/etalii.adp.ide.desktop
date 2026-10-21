//! A view report returns promptly on the document a wedged client was looking at (etalii.adp spec 030, task T061).
//!
//! Ported from the standalone host's
//! `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/C4Session.UpdateViewOnTheRealCorpus.Tests.cs` at `8b6ccfc`:
//! the industrial plant example's `bottling-mes.plant-landscape.adp`, copied out of the examples so nothing beside them
//! is written, opened, its baseline taken, and two viewport reports timed on this thread against a patience no
//! ordinary machine comes near. The baseline is asserted not empty first, so a document that failed to parse cannot
//! pass by answering instantly.

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use adp_core::Catalogue;
use adp_diagram::{DiagramDelta, DiagramTools, DiagramViewport, DiagramWorkspace};
use adp_disl::DislPluginFunctions;

const PATIENCE: Duration = Duration::from_secs(30);

#[test]
fn update_view_on_the_document_that_wedged_a_client_returns() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/examples/industrial-plant/architecture");
    let folder = tempfile::tempdir().unwrap();
    for entry in fs::read_dir(&source).unwrap().flatten() {
        if entry.path().is_file() {
            fs::copy(entry.path(), folder.path().join(entry.file_name())).unwrap();
        }
    }
    let registration = folder.path().join("bottling-mes.plant-landscape.adp");
    assert!(
        registration.is_file(),
        "the registration the wedged client opened is missing"
    );

    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    let mut tools = adp_diagram_c4::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
        DislPluginFunctions::none()
    }));
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (_, mut session) = workspace.open(&mut tools, &registration).unwrap();
    let baseline = session.baseline(workspace.documents()).unwrap();
    let delivered: usize = baseline
        .iter()
        .map(|d| match d {
            DiagramDelta::Add(elements) => elements.len(),
            _ => 0,
        })
        .sum();
    assert!(delivered > 1, "the corpus is loaded");

    let clock = Instant::now();
    session
        .update_view(workspace.documents(), DiagramViewport::new(0.0, 0.0, 4000.0, 3000.0))
        .unwrap();
    let first = clock.elapsed();
    let clock = Instant::now();
    session
        .update_view(
            workspace.documents(),
            DiagramViewport::new(500.0, 500.0, 1500.0, 1200.0),
        )
        .unwrap();
    let second = clock.elapsed();
    assert!(
        first < PATIENCE && second < PATIENCE,
        "UpdateView took {first:?} then {second:?} against {PATIENCE:?} of patience"
    );
}
