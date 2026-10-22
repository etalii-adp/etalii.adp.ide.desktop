//! The open queries of a project: one reading per `.rq` file, shared by every view of it, and the registrations that
//! store their positions (etalii.adp spec 030, task T084).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/sparql/backend/EtAlii.Adp.Diagram.Sparql/
//! SparqlDocumentStore.cs` and the read-only lifecycle it uses (`DocumentLifecycle`). It is the reading half of a
//! store and no more: nothing here writes a `.rq`, so the byte-identical round trip holds by construction. A query that
//! is missing or cannot be read opens as a state named in the module's words; one that does not parse opens with an
//! empty model and the parser's sentence. A change on disk reaches the store through the project's reconciliation
//! ([`ModuleDocuments`]): a reload that cannot read the file keeps the last good query, a deleted file clears it, a
//! change to a registration is read again without reading the query, and the store's own write of a registration is
//! ignored.

use std::fs;
use std::path::{Path, PathBuf};

use adp_diagram::{DiagramDocuments, DiskChange, ModuleDocuments};

use crate::model::SparqlQueryModel;
use crate::parser;

/// The key the open queries are kept under in the project's documents.
pub const STATE_KEY: &str = "sparql";

/// One open query: its text, what the parser read, and why it cannot be drawn when it cannot.
#[derive(Debug, Clone)]
pub struct OpenQuery {
    /// The query's full path.
    pub path: PathBuf,
    /// The text as last read; empty when it could not be.
    pub text: String,
    /// What the parser read; empty when the query is unavailable or does not parse.
    pub model: SparqlQueryModel,
    /// Whether the text was read and parsed.
    pub usable: bool,
    /// Why the query cannot be drawn: missing, unreadable, or the parser's sentence; empty when it can.
    pub error: String,
    /// Whether the text was read at all.
    pub readable: bool,
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The text of the file at `path` as .NET's `File.ReadAllText` reads it: UTF-8, its byte order mark dropped.
fn read(path: &Path) -> std::io::Result<String> {
    fs::read(path).map(|bytes| {
        let text = String::from_utf8_lossy(&bytes).into_owned();
        text.strip_prefix('\u{feff}').map(str::to_owned).unwrap_or(text)
    })
}

impl OpenQuery {
    /// The query at `path`, as it now reads.
    pub fn open(path: &Path) -> OpenQuery {
        match read(path) {
            Ok(text) => OpenQuery::parsed(path, text),
            Err(error) => OpenQuery::unavailable(path, error),
        }
    }

    fn parsed(path: &Path, text: String) -> OpenQuery {
        let (model, usable, error) = match parser::parse(&text) {
            Ok(model) => (model, true, String::new()),
            Err(error) => (SparqlQueryModel::default(), false, error.message),
        };
        OpenQuery {
            path: path.to_path_buf(),
            text,
            model,
            usable,
            error,
            readable: true,
        }
    }

    /// A query that is missing or cannot be read opens as a state this module names rather than as an empty query.
    fn unavailable(path: &Path, error: std::io::Error) -> OpenQuery {
        let name = file_name(path);
        let error = if error.kind() == std::io::ErrorKind::NotFound {
            format!(
                "{name} does not exist. Queries are authored in a text editor; this diagram draws an existing .rq file."
            )
        } else {
            format!("{name} could not be read: {error}")
        };
        OpenQuery {
            path: path.to_path_buf(),
            text: String::new(),
            model: SparqlQueryModel::default(),
            usable: false,
            error,
            readable: false,
        }
    }

    /// The model when the query parsed.
    pub fn usable_model(&self) -> Option<&SparqlQueryModel> {
        self.usable.then_some(&self.model)
    }
}

/// One registration of a query: its path, the query it names, and its text as last read or written.
#[derive(Debug)]
struct Registration {
    path: PathBuf,
    query: PathBuf,
    text: String,
}

fn read_text(path: &Path) -> String {
    read(path).unwrap_or_default()
}

/// Whether `a` and `b` name one file: compared by their parts, so a `/` and a `\` name the same folder on Windows,
/// case folded where the platform ignores case.
fn same(a: &Path, b: &Path) -> bool {
    let key = |path: &Path| {
        let parts: Vec<String> = path
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        let text = parts.join("/");
        if cfg!(any(windows, target_os = "macos")) {
            text.to_lowercase()
        } else {
            text
        }
    };
    key(a) == key(b)
}

/// The open queries of one project, and their registrations.
#[derive(Debug, Default)]
pub struct OpenQueries {
    queries: Vec<OpenQuery>,
    registrations: Vec<Registration>,
}

impl OpenQueries {
    pub fn get(&self, query: &Path) -> Option<&OpenQuery> {
        self.queries.iter().find(|q| same(&q.path, query))
    }

