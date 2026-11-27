//! The Timeline description against its three cases (etalii.adp spec 035, T132), on the shell's host: its elements
//! placed on the time axis at their seconds, a time read in every form it is written in, a move and a new element
//! written as finely as the time was, give and remove end as the change of the entry's `end`, and Arrange diagram by
//! `layout.rowPacking`. The roadmap and findings cases pass; of the add case the steps that add a document of another
//! tool type are listed.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use adp_atd_check::{FsFiles, check};
use adp_atd_validate::{cases_of, read_case, validate_description};

/// The differences of the cases, by case, each as `step:part`.
fn listed() -> BTreeMap<&'static str, BTreeSet<String>> {
    // The add case adds documents of the C4 tool types and others beside the timeline. The runner of one description
    // adds a document of its own tool type only, so each of those steps is refused where the case makes it.
    let foreign = [2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13];
    BTreeMap::from([
        ("add", foreign.iter().map(|step| format!("{step}:refused")).collect()),
        ("findings", BTreeSet::new()),
        ("roadmap", BTreeSet::new()),
    ])
}

#[test]
fn the_timeline_cases_pass_but_for_what_is_listed() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/definitions/diagrams/timeline.atd");
    let checked = check(&FsFiles, &root);
    let cases: Vec<_> = cases_of(&root).iter().map(|dir| read_case(dir).unwrap()).collect();
    assert_eq!(cases.len(), 3);
    let results = validate_description(&checked, &adp_shell::runtime_host(), &cases).unwrap();
    let listed = listed();
    for result in &results {
        let found: BTreeSet<String> = result
            .differences
            .iter()
            .map(|d| match d.step {
                Some(step) => format!("{step}:{}", d.part),
                None => format!("read:{}", d.part),
            })
            .collect();
        assert_eq!(Some(&found), listed.get(result.name.as_str()), "{}", result.name);
    }
}
