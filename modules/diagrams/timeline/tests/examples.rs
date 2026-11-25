//! The timeline's Marvel universe examples (`tests/examples/marvel-universe/` and its key events in
//! `tests/examples/marvel-universe-overview/`, the same bytes as the standalone host's
//! `src/diagrams/timeline/examples/`): a reader who opens one meets no finding, sees every element and relation it
//! writes, and does not wait for it.

mod common;

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use adp_diagram::DiagramWorkspace;
use adp_diagram_timeline::arrangement;

const EXAMPLES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/examples");

const FULL: &str = "marvel-universe";

const OVERVIEW: &str = "marvel-universe-overview";

/// How long opening the example and drawing its first view may take, in a debug build on a slow runner.
const PATIENCE: Duration = Duration::from_secs(30);

struct Opened {
    _folder: tempfile::TempDir,
    workspace: DiagramWorkspace,
    session: adp_diagram::DiagramSession,
    took: Duration,
}

/// The example `name` opened as a reader opens it: its registration, in a folder of its own.
fn open(name: &str) -> Opened {
    let folder = tempfile::tempdir().unwrap();
    for extension in ["adp", "tml"] {
        let file = format!("{name}.{extension}");
        fs::copy(Path::new(EXAMPLES).join(name).join(&file), folder.path().join(&file)).unwrap();
    }
    let started = Instant::now();
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (_, mut session) = workspace
        .open(&mut common::tools(), Path::new(&format!("{name}.adp")))
        .unwrap();
    let deltas = session.baseline(workspace.documents()).unwrap();
    let took = started.elapsed();
    assert!(!deltas.is_empty(), "opening the example draws nothing");
    Opened {
        _folder: folder,
        workspace,
        session,
        took,
    }
}

/// The number of entries the body `name` writes under `elements:` and under `connections:`.
fn written(name: &str) -> (usize, usize) {
    let text = fs::read_to_string(Path::new(EXAMPLES).join(name).join(format!("{name}.tml"))).unwrap();
    let (elements, relations) = text.split_once("\nconnections:").unwrap();
    (
        elements.matches("\n  - id: ").count(),
        relations.matches("\n  - id: ").count(),
    )
}

/// The elements and relations the opened example `name` reads.
fn read(opened: &Opened, name: &str) -> (usize, usize) {
    let (elements, links) = arrangement::read(
        &opened
            .workspace
            .documents()
            .get(&opened.workspace.root().join(format!("{name}.tml")))
            .unwrap()
            .model()
            .diagram,
    );
    (elements.len(), links.len())
}

fn findings(opened: &Opened) -> Vec<adp_disl::DislFinding> {
    opened
        .session
        .findings(opened.workspace.documents())
        .unwrap_or_default()
}

#[test]
fn the_marvel_universe_opens_without_a_finding() {
    let opened = open(FULL);

    let findings = findings(&opened);

    assert!(findings.is_empty(), "{findings:?}");
}

#[test]
fn the_marvel_universe_reads_every_element_and_relation() {
    let opened = open(FULL);

    assert_eq!((428, 487), written(FULL));
    assert_eq!(written(FULL), read(&opened, FULL));
}

#[test]
fn the_marvel_universe_opens_in_reasonable_time() {
    let opened = open(FULL);

    assert!(opened.took < PATIENCE, "opening took {:?}", opened.took);
}

#[test]
fn its_key_events_open_without_a_finding() {
    let opened = open(OVERVIEW);

    let findings = findings(&opened);

    assert!(findings.is_empty(), "{findings:?}");
}

#[test]
fn its_key_events_read_every_element_and_relation() {
    let opened = open(OVERVIEW);

    assert_eq!((47, 52), written(OVERVIEW));
    assert_eq!(written(OVERVIEW), read(&opened, OVERVIEW));
}
