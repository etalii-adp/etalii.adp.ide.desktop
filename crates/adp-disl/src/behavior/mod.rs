//! Interpreting a specification's behaviour (DISL section 9): operations, hooks, deletion and retype policies, the
//! actions they run against a working diagram, the ids new elements get, and the writing of the model changes they
//! make as FBL changes.

pub(crate) mod actions;
pub mod change;
pub mod deletion;
pub mod extension;
pub mod hooks;
pub mod ids;
pub mod operations;
pub mod retype;
pub mod write;
