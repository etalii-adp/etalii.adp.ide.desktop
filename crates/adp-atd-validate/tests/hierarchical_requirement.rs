//! The hierarchical requirement designer's description against its twelve cases (etalii.adp spec 076, T050, FR-002,
//! FR-003): a designer runs through the runtime, and every case passes, apart from the differences listed here by name
//! with their cause. A regression or a correction shows here. `ETALII_ADP` runs it on another clone of etalii.adp.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use adp_atd_check::{FsFiles, check};
use adp_atd_runtime::RuntimeHost;
use adp_atd_validate::{cases_of, read_case, validate_description};

fn designers() -> PathBuf {
    std::env::var_os("ETALII_ADP")
        .map_or_else(
            || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp"),
            PathBuf::from,
        )
        .join("definitions/designers")
}

/// The differences of the cases, by case, each as `step:part`, with their causes (T064 corrects the description; the
/// rest wait on Peter or on later tasks).
const LISTED: &[(&str, &[&str])] = &[
    // A refusal the description does not state yet (T064): an assessment's text of two lines; its undo and redo then
    // differ too.
    ("assessments", &["6:refused", "6:bytes", "7:bytes", "8:bytes"]),
    // T064: 'urgent' names no state, and the case words it "could not be written: …", the reference runner's prefix
    // (a question for Peter); a second category named Functional is not refused; the redo of a state's deletion.
    (
        "classification",
        &["12:refused", "13:refused", "13:findings", "18:bytes"],
    ),
    // T064: the description states no bounds of a column's depth and width; T062: two refusals after an outside write.
    (
        "columns",
        &["6:refused", "7:refused", "8:refused", "9:refused", "10:refused"],
    ),
    // T064: an empty filter is not refused; T062: a refusal after an outside write.
    ("filters", &["4:refused", "5:refused", "6:refused"]),
    // `hierarchy.add` is the shell's gesture that adds a document from the designer's template (a question for Peter).
    ("new-document", &["1:refused", "2:refused"]),
    // T064: a move into the item itself is refused in the reference host's words; `fbl.stale-view-data` is an error
    // there and information in FBL section 13 (a question for Peter).
    ("structure", &["9:refused", "11:findings"]),
    ("structure-lf", &["9:refused", "11:findings"]),
    // T064: a tag's name with # and a second tag named Customer are not refused, and their undo and redo differ.
    (
        "tags",
        &[
            "9:refused",
            "9:bytes",
            "10:refused",
            "10:bytes",
            "10:findings",
            "12:bytes",
            "13:bytes",
        ],
    ),
    // T064: a title of two lines is not refused, and its undo and redo differ.
    ("text", &["6:refused", "6:bytes", "7:bytes", "8:bytes"]),
    // T062: the outside write and what follows it.
    (
        "two-writers",
        &[
            "2:refused",
            "2:bytes",
            "2:findings",
            "3:refused",
            "3:bytes",
            "3:findings",
            "4:refused",
            "4:bytes",
            "4:findings",
            "5:bytes",
            "5:findings",
            "6:bytes",
            "6:findings",
        ],
    ),
];

#[test]
fn the_requirement_cases_pass_but_for_what_is_listed() {
    let root = designers().join("hierarchical-requirement.atd");
    let checked = check(&FsFiles, &root);
    let cases: Vec<_> = cases_of(&root).iter().map(|dir| read_case(dir).unwrap()).collect();
    assert_eq!(cases.len(), 12, "the designer has its twelve cases");
    let found: BTreeMap<String, BTreeSet<String>> = validate_description(&checked, &RuntimeHost::default(), &cases)
        .expect("a designer has a runtime")
        .into_iter()
        .map(|result| {
            let places = result
                .differences
                .iter()
                .map(|d| match d.step {
                    Some(step) => format!("{step}:{}", d.part),
                    None => format!("read:{}", d.part),
                })
                .collect();
            (result.name, places)
        })
        .collect();
    let listed: BTreeMap<&str, &[&str]> = LISTED.iter().copied().collect();
    let mut unexpected = Vec::new();
    for (case, places) in &found {
        let expected: BTreeSet<String> = listed
            .get(case.as_str())
            .into_iter()
            .flat_map(|p| p.iter())
            .map(|p| (*p).to_owned())
            .collect();
        if *places != expected {
            unexpected.push(format!("{case}: {places:?}\n  expected {expected:?}"));
        }
    }
    for case in listed.keys() {
        assert!(found.contains_key(*case), "{case} is a case");
    }
    assert!(unexpected.is_empty(), "{}", unexpected.join("\n"));
}
