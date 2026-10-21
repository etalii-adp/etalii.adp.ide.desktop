//! FBL, the Format Binding Language: families, splices, routing and the registration file (etalii.adp spec 030,
//! FR-031 to FR-033). This crate grows task by task; it starts with the splice catalogue and the edit history of FBL
//! sections 6.1, 6.5 and 7.1, which every write of every family goes through.

pub mod splice;

pub use splice::{Edit, EditHistory, Operation, Splice, SpliceError, apply};
