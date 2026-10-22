//! The one owner of the open Wardley maps: one reading per `.owm`, shared by every view of it, with the identities
//! held beside it (etalii.adp spec 030, task T070).
//!
//! Ported from the standalone host's `WardleyDocumentStore.cs` and `IWardleyDocumentStore.cs` at `8b6ccfc`, over the
//! core's `LifecycleDocumentStore`. A body's text reads as a document and a map; a missing body opens as an empty map,
//! created on the first save. The identities are reconciled once, the first time they are asked for, and held for as
//! long as the document is: an unmatched element is given a fresh ShortGuid, so two callers reconciling apart would
//! mint two different sets. A save reconciles them against what is held (anything that survived keeps its id) and
//! only then writes the sidecar; a reload, a restored text included, reconciles them without writing it. A rename
//! carries its component's identity to the new name ([`rekey`]).
//!
//! The ids a reconciliation mints come from the store's [`IdSource`]: ShortGuids, or for a replay the ids the
//! standalone host generated ([`set_ids`]).

use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_diagram::{DiagramDocuments, DiskChange, ModuleDocuments};
use adp_disl::IdSource;

use crate::identities::{self, ShortGuids, WardleyIdentityEntry};
use crate::lines::LineDocument;
use crate::model::{ParseProblem, WardleyMap};
use crate::parser;

/// The key the Wardley maps are kept under in the project's documents.
pub const STATE_KEY: &str = "wardley";

const BOM: &str = "\u{feff}";

/// One open `.owm`: its text and its reading, or why it could not be read, and the identities held beside it.
#[derive(Debug, Clone)]
pub struct WardleyDocument {
    pub body_path: PathBuf,
    /// The text without its byte order mark.
    pub text: String,
    /// Whether the file began with a UTF-8 byte order mark, which an edit's write puts back.
    pub bom: bool,
    pub map: Rc<WardleyMap>,
    /// The lines the parser passed over, with why.
    pub problems: Vec<ParseProblem>,
    /// Why the body could not be read, or none when it could.
    pub unreadable: Option<String>,
    /// The identities of the map's elements, once they were asked for.
    identities: Option<Vec<WardleyIdentityEntry>>,
}

impl WardleyDocument {
    /// The document of the body at `body_path` as a first open reads it: a body that does not exist yet is a new,
    /// empty map that can be saved; one that is there and cannot be read is unreadable.
    fn read(body_path: &Path) -> WardleyDocument {
        match fs::read(body_path) {
            Ok(bytes) => WardleyDocument::of(body_path, &String::from_utf8_lossy(&bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => WardleyDocument::of(body_path, ""),
            Err(error) => WardleyDocument {
                unreadable: Some(error.to_string()),
                ..WardleyDocument::of(body_path, "")
            },
        }
    }

    fn of(body_path: &Path, text: &str) -> WardleyDocument {
        let (bom, text) = match text.strip_prefix(BOM) {
            Some(rest) => (true, rest),
            None => (false, text),
        };
        let (map, problems) = parser::outcome(&LineDocument::parse(text));
        WardleyDocument {
            body_path: body_path.to_path_buf(),
            text: text.to_owned(),
            bom,
            map: Rc::new(map),
            problems,
            unreadable: None,
            identities: None,
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

    fn bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.text.len() + 3);
        if self.bom {
            bytes.extend_from_slice(BOM.as_bytes());
        }
        bytes.extend_from_slice(self.text.as_bytes());
        bytes
    }
}

/// The open Wardley maps of one project, and where the ids of their new elements come from.
#[derive(Default)]
pub struct WardleyStore {
    documents: Vec<WardleyDocument>,
    ids: Option<Rc<dyn IdSource>>,
}

impl std::fmt::Debug for WardleyStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WardleyStore")
            .field("documents", &self.documents)
            .finish_non_exhaustive()
    }
}

impl WardleyStore {
    pub fn get(&self, body_path: &Path) -> Option<&WardleyDocument> {
        self.documents.iter().find(|d| d.body_path == body_path)
    }

    fn index_or_load(&mut self, body_path: &Path) -> usize {
        match self.documents.iter().position(|d| d.body_path == body_path) {
            Some(at) => at,
            None => {
                self.documents.push(WardleyDocument::read(body_path));
                self.documents.len() - 1
            }
        }
    }

    fn reconcile_with(&self, map: &WardleyMap, known: &[WardleyIdentityEntry]) -> Vec<WardleyIdentityEntry> {
        match &self.ids {
            Some(ids) => identities::reconcile(map, known, ids.as_ref()),
            None => identities::reconcile(map, known, &ShortGuids),
        }
    }

    /// Installs `document` in place of the one of its body, reconciling the identities held for that body against it
    /// without writing them (`Installed`).
    fn install(&mut self, mut document: WardleyDocument) {
        let at = self.documents.iter().position(|d| d.body_path == document.body_path);
        let held = at.and_then(|at| self.documents[at].identities.clone());
        document.identities = Some(self.reconcile_with(&document.map, held.as_deref().unwrap_or_default()));
        match at {
            Some(at) => self.documents[at] = document,
            None => self.documents.push(document),
        }
    }

