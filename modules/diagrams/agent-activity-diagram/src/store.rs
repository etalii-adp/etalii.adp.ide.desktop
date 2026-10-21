//! The open activity files: one per body, shared by every view of it, kept in the project's documents (etalii.adp
//! spec 030, task T075).
//!
//! Ported from the standalone host's `src/diagrams/agent-activity-diagram/backend/EtAlii.Adp.Diagram.AgentActivityDiagram/
//! AadDocumentStore.cs` and `AadSession.cs` (`Shown`) at `8b6ccfc`. A file the binding cannot read - not YAML, or not
//! an activity file - is held as unreadable with the reader's own sentence, and nothing is written to it until it can
//! be read; the diagram keeps showing what it last read until then. Agents write an activity file while it is open,
//! so a change on disk reaches the store through the project's reconciliation ([`ModuleDocuments`]): the store's own
//! write is ignored, a change read again, a delete ends the document.

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_diagram::{DiagramDocuments, DiskChange, ModuleDocuments};
use adp_layout::Placements;

use crate::body::AadBody;
use crate::model::AadModel;

/// The key the activity files are kept under in the project's documents.
pub const STATE_KEY: &str = "agent-activity-diagram";

const BOM: &str = "\u{feff}";

/// One open `.aad`: its text, its reading, and what the diagram shows of it.
#[derive(Debug)]
pub struct AadDocument {
    pub body_path: PathBuf,
    /// The text without its byte order mark.
    pub text: String,
    /// Whether the file began with a UTF-8 byte order mark, which a write puts back.
    pub bom: bool,
    pub body: Rc<AadBody>,
    /// What the diagram shows: what the file says now, or what it last said that could be read.
    pub model: Rc<AadModel>,
    /// Why the body could not be read, or none when it could.
    pub unreadable: Option<String>,
    /// Where the layout last placed every element, kept so that one change moves the rest a little.
    pub placed: RefCell<Option<(Rc<AadModel>, Rc<Placements>)>>,
}

impl AadDocument {
    pub fn of(body_path: &Path, text: &str) -> AadDocument {
        let (bom, text) = match text.strip_prefix(BOM) {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        let body = AadBody::parse(text);
        let reason = body.unreadable();
        AadDocument {
            body_path: body_path.to_path_buf(),
            text: text.to_owned(),
            bom,
            model: Rc::new(AadModel::read(&body)),
            body: Rc::new(body),
            unreadable: (!reason.is_empty()).then_some(reason),
            placed: RefCell::new(None),
        }
    }

    /// The document as a later reading of it has it: a file that can no longer be read keeps showing what was last
    /// read, and the layout starts from where it last placed everything.
    fn after(self, previous: Option<&AadDocument>) -> AadDocument {
        let Some(previous) = previous else { return self };
        let mut next = self;
        if next.unreadable.is_some() {
            next.model = previous.model.clone();
        }
        *next.placed.borrow_mut() = previous.placed.borrow().clone();
        next
    }

    fn read(body_path: &Path) -> AadDocument {
        match fs::read(body_path) {
            Ok(bytes) => AadDocument::of(body_path, &String::from_utf8_lossy(&bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => AadDocument::of(body_path, ""),
            Err(error) => {
                let mut document = AadDocument::of(body_path, "");
                document.unreadable = Some(error.to_string());
                document
            }
        }
    }

    /// Whether it can be edited: it read.
    pub fn is_usable(&self) -> bool {
        self.unreadable.is_none()
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

/// The open activity files of one project.
#[derive(Debug, Default)]
pub struct AadStore {
    documents: Vec<AadDocument>,
}

impl AadStore {
    pub fn get(&self, body_path: &Path) -> Option<&AadDocument> {
        self.documents.iter().find(|d| d.body_path == body_path)
    }

    fn put(&mut self, document: AadDocument) {
        match self.documents.iter().position(|d| d.body_path == document.body_path) {
            Some(at) => {
                let previous = std::mem::replace(&mut self.documents[at], AadDocument::of(&document.body_path, ""));
                self.documents[at] = document.after(Some(&previous));
            }
            None => self.documents.push(document),
        }
    }

    fn reload(&mut self, body_path: &Path) {
        self.put(AadDocument::read(body_path));
    }
}

impl ModuleDocuments for AadStore {
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
                self.put(AadDocument::of(path, ""));
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
pub fn get<'a>(documents: &'a DiagramDocuments, body_path: &Path) -> Option<&'a AadDocument> {
    documents.module_state_ref::<AadStore>(STATE_KEY)?.get(body_path)
}

/// The document of `body_path`, read the first time it is asked for.
pub fn get_or_load<'a>(documents: &'a mut DiagramDocuments, body_path: &Path) -> &'a AadDocument {
    let store = documents.module_state::<AadStore>(STATE_KEY);
    let at = match store.documents.iter().position(|d| d.body_path == body_path) {
        Some(at) => at,
        None => {
            store.documents.push(AadDocument::read(body_path));
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
    let mut document = AadDocument::of(body_path, text);
    document.bom = bom;
    documents.module_state::<AadStore>(STATE_KEY).put(document);
    documents.mark_changed(body_path);
    Ok(())
}

/// The document of `body_path` as the file is now: read again when an agent wrote it since it was read, so an edit is
/// made on what the agent wrote (`IEditSavingDocumentStore`).
pub fn current<'a>(documents: &'a mut DiagramDocuments, body_path: &Path) -> &'a AadDocument {
    get_or_load(documents, body_path);
    if let DiskChange::Reloaded(_) = documents.module_state::<AadStore>(STATE_KEY).reconcile(body_path) {
        documents.mark_changed(body_path);
    }
    get_or_load(documents, body_path)
}

/// Reads the body at `body_path` again from disk, as a change from outside asks.
pub fn reload(documents: &mut DiagramDocuments, body_path: &Path) {
    documents.module_state::<AadStore>(STATE_KEY).reload(body_path);
    documents.mark_changed(body_path);
}
