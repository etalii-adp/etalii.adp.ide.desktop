//! Documents and tabs: open, unsaved state, save, reconcile (etalii.adp spec 030, FR-014, FR-023, task T027).

pub mod reconcile;
pub mod tabs;
pub mod text;

pub use reconcile::{Reconciliation, Resolution, SelfWriteGuard, reconcile_text};
pub use tabs::{Closing, Tab, Tabs};
pub use text::TextFile;
