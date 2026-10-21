//! The `row-packed` layout mode (etalii.adp spec 030, FR-018), ported from the standalone host's
//! `src/client/src/canvas/library/layout/rowPackedLayout.ts` at `8b6ccfc`.
//!
//! Every element drawn at one declared width, packed along the rows it already sits on in the order of its manual
//! horizontal position. Built for diagrams whose x means time, to be read for their order rather than their durations.
//! A definition allowing the mode without declaring [`LayoutDefinition::row_packed`] lays out as manual, the honest
//! fallback.

use std::collections::{HashMap, HashSet};

use crate::algorithm::{
    LayoutAlgorithm, LayoutDefinition, LayoutElement, LayoutInput, LayoutMode, LayoutPlacement, Placements, Point,
    RowPackedDeclaration, RowPackedWidth,
};
use crate::{number_cmp, ordinal_cmp};

/// The `row-packed` mode.
#[derive(Debug, Clone, Copy, Default)]
pub struct RowPackedLayout;

impl LayoutAlgorithm for RowPackedLayout {
    fn mode(&self) -> LayoutMode {
        LayoutMode::RowPacked
    }

    fn place(
        &self,
        input: &LayoutInput,
        definition: &LayoutDefinition,
        _previous: Option<&Placements>,
    ) -> Option<Placements> {
        let declaration = definition.row_packed.as_ref()?;
        let mut positions = Placements::new();
        for packed in pack(input, declaration) {
            let element = packed.element;
            let width = width_of(element, declaration);
            let x = packed.placed_left + width / 2.0;
            let placement = LayoutPlacement {
                x,
                y: element.y,
                width: takes_width(element, declaration).then_some(width),
            };
            positions.insert(element.id.clone(), placement);
        }
        Some(positions)
    }

    /// A placed-space x read as a left edge, mapped to the manual left edge it stands for: between the two placed
    /// neighbours it falls between, in proportion; beyond either end, at the same distance from the nearest one; with
    /// nothing placed, unchanged. The row is kept as it is.
    fn inverse(&self, point: Point, input: &LayoutInput, definition: &LayoutDefinition) -> Option<Point> {
        let Some(declaration) = definition.row_packed.as_ref() else {
            return Some(point);
        };
        let packed = pack(input, declaration);
        if packed.is_empty() {
            return Some(point);
        }

        let mut before: Option<&Packed> = None;
        let mut after: Option<&Packed> = None;
        for entry in &packed {
            if entry.placed_left <= point.x {
                before = Some(entry);
            } else {
                after = Some(entry);
                break;
            }
        }

        let x = match (before, after) {
            (None, Some(after)) => after.manual_left - (after.placed_left - point.x),
            (Some(before), None) => before.manual_left + (point.x - before.placed_left),
            (Some(before), Some(after)) => {
                let fraction = (point.x - before.placed_left) / (after.placed_left - before.placed_left);
                before.manual_left + fraction * (after.manual_left - before.manual_left)
            }
            (None, None) => point.x,
        };
        Some(Point { x, y: point.y })
    }
}

/// One element as the packing placed it: its left edge in both spaces.
struct Packed<'a> {
    element: &'a LayoutElement,
    /// The left edge in the model's own space: the element's manual position.
    manual_left: f64,
    /// The left edge the packing gave it.
    placed_left: f64,
}

/// Whether the declared width applies to this element: to every element when no types are listed.
fn takes_width(element: &LayoutElement, declaration: &RowPackedDeclaration) -> bool {
    match &declaration.types {
        None => true,
        Some(types) => element.element_type.as_ref().is_some_and(|kind| types.contains(kind)),
    }
}

/// The width an element is placed with: the declared one (its own share where the width is bound) or its own.
fn width_of(element: &LayoutElement, declaration: &RowPackedDeclaration) -> f64 {
    if !takes_width(element, declaration) {
        return element.width;
    }

    element.packed_width.unwrap_or(match declaration.width {
        RowPackedWidth::Fixed(width) => width,
        RowPackedWidth::Bound => element.width,
    })
}

/// A row as a key: its number's bits, with `-0` read as `0`, as a JavaScript `Map` keys numbers.
fn row_key(row: f64) -> u64 {
    if row == 0.0 { 0.0f64.to_bits() } else { row.to_bits() }
}