    /// The text of `registration` as last read or written; empty when it is not open.
    pub fn registration_text(&self, registration: &Path) -> &str {
        self.registrations
            .iter()
            .find(|r| same(&r.path, registration))
            .map_or("", |r| r.text.as_str())
    }
}

impl ModuleDocuments for OpenQueries {
    fn reconcile(&mut self, path: &Path) -> DiskChange {
        if let Some(registration) = self.registrations.iter_mut().find(|r| same(&r.path, path)) {
            let text = read_text(path);
            if text == registration.text {
                return DiskChange::Unchanged;
            }
            registration.text = text;
            return DiskChange::Reloaded(registration.query.clone());
        }
        let Some(query) = self.queries.iter_mut().find(|q| same(&q.path, path)) else {
            return DiskChange::NotTracked;
        };
        if !path.exists() {
            // The watcher's delete clears the last good query.
            *query = OpenQuery::open(&query.path.clone());
            return DiskChange::Deleted(query.path.clone());
        }
        match read(path) {
            // A reload that cannot read keeps the last good query.
            Err(_) => DiskChange::Unchanged,
            Ok(text) if query.readable && text == query.text => DiskChange::Unchanged,
            Ok(text) => {
                *query = OpenQuery::parsed(&query.path.clone(), text);
                DiskChange::Reloaded(query.path.clone())
            }
        }
    }

    fn close(&mut self, body_path: &Path) {
        self.queries.retain(|q| !same(&q.path, body_path));
        self.registrations.retain(|r| !same(&r.query, body_path));
    }
}

/// The query at `query` as the project's documents hold it, or none before it was opened.
pub fn get<'a>(documents: &'a DiagramDocuments, query: &Path) -> Option<&'a OpenQuery> {
    documents.module_state_ref::<OpenQueries>(STATE_KEY)?.get(query)
}

/// The text of `registration` as the project's documents hold it.
pub fn registration_text<'a>(documents: &'a DiagramDocuments, registration: &Path) -> &'a str {
    documents
        .module_state_ref::<OpenQueries>(STATE_KEY)
        .map_or("", |s| s.registration_text(registration))
}

/// Opens the query at `query`, read the first time it is asked for, with `registration` storing its positions.
pub fn open(documents: &mut DiagramDocuments, query: &Path, registration: Option<&Path>) {
    let store = documents.module_state::<OpenQueries>(STATE_KEY);
    if store.get(query).is_none() {
        store.queries.push(OpenQuery::open(query));
    }
    if let Some(registration) = registration {
        match store.registrations.iter_mut().find(|r| same(&r.path, registration)) {
            Some(open) => open.text = read_text(registration),
            None => store.registrations.push(Registration {
                path: registration.to_path_buf(),
                query: query.to_path_buf(),
                text: read_text(registration),
            }),
        }
    }
}

/// Writes `text` as the registration at `registration`, so every view of its query draws the stored positions; why
/// when the write failed, with nothing changed.
pub fn save_registration(documents: &mut DiagramDocuments, registration: &Path, text: &str) -> Result<(), String> {
    adp_documents::text::write_atomically(registration, text.as_bytes())
        .map_err(|e| format!("Could not store the position: {e}"))?;
    let store = documents.module_state::<OpenQueries>(STATE_KEY);
    let query = store
        .registrations
        .iter_mut()
        .find(|r| same(&r.path, registration))
        .map(|open| {
            open.text = text.to_owned();
            open.query.clone()
        });
    if let Some(query) = query {
        documents.mark_changed(&query);
    }
    Ok(())
}
