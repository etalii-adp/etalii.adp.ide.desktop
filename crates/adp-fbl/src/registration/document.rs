//! A document a registration opens (FBL sections 8 and 9.3): the registration, the body it names and, while it exists
//! and the registration has no `layout:` block, the legacy layout beside the body (FBL section 8.7). A change goes to
//! the file that holds what it changes, each edit is one step of one history, and a placement in a view the body does
//! not hold is refused (FBL 0.11 section 9.3).

use std::collections::HashSet;
use std::path::Path;
use std::rc::Rc;

use crate::documents::types::FblBinding;
use crate::history::{OpenBody, UndoResult};
use crate::model::{FblModel, FblOptions};
use crate::planning::{ModelChange, PlanResult};
use crate::registration::legacy_sidecar::LegacySidecar;
use crate::registration::{OpenRegistration, locate_body};

/// FBL 0.11 section 9.3's sentence for a placement in a view that is not in the body.
pub const VIEW_NOT_IN_DOCUMENT: &str = "This view is not in the document, so its positions cannot be stored.";

/// The file an edit of a document was written to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditedFile {
    Registration,
    Body,
    Sidecar,
}

/// A registration with the body and the legacy layout it opens.
#[derive(Debug)]
pub struct OpenDocument {
    registration: OpenRegistration,
    registration_name: String,
    /// The body, by its path as the registration names it (FBL section 8.2).
    body: Option<(String, OpenBody)>,
    /// The legacy layout, by its path as the body's name gives it.
    sidecar: Option<(String, LegacySidecar)>,
    done: Vec<EditedFile>,
    undone: Vec<EditedFile>,
    last: Option<EditedFile>,
}

impl OpenDocument {
    /// Opens the registration at `path` with its bytes `bytes`. What FBL leaves to the host is in `options`; the
    /// reading's origin and view and the body's name are the registration's.
    pub fn open(
        bytes: Vec<u8>,
        path: &Path,
        binding: Rc<FblBinding>,
        options: FblOptions,
        workspace_root: &Path,
    ) -> OpenDocument {
        let mut registration = OpenRegistration::open_with(bytes, &binding);
        let name = |p: &Path| {
            p.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        };
        let location = locate_body(path, registration.document(), &binding, workspace_root);
        let mut body = None;
        let mut sidecar = None;
        if let Some(body_path) = location.path.filter(|_| location.exists) {
            let document = registration.document();
            let body_name = document.body().map(str::to_owned).unwrap_or_else(|| name(&body_path));
            let options = FblOptions {
                file_name: body_name.clone(),
                origin: Some(document.origin.clone()),
                view: document.view().map(str::to_owned),
                ..options
            };
            if let Ok(body_bytes) = std::fs::read(&body_path) {
                let open = OpenBody::open(body_bytes, binding.clone(), Rc::new(options));
                registration.known_ids = Some(open.model().elements.iter().map(|e| e.id.clone()).collect());
                body = Some((body_name.clone(), open));
                if let Some(pattern) = binding
                    .registration
                    .legacy_layout
                    .as_deref()
                    .filter(|_| registration.document().layout.is_none())
                {
                    let sidecar_path = LegacySidecar::path_for(pattern, &body_path);
                    if let Ok(sidecar_bytes) = std::fs::read(&sidecar_path) {
                        let folder = Path::new(&body_name).parent().unwrap_or(Path::new(""));
                        let sidecar_name = folder.join(name(&sidecar_path)).to_string_lossy().replace('\\', "/");
                        sidecar = Some((sidecar_name, LegacySidecar::open(sidecar_bytes)));
                    }
                }
            }
        }
        OpenDocument {
            registration,
            registration_name: name(path),
            body,
            sidecar,
            done: Vec::new(),
            undone: Vec::new(),
            last: None,
        }
    }

    pub fn registration(&self) -> &OpenRegistration {
        &self.registration
    }

    pub fn body(&self) -> Option<&OpenBody> {
        self.body.as_ref().map(|(_, body)| body)
    }

    /// The bytes of the file a change or an undo can write, by its path as an edit names it.
    pub fn file(&self, name: &str) -> Option<&[u8]> {
        if let Some((_, body)) = self.body.as_ref().filter(|(n, _)| n == name) {
            return Some(body.bytes());
        }
        if let Some((_, sidecar)) = self.sidecar.as_ref().filter(|(n, _)| n == name) {
            return Some(sidecar.bytes());
        }
        (self.registration_name == name).then(|| self.registration.bytes())
    }

    /// The name of the file the last change, undo or redo wrote; none for the registration, which an edit does not
    /// name when it is the document.
    pub fn last_file(&self) -> Option<&str> {
        match self.last? {
            EditedFile::Registration => None,
            EditedFile::Body => self.body.as_ref().map(|(n, _)| n.as_str()),
            EditedFile::Sidecar => self.sidecar.as_ref().map(|(n, _)| n.as_str()),
        }
    }

