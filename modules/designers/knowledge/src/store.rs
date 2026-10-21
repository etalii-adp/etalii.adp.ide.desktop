//! Reads a knowledge file from disk (the standalone host's `KnowledgeDocumentStore` at 8b6ccfc). **Opening writes
//! nothing**: a file's bytes are kept exactly as they were, and an unchanged file is never rewritten - not to add an
//! id, not to tidy it.

use std::io::ErrorKind;
use std::path::Path;

use crate::body::KnowledgeBody;

/// What reading a knowledge file gave: its body, or why it could not be read from disk at all.
pub struct KnowledgeRead {
    pub body: Option<KnowledgeBody>,
    pub refusal: String,
}

/// Reads the body at `path`. Never fails for what a file or a disk can do.
pub fn read(path: &Path) -> KnowledgeRead {
    match std::fs::read(path) {
        Ok(bytes) => KnowledgeRead {
            body: Some(KnowledgeBody::read(bytes, &path.to_string_lossy())),
            refusal: String::new(),
        },
        Err(e) => KnowledgeRead {
            body: None,
            refusal: if e.kind() == ErrorKind::NotFound {
                "The file no longer exists.".into()
            } else {
                format!("The file cannot be read: {e}")
            },
        },
    }
}
