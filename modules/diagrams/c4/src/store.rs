//! The one owner of the open C4 documents: one per `.dsl`, shared by every view of it (etalii.adp spec 030, task
//! T061).
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4/C4DocumentStore.cs` and
//! `IC4DocumentStore.cs` at `8b6ccfc`, over the core's `LifecycleDocumentStore` and `DocumentEditRun`. Several
//! diagrams open one document, which is what "model once, view many" means, so they share one reading of it, kept in
//! the project's `DiagramDocuments` under [`STATE_KEY`]; an edit through any view reaches all of them. Every text
//! reads: the parser passes over what it cannot read and the rules report it, so a body is unreadable only when the
//! read itself failed. An edit is made on a copy of the lines, written whole, and read again. A change on disk reaches
//! the store through the project's reconciliation ([`ModuleDocuments`]), as the standalone host's reload bridge
//! reached its store: the store's own write is ignored, a change read again, a delete ends the document.

use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_diagram::{DiagramDocuments, DiskChange, ModuleDocuments};

use crate::lines::LineDocument;
use crate::model::C4Workspace;
use crate::parser;

/// The key the C4 documents are kept under in the project's documents.
pub const STATE_KEY: &str = "c4";

const BOM: &str = "\u{feff}";

/// One open `.dsl`: its text and its reading, or why it could not be read.
#[derive(Debug, Clone)]
pub struct C4Document {
    pub body_path: PathBuf,
    /// The text without its byte order mark.
    pub text: String,
    /// Whether the file began with a UTF-8 byte order mark, which a write puts back.
    pub bom: bool,
    pub workspace: Rc<C4Workspace>,
    /// Why the body could not be read, or none when it could.
    pub unreadable: Option<String>,
}

impl C4Document {
    /// The document of the body at `body_path` as a first open reads it: a body that does not exist yet is a new,
    /// empty document that can be saved; one that is there and cannot be read is unreadable.
    fn read(body_path: &Path) -> C4Document {
        match fs::read(body_path) {
            Ok(bytes) => C4Document::of(body_path, &String::from_utf8_lossy(&bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => C4Document::of(body_path, ""),
            Err(error) => C4Document {
                body_path: body_path.to_path_buf(),
                text: String::new(),
                bom: false,
                workspace: Rc::new(C4Workspace::default()),
                unreadable: Some(error.to_string()),
            },
        }
    }

    fn of(body_path: &Path, text: &str) -> C4Document {
        let (bom, text) = match text.strip_prefix(BOM) {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        C4Document {
            body_path: body_path.to_path_buf(),
            text: text.to_owned(),
            bom,
            workspace: Rc::new(parser::parse_text(text)),
            unreadable: None,
        }
    }

    /// Whether it can be edited: its read succeeded.
    pub fn is_usable(&self) -> bool {
        self.unreadable.is_none()
    }

    /// The text's lines, to edit a copy of.
    pub fn lines(&self) -> LineDocument {
        LineDocument::parse(&self.text)
    }

    /// The bytes the body holds when it is what this document read or wrote.
    fn bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.text.len() + 3);
        if self.bom {
            bytes.extend_from_slice(BOM.as_bytes());
        }
        bytes.extend_from_slice(self.text.as_bytes());
        bytes
    }
}

/// The open C4 documents of one project.
#[derive(Debug, Default)]
pub struct C4Store {
    documents: Vec<C4Document>,
}

impl C4Store {
    pub fn get(&self, body_path: &Path) -> Option<&C4Document> {
        self.documents.iter().find(|d| d.body_path == body_path)
    }

    fn put(&mut self, document: C4Document) {
        match self.documents.iter_mut().find(|d| d.body_path == document.body_path) {
            Some(slot) => *slot = document,
            None => self.documents.push(document),
        }
    }

    /// Reads the body again; false when a read that fails keeps the last good document.
    fn reload(&mut self, body_path: &Path) -> bool {
        let usable = self.get(body_path).is_some_and(C4Document::is_usable);
        let document = match fs::read(body_path) {
            Ok(bytes) => C4Document::of(body_path, &String::from_utf8_lossy(&bytes)),
            Err(_) if usable => return false,
            Err(_) => C4Document::read(body_path),
        };
        self.put(document);
        true
    }
}

impl ModuleDocuments for C4Store {
    /// A body that holds what its document read or wrote, the store's own write included, needs nothing; one that is
    /// gone ends its document; any other change is read again.
    fn reconcile(&mut self, path: &Path) -> DiskChange {
        let Some(document) = self.get(path) else {
            return DiskChange::NotTracked;
        };
        match fs::read(path) {
            Ok(bytes) if bytes == document.bytes() => DiskChange::Unchanged,
            Ok(_) => {
                self.reload(path);
                DiskChange::Reloaded(path.to_path_buf())
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                self.put(C4Document::of(path, ""));
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
pub fn get<'a>(documents: &'a DiagramDocuments, body_path: &Path) -> Option<&'a C4Document> {
    documents.module_state_ref::<C4Store>(STATE_KEY)?.get(body_path)
}

/// The document of `body_path`, read the first time it is asked for.
pub fn get_or_load<'a>(documents: &'a mut DiagramDocuments, body_path: &Path) -> &'a C4Document {
    let store = documents.module_state::<C4Store>(STATE_KEY);
    let at = match store.documents.iter().position(|d| d.body_path == body_path) {
        Some(at) => at,
        None => {
            store.documents.push(C4Document::read(body_path));
            store.documents.len() - 1
        }
    };
    &store.documents[at]
}

/// Writes `lines` as the text of the body at `body_path` and reads it again, so every view of it refreshes; the
/// sentence that says why when the write failed, with nothing changed.
pub fn save(documents: &mut DiagramDocuments, body_path: &Path, lines: &LineDocument) -> Result<(), String> {
    let name = body_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let document = get_or_load(documents, body_path);
    if let Some(reason) = &document.unreadable {
        // An entry standing in for a body that could not be read is never written over the real file.
        return Err(format!(
            "{name} could not be read, so nothing can be edited until it can: {reason}"
        ));
    }
    let bom = document.bom;
    let text = lines.text();
    let mut bytes = Vec::with_capacity(text.len() + 3);
    if bom {
        bytes.extend_from_slice(BOM.as_bytes());
    }
    bytes.extend_from_slice(text.as_bytes());
    if let Some(folder) = body_path.parent() {
        // A body that lives beside nothing yet gets its folder.
        fs::create_dir_all(folder).map_err(|e| format!("{name} could not be written: {e}"))?;
    }
    adp_documents::text::write_atomically(body_path, &bytes)
        .map_err(|e| format!("{name} could not be written: {e}"))?;
    let mut document = C4Document::of(body_path, &text);
    document.bom = bom;
    documents.module_state::<C4Store>(STATE_KEY).put(document);
    documents.mark_changed(body_path);
    Ok(())
}

/// Reads the body at `body_path` again from disk, as a change from outside asks. A read that fails, a body missing
/// for the instant a publish replaces it included, keeps the last good document and tells nobody: only the watcher's
/// delete ([`body_deleted`]) ends it.
pub fn reload(documents: &mut DiagramDocuments, body_path: &Path) {
    if documents.module_state::<C4Store>(STATE_KEY).reload(body_path) {
        documents.mark_changed(body_path);
    }
}

/// Ends the document of a body the watcher reports deleted: what a first open of a missing body shows, a new, empty
/// document that can be saved, so that a deleted diagram is not drawn for ever.
pub fn body_deleted(documents: &mut DiagramDocuments, body_path: &Path) {
    documents
        .module_state::<C4Store>(STATE_KEY)
        .put(C4Document::of(body_path, ""));
    documents.mark_changed(body_path);
}

/// Tells every view of the body at `body_path` to show it again without changing it: a move changes where an element
/// is drawn, not what the model says (`IC4DocumentStore.Touch`).
pub fn touch(documents: &mut DiagramDocuments, body_path: &Path) {
    get_or_load(documents, body_path);
    documents.mark_changed(body_path);
}
