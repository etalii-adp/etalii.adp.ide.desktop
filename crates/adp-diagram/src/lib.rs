//! Diagram sessions: open a diagram of a DISL and FBL tool type, edit it through FBL as commands on the project's
//! history, undo, and reconcile a body changed on disk (etalii.adp spec 030, FR-015 to FR-018, FR-023, FR-031 to
//! FR-033; tasks T056 to T058; plan.md, "Project Structure": `adp-diagram`).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/backend/EtAlii.Adp.Diagram` (the session contract, deltas and
//! their diff, sibling naming, the document change handler, the reload seam, the viewport registry, validators and
//! empty document factories), `src/backend/EtAlii.Adp.Diagram.Modules` (the frames every module's session, store,
//! action and property providers and element resolver shared, `FblBodyEdits`, `DislDiagramDefinition`,
//! `RecentBodies`, `TextMemo`, the DISL toolbox), and the half of `src/backend/EtAlii.Adp.Workspace/DiagramService*.cs`
//! and `DiagramDocumentReloadBridge.cs` that opens a diagram from a body or a registration and follows changes on
//! disk. The gRPC contract's messages are plain Rust types; the shell calls the core in process.
//!
//! How the parts fit:
//!
//! - [`tool::DiagramTools`] loads a tool type of the catalogue: its specification ([`disl_definition`]), its
//!   binding, and the [`module::DiagramModule`] registered for its origin, the tool type's own code.
//! - [`workspace::DiagramWorkspace`] is one project: it opens a diagram from a registration or a body, keeps the open
//!   [`document::DiagramDocuments`] (one per body, shared by every view) and the project's history, and reconciles a
//!   change on disk, ending the history there.
//! - [`session::DiagramSession`] is one view: its baseline and viewport deltas, and the gestures, property edits and
//!   actions it plans as one [`document::DocumentEdit`] each, which the history applies, undoes and redoes as splices.
//! - [`reload::DiagramDocumentReloadBridge`] says which documents a watcher's event reaches.
//! - [`module::DiagramModule`] is what a tool type's module adds where its specification cannot say it: the persistence
//!   plugin its binding reads with, the actions it runs itself and the positions its layout gives. A body whose reader
//!   plugin the host lacks opens unreadable, with the finding that names the plugin. A module may instead read, draw
//!   and edit its documents itself ([`module::ModuleView`]), as the standalone host's C4 module did.

pub mod actions;
pub mod body_edits;
pub mod change_handler;
pub mod definition;
pub mod disl_definition;
pub mod document;
pub mod element;
pub mod memo;
pub mod module;
pub mod problems;
pub mod reload;
pub mod session;
pub mod sibling_naming;
pub mod sizing;
pub mod tool;
pub mod viewport_registry;
pub mod workspace;

pub use change_handler::{DiagramDocumentChangeHandler, DocumentChange};
pub use definition::{DiagramDefinition, ToolboxItem};
pub use disl_definition::DislDiagramDefinition;
pub use document::{DiagramChange, DiagramDocument, DiagramDocuments, DiskChange, DocumentEdit, ModuleDocuments};
pub use element::{DiagramDelta, DiagramElement, DiagramViewport, ElementKind};
pub use module::{DiagramModule, Export, ExportOutput, ModulePlacement, ModuleView};
pub use problems::{DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity};
pub use reload::{DiagramDocumentReloadBridge, FileEvent, Reload};
pub use session::{ActionOutcome, ActionTarget, DiagramSession};
pub use sizing::{Extent, Length, Resize, ResizedEdge, SizeSource, SizeSources};
pub use tool::{DiagramTool, DiagramTools};
pub use workspace::{DiagramWorkspace, OpenedDiagram};
