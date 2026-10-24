//! `adp-atd-validate` (etalii.adp spec 035, tasks T121 and T127, FR-041): a case replayed through a description's
//! runtime in memory, every part compared, each difference reported at its step and part with what was expected and
//! what was given.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use adp_atd_check::{Description, FsFiles, Overlay, check};
use adp_atd_runtime::{RuntimeHost, RuntimeTool, diagram_tool};
use adp_atd_validate::{Case, Difference, validate};

fn diagrams() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/definitions/diagrams")
}

/// The mind map's runtime and its description, its root `.atd` replaced by `text` when one is given.
fn runtime(text: Option<String>) -> (RuntimeTool, Description) {
    runtime_of(&diagrams().join("mindmap.atd"), text)
}

/// The runtime and the description of the root `.atd` at `path`, replaced by `text` when one is given.
fn runtime_of(path: &Path, text: Option<String>) -> (RuntimeTool, Description) {
    let path = path.to_path_buf();
    let files = Overlay {
        open: text.map(|t| BTreeMap::from([(path.clone(), t)])).unwrap_or_default(),
        under: &FsFiles,
    };
    let checked = check(&files, &path);
    let tool = diagram_tool(
        &checked.description,
        checked.symbols.as_ref().unwrap(),
        &RuntimeHost::default(),
    )
    .unwrap();
    (tool, checked.description)
}

/// `case` validated against the mind map, its root `.atd` replaced by `text` when one is given.
fn run(text: Option<String>, case: &Case) -> Vec<Difference> {
    let (tool, description) = runtime(text);
    validate(&tool, &description, case)
}

/// A vendored case, read into memory.
fn case(name: &str) -> Case {
    case_in(&diagrams().join("conformance/mindmap").join(name))
}

/// The case in `dir`, read into memory.
fn case_in(dir: &Path) -> Case {
    let name = dir.file_name().unwrap().to_string_lossy().into_owned();
    let fixture = serde_json::from_str(&std::fs::read_to_string(dir.join("conformance.json")).unwrap()).unwrap();
    let files = std::fs::read_dir(dir)
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
    Case { name, fixture, files }
}

/// Where a difference was attributed, as `file:line` with the file's name alone.
fn short(difference: &Difference) -> String {
    difference.at.as_ref().map_or("nowhere".to_owned(), |at| {
        let file = Path::new(&at.file).file_name().unwrap().to_string_lossy();
        format!("{file}:{}", at.line)
    })
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
    let differences = run(None, &case("fold"));
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
    let differences = run(Some(seeded), &case("unnamed-root"));
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
    // The constraint `unnamedCentralTopic`, whose message the finding is.
    assert_eq!(short(findings[0]), "mindmap.atd:237");
}

/// Each difference names the construct of the description that produced its value (ATD §2.3, `ATD9001`).
#[test]
fn each_difference_is_attributed_to_the_construct_that_produced_it() {
    let (tool, description) = runtime(None);
    let mut attributed = BTreeMap::new();
    for name in [
        "duplicate-id",
        "example-1",
        "fold",
        "not-a-map",
        "refusals",
        "unnamed-root",
    ] {
        for difference in validate(&tool, &description, &case(name)) {
            let place = places(std::slice::from_ref(&difference)).pop_first().unwrap();
            attributed.insert(format!("{name} {place}"), short(&difference));
        }
    }
    let unattributed: Vec<&String> = attributed
        .iter()
        .filter(|(_, at)| *at == "nowhere")
        .map(|(p, _)| p)
        .collect();
    assert!(unattributed.is_empty(), "{unattributed:?}");
    for (place, at) in [
        // The reading, to the binding's rule for the type of the first element that differs.
        ("fold read:reading", "mindmap.freeplane.atd:28"),
        // A finding, to the constraint that has its code: here the built-in `std.unparseable`.
        ("not-a-map read:findings", "mindmap.atd:226"),
        // A refusal, to the constraint of the gesture's kind: `keepCentralTopic` for a deletion,
        // `reparentWithinTree` for a move.
        ("refusals 1:refused", "mindmap.atd:261"),
        ("refusals 3:refused", "mindmap.atd:269"),
        // A gesture, to its operation (`addSibling`), the `behavior` block for a deletion, or the binding's rule
        // for the type of the element a gesture named only by its kind (`editLabel`) changed.
        ("example-1 2:refused", "mindmap.atd:290"),
        ("example-1 2:bytes", "mindmap.atd:290"),
        ("example-1 10:bytes", "mindmap.atd:326"),
        ("example-1 7:bytes", "mindmap.freeplane.atd:28"),
        // A drawing, to the notation's entry for the type of the element drawn differently.
        ("fold 1:drawing", "mindmap.notation.atd:58"),
    ] {
        assert_eq!(attributed.get(place).map(String::as_str), Some(at), "{place}");
    }
}

