//! The one owner of the open Databricks files: one per body, shared by every view of it, whichever of the family's
//! three readings each shows (etalii.adp spec 030, task T081).
//!
//! Ported from the standalone host's `src/diagrams/databricks/backend/EtAlii.Adp.Diagram.Databricks/
//! DatabricksDocumentStore.cs` and `IDatabricksDocumentStore.cs` at `8b6ccfc`, over the core's document lifecycle, and
//! kept in the project's `DiagramDocuments` under [`STATE_KEY`]. A text that is not YAML is kept with why, so nothing
//! is written to it until it reads again. The registrations the views were opened through are kept too, with the
//! bytes last read or written, so a position stored in one's `layout:` block reaches every view of its body, and the
//! module's own write of one is not taken for a change from outside.

use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_diagram::{DiagramDocuments, DiskChange, ModuleDocuments};

use crate::lines::LineDocument;
use crate::model::DatabricksModel;
use crate::parser;

/// The key the documents are kept under in the project's documents.
pub const STATE_KEY: &str = "databricks";

const BOM: &str = "\u{feff}";

/// One open Databricks file: its text and its readings, or why it could not be read.
#[derive(Debug, Clone)]
pub struct DatabricksDocument {
    pub body_path: PathBuf,
    /// The text without its byte order mark.
    pub text: String,
    /// Whether the file began with a UTF-8 byte order mark, which a write puts back.
    pub bom: bool,
    pub model: Rc<DatabricksModel>,
    /// The 0-based line the reader stopped on and what it said, for a text that is not YAML.
    pub problems: Vec<(usize, String)>,
    /// Why the body could not be read, or none when it could.
    pub unreadable: Option<String>,
}

impl DatabricksDocument {
    /// Whether it can be edited: it read.
    pub fn is_usable(&self) -> bool {
        self.unreadable.is_none()
    }

    fn bytes(&self) -> Vec<u8> {
        with_bom(self.bom, &self.text)
    }
}

fn with_bom(bom: bool, text: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(text.len() + 3);
    if bom {
        bytes.extend_from_slice(BOM.as_bytes());
    }
    bytes.extend_from_slice(text.as_bytes());
    bytes
}

/// A registration a view was opened through: its body, and its bytes as last read or written.
#[derive(Debug, Clone)]
struct Registration {
    path: PathBuf,
    body_path: PathBuf,
    bytes: Option<Vec<u8>>,
}

/// The open Databricks files of one project.
#[derive(Debug, Default)]
pub struct DatabricksStore {
    documents: Vec<DatabricksDocument>,
    registrations: Vec<Registration>,
}

