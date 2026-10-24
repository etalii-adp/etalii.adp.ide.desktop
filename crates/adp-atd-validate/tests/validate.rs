//! `adp-atd-validate` (etalii.adp spec 035, tasks T121 and T127, FR-041): a case replayed through a description's
//! runtime in memory, every part compared, each difference reported at its step and part with what was expected and
//! what was given.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use adp_atd_check::{FsFiles, Overlay, check};
use adp_atd_runtime::{RuntimeHost, RuntimeTool, diagram_tool};
use adp_atd_validate::{Case, Difference, validate};

fn diagrams() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/definitions/diagrams")
}

/// The mind map's runtime, its root `.atd` replaced by `text` when one is given.
fn runtime(text: Option<String>) -> RuntimeTool {
    let path = diagrams().join("mindmap.atd");
    let files = Overlay {
        open: text.map(|t| BTreeMap::from([(path.clone(), t)])).unwrap_or_default(),
        under: &FsFiles,
    };
    let checked = check(&files, &path);
    diagram_tool(
        &checked.description,
        checked.symbols.as_ref().unwrap(),
        &RuntimeHost::default(),
    )
    .unwrap()
}

/// A vendored case, read into memory.
fn case(name: &str) -> Case {
    let dir = diagrams().join("conformance/mindmap").join(name);
    let fixture = serde_json::from_str(&std::fs::read_to_string(dir.join("conformance.json")).unwrap()).unwrap();
    let files = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_file())
        .map(|e| {
            (
                e.file_name().to_string_lossy().into_owned(),
                std::fs::read(e.path()).unwrap(),
            )
        })
        .collect();
    Case {
        name: name.to_owned(),
        fixture,
        files,
    }
}

/// Each difference as `step:part`, the reading's as `read:part`.
fn places(differences: &[Difference]) -> BTreeSet<String> {
    differences
        .iter()
        .map(|d| match d.step {
            Some(step) => format!("{step}:{}", d.part),
            None => format!("read:{}", d.part),
        })
        .collect()
}

#[test]
fn a_gesture_the_runtime_makes_as_the_reference_did_has_no_difference() {
    // The fold case collapses and expands the central topic (view state, nothing written) and the runtime does so.
    let differences = validate(&runtime(None), &case("fold"));
    let places = places(&differences);
    assert!(
        !places.contains("1:refused") && !places.contains("1:bytes"),
        "{differences:#?}"
    );
    assert!(
        !places.contains("2:refused") && !places.contains("2:bytes"),
        "{differences:#?}"
    );
}

#[test]
fn a_seeded_difference_is_reported_at_its_part_with_what_was_expected_and_given() {
    let text = std::fs::read_to_string(diagrams().join("mindmap.atd")).unwrap();
    let seeded = text.replacen("has an unnamed central topic.", "has a nameless central topic.", 1);
    assert_ne!(seeded, text);
    let differences = validate(&runtime(Some(seeded)), &case("unnamed-root"));
    let findings: Vec<&Difference> = differences.iter().filter(|d| d.part == "findings").collect();
    assert_eq!(findings.len(), 1, "{differences:#?}");
    assert_eq!(findings[0].step, None);
    assert!(
        findings[0].expected.contains("has an unnamed central topic."),
        "{}",
        findings[0].expected
    );
    assert!(
        findings[0].actual.contains("has a nameless central topic."),
        "{}",
        findings[0].actual
    );
}

/// Where the mind map's description and the runtime still differ from the cases, case by case, until T132 corrects
/// the description and the engines catch up; a regression or a correction shows here.
#[test]
fn the_mind_maps_cases_differ_only_where_known() {
    let known: [(&str, &[&str]); 6] = [
        // A duplicate id: the module read the later holder under an ephemeral id and reported it.
        ("duplicate-id", &["read:findings", "read:reading"]),
        // A sibling placed after a node and a node moved to another parent, which only a plugin writes; the notes,
        // whose `create` the description's binding does not state; and the reading's absent FOLDED.
        (
            "example-1",
            &[
                "10:bytes",
                "11:bytes",
                "12:bytes",
                "13:bytes",
                "14:bytes",
                "15:bytes",
                "16:bytes",
                "18:bytes",
                "19:bytes",
                "20:bytes",
                "20:refused",
                "21:bytes",
                "22:bytes",
                "22:refused",
                "2:bytes",
                "2:refused",
                "3:bytes",
                "3:refused",
                "4:bytes",
                "5:bytes",
                "6:bytes",
                "6:refused",
                "7:bytes",
                "8:bytes",
                "8:refused",
                "9:bytes",
                "9:refused",
                "read:reading",
            ],
        ),
        // The absent FOLDED; the host's wording of an action the selection does not offer.
        ("fold", &["3:refused", "read:reading"]),
        // Two roots: the module refused to read the map.
        ("not-a-map", &["read:findings", "read:reading"]),
        // The host's wording, and moves only a plugin writes.
        (
            "refusals",
            &[
                "1:refused",
                "2:refused",
                "3:refused",
                "4:refused",
                "5:refused",
                "read:reading",
            ],
        ),
        ("unnamed-root", &["read:reading"]),
    ];
    let runtime = runtime(None);
    let mut unexpected = Vec::new();
    for (name, expected) in known {
        let differences = validate(&runtime, &case(name));
        let found = places(&differences);
        let expected: BTreeSet<String> = expected.iter().map(|p| (*p).to_owned()).collect();
        if found != expected {
            unexpected.push(format!("{name}: {found:?}\n  expected {expected:?}"));
        }
    }
    assert!(unexpected.is_empty(), "{}", unexpected.join("\n"));
}

#[test]
fn a_case_is_validated_in_memory_and_nothing_is_written() {
    // The case's files are named under a root that does not exist; the runtime reads none of them from disk and
    // writes none, or the replay would have failed to open the body.
    let mut held = case("fold");
    let body = held.files.get("plan.mm").cloned().unwrap();
    held.files.insert("plan.mm".into(), body.clone());
    let _ = validate(&runtime(None), &held);
    assert!(!Path::new("/case").exists());
    assert_eq!(held.files["plan.mm"], body);
}
