//! Loading a specification (DISL section 14.1): the parse, the version, the names, the inheritance and the
//! compiling of every expression in its context, and the typed view of the specification that comes of it.

pub mod diagnostic;
pub(crate) mod inheritance;
pub mod loader;
pub mod metamodel;
pub(crate) mod names;
pub mod specification;
pub(crate) mod walker;
