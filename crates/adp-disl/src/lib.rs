//! The DISL and DID runtime (etalii.adp spec 030, FR-030), ported from the standalone host's
//! `EtAlii.Adp.Specification.Disl` at 8b6ccfc. It loads a DISL specification (DISL section 14.1), compiles every
//! expression of it in its CEL context (section 12), builds the elements a diagram's model holds from a format
//! binding's reading (sections 4, 11.5 and 14.2), derives what a host shows of it - findings, gesture refusals, the
//! palette, context menus and property rows (sections 7 and 8) - and interprets its behaviour into model changes a
//! host writes through FBL (section 9). Its modules follow the standalone host's folders: [`loading`],
//! [`expressions`], [`model`], [`derivation`], [`behavior`] and [`bundling`].

pub mod behavior;
pub mod bundling;
pub mod derivation;
pub mod expressions;
pub mod json;
pub mod loading;
pub mod model;

pub use behavior::change::{DislChange, DislTransaction, HostAction};
pub use behavior::deletion::DislConfirmation;
pub use behavior::ids::IdSource;
pub use behavior::operations::{DislInvocation, DislOperationParameter};
pub use bundling::{BundledDefinition, DefinitionProvenance};
pub use derivation::context_menu::{DerivedMenuEntry, DerivedMenuGroup, DerivedShortcut, DislMenuTarget};
pub use derivation::env::DislEnv;
pub use derivation::finding::{DislConstraintOptions, DislFinding, DislReaderFinding};
pub use derivation::form_commit::{FormRequest, FormRequestField};
pub use derivation::forms::DerivedRow;
pub use derivation::toolbox::DerivedTool;
pub use derivation::wire_ids::WireIdMap;
pub use expressions::contexts;
pub use expressions::plugins::DislPluginFunctions;
pub use loading::diagnostic::{DislDiagnostic, DislSeverity};
pub use loading::loader::{DislLoadResult, DislLoader};
pub use loading::metamodel::{
    DislAttribute, DislEnum, DislEnumValue, DislExpression, DislFunctionDeclaration, DislIdRule, DislMetamodel,
    DislParameter, DislRecursion, DislRelationEnd, DislType,
};
pub use loading::specification::DislSpecification;
pub use model::builder::DislModel;
pub use model::diagram::DislDiagram;
pub use model::element::{DislElement, NewElement};
