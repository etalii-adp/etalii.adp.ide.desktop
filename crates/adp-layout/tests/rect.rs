//! The rectangle (etalii.adp spec 030, FR-018, task T054), ported from the standalone host's
//! `EtAlii.Adp.Layout.Tests/Rect.Tests.cs` at `8b6ccfc`.

use adp_layout::Rect;

#[test]
fn the_derived_edges_follow_the_position_and_size() {
    // Arrange.
    let rect = Rect::new(10.0, 20.0, 30.0, 40.0);

    // Act and assert.
    assert_eq!(rect.right(), 40.0);
    assert_eq!(rect.bottom(), 60.0);
    assert_eq!(rect.center_x(), 25.0);
    assert_eq!(rect.center_y(), 40.0);
}

#[test]
fn overlaps_only_where_they_share_area() {
    let cases = [
        (5.0, 5.0, true),
        (10.0, 0.0, false),
        (0.0, 10.0, false),
        (-10.0, 0.0, false),
        (9.5, 9.5, true),
    ];
    for (x, y, expected) in cases {
        // Arrange: two 10 by 10 squares; touching edges share no area.
        let first = Rect::new(0.0, 0.0, 10.0, 10.0);
        let second = Rect::new(x, y, 10.0, 10.0);

        // Act and assert, both ways round.
        assert_eq!(first.overlaps(&second), expected, "({x}, {y})");
        assert_eq!(second.overlaps(&first), expected, "({x}, {y})");
    }
}

#[test]
fn contains_only_what_lies_entirely_inside() {
    let cases = [
        (10.0, 10.0, 80.0, 80.0, true),
        (0.0, 0.0, 100.0, 100.0, true),
        (-1.0, 10.0, 80.0, 80.0, false),
        (10.0, 10.0, 91.0, 80.0, false),
        (10.0, 10.0, 80.0, 91.0, false),
    ];
    for (x, y, width, height, expected) in cases {
        // Arrange.
        let outer = Rect::new(0.0, 0.0, 100.0, 100.0);

        // Act and assert.
        assert_eq!(
            outer.contains(&Rect::new(x, y, width, height)),
            expected,
            "({x}, {y}, {width}, {height})"
        );
    }
}
