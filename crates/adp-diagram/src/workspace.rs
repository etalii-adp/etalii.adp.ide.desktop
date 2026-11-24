//! One project's diagrams: opening one from a body or from its registration, the project's history every edit goes
//! through, and a change on disk reconciled with the history's barrier (etalii.adp spec 030, FR-016, FR-023;
//! contracts/core-shell.md, `open`, `undo`, `redo`; Story 2 scenario 5).
//!
//! Ported from the standalone host's `EtAlii.Adp.Workspace/DiagramService.cs` and `DiagramService.Open.cs` at
//! `8b6ccfc`, the half that resolves what a request names and opens its session: the path must be inside the project,
//! a registration names its tool type on line 1 and its body by FBL section 8.2, and a body opens as the tool type
//! that claims its extension, with the registration beside it when there is one. The gRPC stream, the pump and the
//! editor fallback are the shell's.

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;

use adp_fbl::RegistrationDocument;
use adp_fbl::registration::locate_body;
use adp_history::{Command, History, Outcome};

use crate::document::{DiagramDocuments, DiskChange, DocumentEdit};
use crate::session::DiagramSession;
use crate::tool::{DiagramTool, DiagramTools};

/// The refusal of a path that names no diagram, as the standalone host words it.
pub const CANNOT_OPEN: &str = "The diagram cannot be opened.";

/// Whether the file at `path` is a registration: its name ends in `.adp`, a folder's `.adp` alone included
/// (`DiagramFilePair.IsRegistrationFile`), which is how a folder subject such as `ansible/structure` is registered.
pub fn is_registration(path: &Path) -> bool {
    path.file_name()
        .is_some_and(|n| n.to_string_lossy().to_lowercase().ends_with(".adp"))
}

/// What an open resolved: the tool type, the body and the registration it was opened through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedDiagram {
    pub origin: String,
    pub body_path: PathBuf,
    pub registration_path: Option<PathBuf>,
}

/// One project's open diagrams and its history.
pub struct DiagramWorkspace {
    root: PathBuf,
    documents: DiagramDocuments,
    history: History<DiagramDocuments>,
    /// The edits run since [`DiagramWorkspace::take_executed`] was last asked, in order.
    executed: Vec<DocumentEdit>,
    /// What reads the documents of other kinds of tool for the links index (FBL section 17.3): a designer's.
    indexers: Vec<LinkIndexer>,
    /// The open diagrams whose load-time validation has changes to write, with their first edit.
    unchecked: std::collections::HashSet<PathBuf>,
}

/// What reads a file of another kind of tool than a diagram for the workspace's links index: the document as the
/// index holds it, or none when the file is not one it reads.
pub type LinkIndexer = Rc<dyn Fn(&Path) -> Option<adp_links::IndexedDocument>>;

