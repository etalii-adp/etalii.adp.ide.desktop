//! A diagram's scene (etalii.adp spec 030, tasks T046 and T055): a DISL specification's notation, a diagram's model
//! and its elements' positions turned into a flat, ordered list of drawing primitives in diagram coordinates, and
//! that list culled to a viewport. Host-neutral: no UI toolkit; the shell paints the primitives.
//!
//! Ported from the standalone host's TypeScript canvas library at 8b6ccfc (`src/client/src/canvas/`):
//!
//! - [`notation`]: `library/disl/`, the notation compiler, the shape catalog, GeomExprs and the bowed arc;
//! - [`definition`]: `library/definition/`, what a definition states and its labels, lists, ornaments and stubs;
//! - [`shapes`]: `library/shapes/`, the one outline per shape and the arrow banner's segments;
//! - [`connectors`]: `connectors.ts`, where a line meets a box and the curve it takes;
//! - [`text`]: `label/textMetrics.ts`, behind a [`text::TextMetrics`] the shell implements;
//! - [`scene`]: what `library/DiagramCanvas.tsx` computes to draw, without React, events or the DOM.
//!
//! The specification and the model are the DISL runtime's (`adp-disl`), and every expression of a notation is
//! evaluated by its CEL ([`evaluation`]) where the standalone translated each into a template over its payload.

pub mod connectors;
pub mod definition;
pub mod evaluation;
pub mod geometry;
pub mod label_placement;
pub mod notation;
pub mod scene;
pub mod shapes;
pub mod style;
pub mod text;

pub use definition::DiagramDefinition;
pub use evaluation::Evaluator;
pub use geometry::{Path, PathCommand, Point, Rect};
pub use notation::{CompileError, compile_notation};
pub use scene::{Primitive, Scene, SceneOptions, build_scene, cull};
pub use style::Theme;
pub use text::{EstimatedMetrics, TextMetrics};
