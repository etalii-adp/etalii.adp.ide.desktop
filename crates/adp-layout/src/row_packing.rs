//! The arrangement a row canvas offers as "Arrange diagram" (etalii.adp spec 030, FR-018), ported from the standalone
//! host's `EtAlii.Adp.Layout/RowPacking.cs` at `8b6ccfc`: every item on a row of its own choosing, in as few rows as
//! the items allow, with linked items on rows close together.
//!
//! **Across is the data; only the row is ours.** On a timeline or a hype cycle an item's horizontal extent is its
//! dates, so an arrangement may change nothing but rows. That makes the problem interval partitioning, which a sweep
//! from left to right solves exactly: an item goes on a row that is free where it starts, and a new row is opened only
//! when every row is busy there, at which point that many items overlap one point, so no arrangement could use fewer.
//!
//! **Which free row is the clutter question.** Among the rows that are free, an item takes the one nearest the rows of
//! the items it is linked to that are already placed, so a chain of non-overlapping linked items runs along one row and
//! its links are drawn level. Then whole rows swap places while that shortens the links in total, which pulls linked
//! rows together without adding a row.
//!
//! **Some links want neighbouring rows, not one.** A link drawn level through a label (a timeline moment's, whose label
//! sits to its right where its outgoing line runs) is clutter, so a pair named as *apart* keeps the attraction but
//! never takes the same row: the nearest row is the one beside it, and when the shared row is the only free one, a row
//! is opened instead, the one place clarity is bought with a row.
//!
//! **Deterministic**: ties fall to the lower row and the original order, so arranging an arranged diagram changes
//! nothing.

use std::collections::{HashMap, HashSet};

use crate::{number_cmp, ordinal_cmp};

/// The most whole passes the row-swapping makes before it settles for what it has.
const SWAP_PASSES: usize = 64;

/// One thing a row canvas draws: where it starts and ends across, and how many rows it is tall.
#[derive(Debug, Clone, PartialEq)]
pub struct RowItem {
    /// Its id, unique among the items packed together.
    pub id: String,
    /// Its leftmost extent, label included, in the canvas's own x units.
    pub left: f64,
    /// Its rightmost extent, likewise.
    pub right: f64,
    /// How many rows it occupies from its top row; one for everything but a tall note.
    pub rows: usize,
}

impl RowItem {
    /// An item one row tall.
    pub fn new(id: impl Into<String>, left: f64, right: f64) -> Self {
        Self {
            id: id.into(),
            left,
            right,
            rows: 1,
        }
    }

    /// An item `rows` rows tall.
    pub fn tall(id: impl Into<String>, left: f64, right: f64, rows: usize) -> Self {
        Self {
            id: id.into(),
            left,
            right,
            rows,
        }
    }
}

