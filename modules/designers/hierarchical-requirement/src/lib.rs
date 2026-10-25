//! The Hierarchical Requirement Designer (etalii.adp spec 036, after the standalone host's
//! `src/designers/hierarchical-requirement` at a7fdf03): the requirements of one subject as items and requirements in
//! one Markdown file - a DED definition of the designer type `etalii/hierarchical-requirement`, read and written
//! through the vendored `hierarchical-requirement.fbl` and laid out on the `hierarchy` surface as
//! `hierarchical-requirement.des` says.
//!
//! - [`vocabulary`] and [`definition`]: the specification's words, written out, and what the vendored definition
//!   says that the binding does not carry.
//! - [`model`] and [`body`]: the document, and how a Markdown file and its registration are read into it.
//! - [`mapper`]: the document onto the host-neutral `adp_desl::hierarchy` surface.
//! - [`session`], [`gestures`] and [`documents`]: one open document on one connection, the operations a gesture
//!   names, and how its two files are read and written.

pub mod body;
pub mod definition;
pub mod documents;
pub mod gestures;
pub mod mapper;
pub mod model;
pub mod session;
pub mod vocabulary;

pub use definition::ORIGIN;
