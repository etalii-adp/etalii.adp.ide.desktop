//! The Knowledge designer (etalii.adp spec 030, FR-022, after the standalone host's `src/designers/knowledge` at
//! 8b6ccfc): a table of rows and typed properties, with views of it, kept in one plain YAML, JSON or XML file - a DED
//! definition of the designer type `etalii/knowledge`, read and written through the vendored `knowledge.fbl` and
//! laid out as `knowledge.des` says, through the DESL runtime of `adp-desl`.
//!
//! The module has no parser and no writer of its own: a file is read through FBL ([`body`]), into the DESL
//! runtime's table content ([`model`]), checked by the designer's rules ([`validator`]), related to the other
//! knowledge files it names ([`relations`]), and mapped onto the table model ([`mapper`], [`calculations`]). A
//! connection holds a [`session`], whose gestures become changes of the file ([`edits`]), shown at once
//! ([`projection`]) and written behind ([`command`]). A rename of a table that others relate to is followed by
//! [`target_rename`]. A row selected in a table is answered for in the property grid by [`row_context`], and the
//! explorer's entry that adds a relation is [`context_actions`].

pub mod body;
pub mod calculations;
pub mod command;
pub mod context_actions;
pub mod definition;
pub mod designer;
pub mod edits;
pub mod mapper;
pub mod model;
pub mod project;
pub mod projection;
pub mod relations;
pub mod row_context;
pub mod session;
pub mod store;
pub mod target_rename;
pub mod theme;
pub mod validator;
pub mod values;
pub mod vocabulary;

pub use body::KnowledgeBody;
pub use definition::ORIGIN;
pub use model::{KnowledgeProperty, KnowledgeTable};
pub use session::KnowledgeSession;
