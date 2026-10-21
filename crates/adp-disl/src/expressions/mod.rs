//! DISL's CEL (DISL section 12): the contexts every expression is compiled in, the function library on top of CEL's
//! standard environment, the `yearMonth` and colour values it works with, and the plugins' and the specification's
//! own functions.

pub(crate) mod color;
pub mod contexts;
pub(crate) mod library;
pub mod plugins;
pub(crate) mod user_functions;
pub mod year_month;
