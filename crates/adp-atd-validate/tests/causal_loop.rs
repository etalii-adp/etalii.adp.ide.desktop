//! The causal loop diagram description against its four cases, recorded from the module (etalii.adp spec 076, T022,
//! FR-001, FR-006): every case passes on the declared binding, its edges drawn and its self-organising action run.
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

/// The differences of the cases, by case: none is listed.
const LISTED: &[(&str, &[&str])] = &[];

#[test]
fn the_causal_loop_cases_pass() {
    let root = diagrams().join("causal-loop-diagram.atd");
    let checked = check(&FsFiles, &root);
    let cases: Vec<_> = cases_of(&root).iter().map(|dir| read_case(dir).unwrap()).collect();
    assert_eq!(cases.len(), 4, "the causal loop diagram has its four cases");
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
