//! The one owner of the open pipelines: one per body, shared by every view of it, read with the templates of the
//! project it was opened in (etalii.adp spec 030, task T078).
//!
//! Ported from the standalone host's `src/diagrams/azure-devops-pipeline/backend/EtAlii.Adp.Diagram.AzureDevOpsPipeline/
//! PipelineDocumentStore.cs` and `IPipelineDocumentStore.cs` at `8b6ccfc`, over the core's document lifecycle, and kept
//! in the project's `DiagramDocuments` under [`STATE_KEY`]. A pipeline's reading needs its workspace root to follow its
//! templates, so each document keeps the root it was opened from, and the templates read are kept per root until a
//! change on disk asks for them again. A text that is not YAML is kept with why, so nothing is written to it until it
//! reads again.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_diagram::{DiagramDocuments, DiskChange, ModuleDocuments};

use crate::lines::LineDocument;
use crate::model::PipelineModel;
use crate::parser;
use crate::templates::PipelineTemplates;

/// The key the pipelines are kept under in the project's documents.
pub const STATE_KEY: &str = "azure-devops-pipeline";

const BOM: &str = "\u{feff}";

/// One open pipeline: its text and its reading, or why it could not be read.
#[derive(Debug, Clone)]
pub struct PipelineDocument {
    pub root_path: PathBuf,
    pub body_path: PathBuf,
    /// The text without its byte order mark.
    pub text: String,
    /// Whether the file began with a UTF-8 byte order mark, which a write puts back.
    pub bom: bool,
    pub model: Rc<PipelineModel>,
    /// Why the body could not be read, or none when it could.
    pub unreadable: Option<String>,
}

impl PipelineDocument {
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

/// The open pipelines of one project, and the templates each workspace root has read.
#[derive(Debug, Default)]
pub struct PipelineStore {
    documents: Vec<PipelineDocument>,
    templates: HashMap<PathBuf, Rc<PipelineTemplates>>,
}

impl PipelineStore {
    pub fn get(&self, body_path: &Path) -> Option<&PipelineDocument> {
        self.documents.iter().find(|d| d.body_path == body_path)
    }

    /// The templates of the workspace at `root`.
    pub fn resolver(&mut self, root: &Path) -> Rc<PipelineTemplates> {
        self.templates
            .entry(root.to_path_buf())
            .or_insert_with(|| Rc::new(PipelineTemplates::new(root)))
            .clone()
    }

    /// The reading of `document`, the body at `body_path` of the workspace at `root`.
    pub fn model_of(&mut self, root: &Path, body_path: &Path, document: &LineDocument) -> PipelineModel {
        let resolver = self.resolver(root);
        parser::outcome(document, Some(&resolver), body_path).model
    }

    fn read(&mut self, root: &Path, body_path: &Path, text: &str) -> PipelineDocument {
        let (bom, text) = match text.strip_prefix(BOM) {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        let resolver = self.resolver(root);
        let outcome = parser::outcome(&LineDocument::parse(text), Some(&resolver), body_path);
        PipelineDocument {
            root_path: root.to_path_buf(),
            body_path: body_path.to_path_buf(),
            text: text.to_owned(),
            bom,
            model: Rc::new(outcome.model),
            unreadable: outcome.unreadable,
        }
    }

    /// The document as a first open reads it: one that cannot be read opens empty.
    fn load(&mut self, root: &Path, body_path: &Path) -> PipelineDocument {
        let text = fs::read(body_path)
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_default();
        self.read(root, body_path, &text)
    }

    fn put(&mut self, document: PipelineDocument) {
        match self.documents.iter_mut().find(|d| d.body_path == document.body_path) {
            Some(slot) => *slot = document,
            None => self.documents.push(document),
        }
    }

    /// Forgets the templates read, since what changed on disk may have been one of them.
    fn forget_templates(&mut self) {
        for resolver in self.templates.values() {
            resolver.forget();
        }
    }
}

impl ModuleDocuments for PipelineStore {
    /// A body that holds what its document read or wrote needs nothing; one that is gone ends its document; any other
    /// change is read again, templates and all.
    fn reconcile(&mut self, path: &Path) -> DiskChange {
        let Some(document) = self.get(path) else {
            return DiskChange::NotTracked;
        };
        let root = document.root_path.clone();
        match fs::read(path) {
            Ok(bytes) if bytes == document.bytes() => DiskChange::Unchanged,
            Ok(bytes) => {
                self.forget_templates();
                let document = self.read(&root, path, &String::from_utf8_lossy(&bytes));
                self.put(document);
                DiskChange::Reloaded(path.to_path_buf())
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                self.forget_templates();
                let document = self.read(&root, path, "");
                self.put(document);
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
pub fn store(documents: &mut DiagramDocuments) -> &mut PipelineStore {
    documents.module_state::<PipelineStore>(STATE_KEY)
}

/// The document of `body_path` as the project's documents hold it, or none before it was opened.
pub fn get<'a>(documents: &'a DiagramDocuments, body_path: &Path) -> Option<&'a PipelineDocument> {
    documents.module_state_ref::<PipelineStore>(STATE_KEY)?.get(body_path)
}

/// The document of `body_path` in the workspace at `root`, read the first time it is asked for.
pub fn get_or_load<'a>(documents: &'a mut DiagramDocuments, root: &Path, body_path: &Path) -> &'a PipelineDocument {
    let store = store(documents);
    if store.get(body_path).is_none() {
        let document = store.load(root, body_path);
        store.documents.push(document);
    }
    store.get(body_path).expect("just loaded")
}

/// Writes `text` as the body at `body_path` and reads it again, so every view of it refreshes; why when the write
/// failed, with nothing changed.
pub fn save(documents: &mut DiagramDocuments, root: &Path, body_path: &Path, text: &str) -> Result<(), String> {
    let name = body_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let current = get_or_load(documents, root, body_path);
    if let Some(reason) = &current.unreadable {
        // Writing a document that never read would replace a file somebody can still fix with one this module made.
        return Err(format!("{name} does not parse, so it was not written. {reason}"));
    }
    let bom = current.bom;
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
    let store = store(documents);
    let mut document = store.read(root, body_path, text);
    document.bom = bom;
    store.put(document);
    documents.mark_changed(body_path);
    Ok(())
}
