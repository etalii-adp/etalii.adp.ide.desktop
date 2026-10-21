//! The layout seam (etalii.adp spec 030, FR-018), ported from the standalone host's client canvas library at
//! `8b6ccfc`: `src/client/src/canvas/library/layout/layoutAlgorithm.ts` and the layout part of
//! `definition/diagramDefinition.ts` (diagram-library Requirement 8).
//!
//! One layout mode answers behind one seam, [`LayoutAlgorithm`]: the canvas asks for the active mode's algorithm and
//! applies whatever comes back, so a further algorithm is an addition to [`LAYOUT_ALGORITHMS`], never a change to the
//! canvas. A mode with no algorithm lays out as manual: the honest fallback, because inventing placements for an
//! unimplemented mode would be worse than leaving the model's own.

use std::collections::BTreeMap;

use crate::row_packed::RowPackedLayout;
use crate::tiered_force::TieredForceLayout;
use crate::tree::TreeLayout;

/// The layout modes a definition may allow (diagram-library Requirement 8.1), by the names the standalone host's
/// definitions use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayoutMode {
    /// `manual`: the model's own positions.
    Manual,
    /// `horizontal-flow`: declared, with no algorithm at the reference commit.
    HorizontalFlow,
    /// `vertical-flow`: declared, with no algorithm at the reference commit.
    VerticalFlow,
    /// `tree`: a hierarchy fanning out along [`LayoutDefinition::tree_direction`].
    Tree,
    /// `layered-graph`: declared, with no algorithm at the reference commit.
    LayeredGraph,
    /// `row-packed`: one declared width, packed along the rows the elements already sit on.
    RowPacked,
    /// `tiered-force`: rings by element type, drawn together by connections and held apart.
    TieredForce,
}

impl LayoutMode {
    /// Every mode, in the order the standalone host declares them.
    pub const ALL: [LayoutMode; 7] = [
        LayoutMode::Manual,
        LayoutMode::HorizontalFlow,
        LayoutMode::VerticalFlow,
        LayoutMode::Tree,
        LayoutMode::LayeredGraph,
        LayoutMode::RowPacked,
        LayoutMode::TieredForce,
    ];

    /// The name a definition writes the mode with.
    pub fn name(self) -> &'static str {
        match self {
            LayoutMode::Manual => "manual",
            LayoutMode::HorizontalFlow => "horizontal-flow",
            LayoutMode::VerticalFlow => "vertical-flow",
            LayoutMode::Tree => "tree",
            LayoutMode::LayeredGraph => "layered-graph",
            LayoutMode::RowPacked => "row-packed",
            LayoutMode::TieredForce => "tiered-force",
        }
    }

    /// The mode a definition names, or `None` for a name no mode has.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.name() == name)
    }
}

/// The direction a `tree` mode grows in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TreeDirection {
    #[default]
    LeftToRight,
    RightToLeft,
    TopDown,
    BottomUp,
}

/// The width the `row-packed` mode draws an element of the listed types with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RowPackedWidth {
    /// One width for every such element.
    Fixed(f64),
    /// A width bound per element, resolved by the caller into [`LayoutElement::packed_width`]; an element whose binding
    /// yields nothing keeps its own width.
    Bound,
}

/// What the `row-packed` mode draws every element as.
#[derive(Debug, Clone, PartialEq)]
pub struct RowPackedDeclaration {
    /// The width every element of the listed types is drawn with; every element's, without [`Self::types`].
    pub width: RowPackedWidth,
    /// The least space between two elements on one row.
    pub gap: f64,
    /// The element types drawn at [`Self::width`]. An element of any other type keeps its own width and height, and is
    /// still placed in the order of its left edge and clear of its neighbours on every row it covers. `None`, every
    /// element takes the width.
    pub types: Option<Vec<String>>,
    /// The distance between two row lines. Declared, an element occupies every row line from `floor(top / row_step)` to
    /// `floor((bottom - 1) / row_step)`, so one two rows tall keeps both clear; `None`, an element's row is its
    /// centre's height, as for elements one row tall.
    pub row_step: Option<f64>,
    /// Whether connections spread the packing: an element starts no earlier than halfway along every element already
    /// placed that connects into it, plus [`Self::gap`], so causes stand to the left of their effects rather than
    /// stacked in one column.
    pub follow_connections: bool,
}

impl RowPackedDeclaration {
    /// One fixed width for every element, with `gap` between neighbours, and nothing else declared.
    pub fn new(width: f64, gap: f64) -> Self {
        Self {
            width: RowPackedWidth::Fixed(width),
            gap,
            types: None,
            row_step: None,
            follow_connections: false,
        }
    }
}

/// The part of a diagram definition the layouts read.
///
/// The standalone host's `LayoutDefinition` also carries what a drag means under an automatic mode, the path to an
/// element's pinned flag, per-mode overrides of the definition and the mode toggle; those are the canvas's, and arrive
/// here only as [`LayoutElement::pinned`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LayoutDefinition {
    /// The allowed modes; the first is the default.
    pub modes: Vec<LayoutMode>,
    /// The rings of the `tiered-force` mode, from the centre outwards: each entry lists the element types on one ring.
    /// A type no entry names is placed on a ring beyond the last.
    pub tiers: Vec<Vec<String>>,
    /// The direction a `tree` mode grows in; left to right when `None`.
    pub tree_direction: Option<TreeDirection>,
    /// What the `row-packed` mode draws every element as; without it that mode lays out as manual.
    pub row_packed: Option<RowPackedDeclaration>,
}

