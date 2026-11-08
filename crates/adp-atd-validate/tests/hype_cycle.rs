//! The Gartner hype cycle graph description against its four cases, recorded from the module (etalii.adp spec 076,
//! T036, FR-001, FR-006): every case passes on the declared binding, on its time axis, with its change rules run.
//! A regression or a correction shows here. `ETALII_ADP` runs it on another clone of etalii.adp.

mod listed;

/// The differences of the cases, by case: none is listed.
const LISTED: &[(&str, &[&str])] = &[];

#[test]
fn the_hype_cycle_cases_pass() {
    let found = listed::differences(&listed::definitions("diagrams").join("gartner-hype-cycle-graph.atd"), 4);
    listed::assert_listed(&found, LISTED);
}

/// SC-005: a name taken off the list fails the test above, and so does a difference not listed.
#[test]
fn its_list_is_held_both_ways() {
    listed::held_both_ways(LISTED, "findings");
}
