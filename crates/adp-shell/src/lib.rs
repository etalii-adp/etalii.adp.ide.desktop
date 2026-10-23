//! The desktop host's window (etalii.adp spec 030, research R2, task T021): one native window per workspace, drawn
//! with egui on wgpu, exposed to screen readers through AccessKit, in VS Code's default theme following the system.
//!
//! The window is a library that the `adp-desktop` binary runs, so that the tests under `tests/` drive the shell as the
//! window does: the accessibility tree a frame hands the platform is read there (task T088).

mod a11y;
mod add_dialog;
mod atd_client;
mod canvas;
mod documents;
mod editor;
mod explorer;
mod highlight;
mod markdown;
mod problems;
mod properties;
mod ribbon;
/// The recording painter, for `atd record` (spec 035, T051).
pub use canvas::recording;
pub mod shell;
mod table;
// The table's tests sit beside it, declared here so that the benchmark, which includes the table module, leaves them out.
#[cfg(test)]
mod screenshot;
#[cfg(test)]
#[path = "table/tests.rs"]
mod table_tests;
pub mod theme;
