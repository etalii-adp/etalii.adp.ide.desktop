//! The open `.skv` documents: one per body, shared by every view of it (etalii.adp spec 030, task T077).
//!
//! Ported from the standalone host's `SankeyDocumentStore.cs` at `8b6ccfc`, over the core's `LifecycleDocumentStore`,
//! kept in the project's `DiagramDocuments` under [`STATE_KEY`] as the C4 module keeps its documents. A text that is
//! not YAML still opens, empty, with why; nothing is written over it until it reads again. A change on disk reaches
//! the store through the project's reconciliation ([`ModuleDocuments`]): the store's own write is ignored, a change
//! read again, a delete ends the document.

use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_diagram::{DiagramDocuments, DiskChange, ModuleDocuments};

use crate::lines::LineDocument;
use crate::model::SankeyModel;
use crate::parser;

/// The key the Sankey documents are kept under in the project's documents.
pub const STATE_KEY: &str = "sankey";

const BOM: &str = "\u{feff}";

/// One open `.skv`: its text, its reading, and why it could not be read, if it could not.
#[derive(Debug, Clone)]
pub struct SankeyDocument {
    pub body_path: PathBuf,
    /// The text without its byte order mark.
    pub text: String,
    /// Whether the file began with a UTF-8 byte order mark, which a write puts back.
    pub bom: bool,
    pub model: Rc<SankeyModel>,
    /// Why the body could not be read, or none when it could.
    pub unreadable: Option<String>,
}

impl SankeyDocument {
    /// The document as a first open reads it: a body that does not exist yet is a new, empty document.
    fn read(body_path: &Path) -> SankeyDocument {
        match fs::read(body_path) {
            Ok(bytes) => SankeyDocument::of(body_path, &String::from_utf8_lossy(&bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => SankeyDocument::of(body_path, ""),
            Err(error) => SankeyDocument {
                body_path: body_path.to_path_buf(),
                text: String::new(),
                bom: false,
                model: Rc::new(SankeyModel::default()),
                unreadable: Some(error.to_string()),
            },
        }
    }

    fn of(body_path: &Path, text: &str) -> SankeyDocument {
        let (bom, text) = match text.strip_prefix(BOM) {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        let outcome = parser::outcome(&LineDocument::parse(text));
        SankeyDocument {
            body_path: body_path.to_path_buf(),
            text: text.to_owned(),
            bom,
            model: Rc::new(outcome.model),
            unreadable: outcome.unreadable,
        }
    }

    /// Whether it can be edited: it was read.
    pub fn is_usable(&self) -> bool {
        self.unreadable.is_none()
    }

    /// The text's lines, to edit a copy of.
    pub fn lines(&self) -> LineDocument {
        LineDocument::parse(&self.text)
    }

    fn bytes(&self) -> Vec<u8> {
        bytes_of(self.bom, &self.text)
    }
}

fn bytes_of(bom: bool, text: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(text.len() + 3);
    if bom {
        bytes.extend_from_slice(BOM.as_bytes());
    }
    bytes.extend_from_slice(text.as_bytes());
    bytes
}

/// The open Sankey documents of one project.
#[derive(Debug, Default)]
pub struct SankeyStore {
    documents: Vec<SankeyDocument>,
}

impl SankeyStore {
    pub fn get(&self, body_path: &Path) -> Option<&SankeyDocument> {
        self.documents.iter().find(|d| d.body_path == body_path)
    }

    fn put(&mut self, document: SankeyDocument) {
        match self.documents.iter_mut().find(|d| d.body_path == document.body_path) {
            Some(slot) => *slot = document,
            None => self.documents.push(document),
        }
    }
}

impl ModuleDocuments for SankeyStore {
    fn reconcile(&mut self, path: &Path) -> DiskChange {
        let Some(document) = self.get(path) else {
            return DiskChange::NotTracked;
        };
        match fs::read(path) {
            Ok(bytes) if bytes == document.bytes() => DiskChange::Unchanged,
            Ok(bytes) => {
                self.put(SankeyDocument::of(path, &String::from_utf8_lossy(&bytes)));
                DiskChange::Reloaded(path.to_path_buf())
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                self.put(SankeyDocument::of(path, ""));
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

/// The document of `body_path`, or none before it was opened.
pub fn get<'a>(documents: &'a DiagramDocuments, body_path: &Path) -> Option<&'a SankeyDocument> {
    documents.module_state_ref::<SankeyStore>(STATE_KEY)?.get(body_path)
}

/// The document of `body_path`, read the first time it is asked for.
pub fn get_or_load<'a>(documents: &'a mut DiagramDocuments, body_path: &Path) -> &'a SankeyDocument {
    let store = documents.module_state::<SankeyStore>(STATE_KEY);
    let at = match store.documents.iter().position(|d| d.body_path == body_path) {
        Some(at) => at,
        None => {
            store.documents.push(SankeyDocument::read(body_path));
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
/// why when the write failed, with nothing changed.
pub fn save(documents: &mut DiagramDocuments, body_path: &Path, text: &str) -> Result<(), String> {
    let name = file_name(body_path);
    let document = get_or_load(documents, body_path);
    if let Some(reason) = &document.unreadable {
        return Err(format!(
            "{name} could not be read, so nothing can be edited until it can: {reason}"
        ));
    }
    let bom = document.bom;
    if let Some(folder) = body_path.parent() {
        fs::create_dir_all(folder).map_err(|e| format!("{name} could not be written: {e}"))?;
    }
    adp_documents::text::write_atomically(body_path, &bytes_of(bom, text))
        .map_err(|e| format!("{name} could not be written: {e}"))?;
    let mut document = SankeyDocument::of(body_path, text);
    document.bom = bom;
    documents.module_state::<SankeyStore>(STATE_KEY).put(document);
    documents.mark_changed(body_path);
    Ok(())
}

/// Writes `text` back as the body, whatever the document read: what an undo restores (`RestoreDocumentCommand`).
pub fn restore(documents: &mut DiagramDocuments, body_path: &Path, text: &str) -> Result<(), String> {
    let bom = get_or_load(documents, body_path).bom;
    adp_documents::text::write_atomically(body_path, &bytes_of(bom, text))
        .map_err(|e| format!("Could not restore the file: {e}"))?;
    let mut document = SankeyDocument::of(body_path, text);
    document.bom = bom;
    documents.module_state::<SankeyStore>(STATE_KEY).put(document);
    documents.mark_changed(body_path);
    Ok(())
}
