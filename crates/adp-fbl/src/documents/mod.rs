//! FBL documents (FBL sections 2, 3 and 14.1; the standalone host's `Documents`): the typed form of a binding, and
//! the loader that maps a document's JSON onto it, checking what can be checked without a DISL specification.

pub mod loader;
pub mod types;

pub use loader::{LoadProblem, ProblemSeverity, load, load_file, resolve_reference};
pub use types::*;
