//! What a host shows of a diagram, derived from the specification (DISL sections 7 and 8): the findings of its
//! constraints, the refusals of a gesture, the palette, the context menus and the property rows, with what every
//! derivation shares.

pub mod constraints;
pub mod context_menu;
pub mod env;
pub(crate) mod evaluation;
pub mod finding;
pub mod form_commit;
pub mod forms;
pub mod gestures;
pub mod toolbox;
pub mod wire_ids;