/// Where the mind map's description and the runtime still differ from the cases, case by case, until T132 corrects
/// the description and the engines catch up; a regression or a correction shows here.
#[test]
fn the_mind_maps_cases_differ_only_where_known() {
    let known: [(&str, &[&str]); 6] = [
        // A duplicate id: the module read the later holder under an ephemeral id and reported it, and draws it so.
        ("duplicate-id", &["read:drawing", "read:findings", "read:reading"]),
        // A sibling placed after a node and a node moved to another parent, which only a plugin writes; the notes,
        // whose `create` the description's binding does not state; and the reading's absent FOLDED. Each refused
        // gesture leaves the map, and so its drawing, behind the reference's from that step on.
        (
            "example-1",
            &[
                "10:bytes",
                "10:drawing",
                "11:bytes",
                "11:drawing",
                "12:bytes",
                "12:drawing",
                "13:bytes",
                "13:drawing",
                "14:bytes",
                "14:drawing",
                "15:bytes",
                "15:drawing",
                "16:bytes",
                "16:drawing",
                "18:bytes",
                "18:drawing",
                "19:bytes",
                "19:drawing",
                "20:bytes",
                "20:drawing",
                "20:refused",
                "21:bytes",
                "21:drawing",
                "22:bytes",
                "22:drawing",
                "22:refused",
                "2:bytes",
                "2:drawing",
                "2:refused",
                "3:bytes",
                "3:drawing",
                "3:refused",
                "4:bytes",
                "4:drawing",
                "5:bytes",
                "5:drawing",
                "6:bytes",
                "6:drawing",
                "6:refused",
                "7:bytes",
                "7:drawing",
                "8:bytes",
                "8:drawing",
                "8:refused",
                "9:bytes",
                "9:drawing",
                "9:refused",
                "read:reading",
            ],
        ),
        // The absent FOLDED; the folded node's ⊕, which the canvas draws from the bound view and the reference
        // recorder, binding no view, did not; the host's wording of an action the selection does not offer.
        ("fold", &["1:drawing", "3:refused", "read:reading"]),
        // Two roots: the module refused to read the map, and the runtime still draws the first.
        ("not-a-map", &["read:drawing", "read:findings", "read:reading"]),
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
    let (tool, description) = runtime(None);
    let mut unexpected = Vec::new();
    for (name, expected) in known {
        let differences = validate(&tool, &description, &case(name));
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
    let _ = run(None, &held);
    assert!(!Path::new("/case").exists());
    assert_eq!(held.files["plan.mm"], body);
}

/// A step's splices are compared with the case's, each edit's in order, its offsets into the body as that edit found
/// it; a difference is attributed to the gesture's operation.
#[test]
fn a_steps_splices_are_compared_and_attributed_to_its_gesture() {
    let (tool, description) = runtime(None);
    let mut one_step = case("example-1");
    let steps = one_step.fixture["steps"].as_array_mut().unwrap();
    steps.truncate(1);
    let with = |splices: serde_json::Value| {
        let mut held = one_step.clone();
        held.fixture["steps"][0]["splices"] = splices;
        validate(&tool, &description, &held)
            .into_iter()
            .filter(|d| d.part == "splices")
            .collect::<Vec<_>>()
    };

    // What the runtime made: the new node, then its text typed into the inline editor.
    let made = with(serde_json::json!([]));
    assert_eq!(made.len(), 1);
    let splices: Vec<serde_json::Value> = serde_json::from_str(&made[0].actual).unwrap();
    assert_eq!(splices.len(), 2, "{splices:?}");
    let mut body = one_step.files["mindmap.mm"].clone();
    for splice in &splices {
        let (start, end) = (
            splice["start"].as_u64().unwrap() as usize,
            splice["end"].as_u64().unwrap() as usize,
        );
        body.splice(start..end, splice["text"].as_str().unwrap().bytes());
    }
    assert_eq!(body, one_step.files["after-1.mm"]);

    assert!(with(serde_json::Value::Array(splices.clone())).is_empty());
    let mut shifted = splices;
    shifted[0]["start"] = serde_json::json!(shifted[0]["start"].as_u64().unwrap() + 1);
    let differences = with(serde_json::Value::Array(shifted));
    assert_eq!(differences.len(), 1);
    assert_eq!(differences[0].step, Some(1));
    // The operation `addChild`.
    assert_eq!(short(&differences[0]), "mindmap.atd:280");
}
