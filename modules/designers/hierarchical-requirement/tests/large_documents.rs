//! SC-005 (etalii.adp spec 036; the standalone host's `LargeDocument.Tests.cs` and `NistControls.Tests.cs` at a7fdf03,
//! T056): a document of 100 items and 500 requirements opens, and shows a change another program made, each within two
//! seconds; the NIST SP 800-53 example, more than four times that size, reads as its readme says, without a finding,
//! opens and shows a change within four seconds, and an edit of one title writes only that line.
//!
//! The bounds are held in a test build, which is slower than the build a person runs, so a reading that grows faster
//! than the document fails them; `cargo bench -p adp-shell --bench hierarchy` prints the times of a release build.

mod common;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use adp_designer_hierarchical_requirement::model::{Document, Entry};
use adp_designer_hierarchical_requirement::operations::text::SET_TITLE;
use adp_designer_hierarchical_requirement::samples::large_document;
use adp_designer_hierarchical_requirement::session::RequirementSession;
use adp_desl::hierarchy::model::{SurfaceModel, roles};
use adp_desl::hierarchy::session::HierarchySurfaceSession;
use common::{examples_folder, models, run};

/// SC-005's bound for 600 entries.
const LIMIT: Duration = Duration::from_secs(2);
/// The standalone host's bound for the 2,616 entries of the NIST example.
const NIST_LIMIT: Duration = Duration::from_secs(4);

fn nist_folder() -> PathBuf {
    examples_folder().join("nist-sp-800-53")
}

/// The NIST example and its registration copied into a folder of its own; the folder and the body.
fn nist_copy() -> (tempfile::TempDir, PathBuf) {
    let folder = tempfile::tempdir().unwrap();
    for name in ["nist-sp-800-53.md", "nist-sp-800-53.adp"] {
        fs::copy(nist_folder().join(name), folder.path().join(name)).unwrap();
    }
    let body = folder.path().join("nist-sp-800-53.md");
    (folder, body)
}

/// Opens `body`, with the registration beside it when there is one; the session and how long opening took.
fn open(body: &Path) -> (RequirementSession, Duration) {
    let registration = body.with_extension("adp");
    let started = Instant::now();
    let session = RequirementSession::open(body, registration.exists().then_some(registration));
    let model = session.model();
    let took = started.elapsed();
    assert!(!model.nodes.is_empty());
    (session, took)
}

/// Writes `text` to `body` as another program does and tells the session; the model it shows and how long that took.
fn change_outside(session: &mut RequirementSession, body: &Path, text: &str) -> (SurfaceModel, Duration) {
    fs::write(body, text).unwrap();
    let started = Instant::now();
    session.file_changed(body);
    let shown = models(&session.take_changes());
    let took = started.elapsed();
    (shown.into_iter().last().expect("the change is shown"), took)
}

fn count(model: &SurfaceModel, role: &str) -> usize {
    model.nodes.iter().filter(|n| n.role == role).count()
}

#[test]
fn a_document_of_100_items_and_500_requirements_opens_and_shows_an_outside_change_each_within_two_seconds() {
    // Arrange.
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join("large.md");
    fs::write(&body, large_document()).unwrap();

    // Act: opened.
    let (mut session, opening) = open(&body);
    let opened = session.model();

    // Assert.
    assert_eq!(100, count(&opened, roles::LEVEL));
    assert_eq!(500, count(&opened, roles::ENTRY));
    assert!(opening < LIMIT, "Opening took {} ms.", opening.as_millis());

    // Act: another program changes one title.
    let changed = large_document().replace("Requirement 437 holds.", "Requirement 437 holds twice.");
    let (shown, showing) = change_outside(&mut session, &body, &changed);

    // Assert.
    assert!(shown.nodes.iter().any(|n| n.title == "Requirement 437 holds twice."));
    assert!(showing < LIMIT, "Showing the change took {} ms.", showing.as_millis());
}

