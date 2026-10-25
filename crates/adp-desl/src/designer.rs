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
    /// How it recognises its document by the document's own first line, when it does.
    pub claim: Option<DesignerClaim>,
}

/// How a designer type recognises its document by the document's own content rather than by a registration: the
/// envelope on its first line (DED section 2, FBL sections 12.1 and 12.3), as the standalone host's `DesignerClaim` at
/// a7fdf03 says it. A body of one of `extensions` that `matches` is the type's document, and opens without a
/// registration.
#[derive(Clone)]
pub struct DesignerClaim {
    /// The extensions the claim covers, dot included; each is shared with other tools.
    pub extensions: Vec<String>,
    /// Whether a file of that name, whose first bytes are given, is the type's document: the module answers it with its
    /// binding's marker, through the FBL library's routing.
    pub matches: fn(&str, &[u8]) -> bool,
    /// The line a file needs on its first line to be the type's document, for a refusal to name.
    pub envelope: String,
    /// Whether the registration is created with the first setting the user changes (`createOnFirstPlacement`, FBL
    /// section 8.4), so that a new document is its body alone.
    pub creates_registration: bool,
}

impl DesignerClaim {
    /// Whether the claim covers files of `extension`, ignoring case.
    pub fn covers(&self, extension: &str) -> bool {
        self.extensions.iter().any(|e| e.eq_ignore_ascii_case(extension))
    }
}

// A claim is the same claim when it says the same things; what answers `matches` is the module's, and is not compared.
impl PartialEq for DesignerClaim {
    fn eq(&self, other: &Self) -> bool {
        self.extensions == other.extensions
            && self.envelope == other.envelope
            && self.creates_registration == other.creates_registration
    }
}

impl Eq for DesignerClaim {}

impl std::fmt::Debug for DesignerClaim {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DesignerClaim")
            .field("extensions", &self.extensions)
            .field("envelope", &self.envelope)
            .field("creates_registration", &self.creates_registration)
            .finish_non_exhaustive()
    }
}

/// How a designer module opens its sessions, keyed by the designer type's origin. The host resolves which document
/// a path names; the module learns nothing of the project beyond the root and the two files it is handed.
pub trait DesignerSessionFactory {
    fn origin(&self) -> &str;

    fn open(&self, root: &Path, registration: &Path, body: &Path) -> Box<dyn DesignerSession>;
}
