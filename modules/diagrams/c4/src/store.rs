//! The one owner of the open C4 documents: one per `.dsl`, shared by every view of it (etalii.adp spec 030, task
//! T061).
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4/C4DocumentStore.cs` and
//! `IC4DocumentStore.cs` at `8b6ccfc`, over the core's `LifecycleDocumentStore` and `DocumentEditRun`. Several
//! diagrams open one document, which is what "model once, view many" means, so they share one reading of it, kept in
//! the project's `DiagramDocuments` under [`STATE_KEY`]; an edit through any view reaches all of them. Every text
//! reads: the parser passes over what it cannot read and the rules report it, so a body is unreadable only when the
//! read itself failed. An edit is made on a copy of the lines, written whole, and read again.

use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_diagram::DiagramDocuments;

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
    fn read(body_path: &Path) -> C4Document {
        match fs::read(body_path) {
            Ok(bytes) => C4Document::of(body_path, &String::from_utf8_lossy(&bytes)),
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
    let bom = get_or_load(documents, body_path).bom;
    let text = lines.text();
    let mut bytes = Vec::with_capacity(text.len() + 3);
    if bom {
        bytes.extend_from_slice(BOM.as_bytes());
    }
    bytes.extend_from_slice(text.as_bytes());
    adp_documents::text::write_atomically(body_path, &bytes).map_err(|e| {
        format!(
            "{} could not be written: {e}",
            body_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        )
    })?;
    let mut document = C4Document::of(body_path, &text);
    document.bom = bom;
    let store = documents.module_state::<C4Store>(STATE_KEY);
    match store.documents.iter_mut().find(|d| d.body_path == body_path) {
        Some(slot) => *slot = document,
        None => store.documents.push(document),
    }
    documents.mark_changed(body_path);
    Ok(())
}

/// Reads the body at `body_path` again from disk, as a change from outside asks.
pub fn reload(documents: &mut DiagramDocuments, body_path: &Path) {
    let document = C4Document::read(body_path);
    let store = documents.module_state::<C4Store>(STATE_KEY);
    match store.documents.iter_mut().find(|d| d.body_path == body_path) {
        Some(slot) => *slot = document,
        None => store.documents.push(document),
    }
    documents.mark_changed(body_path);
}

/// Tells every view of the body at `body_path` to show it again without changing it: a move changes where an element
/// is drawn, not what the model says (`IC4DocumentStore.Touch`).
pub fn touch(documents: &mut DiagramDocuments, body_path: &Path) {
    get_or_load(documents, body_path);
    documents.mark_changed(body_path);
}
