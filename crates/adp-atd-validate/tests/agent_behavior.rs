//! The agent behavior modelling description against its five cases, recorded from the module (etalii.adp spec 076,
//! T003, FR-001, FR-006): every case passes on the declared binding, apart from the differences listed here by name.
//! A regression or a correction shows here. `ETALII_ADP` runs it on another clone of etalii.adp.

mod listed;

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
    let found = listed::differences(&listed::definitions("diagrams").join("agent-behavior-modelling.atd"), 5);
    listed::assert_listed(&found, LISTED);
}

/// SC-005: a name taken off the list fails the test above, and so does a difference not listed.
#[test]
fn its_list_is_held_both_ways() {
    listed::held_both_ways(LISTED, "findings");
}