impl LayoutDefinition {
    /// A definition allowing `modes` and declaring nothing else.
    pub fn with_modes(modes: &[LayoutMode]) -> Self {
        Self {
            modes: modes.to_vec(),
            ..Self::default()
        }
    }

    /// The mode a diagram opens in: the first allowed, or manual when none is.
    pub fn default_mode(&self) -> LayoutMode {
        self.modes.first().copied().unwrap_or(LayoutMode::Manual)
    }
}

/// One element as a layout pass reads it. `x` and `y` are its centre.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LayoutElement {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// The tree parent, where the notation nests structurally.
    pub parent_id: Option<String>,
    /// The element's type, for a layout that treats types differently.
    pub element_type: Option<String>,
    /// The reader put this element where it is, and it stays there: a layout returns no placement for it, and may treat
    /// it as a fixed point the others arrange themselves around.
    pub pinned: bool,
    /// The width a packing layout draws this element with, where the layout declares one per element: the declared
    /// width resolved for it. `None`, the layout's own width applies.
    pub packed_width: Option<f64>,
    /// The room this element needs clear before its left edge, for a label drawn to its left. A layout that packs
    /// elements side by side keeps it free of the element before; `None`, none.
    pub leading: Option<f64>,
}

impl LayoutElement {
    /// An element with its id, centre and size, and nothing else stated.
    pub fn new(id: impl Into<String>, x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            id: id.into(),
            x,
            y,
            width,
            height,
            ..Self::default()
        }
    }
}

/// One connection as a layout pass reads it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LayoutConnection {
    pub source_id: String,
    pub target_id: String,
}

impl LayoutConnection {
    pub fn new(source_id: impl Into<String>, target_id: impl Into<String>) -> Self {
        Self {
            source_id: source_id.into(),
            target_id: target_id.into(),
        }
    }
}

/// What a layout pass reads: positions, sizes, and the two structures a hierarchy can hang on.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LayoutInput {
    pub elements: Vec<LayoutElement>,
    pub connections: Vec<LayoutConnection>,
}

/// A point in a canvas's own units.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// One element's placement: its centre, and its width where the layout decides that too.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LayoutPlacement {
    pub x: f64,
    pub y: f64,
    pub width: Option<f64>,
}

impl LayoutPlacement {
    pub const fn at(x: f64, y: f64) -> Self {
        Self { x, y, width: None }
    }
}

/// Where each element goes, by id.
pub type Placements = BTreeMap<String, LayoutPlacement>;

/// One layout mode behind one seam (diagram-library Requirement 8.1).
pub trait LayoutAlgorithm: Sync {
    /// The mode this algorithm answers for.
    fn mode(&self) -> LayoutMode;

    /// Where each element goes, or `None` for the model's own positions, untouched.
    ///
    /// `previous` is what this algorithm returned the last time, where the canvas has it: a layout that settles by
    /// steps starts from there, so one change to the model moves the rest a little and not everything from nothing.
    /// It is absent on a first pass, which is what makes the picture a reopened document gives depend on the document
    /// alone.
    fn place(
        &self,
        input: &LayoutInput,
        definition: &LayoutDefinition,
        previous: Option<&Placements>,
    ) -> Option<Placements>;

    /// What a pass of this layout depends on, as text. An algorithm that states it is asked again only when it
    /// changes, and is then handed its previous placements to start from; one without it is asked on every change of
    /// the model.
    fn signature(&self, _input: &LayoutInput) -> Option<String> {
        None
    }

    /// The point in the model's own space that `point`, in the placed space, stands for: what a drop under this layout
    /// means to a module that places by the model's positions. `None` leaves a drop's position as the pointer's.
    fn inverse(&self, _point: Point, _input: &LayoutInput, _definition: &LayoutDefinition) -> Option<Point> {
        None
    }
}

/// Manual: the model's own positions, untouched, and the default for any mode no algorithm answers for. It returns
/// `None` rather than a copied map, which keeps "untouched" literal: there is no second position store to drift from
/// the model.
#[derive(Debug, Clone, Copy, Default)]
pub struct ManualLayout;

impl LayoutAlgorithm for ManualLayout {
    fn mode(&self) -> LayoutMode {
        LayoutMode::Manual
    }

    fn place(
        &self,
        _input: &LayoutInput,
        _definition: &LayoutDefinition,
        _previous: Option<&Placements>,
    ) -> Option<Placements> {
        None
    }
}

/// The registered algorithms. `horizontal-flow`, `vertical-flow` and `layered-graph` have none at the reference
/// commit, so they lay out as manual.
pub static LAYOUT_ALGORITHMS: [&dyn LayoutAlgorithm; 4] =
    [&ManualLayout, &TreeLayout, &RowPackedLayout, &TieredForceLayout];

/// The algorithm that answers for `mode`, or `None` where the mode lays out as manual for want of one.
pub fn layout_algorithm_for(mode: LayoutMode) -> Option<&'static dyn LayoutAlgorithm> {
    LAYOUT_ALGORITHMS
        .iter()
        .copied()
        .find(|algorithm| algorithm.mode() == mode)
}
