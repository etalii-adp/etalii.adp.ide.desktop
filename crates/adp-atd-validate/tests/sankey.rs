//! The Sankey description against its case, recorded from the module (etalii.adp spec 035, T132): the bars and bands
//! laid out by `layout.flowRelaxation`, Arrange diagram by `layout.crossingOrder`, a dropped node written before the
//! first node below it in its column, a node dragged within its column reordered by its `move` hook, and a drag by a
//! distance counted from a node's centre, as the module counts it (ATD §2.3). Nothing is listed. `ETALII_ADP` runs it
//! on another clone.

mod listed;

/// The differences of the case, by case: none is listed.
const LISTED: &[(&str, &[&str])] = &[];

#[test]
fn the_sankey_case_passes() {
    let found = listed::differences(&listed::definitions("diagrams").join("sankey.atd"), 1);
    listed::assert_listed(&found, LISTED);
}

/// SC-005: a name taken off the list fails the test above, and so does a difference not listed.
#[test]
fn its_list_is_held_both_ways() {
    listed::held_both_ways(LISTED, "brightwater-coffee");
}
