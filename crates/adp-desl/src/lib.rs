//! The DESL and DED runtime for designers, with windowed table readings (etalii.adp spec 030, FR-030, task T047).
//!
//! - [`loading`] loads a DESL specification (DESL section 10.1) with its findings, refusing a later major version
//!   with the reason, as `adp-disl` loads DISL.
//! - [`ded`] recognises a DED definition by its envelope and checks it (DED sections 2 and 4).
//! - [`table`] is the table a designer of the `table` kind lays out (DESL sections 7.2 to 7.6): the model that
//!   crosses to the shell (the standalone host's `EtAlii.Adp.Designer.TableModel` at 8b6ccfc), the content a
//!   designer module reads its document into, the view engine that filters, sorts, groups and nests its rows on the
//!   core side (the standalone host's `KnowledgeViewEngine` and `KnowledgeGroups`, made host-neutral by going by the
//!   DESL value type's editor, key and sort rather than by the Knowledge designer's type names), and the session a
//!   connection holds, which answers a window of lines in the active view's order (`IDesignerSession`).
//! - [`designer`] is what a designer module is (`DesignerDefinition`, `DesignerFormat`, `IDesignerSessionFactory`).

pub mod ded;
pub mod designer;
pub(crate) mod json;
pub mod loading;
pub(crate) mod schema;
pub mod table;

pub use ded::{DedEnvelope, DedFormat};
pub use designer::{DesignerDefinition, DesignerFormat, DesignerSessionFactory};
pub use loading::diagnostic::{DeslDiagnostic, DeslSeverity};
pub use loading::loader::{DeslLoadResult, DeslLoader};
pub use loading::specification::{
    DeslAttribute, DeslContainment, DeslElementType, DeslEnum, DeslEnumValue, DeslIdRule, DeslLanguage, DeslMetamodel,
    DeslOptions, DeslPersistence, DeslSpecification, DeslSurface, DeslTable, DeslValueType, Editor, SortKind,
};
pub use table::session::DesignerSession;
