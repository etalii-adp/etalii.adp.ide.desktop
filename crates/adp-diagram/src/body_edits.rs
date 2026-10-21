//! The edit side of a body read and written through an FBL binding: where a span lies in lines, and planned edits
//! applied together (etalii.adp spec 030, FR-031, SC-006); ported from the standalone host's
//! `EtAlii.Adp.Diagram.Modules/FblBodyEdits.cs` at `8b6ccfc`.
//!
//! Applying a change reads the whole body again, so N changes one by one cost N readings, which made arranging a
//! large graph take seconds. Edits planned against one body, each touching its own entry alone, are applied as one
//! when their splices keep apart and none needs a snapshot to undo; [`combine`] says whether they can be. The
//! documents of this crate plan a transaction change after change instead (`DiagramDocument::plan`), which is what
//! the standalone host fell back to whenever the edits could not be combined.

use adp_fbl::PlannedEdit;
use adp_fbl::text::{BodyText, Span};

/// The 0-based first and last lines `span` covers in `bytes`; a span counts UTF-8 bytes.
pub fn lines_of(bytes: &[u8], span: Span) -> (usize, usize) {
    let text = BodyText::new(bytes.to_vec());
    (
        text.line_index_at(span.start),
        text.line_index_at(span.start.max(span.end.saturating_sub(1))),
    )
}

/// The edits of a batch, each planned against the same body, as one edit: when no two of them touch and none needs a
/// snapshot to undo; else none, and they are applied one by one.
pub fn combine(edits: &[PlannedEdit]) -> Option<PlannedEdit> {
    let mut edits: Vec<&PlannedEdit> = edits.iter().filter(|e| !e.splices.is_empty()).collect();
    if edits.is_empty() || edits.iter().any(|e| e.snapshot) {
        return None;
    }
    let start = |e: &PlannedEdit| e.splices.iter().map(|s| s.start).min().unwrap_or_default();
    let end = |e: &PlannedEdit| e.splices.iter().map(|s| s.end).max().unwrap_or_default();
    edits.sort_by_key(|e| start(e));
    if edits.windows(2).any(|pair| end(pair[0]) >= start(pair[1])) {
        return None;
    }
    Some(PlannedEdit {
        splices: edits.iter().flat_map(|e| e.splices.iter().cloned()).collect(),
        snapshot: false,
    })
}

#[cfg(test)]
mod tests {
    use adp_fbl::{Operation, Splice};

    use super::*;

    fn edit(start: usize, end: usize, text: &str) -> PlannedEdit {
        PlannedEdit {
            splices: vec![Splice {
                operation: Operation::ReplaceValue,
                start,
                end,
                text: text.into(),
            }],
            snapshot: false,
        }
    }

    #[test]
    fn a_span_lies_on_the_lines_it_covers() {
        let bytes = b"one\ntwo\nthree\n";
        assert_eq!(lines_of(bytes, Span::new(4, 7)), (1, 1));
        assert_eq!(
            lines_of(bytes, Span::new(4, 8)),
            (1, 1),
            "a span ending on the line break stays on its line"
        );
        assert_eq!(lines_of(bytes, Span::new(2, 10)), (0, 2));
        assert_eq!(lines_of(bytes, Span::new(5, 5)), (1, 1));
    }

    #[test]
    fn edits_that_keep_apart_are_one_edit_in_body_order() {
        let combined = combine(&[edit(10, 12, "b"), edit(0, 2, "a")]).expect("combined");
        let starts: Vec<usize> = combined.splices.iter().map(|s| s.start).collect();
        assert_eq!(starts, [0, 10]);
    }

    #[test]
    fn edits_that_touch_or_need_a_snapshot_are_not_combined() {
        assert!(combine(&[edit(0, 2, "a"), edit(2, 4, "b")]).is_none());
        let mut snapshot = edit(0, 2, "a");
        snapshot.snapshot = true;
        assert!(combine(&[snapshot, edit(5, 6, "b")]).is_none());
        assert!(combine(&[]).is_none());
    }
}
