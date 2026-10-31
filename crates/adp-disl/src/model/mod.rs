//! The model a diagram's expressions read (DISL sections 4, 11.5, 12.2 and 14.2): its elements and the diagram
//! that holds them, the values attributes take, the model built from a format binding's reading, and what the
//! specification tells a binding about ids and attribute types.

pub mod attribute_types;
pub mod budgets;
pub mod builder;
pub(crate) mod cycles;
pub mod diagram;
pub mod element;
pub(crate) mod knots;
pub mod natural_ids;
pub(crate) mod type_map;
pub(crate) mod values;
pub(crate) mod views;
pub(crate) mod written;