impl DiagramWorkspace {
    /// The diagrams of the project at `root`, none open yet.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        DiagramWorkspace {
            root: root.into(),
            documents: DiagramDocuments::default(),
            history: History::default(),
            executed: Vec::new(),
            indexers: Vec::new(),
            unchecked: std::collections::HashSet::new(),
        }
    }

    /// Diagrams held in memory, named under `root` but never read from it or written to it (etalii.adp spec 035,
    /// T126): opened with [`DiagramWorkspace::open_bytes`].
    pub fn in_memory(root: impl Into<PathBuf>) -> Self {
        DiagramWorkspace {
            root: root.into(),
            documents: DiagramDocuments::in_memory(),
            history: History::default(),
            executed: Vec::new(),
            indexers: Vec::new(),
            unchecked: std::collections::HashSet::new(),
        }
    }

    /// Opens a diagram of `tool` from its bytes, named by its paths, and a view of it.
    pub fn open_bytes(
        &mut self,
        tool: Rc<DiagramTool>,
        body_path: &Path,
        registration_path: Option<&Path>,
        body: Vec<u8>,
        registration: Option<Vec<u8>>,
    ) -> DiagramSession {
        self.open_bytes_beside(tool, body_path, registration_path, body, registration, &|_| None)
    }

    /// [`DiagramWorkspace::open_bytes`], with the bytes of the files the binding keeps beside the body as `beside` gives
    /// them by their paths (FBL section 8.7).
    pub fn open_bytes_beside(
        &mut self,
        tool: Rc<DiagramTool>,
        body_path: &Path,
        registration_path: Option<&Path>,
        body: Vec<u8>,
        registration: Option<Vec<u8>>,
        beside: &dyn Fn(&Path) -> Option<Vec<u8>>,
    ) -> DiagramSession {
        self.open_bytes_in_folder(tool, body_path, registration_path, body, registration, beside, None)
    }

    /// [`DiagramWorkspace::open_bytes_beside`] of a folder subject whose files `folder` holds, by their relative,
    /// `/`-separated paths (FBL section 10; etalii.adp spec 035, T132).
    #[allow(clippy::too_many_arguments)]
    pub fn open_bytes_in_folder(
        &mut self,
        tool: Rc<DiagramTool>,
        body_path: &Path,
        registration_path: Option<&Path>,
        body: Vec<u8>,
        registration: Option<Vec<u8>>,
        beside: &dyn Fn(&Path) -> Option<Vec<u8>>,
        folder: Option<std::collections::BTreeMap<String, Vec<u8>>>,
    ) -> DiagramSession {
        let session =
            DiagramSession::new(body_path.to_path_buf()).in_default_viewpoint(tool.definition.specification());
        self.documents
            .open_bytes_in_folder(tool, body_path, registration_path, body, registration, beside, folder);
        session
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Adds a reader of another kind of tool's documents to the links index.
    pub fn add_link_indexer(&mut self, indexer: LinkIndexer) {
        self.indexers.push(indexer);
    }

    /// Reads the workspace's links index again (FBL section 17.3): every document under the root whose tool type
    /// takes part in links, the open ones as they are now, and gives it to the open documents.
    pub fn index_links(&mut self, tools: &mut DiagramTools) {
        let mut found = Vec::new();
        let mut scratch = DiagramDocuments::default();
        for path in files_under(&normalise(&self.root)) {
            if self.documents.get(&path).is_some() {
                continue;
            }
            let name = file_name(&path);
            let mut indexed = None;
            for origin in tools.for_file(&name) {
                let Ok(tool) = tools.get(&origin) else { continue };
                if crate::links::document_id_rule(tool.definition.specification()).is_none() {
                    continue;
                }
                if let Ok(document) = scratch.open(tool, &path, None) {
                    indexed = crate::links::indexed(document);
                }
                scratch.close(&path);
                if indexed.is_some() {
                    break;
                }
            }
            if indexed.is_none() {
                indexed = self.indexers.iter().find_map(|read| read(&path));
            }
            found.extend(indexed);
        }
        self.documents
            .set_link_index(adp_links::Workspace::new(self.root.clone(), found));
    }

    /// The load-time validation of the open diagram at `body_path` (FBL section 17.4): the links index read again, and
    /// what it changes, a document id when the diagram has none and every link that resolved rewritten where it
    /// differs, held for the diagram's first edit, which writes it in the same step of the history. Opening alone
    /// writes nothing, as it never has; nothing is held when nothing differs, nor for a diagram that cannot be
    /// written, whose links are still resolved and shown.
    pub fn check_links(&mut self, tools: &mut DiagramTools, body_path: &Path) -> Result<(), String> {
        let Some(document) = self.documents.get(body_path) else {
            return Err(crate::document::NOT_OPEN.to_owned());
        };
        if !crate::links::takes_part(document) {
            return Ok(());
        }
        self.index_links(tools);
        let document = self.documents.get(body_path).expect("still open");
        if document.refusal().is_some() {
            return Ok(());
        }
        let index = document.link_index().cloned().expect("just indexed");
        if !crate::links::load_changes(document, &index).is_empty() {
            self.unchecked.insert(body_path.to_path_buf());
        }
        Ok(())
    }

    /// Whether the open diagram at `body_path` has load-time changes its first edit will write.
    pub fn holds_link_changes(&self, body_path: &Path) -> bool {
        self.unchecked.contains(body_path)
    }

    /// The edit of what load-time validation changes in the diagram `edit` is made to, planned against the bytes
    /// `edit` leaves: none when nothing differs then.
    fn load_edit_after(&self, edit: &DocumentEdit) -> Option<DocumentEdit> {
        let document = self.documents.get(&edit.body_path)?;
        let mut body = document.body_bytes().to_vec();
        let mut registration = document.registration_bytes().map(<[u8]>::to_vec);
        for step in &edit.steps {
            match step.file {
                crate::document::EditedFile::Body => {
                    body = adp_fbl::history::apply_in_order(&body, &step.splices).ok()?;
                }
                crate::document::EditedFile::Registration => {
                    registration =
                        Some(adp_fbl::history::apply_in_order(registration.as_deref()?, &step.splices).ok()?);
                }
                _ => {}
            }
        }
        let mut scratch = DiagramDocuments::in_memory();
        if let Some(index) = document.link_index() {
            scratch.set_link_index((**index).clone());
        }
        let after = scratch.open_bytes_beside(
            document.tool().clone(),
            &edit.body_path,
            document.registration_path(),
            body,
            registration,
            &|_| None,
        );
        let index = after.link_index()?.clone();
        let changes = crate::links::load_changes(after, &index);
        if changes.is_empty() {
            return None;
        }
        after.plan(crate::links::CHECK_LINKS, &changes).ok()
    }

    pub fn documents(&self) -> &DiagramDocuments {
        &self.documents
    }

    pub fn history(&self) -> &History<DiagramDocuments> {
        &self.history
    }

    /// Opens the diagram `path` names, a registration or a body, and a view of it: through the tool type's module when
    /// it reads its documents itself, else through its specification and binding.
    pub fn open(&mut self, tools: &mut DiagramTools, path: &Path) -> Result<(OpenedDiagram, DiagramSession), String> {
        if let Some(opened) = self.open_by_module(tools, path) {
            return opened;
        }
        let opened = self.resolve(tools, path)?;
        let tool = tools.get(&opened.origin)?;
        let session =
            DiagramSession::new(opened.body_path.clone()).in_default_viewpoint(tool.definition.specification());
        self.documents
            .open(tool, &opened.body_path, opened.registration_path.as_deref())?;
        Ok((opened, session))
    }

    /// Opens `path` through the module of its tool type, when that module reads its documents itself; none when the
    /// tool type is drawn from its specification.
    fn open_by_module(
        &mut self,
        tools: &DiagramTools,
        path: &Path,
    ) -> Option<Result<(OpenedDiagram, DiagramSession), String>> {
        let full = normalise(&self.root.join(path));
        if !full.starts_with(normalise(&self.root)) || !full.is_file() {
            return None;
        }
        let origin = if is_registration(&full) {
            RegistrationDocument::read(fs::read(&full).ok()?).origin
        } else {
            tools
                .for_file(&file_name(&full))
                .into_iter()
                .find(|origin| tools.opened_by_module(origin))?
        };
        if !tools.opened_by_module(&origin) {
            return None;
        }
        let module = tools.module(&origin)?;
        Some(
            module
                .open(&self.root, &full, &origin, &mut self.documents)
                .map(|(opened, view)| {
                    let session = DiagramSession::of_module(opened.body_path.clone(), view);
                    (opened, session)
                }),
        )
    }

    /// What `path` names: the tool type, the body and the registration, or the refusal.
    pub fn resolve(&self, tools: &mut DiagramTools, path: &Path) -> Result<OpenedDiagram, String> {
        let full = normalise(&self.root.join(path));
        if !full.starts_with(normalise(&self.root)) || !full.is_file() {
            return Err(CANNOT_OPEN.to_owned());
        }
        if is_registration(&full) {
            let bytes = fs::read(&full).map_err(|_| CANNOT_OPEN.to_owned())?;
            let registration = RegistrationDocument::read(bytes);
            let origin = registration.origin.clone();
            let tool = tools.get(&origin)?;
            let location = locate_body(&full, &registration, &tool.binding, &self.root);
            if let Some(refusal) = location.refusal {
                return Err(refusal);
            }
            let Some(body_path) = location.path.filter(|_| location.exists) else {
                return Err(format!("The body of '{}' is missing.", file_name(&full)));
            };
            return Ok(OpenedDiagram {
                origin,
                body_path,
                registration_path: Some(full),
            });
        }
        let name = file_name(&full);
        let origin = tools
            .for_file(&name)
            .into_iter()
            .next()
            .ok_or_else(|| CANNOT_OPEN.to_owned())?;
        let registration_path = full
            .file_stem()
            .map(|stem| full.with_file_name(format!("{}.adp", stem.to_string_lossy())))
            .filter(|candidate| candidate.is_file())
            .filter(|candidate| {
                fs::read(candidate)
                    .map(|bytes| RegistrationDocument::read(bytes).origin == origin)
                    .unwrap_or(false)
            });
        Ok(OpenedDiagram {
            origin,
            body_path: full,
            registration_path,
        })
    }

    /// Closes the document of the body at `body_path`, once no view shows it.
    pub fn close(&mut self, body_path: &Path) {
        self.unchecked.remove(body_path);
        self.documents.close(body_path);
    }

    /// Runs `edit` through the history, so it is one undo away; an edit that writes nothing is not recorded.
    pub fn execute(&mut self, edit: DocumentEdit) -> Result<(), String> {
        if edit.is_empty() {
            return Ok(());
        }
        // The first edit of a diagram whose links load-time validation changes writes those changes too, in the same
        // step of the history (FBL section 17.4).
        let load = self
            .unchecked
            .contains(&edit.body_path)
            .then(|| self.load_edit_after(&edit))
            .flatten();
        let outcome = match &load {
            Some(load) => self.history.execute(
                &mut self.documents,
                &adp_history::Composite::new(
                    edit.label.clone(),
                    vec![
                        Box::new(edit.clone()) as Box<dyn Command<DiagramDocuments>>,
                        Box::new(load.clone()),
                    ],
                ),
            ),
            None => self.history.execute(&mut self.documents, &edit),
        };
        match outcome {
            Outcome::Done { .. } => {
                self.unchecked.remove(&edit.body_path);
                self.executed.push(edit);
                self.executed.extend(load);
                Ok(())
            }
            Outcome::Refused(refusal) => Err(refusal.0),
        }
    }

    /// The edits [`DiagramWorkspace::execute`] ran since this was last asked, in order: the splices a gesture made,
    /// for a conformance fixture to compare (etalii.adp spec 035, T127).
    pub fn take_executed(&mut self) -> Vec<DocumentEdit> {
        std::mem::take(&mut self.executed)
    }

    /// Runs any command on the documents through the history, a composite of edits to several documents included.
    pub fn execute_command(&mut self, command: &dyn Command<DiagramDocuments>) -> Outcome {
        self.history.execute(&mut self.documents, command)
    }

    pub fn undo(&mut self) -> Outcome {
        self.history.undo(&mut self.documents)
    }

    pub fn redo(&mut self) -> Outcome {
        self.history.redo(&mut self.documents)
    }

    /// Reconciles a file reported changed on disk: a body or registration read again, or gone, ends the history
    /// there, so undo stops at the change from outside; the store's own write is ignored.
    pub fn reconcile(&mut self, path: &Path) -> DiskChange {
        let change = self.documents.reconcile(path);
        if matches!(change, DiskChange::Reloaded(_) | DiskChange::Deleted(_)) {
            self.history.mark_changed_outside(None);
        }
        change
    }

    /// The body at `body_path` written by another program as `body` (ATD §2.3's `outside` step, for documents held in
    /// memory): its document is read again, and the history ends there with `reason`, as [`DiagramWorkspace::reconcile`]
    /// ends it for a file changed on disk.
    pub fn changed_outside(&mut self, body_path: &Path, body: Vec<u8>, reason: &str) -> Result<(), String> {
        let document = self
            .documents
            .get(body_path)
            .ok_or_else(|| format!("{} is not open", body_path.display()))?;
        let tool = document.tool().clone();
        let registration_path = document.registration_path().map(Path::to_path_buf);
        let registration = document.registration_bytes().map(<[u8]>::to_vec);
        self.documents.close(body_path);
        self.documents.open_bytes_beside(
            tool,
            body_path,
            registration_path.as_deref(),
            body,
            registration,
            &|_| None,
        );
        self.documents.mark_changed(body_path);
        self.history.mark_changed_outside(Some(reason));
        Ok(())
    }

    /// The bodies whose documents changed since this was last asked, for their views to refresh.
    pub fn take_changed(&mut self) -> Vec<PathBuf> {
        self.documents.take_changed()
    }
}

/// Every file under `root`, in path order, but those in a folder the links index skips.
fn files_under(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut folders = vec![root.to_path_buf()];
    while let Some(folder) = folders.pop() {
        let Ok(entries) = fs::read_dir(&folder) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => {
                    if crate::links::is_indexed_folder(&path) {
                        folders.push(path);
                    }
                }
                Ok(kind) if kind.is_file() => found.push(path),
                _ => {}
            }
        }
    }
    found.sort();
    found
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The path with `.` and `..` resolved without touching the file system.
fn normalise(path: &Path) -> PathBuf {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut out = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}
