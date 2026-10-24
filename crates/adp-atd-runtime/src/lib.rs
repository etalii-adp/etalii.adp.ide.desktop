//! The runtime of a tool description (etalii.adp spec 035, research R14, R16): the capability registry, and the
//! lowering of a checked description onto the engines that drive today's tools.

pub mod capabilities;
pub mod lowering;
pub mod tool;

pub use capabilities::{Capabilities, LayoutCapability};
pub use lowering::{LoadedDiagram, load_diagram, lower_diagram};
pub use tool::{RuntimeHost, RuntimeTool, diagram_tool};
