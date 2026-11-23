//! The dependency graph description against its four cases, recorded from the module (etalii.adp spec 035, T132): every
//! case passes on the declared binding, its findings naming elements by the ids they are written with (DISL 0.10
//! `persistence.writtenId`), apart from the difference listed here by name. `ETALII_ADP` runs it on another clone.

mod listed;

/// The differences of the cases, by case.
const LISTED: &[(&str, &[&str])] = &[
    // The same findings in another order: the module interleaves its id findings by line, where DISL's declared order
    // lists them by code. A quirk of the module's, by Peter's ruling (spec 067), for which DISL does not change.
    ("findings", &["read:findings"]),
];

#[test]
fn the_dependency_graph_cases_pass() {
    let found = listed::differences(&listed::definitions("diagrams").join("dependency-graph.atd"), 4);
    listed::assert_listed(&found, LISTED);
}

/// SC-005: a name taken off the list fails the test above, and so does a difference not listed.
#[test]
fn its_list_is_held_both_ways() {
    listed::held_both_ways(LISTED, "findings");
}
