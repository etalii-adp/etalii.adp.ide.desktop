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

use adp_fbl::RegistrationDocument;
use adp_fbl::registration::locate_body;
use adp_history::{Command, History, Outcome};

use crate::document::{DiagramDocuments, DiskChange, DocumentEdit};
use crate::session::DiagramSession;
use crate::tool::DiagramTools;

/// The refusal of a path that names no diagram, as the standalone host words it.
pub const CANNOT_OPEN: &str = "The diagram cannot be opened.";

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
}

impl DiagramWorkspace {
    /// The diagrams of the project at `root`, none open yet.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        DiagramWorkspace {
            root: root.into(),
            documents: DiagramDocuments::default(),
            history: History::default(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
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
        self.documents
            .open(tool, &opened.body_path, opened.registration_path.as_deref())?;
        let session = DiagramSession::new(opened.body_path.clone());
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
        let origin = if full.extension().is_some_and(|e| e.eq_ignore_ascii_case("adp")) {
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
        let is_registration = full.extension().is_some_and(|e| e.eq_ignore_ascii_case("adp"));
        if is_registration {
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
        self.documents.close(body_path);
    }

    /// Runs `edit` through the history, so it is one undo away; an edit that writes nothing is not recorded.
    pub fn execute(&mut self, edit: DocumentEdit) -> Result<(), String> {
        if edit.is_empty() {
            return Ok(());
        }
        match self.history.execute(&mut self.documents, &edit) {
            Outcome::Done { .. } => Ok(()),
            Outcome::Refused(refusal) => Err(refusal.0),
        }
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

    /// The bodies whose documents changed since this was last asked, for their views to refresh.
    pub fn take_changed(&mut self) -> Vec<PathBuf> {
        self.documents.take_changed()
    }
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
