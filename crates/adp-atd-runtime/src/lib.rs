//! The runtime of a tool description (etalii.adp spec 035, research R14, R16): the capability registry; the lowering
//! onto the engines follows in the runtime's pull request.

pub mod capabilities;

pub use capabilities::{Capabilities, LayoutCapability};