    /// Reads the body again; false when a read that fails keeps the last good document.
    fn reload(&mut self, body_path: &Path) -> bool {
        let usable = self.get(body_path).is_some_and(WardleyDocument::is_usable);
        let document = match fs::read(body_path) {
            Ok(bytes) => WardleyDocument::of(body_path, &String::from_utf8_lossy(&bytes)),
            Err(_) if usable => return false,
            Err(_) => WardleyDocument::read(body_path),
        };
        if self
            .get(body_path)
            .is_some_and(|held| held.text == document.text && held.bom == document.bom)
        {
            return false;
        }
        self.install(document);
        true
    }
}

impl ModuleDocuments for WardleyStore {
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
                self.install(WardleyDocument::of(path, ""));
                DiskChange::Deleted(path.to_path_buf())
            }
            Err(_) => DiskChange::Unchanged,
        }
    }

    fn close(&mut self, body_path: &Path) {
        self.documents.retain(|d| d.body_path != body_path);
    }
}

/// Where the ids of the project's new elements come from from now on: `ids`, or ShortGuids when none.
pub fn set_ids(documents: &mut DiagramDocuments, ids: Option<Rc<dyn IdSource>>) {
    documents.module_state::<WardleyStore>(STATE_KEY).ids = ids;
}

/// The document of `body_path` as the project's documents hold it, or none before it was opened.
pub fn get<'a>(documents: &'a DiagramDocuments, body_path: &Path) -> Option<&'a WardleyDocument> {
    documents.module_state_ref::<WardleyStore>(STATE_KEY)?.get(body_path)
}

/// The document of `body_path`, read the first time it is asked for, with its identities reconciled the first time.
pub fn get_or_load<'a>(documents: &'a mut DiagramDocuments, body_path: &Path) -> &'a WardleyDocument {
    let store = documents.module_state::<WardleyStore>(STATE_KEY);
    let at = store.index_or_load(body_path);
    if store.documents[at].identities.is_none() {
        let recorded = identities::read(body_path);
        let reconciled = store.reconcile_with(&store.documents[at].map, &recorded);
        store.documents[at].identities = Some(reconciled);
    }
    &store.documents[at]
}

/// The identities held for the map at `body_path`; empty before it was opened.
pub fn identities_of<'a>(documents: &'a DiagramDocuments, body_path: &Path) -> &'a [WardleyIdentityEntry] {
    get(documents, body_path)
        .and_then(|d| d.identities.as_deref())
        .unwrap_or_default()
}

/// Moves an identity from one key to another, for a rename: reconciliation matches by key, and a component's key is
/// its name, so without this a rename looks like one element leaving and another arriving.
pub fn rekey(documents: &mut DiagramDocuments, body_path: &Path, kind: &str, old_key: &str, new_key: &str) {
    get_or_load(documents, body_path);
    let store = documents.module_state::<WardleyStore>(STATE_KEY);
    let at = store.index_or_load(body_path);
    if let Some(held) = store.documents[at].identities.as_mut() {
        for entry in held.iter_mut().filter(|e| e.kind == kind && e.key == old_key) {
            entry.key = new_key.to_owned();
        }
    }
}

/// Writes `lines` as the text of the body at `body_path`, reads it again so every view refreshes, reconciles the
/// identities against what is held and writes the sidecar: the warning the sidecar's write left, or why the body
/// could not be written, with nothing changed.
pub fn save(documents: &mut DiagramDocuments, body_path: &Path, lines: &LineDocument) -> Result<String, String> {
    let name = file_name(body_path);
    let document = get_or_load(documents, body_path);
    if let Some(reason) = &document.unreadable {
        // An entry standing in for a body that could not be read is never written over the real file.
        return Err(format!("{name} could not be read, so it was not written: {reason}"));
    }
    let bom = document.bom;
    let held = document.identities.clone().unwrap_or_default();
    let text = lines.text();
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
    let mut saved = WardleyDocument::of(body_path, &text);
    saved.bom = bom;
    let store = documents.module_state::<WardleyStore>(STATE_KEY);
    let reconciled = store.reconcile_with(&saved.map, &held);
    let warning = identities::write(body_path, &reconciled);
    saved.identities = Some(reconciled);
    let at = store.index_or_load(body_path);
    store.documents[at] = saved;
    documents.mark_changed(body_path);
    Ok(warning)
}

/// Writes `text` over the body at `body_path` as it was before an edit, and reads it again through the store, as the
/// standalone host's `RestoreDocumentCommandHandler` wrote it with `AdpFileWriter.Save` and reloaded: the identities
/// are reconciled, the sidecar is not written.
pub fn restore(documents: &mut DiagramDocuments, body_path: &Path, text: &str) -> Result<(), String> {
    // The text is kept without its byte order mark; the file gets it back as an edit's write gives it.
    let bom = get_or_load(documents, body_path).bom;
    let mut bytes = Vec::with_capacity(text.len() + 3);
    if bom {
        bytes.extend_from_slice(BOM.as_bytes());
    }
    bytes.extend_from_slice(text.as_bytes());
    adp_documents::text::write_atomically(body_path, &bytes).map_err(|e| format!("Could not restore the file: {e}"))?;
    if documents.module_state::<WardleyStore>(STATE_KEY).reload(body_path) {
        documents.mark_changed(body_path);
    }
    Ok(())
}

/// Reads the body at `body_path` again from disk, as a change from outside asks; a read that fails keeps the last
/// good document.
pub fn reload(documents: &mut DiagramDocuments, body_path: &Path) {
    if documents.module_state::<WardleyStore>(STATE_KEY).reload(body_path) {
        documents.mark_changed(body_path);
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}