fn read_nist() -> Document {
    common::read_with(
        &nist_folder().join("nist-sp-800-53.md"),
        &nist_folder().join("nist-sp-800-53.adp"),
    )
    .document
}

/// How many items stand above and at `entry`, counting itself.
fn depth(by_id: &HashMap<&str, &Entry>, entry: &Entry) -> usize {
    let mut depth = 1;
    let mut at = entry;
    while let Some(parent) = at.parent_id.as_deref() {
        at = by_id[parent];
        depth += 1;
    }
    depth
}

#[test]
fn the_nist_controls_read_without_a_finding_as_their_readme_says() {
    // Act.
    let document = read_nist();

    // Assert: every number given once, every reference to a related control resolved, every requirement in an item.
    let findings: Vec<&str> = document.findings.iter().map(|f| f.message.as_str()).collect();
    assert!(findings.is_empty(), "{findings:?}");

    // Twenty families, 300 controls and 714 enhancements, three levels deep, and 1,582 requirements.
    let by_id: HashMap<&str, &Entry> = document.entries.iter().map(|e| (e.id.as_str(), e)).collect();
    let items: Vec<&Entry> = document.entries.iter().filter(|e| e.is_item()).collect();
    assert_eq!(1034, items.len());
    assert_eq!(1582, document.entries.len() - items.len());
    assert_eq!(20, document.entries.iter().filter(|e| e.parent_id.is_none()).count());
    assert_eq!(Some(3), items.iter().map(|e| depth(&by_id, e)).max());
    assert_eq!("AC-2(1) Automated System Account Management", by_id["ITEM-0004"].title);

    // The four baselines as tags, the families collapsed, three stored filters.
    let tags: Vec<&str> = document.tags.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(["low", "moderate", "high", "privacy"], tags.as_slice());
    assert_eq!(20, document.entries.iter().filter(|e| e.collapsed).count());
    let filters: Vec<&str> = document.filters.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        ["High baseline", "Privacy baseline", "Access control"],
        filters.as_slice()
    );
}

#[test]
fn the_nist_controls_open_on_the_page_and_show_an_outside_change_each_within_four_seconds() {
    // Arrange.
    let (_folder, body) = nist_copy();

    // Act: opened.
    let (mut session, opening) = open(&body);

    // Assert.
    assert_eq!(1582, count(&session.model(), roles::ENTRY));
    assert!(opening < NIST_LIMIT, "Opening took {} ms.", opening.as_millis());

    // Act: another program changes the last requirement's title.
    let text = fs::read_to_string(&body)
        .unwrap()
        .replace("REQ-1582 ", "REQ-1582 Changed by an agent: ");
    let (shown, showing) = change_outside(&mut session, &body, &text);

    // Assert.
    assert!(
        shown
            .nodes
            .iter()
            .any(|n| n.id == "REQ-1582" && n.title.starts_with("Changed by an agent: "))
    );
    assert!(
        showing < NIST_LIMIT,
        "Showing the change took {} ms.",
        showing.as_millis()
    );
}

#[test]
fn an_edit_of_one_nist_title_writes_only_that_line() {
    // Arrange.
    let (_folder, body) = nist_copy();
    let before = fs::read_to_string(&body).unwrap();
    let (mut session, _) = open(&body);

    // Act.
    run(
        &mut session,
        SET_TITLE,
        "ITEM-0004",
        "title=AC-2(1) Automated account management",
    );

    // Assert: written, and every other line of the 2,616 entries as it was.
    let after = fs::read_to_string(&body).unwrap();
    let (before, after): (Vec<&str>, Vec<&str>) = (before.lines().collect(), after.lines().collect());
    assert_eq!(before.len(), after.len());
    let changed: Vec<usize> = (0..before.len()).filter(|&i| before[i] != after[i]).collect();
    assert_eq!(1, changed.len(), "{changed:?}");
    assert_eq!(
        "    - ITEM-0004 AC-2(1) Automated account management",
        after[changed[0]]
    );
}
