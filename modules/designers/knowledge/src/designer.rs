//! The Knowledge designer as a module offers it (the standalone host's `Designer`, `KnowledgeSessionFactory` and
//! `KnowledgeDocumentTemplate` at 8b6ccfc): its definition, how it opens a session per connection, and what a new
//! knowledge file starts as.

use std::path::Path;

use adp_desl::table::session::DesignerSession;
use adp_desl::{DesignerDefinition, DesignerFormat, DesignerSessionFactory};

use crate::definition::{self, ORIGIN};
use crate::session::KnowledgeSession;

/// The Knowledge designer's definition: a table of rows and typed properties, with views of it, kept in one plain
/// YAML, JSON or XML file. The formats are in the order they are offered when a table is added; the format is chosen
/// then and never changed.
pub fn definition() -> DesignerDefinition {
    DesignerDefinition {
        origin: ORIGIN.into(),
        title: "Knowledge".into(),
        description: "A table of rows and typed properties, with views of it, kept in a plain text file you own."
            .into(),
        icon: "mdi-table".into(),
        formats: vec![
            DesignerFormat::new("YAML", ".yaml"),
            DesignerFormat::new("JSON", ".json"),
            DesignerFormat::new("XML", ".xml"),
        ],
    }
}

/// Opens a session per connection for a knowledge file. Each connection has its own view and its own window.
pub struct KnowledgeSessionFactory;

impl DesignerSessionFactory for KnowledgeSessionFactory {
    fn origin(&self) -> &str {
        ORIGIN
    }

    fn open(&self, root: &Path, _registration: &Path, body: &Path) -> Box<dyn DesignerSession> {
        Box::new(KnowledgeSession::open_for_editing(body, root))
    }
}

/// What a new knowledge file starts as: the template of the binding its format selects, with a fresh id for
/// everything the template makes - one title property, one view and no rows. None for a format no binding has.
pub fn new_document(format: &DesignerFormat, file_name: &str) -> Option<String> {
    let binding = definition::binding_for(&format.extension)?;
    let bytes = adp_fbl::routing::produce(&binding, Some(ORIGIN), file_name, &|_| definition::new_id())?;
    String::from_utf8(bytes).ok()
}
