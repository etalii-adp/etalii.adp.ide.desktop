//! The open `.fdg` documents: one per body, shared by every view of it, kept in the project's documents (etalii.adp
//! spec 030, task T072).
//!
//! Ported from the standalone host at `8b6ccfc`: the module's `FdgDocumentStore.cs` and `IFdgDocumentStore.cs`, over
//! the core's `LifecycleDocumentStore`. Every text reads, since the parser never fails; a text that is not YAML is
//! kept with its reason, and nothing is edited or written over it until it reads, as a body that could not be read at
//! all is never written over. A change on disk reaches the store through the project's reconciliation
//! ([`ModuleDocuments`]): the store's own write is ignored, a change is read again, a delete ends the document.

use std::fs;
use std::path::{Path, PathBuf};

use adp_diagram::{DiagramDocuments, DiskChange, ModuleDocuments};

use crate::lines::LineDocument;
use crate::model::FdgModel;
use crate::parser;

/// The key the documents are kept under in the project's documents.
pub const STATE_KEY: &str = "functional-decomposition-graph";

const BOM: &str = "\u{feff}";

/// One open `.fdg`: its text, its reading, and why it cannot be edited, when it cannot.
#[derive(Debug, Clone)]
pub struct FdgDocument {
    pub body_path: PathBuf,
    /// The text without its byte order mark.
    pub text: String,
    /// Whether the file began with a UTF-8 byte order mark, which a write puts back.
    pub bom: bool,
    pub model: FdgModel,
    /// Why the body cannot be edited: it could not be read, or it is not YAML; none when it can.
    pub unreadable: Option<String>,
}

impl FdgDocument {
    fn read(body_path: &Path) -> FdgDocument {
        match fs::read(body_path) {
            Ok(bytes) => FdgDocument::of(body_path, &String::from_utf8_lossy(&bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => FdgDocument::of(body_path, ""),
            Err(error) => FdgDocument {
                body_path: body_path.to_path_buf(),
                text: String::new(),
                bom: false,
                model: FdgModel::default(),
                unreadable: Some(error.to_string()),
            },
        }
    }

    /// The document of `text`, as read from the body at `body_path`.
    pub fn of(body_path: &Path, text: &str) -> FdgDocument {
        let (bom, text) = match text.strip_prefix(BOM) {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        let outcome = parser::outcome(&LineDocument::parse(text));
        FdgDocument {
            body_path: body_path.to_path_buf(),
            text: text.to_owned(),
            bom,
            model: outcome.model,
            unreadable: outcome.unreadable,
        }
    }

    /// Whether it can be edited.
    pub fn is_usable(&self) -> bool {
        self.unreadable.is_none()
    }

    /// The text's lines, to edit a copy of.
    pub fn lines(&self) -> LineDocument {
        LineDocument::parse(&self.text)
    }

    fn bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.text.len() + 3);
        if self.bom {
            bytes.extend_from_slice(BOM.as_bytes());
        }
        bytes.extend_from_slice(self.text.as_bytes());
        bytes
    }
}

/// The open documents of one project.
#[derive(Debug, Default)]
pub struct FdgStore {
    documents: Vec<FdgDocument>,
}

impl FdgStore {
    pub fn get(&self, body_path: &Path) -> Option<&FdgDocument> {
        self.documents.iter().find(|d| d.body_path == body_path)
    }

    fn put(&mut self, document: FdgDocument) {
        match self.documents.iter_mut().find(|d| d.body_path == document.body_path) {
            Some(slot) => *slot = document,
            None => self.documents.push(document),
        }
    }
}

impl ModuleDocuments for FdgStore {
    fn reconcile(&mut self, path: &Path) -> DiskChange {
        let Some(document) = self.get(path) else {
            return DiskChange::NotTracked;
        };
        match fs::read(path) {
            Ok(bytes) if bytes == document.bytes() => DiskChange::Unchanged,
            Ok(bytes) => {
                self.put(FdgDocument::of(path, &String::from_utf8_lossy(&bytes)));
                DiskChange::Reloaded(path.to_path_buf())
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                self.put(FdgDocument::of(path, ""));
                DiskChange::Deleted(path.to_path_buf())
            }
            // A read refused for the instant keeps the last good document.
            Err(_) => DiskChange::Unchanged,
        }
    }

    fn close(&mut self, body_path: &Path) {
        self.documents.retain(|d| d.body_path != body_path);
    }
}

/// The document of `body_path` as the project's documents hold it, or none before it was opened.
pub fn get<'a>(documents: &'a DiagramDocuments, body_path: &Path) -> Option<&'a FdgDocument> {
    documents.module_state_ref::<FdgStore>(STATE_KEY)?.get(body_path)
}

/// The document of `body_path`, read the first time it is asked for.
pub fn get_or_load<'a>(documents: &'a mut DiagramDocuments, body_path: &Path) -> &'a FdgDocument {
    let store = documents.module_state::<FdgStore>(STATE_KEY);
    let at = match store.documents.iter().position(|d| d.body_path == body_path) {
        Some(at) => at,
        None => {
            store.documents.push(FdgDocument::read(body_path));
            store.documents.len() - 1
        }
    };
    &store.documents[at]
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Writes `text` as the body at `body_path` and reads it again, so every view of it refreshes; the sentence that says
/// why when it may not be written or the write failed, with nothing changed. A document that could not be read is
/// never written over.
pub fn save(documents: &mut DiagramDocuments, body_path: &Path, text: &str) -> Result<(), String> {
    let name = file_name(body_path);
    let document = get_or_load(documents, body_path);
    if let Some(reason) = &document.unreadable {
        return Err(format!(
            "{name} could not be read, so nothing can be edited until it can: {reason}"
        ));
    }
    write(documents, body_path, text)
}

/// Writes `text` as the body at `body_path` whatever the document read, as an undo restores the text it held.
pub fn write(documents: &mut DiagramDocuments, body_path: &Path, text: &str) -> Result<(), String> {
    let name = file_name(body_path);
    let bom = get_or_load(documents, body_path).bom;
    let mut bytes = Vec::with_capacity(text.len() + 3);
    if bom {
        bytes.extend_from_slice(BOM.as_bytes());
    }
    bytes.extend_from_slice(text.as_bytes());
    if let Some(folder) = body_path.parent() {
        fs::create_dir_all(folder).map_err(|e| format!("{name} could not be written: {e}"))?;
    }
    adp_documents::text::write_atomically(body_path, &bytes)
        .map_err(|e| format!("{name} could not be written: {e}"))?;
    let mut document = FdgDocument::of(body_path, text);
    document.bom = bom;
    documents.module_state::<FdgStore>(STATE_KEY).put(document);
    documents.mark_changed(body_path);
    Ok(())
}
