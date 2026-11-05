//! The agent behavior modelling description against its five cases, recorded from the module (etalii.adp spec 076,
//! T003, FR-001, FR-006): every case passes on the declared binding, apart from the differences listed here by name.
//! A regression or a correction shows here. `ETALII_ADP` runs it on another clone of etalii.adp.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use adp_atd_check::{FsFiles, check};
use adp_atd_runtime::RuntimeHost;
use adp_atd_validate::{cases_of, read_case, validate_description};

fn diagrams() -> PathBuf {
    std::env::var_os("ETALII_ADP")
        .map_or_else(
            || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp"),
            PathBuf::from,
        )
        .join("definitions/diagrams")
}

/// The differences of the cases, by case, each as `step:part`, the reading's as `read:part`, each a module's quirk
/// listed by name with its reason (FR-007).
const LISTED: &[(&str, &[&str])] = &[
    // The module says "This file has no Behavior heading…" at line 1, and "The Behavior section holds no list yet…" at
    // the heading's line; a DISL rule sees the model and no heading, so the description has one sentence and no line.
    // Listed as a module quirk by default: Peter did not rule on spec 076's Q1, and the plan's recommendation was taken.
    ("empty-section", &["read:findings"]),
    ("no-behavior", &["read:findings"]),
];

#[test]
fn the_behavior_model_cases_pass_but_for_what_is_listed() {
    let root = diagrams().join("agent-behavior-modelling.atd");
    let checked = check(&FsFiles, &root);
    let cases: Vec<_> = cases_of(&root).iter().map(|dir| read_case(dir).unwrap()).collect();
    assert_eq!(cases.len(), 5, "the behavior model has its five cases");
    let found: BTreeMap<String, BTreeSet<String>> = validate_description(&checked, &RuntimeHost::default(), &cases)
        .unwrap()
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
