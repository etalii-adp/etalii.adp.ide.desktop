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

/// The differences of the cases, by case, each as `step:part`, with their causes, each waiting on Peter's ruling.
const LISTED: &[(&str, &[&str])] = &[
    // The case words a refusal "could not be written: <reason>", the reference runner's prefix; the description states
    // the reason.
    ("classification", &["12:refused", "13:refused"]),
    ("tags", &["10:refused"]),
    // The same prefix, before the reference host's sentence that a second column of one depth could not be written.
    ("columns", &["9:refused"]),
    // `hierarchy.add` is the shell's gesture that adds a document from the designer's template.
    ("new-document", &["1:refused", "2:refused"]),
    // The prefix again, and `fbl.stale-view-data`, an error in the reference host and information in FBL section 13.
    ("structure", &["9:refused", "11:findings"]),
    ("structure-lf", &["9:refused", "11:findings"]),
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
