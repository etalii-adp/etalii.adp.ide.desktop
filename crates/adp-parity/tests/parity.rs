//! The parity check (etalii.adp spec 030, SC-004, task T050; contracts/parity.md, "The check"): one test per edit script
//! under `parity/scripts/`, which replays it headless in the desktop core and compares every file its golden lists byte
//! for byte with what the desktop host wrote.
//!
//! A script whose tool type cannot open yet in the desktop host, or whose bytes do not match yet, stays a test, ignored
//! with the concrete reason; `parity/README.md` lists each script's state. `cargo test -p adp-parity -- --ignored`
//! replays them all and prints, per script, every step that went otherwise than in the standalone host and the first
//! differing offset of each file with both lines.

use std::fs;

use adp_parity::{check, repository_root, script_names};

#[test]
#[ignore = "cannot open: the C4 container specification (vendor/etalii.adp/definitions/diagrams/c4-container.dis) does not load (/persistence/typeMap/ViewInclude/attributes/members: 'includes' is not a variable here), and the C4 module's residue (the Structurizr DSL reader and writer, the layout sidecar) is not ported"]
fn c4_container_courier() {
    check("c4/container/courier");
}

#[test]
fn freeplane_mindmap_example_1() {
    check("freeplane/mindmap/example-1");
}

#[test]
#[ignore = "differs at byte offset 63: a period's inline rename is refused (its label is computed and has no editText or parse), the timeline's plugin functions (timelineTimes) are absent, a dated position is not a number on the desktop canvas, an operation's input is not asked, and a connect from the menu is not run; parity/README.md lists the steps"]
fn generic_timeline_roadmap() {
    check("generic/timeline/roadmap");
}

/// Every script has its test here and its row in `parity/README.md`, so none is skipped silently.
#[test]
fn every_script_is_checked_and_listed() {
    let tests = include_str!("parity.rs");
    let readme = fs::read_to_string(repository_root().join("parity/README.md")).expect("parity/README.md");
    for name in script_names() {
        assert!(
            tests.contains(&format!("check(\"{name}\")")),
            "{name} has no test in crates/adp-parity/tests/parity.rs"
        );
        assert!(
            readme.contains(&format!("`{name}`")),
            "{name} has no row in parity/README.md"
        );
    }
}