/// The zero-based row of each item: no two items share a row where they overlap or come closer than `gap` across.
///
/// `items` are given in the document's order; `links` are the links between them by id, and a link naming an unknown id
/// is ignored. `apart` holds pairs, by id, that never share a row.
pub fn pack(items: &[RowItem], links: &[(&str, &str)], gap: f64, apart: &[(&str, &str)]) -> HashMap<String, usize> {
    let mut order: Vec<(RowItem, usize)> = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            (
                RowItem {
                    rows: item.rows.max(1),
                    ..item.clone()
                },
                index,
            )
        })
        .collect();
    order.sort_by(|(first, first_index), (second, second_index)| {
        number_cmp(first.left, second.left)
            .then_with(|| number_cmp(first.right, second.right))
            .then_with(|| first_index.cmp(second_index))
    });
    let order: Vec<RowItem> = order.into_iter().map(|(item, _)| item).collect();

    let known: HashSet<&str> = order.iter().map(|item| item.id.as_str()).collect();
    let mut neighbours: HashMap<String, Vec<String>> = order.iter().map(|item| (item.id.clone(), Vec::new())).collect();
    for &(from, to) in links {
        if from != to && known.contains(from) && known.contains(to) {
            neighbours.get_mut(from).expect("a known id").push(to.to_string());
            neighbours.get_mut(to).expect("a known id").push(from.to_string());
        }
    }

    let mut avoided: HashMap<String, Vec<String>> = order.iter().map(|item| (item.id.clone(), Vec::new())).collect();
    for &(first, second) in apart {
        if first != second && known.contains(first) && known.contains(second) {
            avoided.get_mut(first).expect("a known id").push(second.to_string());
            avoided.get_mut(second).expect("a known id").push(first.to_string());
        }
    }

    // Where each row is next free, across.
    let mut free_from: Vec<f64> = Vec::new();
    let mut rows: HashMap<String, usize> = HashMap::new();
    for item in &order {
        let placed: Vec<usize> = neighbours[&item.id]
            .iter()
            .filter_map(|id| rows.get(id).copied())
            .collect();
        let shunned: HashSet<usize> = avoided[&item.id]
            .iter()
            .filter_map(|id| rows.get(id).copied())
            .collect();
        let fits = |top: usize| (top..top + item.rows).all(|r| r >= free_from.len() || free_from[r] <= item.left);
        let clear = |top: usize| fits(top) && !(top..top + item.rows).any(|r| shunned.contains(&r));

        let candidates = (free_from.len() + 1).saturating_sub(item.rows);
        let existing: Vec<usize> = (0..candidates)
            .filter(|&top| fits(top))
            .filter(|&top| clear(top))
            .collect();

        let row = if existing.is_empty() {
            // Every row is busy here, or free only beside a pair kept apart: the lowest top from which the item fits
            // clear of both, opening rows below.
            (0..=free_from.len())
                .find(|&top| clear(top))
                .expect("a row past the last is always clear")
        } else if placed.is_empty() {
            existing[0]
        } else {
            let wanted = placed.iter().map(|&row| row as f64).sum::<f64>() / placed.len() as f64;
            *existing
                .iter()
                .min_by(|&&first, &&second| {
                    number_cmp((first as f64 - wanted).abs(), (second as f64 - wanted).abs()).then(first.cmp(&second))
                })
                .expect("there is a candidate")
        };

        while free_from.len() < row + item.rows {
            free_from.push(f64::NEG_INFINITY);
        }

        for slot in &mut free_from[row..row + item.rows] {
            *slot = item.right + gap;
        }

        rows.insert(item.id.clone(), row);
    }

    let row_count = free_from.len();
    swap_rows(&order, rows, &neighbours, row_count)
}

/// Swaps neighbouring rows while that makes the links shorter in total. A row touched by an item taller than one row
/// stays where it is, because swapping it would tear that item in two.
fn swap_rows(
    items: &[RowItem],
    rows: HashMap<String, usize>,
    neighbours: &HashMap<String, Vec<String>>,
    row_count: usize,
) -> HashMap<String, usize> {
    let mut pinned = HashSet::new();
    for item in items.iter().filter(|item| item.rows > 1) {
        let top = rows[&item.id];
        pinned.extend(top..top + item.rows);
    }

    // Which row each original row is drawn at now.
    let mut at: Vec<usize> = (0..row_count).collect();
    let links: Vec<(usize, usize)> = items
        .iter()
        .flat_map(|item| {
            neighbours[&item.id]
                .iter()
                .filter(|other| ordinal_cmp(&item.id, other).is_lt())
                .map(|other| (rows[&item.id], rows[other]))
        })
        .collect();
    let cost = |at: &[usize]| -> f64 { links.iter().map(|&(from, to)| at[from].abs_diff(at[to]) as f64).sum() };

    let mut current = cost(&at);
    for _ in 0..SWAP_PASSES {
        let mut improved = false;
        for position in 0..row_count.saturating_sub(1) {
            let upper = at
                .iter()
                .position(|&row| row == position)
                .expect("every position is drawn");
            let lower = at
                .iter()
                .position(|&row| row == position + 1)
                .expect("every position is drawn");
            if pinned.contains(&upper) || pinned.contains(&lower) {
                continue;
            }

            at.swap(upper, lower);
            let swapped = cost(&at);
            if swapped < current - 1e-9 {
                current = swapped;
                improved = true;
            } else {
                at.swap(upper, lower);
            }
        }

        if !improved {
            break;
        }
    }

    rows.into_iter().map(|(id, row)| (id, at[row])).collect()
}