    /// The body's reading, with what reading the registration and the legacy layout reports; none without a body.
    pub fn model(&self) -> Option<FblModel> {
        let (_, body) = self.body.as_ref()?;
        let mut model = body.model();
        model
            .findings
            .extend(self.registration.findings(&self.registration_name));
        if let Some((name, sidecar)) = &self.sidecar {
            let view = model
                .select_view(self.registration.document().view())
                .map(|v| v.name.clone());
            model
                .findings
                .extend(sidecar.findings(view.as_deref(), &ids(&model), name));
        }
        Some(model)
    }

    /// Plans and applies `change` on the file that holds what it changes.
    pub fn change(&mut self, change: &ModelChange) -> PlanResult {
        let (result, file) = match change {
            ModelChange::Place { id, x, y } => match self.place(id, *x, *y) {
                Ok((result, file)) => (result, file),
                Err(reason) => return PlanResult::Refused(reason),
            },
            ModelChange::Save | ModelChange::Identify { .. } | ModelChange::Setting(_) => {
                (self.registration.change(change), EditedFile::Registration)
            }
            _ => {
                let Some((_, body)) = &mut self.body else {
                    return PlanResult::Refused("The registration's body is not open, so it cannot be changed.".into());
                };
                let result = body.change(change);
                self.registration.known_ids = Some(ids(&body.model()));
                (result, EditedFile::Body)
            }
        };
        if let PlanResult::Planned(edit) = &result
            && !edit.splices.is_empty()
            && !self.outside_history(change)
        {
            self.done.push(file);
            self.undone.clear();
            self.last = Some(file);
        }
        result
    }

    /// A setting of a block declared `history: false` is no step of the history (FBL 0.7 section 7.1).
    fn outside_history(&self, change: &ModelChange) -> bool {
        matches!(change, ModelChange::Setting(setting) if !self.registration.keeps_in_history(&setting.block))
    }

    /// A placement: refused when the reading shows a view that is not in the body, written to the legacy layout while
    /// it stands in for the `layout:` block, else to the registration.
    fn place(&mut self, id: &str, x: f64, y: f64) -> Result<(PlanResult, EditedFile), String> {
        let model = self.body.as_ref().map(|(_, body)| body.model());
        let header = self.registration.document().view();
        if let Some(model) = &model
            && header.is_some()
            && !model.views.is_empty()
            && model.select_view(header).is_none()
        {
            return Err(VIEW_NOT_IN_DOCUMENT.to_owned());
        }
        if let (Some((_, sidecar)), Some(model)) = (&mut self.sidecar, &model) {
            let view = model.select_view(header).map(|v| v.name.clone());
            let known = ids(model);
            let result = sidecar.change(|s| s.plan_place(view.as_deref(), id, x, y, Some(&known)));
            return Ok((result, EditedFile::Sidecar));
        }
        let change = ModelChange::Place {
            id: id.to_owned(),
            x,
            y,
        };
        Ok((self.registration.change(&change), EditedFile::Registration))
    }

    /// Undoes the last edit of the document, whichever file it wrote.
    pub fn undo(&mut self) -> UndoResult {
        let file = self.done.last().copied().unwrap_or(EditedFile::Registration);
        let result = self.undo_of(file);
        if let UndoResult::Done(_) = result {
            self.done.pop();
            self.undone.push(file);
            self.last = Some(file);
        }
        result
    }

    /// Redoes the last edit undone.
    pub fn redo(&mut self) -> UndoResult {
        let file = self.undone.last().copied().unwrap_or(EditedFile::Registration);
        let result = self.redo_of(file);
        if let UndoResult::Done(_) = result {
            self.undone.pop();
            self.done.push(file);
            self.last = Some(file);
        }
        result
    }

    fn undo_of(&mut self, file: EditedFile) -> UndoResult {
        match (file, &mut self.body, &mut self.sidecar) {
            (EditedFile::Body, Some((_, body)), _) => {
                let result = body.undo();
                self.registration.known_ids = Some(ids(&body.model()));
                result
            }
            (EditedFile::Sidecar, _, Some((_, sidecar))) => sidecar.undo(None),
            _ => self.registration.undo(),
        }
    }

    fn redo_of(&mut self, file: EditedFile) -> UndoResult {
        match (file, &mut self.body, &mut self.sidecar) {
            (EditedFile::Body, Some((_, body)), _) => {
                let result = body.redo();
                self.registration.known_ids = Some(ids(&body.model()));
                result
            }
            (EditedFile::Sidecar, _, Some((_, sidecar))) => sidecar.redo(None),
            _ => self.registration.redo(),
        }
    }
}

fn ids(model: &FblModel) -> HashSet<String> {
    model.elements.iter().map(|e| e.id.clone()).collect()
}
