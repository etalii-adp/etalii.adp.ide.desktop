//! Loading a DESL specification (DESL section 10.1): the [`loader`] in its steps, what it reports
//! ([`diagnostic`]), the specification's [`reading`] into the types of [`specification`].

pub mod diagnostic;
pub mod loader;
pub(crate) mod reading;
pub mod specification;
