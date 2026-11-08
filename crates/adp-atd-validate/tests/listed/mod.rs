//! What each tool type's test holds its cases to (etalii.adp spec 076, FR-007, SC-005): the differences found, by case,
//! each as `step:part`, the reading's as `read:part`, equal to the list of what is ruled on. A name taken off the list
//! fails the test, and so does a difference not listed; `held_both_ways` seeds each in a copy of the list.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use adp_atd_check::{FsFiles, check};
use adp_atd_runtime::RuntimeHost;
use adp_atd_validate::{cases_of, read_case, validate_description};

/// The differences, by case.
pub type Places = BTreeMap<String, BTreeSet<String>>;

/// The folder of etalii.adp's tool descriptions of `kind` (`diagrams`, `designers`): the vendored copy, or the clone
/// `ETALII_ADP` names.
pub fn definitions(kind: &str) -> PathBuf {
    std::env::var_os("ETALII_ADP")
        .map_or_else(
            || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp"),
            PathBuf::from,
        )
        .join("definitions")
        .join(kind)
}

/// The differences of the cases of the description `root`, which has `count` cases.
pub fn differences(root: &Path, count: usize) -> Places {
    let checked = check(&FsFiles, root);
    let cases: Vec<_> = cases_of(root).iter().map(|dir| read_case(dir).unwrap()).collect();
    assert_eq!(cases.len(), count, "{} has its cases", root.display());
    validate_description(&checked, &RuntimeHost::default(), &cases)
        .expect("the description has a runtime")
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
        .collect()
}

/// A list as the places it names.
pub fn places_of(listed: &[(&str, &[&str])]) -> Places {
    listed
        .iter()
        .map(|(case, places)| ((*case).to_owned(), places.iter().map(|p| (*p).to_owned()).collect()))
        .collect()
}

/// Every case whose differences are not what `listed` names, and every listed case that is no case; empty when the
/// list holds.
pub fn unexpected(found: &Places, listed: &Places) -> Vec<String> {
    let none = BTreeSet::new();
    let mut out: Vec<String> = found
        .iter()
        .filter(|(case, places)| *places != listed.get(*case).unwrap_or(&none))
        .map(|(case, places)| format!("{case}: {places:?}\n  expected {:?}", listed.get(case).unwrap_or(&none)))
        .collect();
    out.extend(
        listed
            .keys()
            .filter(|case| !found.contains_key(*case))
            .map(|case| format!("{case} is listed and is no case")),
    );
    out
}

/// Fails with what differs when `found` is not what `listed` names.
pub fn assert_listed(found: &Places, listed: &[(&str, &[&str])]) {
    let unexpected = unexpected(found, &places_of(listed));
    assert!(unexpected.is_empty(), "{}", unexpected.join("\n"));
}

/// SC-005, seeded in copies: with the differences as `listed` names them, and `case` among the cases, the list holds;
/// a difference added to `case` fails it; and each name taken off the list fails it, as each listed difference that is
/// no longer found does.
pub fn held_both_ways(listed: &[(&str, &[&str])], case: &str) {
    let list = places_of(listed);
    let mut found = list.clone();
    found.entry(case.to_owned()).or_default();
    assert!(unexpected(&found, &list).is_empty(), "the list holds for what it names");

    let mut seeded = found.clone();
    seeded.entry(case.to_owned()).or_default().insert("1:seeded".to_owned());
    assert!(
        !unexpected(&seeded, &list).is_empty(),
        "a difference not listed fails the test"
    );

    for (listed_case, places) in &list {
        for place in places {
            let mut shorter = list.clone();
            shorter.entry(listed_case.clone()).or_default().remove(place);
            assert!(
                !unexpected(&found, &shorter).is_empty(),
                "{listed_case} {place} taken off the list fails the test"
            );
            let mut corrected = found.clone();
            corrected.entry(listed_case.clone()).or_default().remove(place);
            assert!(
                !unexpected(&corrected, &list).is_empty(),
                "{listed_case} {place} no longer found fails the test until it is taken off the list"
            );
        }
    }
}
