//! The overlap pass (etalii.adp spec 030, FR-018, task T054), ported from the standalone host's
//! `EtAlii.Adp.Layout.Tests/OverlapResolver.Tests.cs` at `8b6ccfc`.

use adp_layout::overlap_resolver::separate;

#[test]
fn separate_leaves_boxes_that_are_clear_where_they_are() {
    // Arrange: two 10 by 10 boxes 20 apart, keeping 5 between them.
    let mut centres = [(0.0, 0.0), (20.0, 0.0)];
    let sizes = [(10.0, 10.0), (10.0, 10.0)];

    // Act.
    let remaining = separate(&mut centres, &sizes, 5.0, 10, 0.5);

    // Assert.
    assert_eq!(remaining, 0);
    assert_eq!(centres, [(0.0, 0.0), (20.0, 0.0)]);
}

#[test]
fn separate_pushes_a_pair_apart_along_the_axis_that_needs_least() {
    // Arrange: side by side and 4 apart, so across needs 11 more and down needs 15.
    let mut centres = [(0.0, 0.0), (4.0, 0.0)];
    let sizes = [(10.0, 10.0), (10.0, 10.0)];

    // Act.
    let remaining = separate(&mut centres, &sizes, 5.0, 10, 0.5);

    // Assert: each moved half the overlap and the nudge, across only.
    assert_eq!(remaining, 0);
    assert_eq!(centres, [(-6.0, 0.0), (10.0, 0.0)]);
}

#[test]
fn separate_reports_the_pairs_still_overlapping_when_the_rounds_run_out() {
    // Arrange: three boxes on one spot, and a single round.
    let mut centres = [(0.0, 0.0), (0.0, 0.0), (0.0, 0.0)];
    let sizes = [(10.0, 10.0), (10.0, 10.0), (10.0, 10.0)];

    // Act.
    let remaining = separate(&mut centres, &sizes, 5.0, 1, 0.5);

    // Assert.
    assert!(remaining > 0);
}
