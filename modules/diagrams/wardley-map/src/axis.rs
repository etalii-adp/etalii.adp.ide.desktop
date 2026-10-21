//! The map's two axes: where the document's coordinates go on the canvas, and the four evolution stages
//! (etalii.adp spec 030, task T070).
//!
//! Ported from the standalone host's `WardleyAxis.cs` and `WardleyEvolution.cs` at `8b6ccfc`. The document writes
//! `[visibility, maturity]` and the canvas draws `(x, y)`: maturity is the horizontal axis, genesis at the left, so
//! `x = maturity`; visibility is the vertical axis with the user need at the top, and canvas y grows downward, so
//! `y = 1 - visibility`. Nothing else in the module converts. The stages' boundaries are the reference renderer's
//! `EvoOffsets` (`custom: 3.5`, `product: 8`, `commodity: 14`) over its axis width of 20.

use crate::model::WardleyCoordinate;

/// The canvas point of a document coordinate, in the map's 0..1 space.
pub fn to_point(coordinate: WardleyCoordinate) -> (f64, f64) {
    (coordinate.maturity, 1.0 - coordinate.visibility)
}

/// The document coordinate of a canvas point: the exact inverse of [`to_point`].
pub fn to_coordinate(x: f64, y: f64) -> WardleyCoordinate {
    WardleyCoordinate::new(1.0 - y, x)
}

/// A coordinate clamped into the map's bounded space: a component cannot be more evolved than commodity.
pub fn clamp(coordinate: WardleyCoordinate) -> WardleyCoordinate {
    WardleyCoordinate::new(
        coordinate.visibility.clamp(0.0, 1.0),
        coordinate.maturity.clamp(0.0, 1.0),
    )
}

/// One of the four evolution stages, with the maturity range it covers: `start` inclusive, `end` exclusive except
/// for Commodity, which includes 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WardleyEvolutionStage {
    pub label: &'static str,
    pub start: f64,
    pub end: f64,
}

/// The reference renderer's axis width, which its offsets are expressed against.
const AXIS_WIDTH: f64 = 20.0;

/// Where Genesis ends and Custom Built begins.
pub const CUSTOM_BUILT: f64 = 3.5 / AXIS_WIDTH;
/// Where Custom Built ends and Product begins.
pub const PRODUCT: f64 = 8.0 / AXIS_WIDTH;
/// Where Product ends and Commodity begins.
pub const COMMODITY: f64 = 14.0 / AXIS_WIDTH;

/// The four stages in axis order, genesis at the left, labelled with the notation's own parentheticals.
pub const STAGES: [WardleyEvolutionStage; 4] = [
    WardleyEvolutionStage {
        label: "Genesis",
        start: 0.0,
        end: CUSTOM_BUILT,
    },
    WardleyEvolutionStage {
        label: "Custom Built",
        start: CUSTOM_BUILT,
        end: PRODUCT,
    },
    WardleyEvolutionStage {
        label: "Product (+rental)",
        start: PRODUCT,
        end: COMMODITY,
    },
    WardleyEvolutionStage {
        label: "Commodity (+utility)",
        start: COMMODITY,
        end: 1.0,
    },
];

/// The stage `maturity` falls in; a value on a boundary belongs to the stage it starts, and a value outside 0..1 to
/// the nearer end's.
pub fn stage_of(maturity: f64) -> WardleyEvolutionStage {
    if maturity < CUSTOM_BUILT {
        STAGES[0]
    } else if maturity < PRODUCT {
        STAGES[1]
    } else if maturity < COMMODITY {
        STAGES[2]
    } else {
        STAGES[3]
    }
}
