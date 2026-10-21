//! A DISL notation read for drawing (the standalone host's `canvas/library/disl/` at 8b6ccfc): the shape, marker and
//! routing catalog, a custom shape's geometry, the bowed arc of a curved line, and the compiler that turns a notation
//! into a [`crate::definition::DiagramDefinition`].
//!
//! The standalone's `disTypes.ts` types the part of a specification its client reads; here the specification is the
//! runtime's [`adp_disl::DislSpecification`], whose JSON the compiler reads key by key, and whose loader decides which
//! DISL versions are read.

pub mod bowed_arc;
pub mod catalog;
pub mod compile;
pub mod geometry_expressions;

pub use compile::{CompileError, compile_notation};