fn read(body_path: &Path, text: &str) -> DatabricksDocument {
    let (bom, text) = match text.strip_prefix(BOM) {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let outcome = parser::outcome(&LineDocument::parse(text));
    DatabricksDocument {
        body_path: body_path.to_path_buf(),
        text: text.to_owned(),
        bom,
        model: Rc::new(outcome.model),
        problems: outcome.problems,
        unreadable: outcome.unreadable,
    }
}

/// The document as a first open reads it: one that cannot be read opens empty.
fn load(body_path: &Path) -> DatabricksDocument {
    let text = fs::read(body_path)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default();
    read(body_path, &text)
}

impl DatabricksStore {
    pub fn get(&self, body_path: &Path) -> Option<&DatabricksDocument> {
        self.documents.iter().find(|d| d.body_path == body_path)
    }

    fn put(&mut self, document: DatabricksDocument) {
        match self.documents.iter_mut().find(|d| d.body_path == document.body_path) {
            Some(slot) => *slot = document,
            None => self.documents.push(document),
        }
    }

    /// Keeps the registration at `path` of the body at `body_path`, with the bytes it holds now.
    pub fn track_registration(&mut self, path: &Path, body_path: &Path) {
        let bytes = fs::read(path).ok();
        match self.registrations.iter_mut().find(|r| r.path == path) {
            Some(slot) => {
                slot.body_path = body_path.to_path_buf();
                slot.bytes = bytes;
            }
            None => self.registrations.push(Registration {
                path: path.to_path_buf(),
                body_path: body_path.to_path_buf(),
                bytes,
            }),
        }
    }

    /// Records `bytes` as what the registration at `path` holds, the module having just written them.
    pub fn wrote_registration(&mut self, path: &Path, bytes: &[u8]) {
        if let Some(slot) = self.registrations.iter_mut().find(|r| r.path == path) {
            slot.bytes = Some(bytes.to_vec());
        }
    }
}

impl ModuleDocuments for DatabricksStore {
    /// A body or registration that holds what was last read or written needs nothing; one that is gone ends its
    /// document; any other change is read again.
    fn reconcile(&mut self, path: &Path) -> DiskChange {
        if let Some(index) = self.registrations.iter().position(|r| r.path == path) {
            let disk = fs::read(path).ok();
            let registration = &mut self.registrations[index];
            if disk == registration.bytes {
                return DiskChange::Unchanged;
            }
            registration.bytes = disk;
            return DiskChange::Reloaded(registration.body_path.clone());
        }
        let Some(document) = self.get(path) else {
            return DiskChange::NotTracked;
        };
        match fs::read(path) {
            Ok(bytes) if bytes == document.bytes() => DiskChange::Unchanged,
            Ok(bytes) => {
                let document = read(path, &String::from_utf8_lossy(&bytes));
                self.put(document);
                DiskChange::Reloaded(path.to_path_buf())
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let document = read(path, "");
                self.put(document);
                DiskChange::Deleted(path.to_path_buf())
            }
            Err(_) => DiskChange::Unchanged,
        }
    }

    fn close(&mut self, body_path: &Path) {
        self.documents.retain(|d| d.body_path != body_path);
        self.registrations.retain(|r| r.body_path != body_path);
    }
}

/// The store of the project's documents.
pub fn store(documents: &mut DiagramDocuments) -> &mut DatabricksStore {
    documents.module_state::<DatabricksStore>(STATE_KEY)
}

/// The document of `body_path` as the project's documents hold it, or none before it was opened.
pub fn get<'a>(documents: &'a DiagramDocuments, body_path: &Path) -> Option<&'a DatabricksDocument> {
    documents.module_state_ref::<DatabricksStore>(STATE_KEY)?.get(body_path)
}

/// The document of `body_path`, read the first time it is asked for.
pub fn get_or_load<'a>(documents: &'a mut DiagramDocuments, body_path: &Path) -> &'a DatabricksDocument {
    let store = store(documents);
    if store.get(body_path).is_none() {
        store.documents.push(load(body_path));
    }
    store.get(body_path).expect("just loaded")
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Writes `text` as the body at `body_path` and reads it again, so every view of it refreshes; why when the write
/// failed, with nothing changed.
pub fn save(documents: &mut DiagramDocuments, body_path: &Path, text: &str) -> Result<(), String> {
    let name = file_name(body_path);
    let current = get_or_load(documents, body_path);
    if let Some(reason) = &current.unreadable {
        // Writing a document that never read would replace a file somebody can still fix with one this module made.
        return Err(format!("{name} does not parse, so it was not written. {reason}"));
    }
    write(documents, body_path, text)
}

/// Writes `text` as the body at `body_path` whatever it held, as a restore does, and reads it again.
pub fn write(documents: &mut DiagramDocuments, body_path: &Path, text: &str) -> Result<(), String> {
    let name = file_name(body_path);
    let bom = get(documents, body_path).is_some_and(|d| d.bom);
    if let Some(folder) = body_path.parent() {
        fs::create_dir_all(folder).map_err(|e| format!("{name} could not be written: {e}"))?;
    }
    adp_documents::text::write_atomically(body_path, &with_bom(bom, text))
        .map_err(|e| format!("{name} could not be written: {e}"))?;
    let mut document = read(body_path, text);
    document.bom = bom;
    store(documents).put(document);
    documents.mark_changed(body_path);
    Ok(())
}
