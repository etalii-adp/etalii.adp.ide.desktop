//! The core's handle per window, its event channel and threads (etalii.adp spec 030, contracts/core-shell.md), and
//! the desktop host's per-user state and its tool-type catalogue.

pub mod catalogue;
pub mod definitions;
pub mod state;
mod workspace;

pub use catalogue::{Catalogue, ToolKind, ToolState, ToolType};
pub use workspace::{ActionProvider, Event, Model, Outcome, ProviderContext, Workspace};
