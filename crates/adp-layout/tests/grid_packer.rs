//! The grid (etalii.adp spec 030, FR-018, task T054), ported from the standalone host's
//! `EtAlii.Adp.Layout.Tests/GridPacker.Tests.cs` at `8b6ccfc`.

use adp_layout::Rect;
use adp_layout::grid_packer;

#[test]
fn cell_fills_the_grid_row_by_row() {
    for (index, x, y) in [(0, 0.0, 0.0), (2, 200.0, 0.0), (3, 0.0, 50.0), (7, 100.0, 100.0)] {
        // Act: three to a row, cells 100 wide and 50 tall.
        let cell = grid_packer::cell(index, 3, 100.0, 50.0);

        // Assert.
        assert_eq!(cell, (x, y), "index {index}");
    }
}

#[test]
fn rows_counts_a_part_filled_last_row() {
    for (count, rows) in [(0, 0), (1, 1), (3, 1), (4, 2)] {
        // Act.
        let actual = grid_packer::rows(count, 3);

        // Assert.
        assert_eq!(actual, rows, "count {count}");
    }
}

#[test]
fn pack_puts_each_item_beside_the_one_before_and_each_row_below_the_tallest_above() {
    // Arrange: two to a row; the first row's tallest item is 30, the second row holds one item.
    let sizes = [(10.0, 20.0), (40.0, 30.0), (50.0, 5.0)];

    // Act.
    let grid = grid_packer::pack(&sizes, 2, 5.0, 7.0);

    // Assert.
    assert_eq!(
        grid.cells,
        vec![
            Rect::new(0.0, 0.0, 10.0, 20.0),
            Rect::new(15.0, 0.0, 40.0, 30.0),
            Rect::new(0.0, 37.0, 50.0, 5.0)
        ]
    );
    assert_eq!(grid.width, 55.0);
    assert_eq!(grid.height, 42.0);
}

#[test]
fn pack_of_nothing_takes_no_room() {
    // Act.
    let grid = grid_packer::pack(&[], 3, 10.0, 10.0);

    // Assert.
    assert!(grid.cells.is_empty());
    assert_eq!(grid.width, 0.0);
    assert_eq!(grid.height, 0.0);
}
