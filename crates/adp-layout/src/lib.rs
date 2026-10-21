//! Layouts and geometry (etalii.adp spec 030, FR-018, task T054).
//!
//! Ported from the standalone host at `8b6ccfc`, from two places. The geometry every layout shares comes from its
//! backend's `EtAlii.Adp.Layout`: the rectangle ([`rect`]), the text width metric ([`text_metric`]), the grid
//! ([`grid_packer`]), longest-path layering and its cycle rules ([`layering`], [`cycle_policy`]), the overlap pass
//! ([`overlap_resolver`]) and the row arrangement of row canvases ([`row_packing`]). The layout modes come from its
//! client's canvas library (`src/client/src/canvas/library/layout/`): the one seam every mode answers through and the
//! manual mode ([`algorithm`]), the tree that grows in a declared direction ([`tree`]), the row-packed mode
//! ([`row_packed`]) and the radiating, deterministic force layout ([`tiered_force`]). Collapse and expand is the fold
//! state the standalone host keeps per viewer (`MindmapConnectionView`), made independent of the mind map ([`fold`]).
//!
//! Nothing here draws: a layout takes ids, sizes and links and gives back positions, so the canvas, the parity runner
//! and the tests all read the same numbers.

pub mod algorithm;
pub mod cycle_policy;
pub mod fold;
pub mod grid_packer;
pub mod layering;
pub mod overlap_resolver;
pub mod rect;
pub mod row_packed;
pub mod row_packing;
pub mod text_metric;
pub mod tiered_force;
pub mod tree;

pub use algorithm::{
    LAYOUT_ALGORITHMS, LayoutAlgorithm, LayoutConnection, LayoutDefinition, LayoutElement, LayoutInput, LayoutMode,
    LayoutPlacement, ManualLayout, Placements, Point, RowPackedDeclaration, RowPackedWidth, TreeDirection,
    layout_algorithm_for,
};
pub use cycle_policy::CyclePolicy;
pub use fold::FoldState;
pub use grid_packer::GridPacking;
pub use rect::Rect;
pub use row_packed::RowPackedLayout;
pub use row_packing::RowItem;
pub use tiered_force::TieredForceLayout;
pub use tree::TreeLayout;

use std::cmp::Ordering;

/// Two texts compared by their UTF-16 code units, as .NET's ordinal comparison and JavaScript's `<` compare them,
/// so an order that a tie falls back on is the standalone host's order for every text.
pub(crate) fn ordinal_cmp(first: &str, second: &str) -> Ordering {
    first.encode_utf16().cmp(second.encode_utf16())
}

/// Two numbers compared as .NET's and JavaScript's sorts compare them: equal when neither is less, so `-0` and `0`
/// tie.
pub(crate) fn number_cmp(first: f64, second: f64) -> Ordering {
    first.partial_cmp(&second).unwrap_or(Ordering::Equal)
}
