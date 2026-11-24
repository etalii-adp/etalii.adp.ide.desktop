//! The agent activity description against its case, recorded from the module (etalii.adp spec 087, T017): every step
//! passes, the folds of a specification's task groups and of a location's pull requests and the archive switch kept
//! in the activity file (DISL 0.11 `self.view.groups`, `view.bind`), the module's heading and switch gestures replayed
//! as the description's wire-id block names them (ATD §2.3). `ETALII_ADP` runs it on another clone.

mod listed;

/// The differences of the case, by case.
const LISTED: &[(&str, &[&str])] = &[
    // The module records on each relation whether it breaks a rule (`breach`), for its own drawing of a breached line;
    // the description says each breach as a finding where it is, so its relation types declare no attribute the case
    // states, and the relations have nothing in common to compare.
    ("two-projects", &["read:reading"]),
];

#[test]
fn the_agent_activity_case_passes() {
    let found = listed::differences(&listed::definitions("diagrams").join("agent-activity-diagram.atd"), 1);
    listed::assert_listed(&found, LISTED);
}

/// SC-005: a name taken off the list fails the test above, and so does a difference not listed.
#[test]
fn its_list_is_held_both_ways() {
    listed::held_both_ways(LISTED, "two-projects");
}
