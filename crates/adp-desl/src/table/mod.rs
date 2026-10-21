//! The table a designer of the `table` kind lays out (DESL sections 7.2 to 7.6): the [`model`] that crosses to the
//! shell, the [`content`] a designer module reads its document into, the view [`engine`] with its [`groups`] and
//! [`values`], the [`mapper`] from content to model, the [`reading`] of a body by the surface's roles, and the
//! [`session`] a connection holds.

pub mod content;
pub mod engine;
pub mod groups;
pub mod mapper;
pub mod model;
pub mod reading;
pub mod session;
pub mod values;
