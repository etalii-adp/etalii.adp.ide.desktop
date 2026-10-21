//! The one owner of the family's open documents (etalii.adp spec 030, task T083): one per body, shared by every reading
//! of it, so an edit made through one reading is what every other reading of the file draws.
//!
//! Ported from the standalone host's `IRdfDocumentStore.cs` and `RdfDocumentStore.cs` at `8b6ccfc`, over the core's
//! document lifecycle, and kept in the project's `DiagramDocuments` under [`STATE_KEY`]. A text that is not Turtle is an
//! ordinary state for a file somebody is editing, so it is held as unreadable with the parser's sentence and the line
//! it stopped at, and nothing is written to it until it reads again.

use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_diagram::{DiagramDocuments, DiskChange, ModuleDocuments};

use crate::lines::LineDocument;
use crate::model::RdfModel;
use crate::parser;

/// The key the documents are kept under in the project's documents.
pub const STATE_KEY: &str = "w3c-rdf";

const BOM: &str = "\u{feff}";

/// One open document: its text, its lines and what it states, or why it could not be read.
#[derive(Debug, Clone)]
pub struct RdfDocument {
    pub body_path: PathBuf,
    /// The text without its byte order mark.
    pub text: String,
    /// Whether the file began with a UTF-8 byte order mark, which a write puts back.
    pub bom: bool,
    pub lines: Rc<LineDocument>,
    pub model: Rc<RdfModel>,
    /// The 0-based line and the sentence the parser stopped at, for a text that does not parse.
    pub problem: Option<(usize, String)>,
    /// Why the body could not be read, or none when it could.
    pub unreadable: Option<String>,
}

impl RdfDocument {
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

    /// The document `text` reads as.
    pub fn read(body_path: &Path, text: &str) -> RdfDocument {
        let (bom, text) = match text.strip_prefix(BOM) {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        let lines = LineDocument::parse(text);
        let outcome = parser::outcome(&lines);
        RdfDocument {
            body_path: body_path.to_path_buf(),
            text: text.to_owned(),
            bom,
            lines: Rc::new(lines),
            model: Rc::new(outcome.model),
            problem: outcome.problem,
            unreadable: outcome.unreadable,
        }
    }
}

/// The open documents of one project.
#[derive(Debug, Default)]
pub struct RdfStore {
    documents: Vec<RdfDocument>,
}

impl RdfStore {
    pub fn get(&self, body_path: &Path) -> Option<&RdfDocument> {
        self.documents.iter().find(|d| d.body_path == body_path)
    }

    fn load(body_path: &Path) -> RdfDocument {
        let text = fs::read(body_path)
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_default();
        RdfDocument::read(body_path, &text)
    }

    fn put(&mut self, document: RdfDocument) {
        match self.documents.iter_mut().find(|d| d.body_path == document.body_path) {
            Some(slot) => *slot = document,
            None => self.documents.push(document),
        }
    }
}

impl ModuleDocuments for RdfStore {
    /// A body that holds what its document read or wrote needs nothing; one that is gone reads as empty; any other
    /// change is read again.
    fn reconcile(&mut self, path: &Path) -> DiskChange {
        let Some(document) = self.get(path) else {
            return DiskChange::NotTracked;
        };
        match fs::read(path) {
            Ok(bytes) if bytes == document.bytes() => DiskChange::Unchanged,
            Ok(bytes) => {
                self.put(RdfDocument::read(path, &String::from_utf8_lossy(&bytes)));
                DiskChange::Reloaded(path.to_path_buf())
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                self.put(RdfDocument::read(path, ""));
                DiskChange::Deleted(path.to_path_buf())
            }
            Err(_) => DiskChange::Unchanged,
        }
    }

    fn close(&mut self, body_path: &Path) {
        self.documents.retain(|d| d.body_path != body_path);
    }
}

/// The store of the project's documents.
pub fn store(documents: &mut DiagramDocuments) -> &mut RdfStore {
    documents.module_state::<RdfStore>(STATE_KEY)
}

/// The document of `body_path` as the project's documents hold it, or none before it was opened.
pub fn get<'a>(documents: &'a DiagramDocuments, body_path: &Path) -> Option<&'a RdfDocument> {
    documents.module_state_ref::<RdfStore>(STATE_KEY)?.get(body_path)
}

/// The document of `body_path`, read the first time it is asked for.
pub fn get_or_load<'a>(documents: &'a mut DiagramDocuments, body_path: &Path) -> &'a RdfDocument {
    let store = store(documents);
    if store.get(body_path).is_none() {
        let document = RdfStore::load(body_path);
        store.documents.push(document);
    }
    store.get(body_path).expect("just loaded")
}

/// Writes `text` as the body at `body_path` and reads it again, so every reading of it refreshes; why when the write
/// failed, with nothing changed.
pub fn save(documents: &mut DiagramDocuments, body_path: &Path, text: &str) -> Result<(), String> {
    let name = file_name(body_path);
    let current = get_or_load(documents, body_path);
    if let Some(reason) = &current.unreadable {
        return Err(format!("{name} does not parse, so it was not written. {reason}"));
    }
    let bom = current.bom;
    let mut bytes = Vec::with_capacity(text.len() + 3);
    if bom {
        bytes.extend_from_slice(BOM.as_bytes());
    }
    bytes.extend_from_slice(text.as_bytes());
    adp_documents::text::write_atomically(body_path, &bytes)
        .map_err(|e| format!("{name} could not be written: {e}"))?;
    let mut document = RdfDocument::read(body_path, text);
    document.bom = bom;
    store(documents).put(document);
    documents.mark_changed(body_path);
    Ok(())
}

/// Writes `text` as the registration at `registration`, as a stored position does.
pub fn save_registration(documents: &mut DiagramDocuments, registration: &Path, text: &str) -> Result<(), String> {
    adp_documents::text::write_atomically(registration, text.as_bytes())
        .map_err(|e| format!("Could not store the position: {e}"))?;
    documents.mark_changed(registration);
    Ok(())
}

/// The file name of `path`.
pub fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}
