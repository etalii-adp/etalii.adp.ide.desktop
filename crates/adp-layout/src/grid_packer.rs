//! The grid a layout fills row by row (etalii.adp spec 030, FR-018), ported from the standalone host's
//! `EtAlii.Adp.Layout/GridPacker.cs` at `8b6ccfc` (code-structure FR-045): a fixed number of items to a row, for the
//! elements a layout has no better place for.

use crate::rect::Rect;

/// Where a grid filled row by row put each item, and how much room the grid takes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GridPacking {
    /// Each item's box, in the order given, measured from the grid's top-left corner.
    pub cells: Vec<Rect>,
    /// The widest row, from its first item's left edge to its last item's right edge.
    pub width: f64,
    /// The rows' heights and the gaps between them; nothing below the last row.
    pub height: f64,
}

/// The top-left corner of the item at `index` in a grid whose cells are all one size: the whole row and column of the
/// index, never a fraction, so a grid is never a diagonal.
///
/// `columns` is how many items a row holds, `pitch_x` the distance from one column's left edge to the next and
/// `pitch_y` the distance from one row's top edge to the next.
///
/// # Panics
///
/// When `columns` is zero.
pub fn cell(index: usize, columns: usize, pitch_x: f64, pitch_y: f64) -> (f64, f64) {
    assert!(columns >= 1, "a grid holds at least one item to a row");
    let (row, column) = (index / columns, index % columns);
    (column as f64 * pitch_x, row as f64 * pitch_y)
}

/// How many rows `count` items fill, a part-filled last row included.
///
/// # Panics
///
/// When `columns` is zero.
pub fn rows(count: usize, columns: usize) -> usize {
    assert!(columns >= 1, "a grid holds at least one item to a row");
    count.div_ceil(columns)
}

/// A grid of items of their own sizes: each item to the right of the one before it on its row, and each row as tall as
/// its tallest item.
///
/// `sizes` holds each item's width and height in the order the grid is filled, `columns` how many items a row holds,
/// `gap_x` the clear space between two items on a row and `gap_y` the clear space between two rows.
///
/// # Panics
///
/// When `columns` is zero.
pub fn pack(sizes: &[(f64, f64)], columns: usize, gap_x: f64, gap_y: f64) -> GridPacking {
    assert!(columns >= 1, "a grid holds at least one item to a row");

    let mut cells = Vec::with_capacity(sizes.len());
    let mut x = 0.0;
    let mut y = 0.0;
    let mut width: f64 = 0.0;
    let mut row_height: f64 = 0.0;
    for (index, &(item_width, item_height)) in sizes.iter().enumerate() {
        if index > 0 && index % columns == 0 {
            y += row_height + gap_y;
            x = 0.0;
            row_height = 0.0;
        }

        cells.push(Rect::new(x, y, item_width, item_height));
        x += item_width + gap_x;
        width = width.max(x - gap_x);
        row_height = row_height.max(item_height);
    }

    let height = if sizes.is_empty() { 0.0 } else { y + row_height };
    GridPacking { cells, width, height }
}
