//! The Knowledge designer's description against its five cases, recorded from the module (etalii.adp spec 076, T075,
//! FR-004 to FR-006): each format's script, the module's refusals and the findings of a file as it is read, each read
//! with the binding that claims its body. A regression or a correction shows here. `ETALII_ADP` runs it on another
//! clone of etalii.adp.

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

/// The differences of the cases, by case, each awaiting Peter's ruling (FR-007).
const LISTED: &[(&str, &[&str])] = &[(
    "findings",
    &[
        // The findings the host gives and no constraint can (knowledge.md, section 7): the FBL reader's dangling
        // references and unknown keys in the module's wording, an entry without an id, an unknown type, an unresolved
        // target or row, a parent cycle; and the constraints' sentences, which the module words with the raw id a
        // dangling reference holds.
        "read:findings",
        // A cell naming no property: the module names it by the id it stores, `r1/p94`; the description's derived id
        // reads `self.property.id`, which a dangling reference does not have, so the runtime names it by its place.
        "read:reading",
    ],
)];

#[test]
fn the_knowledge_cases_pass_but_for_the_hosts_findings() {
    let root = designers().join("knowledge.atd");
    let checked = check(&FsFiles, &root);
    let cases: Vec<_> = cases_of(&root).iter().map(|dir| read_case(dir).unwrap()).collect();
    assert_eq!(cases.len(), 5, "the Knowledge designer has its five cases");
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
