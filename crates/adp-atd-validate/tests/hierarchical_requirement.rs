//! The hierarchical requirement designer's description against its twelve cases (etalii.adp spec 076, T050, FR-002,
//! FR-003): a designer runs through the runtime, and every case passes, apart from the differences listed here by name
//! with their cause. A regression or a correction shows here. `ETALII_ADP` runs it on another clone of etalii.adp.

mod listed;

/// The differences of the cases that Peter has ruled on, by case, each as `step:part`, with his ruling (FR-007).
/// Nothing awaits a ruling.
const LISTED: &[(&str, &[&str])] = &[
    // `fbl.stale-view-data`: an error in the reference host, and information in FBL section 13, which the description
    // follows. Listed by name as a module quirk (Peter, 2026-11-07, spec 076's Q6).
    ("structure", &["11:findings"]),
    ("structure-lf", &["11:findings"]),
];

#[test]
fn the_requirement_cases_pass_but_for_what_is_listed() {
    let found = listed::differences(
        &listed::definitions("designers").join("hierarchical-requirement.atd"),
        12,
    );
    listed::assert_listed(&found, LISTED);
}

/// SC-005: a name taken off the list fails the test above, and so does a difference not listed.
#[test]
fn its_list_is_held_both_ways() {
    listed::held_both_ways(LISTED, "tags");
}