/// The rows an element occupies: every row line its top-to-bottom span crosses where a row step is declared, and
/// otherwise the one its centre sits on.
fn rows_of(element: &LayoutElement, declaration: &RowPackedDeclaration) -> Vec<u64> {
    let Some(step) = declaration.row_step else {
        return vec![row_key(element.y)];
    };
    let first = ((element.y - element.height / 2.0) / step).floor();
    let last = first.max(((element.y + element.height / 2.0 - 1.0) / step).floor());
    let count = (last - first) as usize + 1;
    (0..count).map(|index| row_key(first + index as f64)).collect()
}

/// The one packing both directions read: every element at its placed width (see [`width_of`]), in the order of its
/// manual left edge, on the rows it covers (see [`rows_of`]).
///
/// Elements starting at one manual x are placed as a group. The group's x is the larger of where the previous group
/// ended up and where each row it touches is free again (its last element's right edge plus the gap, plus the room the
/// element needs before it for a label drawn to its left), so a later start is never left of an earlier one, equal
/// starts on different rows share an x, and no two elements on a row come closer than the gap. A second element of the
/// group on the same row follows the first on that row. Nothing further left satisfies all three, which is what makes
/// the result as narrow as they allow.
///
/// The first group keeps its own manual x, so a packed diagram starts where its manual one does. The result is in
/// placement order, which is also placed-left order.
fn pack<'a>(input: &'a LayoutInput, declaration: &RowPackedDeclaration) -> Vec<Packed<'a>> {
    let mut by_start: Vec<(&LayoutElement, f64)> = input
        .elements
        .iter()
        .map(|element| (element, element.x - element.width / 2.0))
        .collect();
    by_start.sort_by(|(first, first_left), (second, second_left)| {
        number_cmp(*first_left, *second_left).then_with(|| ordinal_cmp(&first.id, &second.id))
    });

    let gap = declaration.gap;
    let mut row_end: HashMap<u64, f64> = HashMap::new();
    let mut packed: Vec<Packed<'a>> = Vec::new();
    let mut placed_by_id: HashMap<&str, usize> = HashMap::new();
    // Where each element's causes are, when connections spread the packing.
    let mut sources_of: HashMap<&str, Vec<&str>> = HashMap::new();
    if declaration.follow_connections {
        for connection in &input.connections {
            sources_of
                .entry(connection.target_id.as_str())
                .or_default()
                .push(connection.source_id.as_str());
        }
    }

    let mut previous: Option<f64> = None;
    let mut start = 0;
    while start < by_start.len() {
        let mut end = start;
        while end < by_start.len() && by_start[end].1 == by_start[start].1 {
            end += 1;
        }
        let group = &by_start[start..end];

        let mut group_x = previous.unwrap_or(group[0].1);
        for &(element, _) in group {
            let leading = element.leading.unwrap_or(0.0);
            // After the middle of every cause already placed: effects stand to the right of causes.
            for source_id in sources_of.get(element.id.as_str()).map(Vec::as_slice).unwrap_or(&[]) {
                if let Some(&index) = placed_by_id.get(source_id) {
                    let source = &packed[index];
                    let middle = source.placed_left + width_of(source.element, declaration) / 2.0;
                    group_x = group_x.max(middle + gap + leading);
                }
            }
            for row in rows_of(element, declaration) {
                if let Some(&free) = row_end.get(&row) {
                    group_x = group_x.max(free + gap + leading);
                }
            }
        }

        // A later member of the group follows an earlier one on any row they share.
        let mut placed_rows: HashSet<u64> = HashSet::new();
        let mut rightmost = group_x;
        for &(element, manual_left) in group {
            let rows = rows_of(element, declaration);
            let leading = element.leading.unwrap_or(0.0);
            let mut placed_left = group_x;
            for row in &rows {
                if placed_rows.contains(row) {
                    placed_left = placed_left.max(row_end[row] + gap + leading);
                }
            }
            let right = placed_left + width_of(element, declaration);
            for row in rows {
                placed_rows.insert(row);
                row_end.insert(row, right);
            }
            rightmost = rightmost.max(placed_left);
            placed_by_id.insert(element.id.as_str(), packed.len());
            packed.push(Packed {
                element,
                manual_left,
                placed_left,
            });
        }

        previous = Some(rightmost);
        start = end;
    }
    packed
}
