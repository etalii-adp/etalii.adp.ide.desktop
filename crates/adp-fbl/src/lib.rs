//! FBL, the Format Binding Language: families, splices, routing and the registration file (etalii.adp spec 030,
//! FR-031 to FR-033), ported from the standalone host's `EtAlii.Adp.Specification.Fbl` at 8b6ccfc.
//!
//! A binding ([`documents`]) says how a format maps to a language's model. A body is read through it losslessly by
//! its family ([`families`]: yaml, json, xml, lines and blocks; FBL section 4), the engine ([`rules`]) claims its
//! entries and resolves ids, references and containment (FBL section 5), the planner ([`planning`]) turns a model
//! change into the splices of one edit (FBL section 6), and the history ([`history`]) applies, undoes and redoes
//! those edits (FBL section 7). A registration ([`registration`]) is written by the same splices (FBL section 8).
//! Routing ([`routing`]) finds the bindings a file opens with and makes new bodies from templates (FBL sections
//! 10, 12 and 13). The splice catalogue itself ([`splice`]) is FBL sections 6.1, 6.5 and 7.1. Persistence plugins
//! ([`plugins`]) read the bodies FBL cannot declare, file bodies and folder subjects alike (FBL sections 10 and 11),
//! and the legacy sidecars of FBL section 8.7 are read and written by [`registration::legacy_sidecar`].

pub mod documents;
pub mod expressions;
pub(crate) mod families;
pub mod history;
pub mod model;
pub mod planning;
pub mod plugins;
pub mod registration;
pub mod routing;
pub mod rules;
pub mod splice;
pub mod text;

pub use documents::{FblBinding, FblDocument};
pub use history::{OpenBody, UndoResult};
pub use model::{FblElement, FblModel, FblOptions, Finding, FindingSeverity, IdRequest};
pub use planning::{ModelChange, PlanResult, PlannedEdit};
pub use registration::{OpenRegistration, RegistrationDocument};
pub use rules::reading::BodyReading;
pub use splice::{Edit, EditHistory, Operation, Splice, SpliceError, apply};
