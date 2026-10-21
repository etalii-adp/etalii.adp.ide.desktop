//! The arrangement every row canvas offers: fewest rows, nothing overlapping, linked items close (etalii.adp spec 030,
//! FR-018, task T054), ported from the standalone host's `EtAlii.Adp.Layout.Tests/RowPacking.Tests.cs` at `8b6ccfc`.

use std::collections::{BTreeMap, HashMap, HashSet};

use adp_layout::RowItem;
use adp_layout::row_packing::pack;

fn item(id: &str, left: f64, right: f64) -> RowItem {
    RowItem::new(id, left, right)
}

fn assert_no_overlap(items: &[RowItem], rows: &HashMap<String, usize>, gap: f64) {
    for (i, a) in items.iter().enumerate() {
        for b in &items[i + 1..] {
            let shares_a_row = rows[&a.id] < rows[&b.id] + b.rows && rows[&b.id] < rows[&a.id] + a.rows;
            let apart = a.right + gap <= b.left || b.right + gap <= a.left;
            assert!(!shares_a_row || apart, "{} and {} overlap on a row.", a.id, b.id);
        }
    }
}

#[test]
fn overlapping_items_take_as_many_rows_as_overlap_at_one_point_and_no_more() {
    // Arrange: at x = 15 three items overlap, and nowhere do four.
    let items = [
        item("a", 0.0, 20.0),
        item("b", 10.0, 30.0),
        item("c", 12.0, 18.0),
        item("d", 25.0, 40.0),
        item("e", 35.0, 50.0),
        item("f", 45.0, 60.0),
    ];

    // Act.
    let rows = pack(&items, &[], 0.0, &[]);

    // Assert.
    assert_eq!(rows.values().collect::<HashSet<_>>().len(), 3);
    assert_no_overlap(&items, &rows, 0.0);
}

#[test]
fn the_gap_keeps_two_items_that_would_touch_on_separate_rows() {
    // Act.
    let rows = pack(&[item("a", 0.0, 10.0), item("b", 15.0, 20.0)], &[], 8.0, &[]);

    // Assert.
    assert_ne!(rows["a"], rows["b"]);
}

#[test]
fn a_linked_chain_runs_along_one_row_even_when_another_row_is_free_first() {
    // Arrange: x and y overlap, so there are two rows; a then b then c follow each other and are linked, so they belong
    // on one row, whichever row a took.
    let items = [
        item("x", 0.0, 10.0),
        item("a", 0.0, 10.0),
        item("b", 20.0, 30.0),
        item("c", 40.0, 50.0),
        item("y", 20.0, 30.0),
    ];

    // Act.
    let rows = pack(&items, &[("a", "b"), ("b", "c")], 4.0, &[]);

    // Assert.
    assert_eq!(rows["a"], rows["b"]);
    assert_eq!(rows["b"], rows["c"]);
    assert_no_overlap(&items, &rows, 4.0);
}

#[test]
fn a_pair_kept_apart_takes_the_neighbouring_row_not_the_same_one() {
    // Arrange: m then a, linked and following each other, so they would share a row, but the link would run through
    // m's label, so they are kept apart. Two other items open two rows.
    let items = [
        item("m", 0.0, 10.0),
        item("x", 0.0, 10.0),
        item("y", 0.0, 10.0),
        item("a", 20.0, 30.0),
    ];

    // Act.
    let rows = pack(&items, &[("m", "a")], 4.0, &[("m", "a")]);

    // Assert.
    assert_eq!(rows["m"].abs_diff(rows["a"]), 1);
}

#[test]
fn rows_swap_so_linked_items_end_up_on_neighbouring_rows() {
    // Arrange: three stacked items at the start; a later item linked to the one packed lowest.
    let items = [
        item("p", 0.0, 10.0),
        item("q", 0.0, 10.0),
        item("r", 0.0, 10.0),
        item("later", 0.0, 10.0),
    ];

    // Act: "later" overlaps all three and opens a fourth row; it is linked to "p", on row 0.
    let rows = pack(&items, &[("p", "later")], 0.0, &[]);

    // Assert.
    assert_eq!(rows["p"].abs_diff(rows["later"]), 1);
    assert_no_overlap(&items, &rows, 0.0);
}

#[test]
fn a_tall_item_keeps_every_row_it_covers_clear() {
    // Arrange: a two-row note, and two items under it in time.
    let items = [
        RowItem::tall("note", 0.0, 50.0, 2),
        item("a", 10.0, 20.0),
        item("b", 30.0, 40.0),
    ];

    // Act.
    let rows = pack(&items, &[], 0.0, &[]);

    // Assert.
    assert_no_overlap(&items, &rows, 0.0);
}

#[test]
fn packing_a_packed_arrangement_changes_nothing() {
    // Arrange.
    let items = [
        item("a", 0.0, 20.0),
        item("b", 10.0, 30.0),
        item("c", 25.0, 40.0),
        item("d", 5.0, 8.0),
        item("e", 45.0, 60.0),
    ];
    let links = [("a", "c"), ("b", "e")];
    let first = pack(&items, &links, 2.0, &[]);

    // Act: the same items again, now listed by their rows.
    let mut by_row = items.to_vec();
    by_row.sort_by_key(|item| first[&item.id]);
    let again = pack(&by_row, &links, 2.0, &[]);

    // Assert.
    let sorted = |rows: HashMap<String, usize>| rows.into_iter().collect::<BTreeMap<_, _>>();
    assert_eq!(sorted(first), sorted(again));
}
