//! The one owner of the open `.supply` documents: one per body, shared by every view of it (etalii.adp spec 030, task
//! T076).
//!
//! Ported from the standalone host's `src/diagrams/supply-chain/backend/EtAlii.Adp.Diagram.SupplyChain/
//! SupplyChainDocumentStore.cs` at `8b6ccfc`, over the core's `LifecycleDocumentStore`, and kept in the project's
//! `DiagramDocuments` under [`STATE_KEY`]. A text that is not YAML is kept with why, so that nothing is edited or
//! written until it reads again; every other text reads, its problems reported rather than refused. A change on disk
//! reaches the store through the project's reconciliation ([`ModuleDocuments`]): the store's own write is ignored, a
//! change read again, a delete ends the document.

use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_diagram::{DiagramDocuments, DiskChange, ModuleDocuments};

use crate::lines::LineDocument;
use crate::model::SupplyChainModel;
use crate::parser;

/// The key the supply chain documents are kept under in the project's documents.
pub const STATE_KEY: &str = "supply-chain";

const BOM: &str = "\u{feff}";

/// One open `.supply`: its text and its reading, or why it could not be read.
#[derive(Debug, Clone)]
pub struct SupplyChainDocument {
    pub body_path: PathBuf,
    /// The text without its byte order mark.
    pub text: String,
    /// Whether the file began with a UTF-8 byte order mark, which a write puts back.
    pub bom: bool,
    pub model: Rc<SupplyChainModel>,
    /// Why the body could not be read, or none when it could.
    pub unreadable: Option<String>,
}

impl SupplyChainDocument {
    /// The document of the body at `body_path` as a first open reads it: a body that does not exist yet is a new,
    /// empty document that can be saved.
    fn read(body_path: &Path) -> SupplyChainDocument {
        match fs::read(body_path) {
            Ok(bytes) => SupplyChainDocument::of(body_path, &String::from_utf8_lossy(&bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => SupplyChainDocument::of(body_path, ""),
            Err(error) => SupplyChainDocument {
                body_path: body_path.to_path_buf(),
                text: String::new(),
                bom: false,
                model: Rc::new(SupplyChainModel::default()),
                unreadable: Some(error.to_string()),
            },
        }
    }

    pub fn of(body_path: &Path, text: &str) -> SupplyChainDocument {
        let (bom, text) = match text.strip_prefix(BOM) {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        let outcome = parser::outcome(&LineDocument::parse(text));
        SupplyChainDocument {
            body_path: body_path.to_path_buf(),
            text: text.to_owned(),
            bom,
            model: Rc::new(outcome.model),
            unreadable: outcome.unreadable,
        }
    }

    /// Whether it can be edited: it read.
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

/// The open supply chain documents of one project.
#[derive(Debug, Default)]
pub struct SupplyChainStore {
    documents: Vec<SupplyChainDocument>,
}

impl SupplyChainStore {
    pub fn get(&self, body_path: &Path) -> Option<&SupplyChainDocument> {
        self.documents.iter().find(|d| d.body_path == body_path)
    }

    fn put(&mut self, document: SupplyChainDocument) {
        match self.documents.iter_mut().find(|d| d.body_path == document.body_path) {
            Some(slot) => *slot = document,
            None => self.documents.push(document),
        }
    }

    /// Reads the body again; false when a read that fails keeps the last good document.
    fn reload(&mut self, body_path: &Path) -> bool {
        let usable = self.get(body_path).is_some_and(SupplyChainDocument::is_usable);
        let document = match fs::read(body_path) {
            Ok(bytes) => SupplyChainDocument::of(body_path, &String::from_utf8_lossy(&bytes)),
            Err(_) if usable => return false,
            Err(_) => SupplyChainDocument::read(body_path),
        };
        self.put(document);
        true
    }
}

impl ModuleDocuments for SupplyChainStore {
    /// A body that holds what its document read or wrote needs nothing; one that is gone ends its document; any other
    /// change is read again.
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
                self.put(SupplyChainDocument::of(path, ""));
                DiskChange::Deleted(path.to_path_buf())
            }
            Err(_) => DiskChange::Unchanged,
        }
    }

    fn close(&mut self, body_path: &Path) {
        self.documents.retain(|d| d.body_path != body_path);
    }
}

/// The document of `body_path` as the project's documents hold it, or none before it was opened.
pub fn get<'a>(documents: &'a DiagramDocuments, body_path: &Path) -> Option<&'a SupplyChainDocument> {
    documents
        .module_state_ref::<SupplyChainStore>(STATE_KEY)?
        .get(body_path)
}

/// The document of `body_path`, read the first time it is asked for.
pub fn get_or_load<'a>(documents: &'a mut DiagramDocuments, body_path: &Path) -> &'a SupplyChainDocument {
    let store = documents.module_state::<SupplyChainStore>(STATE_KEY);
    let at = match store.documents.iter().position(|d| d.body_path == body_path) {
        Some(at) => at,
        None => {
            store.documents.push(SupplyChainDocument::read(body_path));
            store.documents.len() - 1
        }
    };
    &store.documents[at]
}

/// Writes `text` as the body at `body_path` and reads it again, so every view of it refreshes; why when the write
/// failed, with nothing changed.
pub fn save(documents: &mut DiagramDocuments, body_path: &Path, text: &str) -> Result<(), String> {
    let name = body_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
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
    let mut document = SupplyChainDocument::of(body_path, text);
    document.bom = bom;
    documents.module_state::<SupplyChainStore>(STATE_KEY).put(document);
    documents.mark_changed(body_path);
    Ok(())
}

/// Reads the body at `body_path` again from disk, as a change from outside asks.
pub fn reload(documents: &mut DiagramDocuments, body_path: &Path) {
    if documents.module_state::<SupplyChainStore>(STATE_KEY).reload(body_path) {
        documents.mark_changed(body_path);
    }
}
