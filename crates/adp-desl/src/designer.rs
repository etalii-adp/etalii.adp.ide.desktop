//! What a designer module is (the standalone host's `DesignerDefinition`, `DesignerFormat` and
//! `IDesignerSessionFactory` at 8b6ccfc): which designer type it serves, how it introduces itself, in which file
//! formats its documents can be created, and how it opens a session on one of them. A designer is identified by an
//! origin and a registration file, as a diagram is, and stores its document in a format the author chooses when
//! adding it, which is chosen then and never changed (DESL section 5.1).

use std::path::Path;

use crate::table::session::DesignerSession;

/// One file format a designer's document can be created in: the name the author chooses it by and the extension
/// the new file takes, dot included and lower-cased on construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesignerFormat {
    pub title: String,
    pub extension: String,
}

impl DesignerFormat {
    pub fn new(title: impl Into<String>, extension: impl Into<String>) -> Self {
        DesignerFormat {
            title: title.into(),
            extension: extension.into().to_lowercase(),
        }
    }
}

/// A designer type as a module offers it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesignerDefinition {
    /// `vendor/type`, unique across all tools: the first line of a document's registration.
    pub origin: String,
    /// The display name: a name, not a description.
    pub title: String,
    /// One sentence saying what the designer is for, from the reader's point of view.
    pub description: String,
    /// The icon's name, or empty for the generic document mark.
    pub icon: String,
    /// The formats a document can be created in, in the order they are offered.
    pub formats: Vec<DesignerFormat>,
}

/// How a designer module opens its sessions, keyed by the designer type's origin. The host resolves which document
/// a path names; the module learns nothing of the project beyond the root and the two files it is handed.
pub trait DesignerSessionFactory {
    fn origin(&self) -> &str;

    fn open(&self, root: &Path, registration: &Path, body: &Path) -> Box<dyn DesignerSession>;
}
