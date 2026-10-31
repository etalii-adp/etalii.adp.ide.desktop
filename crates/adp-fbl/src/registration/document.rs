//! A document a registration opens (FBL sections 8 and 9.3): the registration, the body it names and, while they exist
//! and the registration has no matching block, the legacy layout and the legacy identities beside the body (FBL section
//! 8.7). A change goes to the file that holds what it changes, each edit is one step of one history, whichever files it
//! writes, an element's stored id follows its key in the edit that changes it (FBL 0.12 section 8.6), and a placement
//! in a view the body does not hold is refused (FBL 0.11 section 9.3).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::rc::Rc;

use crate::documents::types::FblBinding;
use crate::history::{OpenBody, UndoResult};
use crate::model::{FblModel, FblOptions};
use crate::planning::{ModelChange, PlanResult};
use crate::registration::legacy_sidecar::{LegacySidecar, NewIdentity};
use crate::registration::{OpenRegistration, locate_body};
use crate::rules::reading::ElementIdentity;
use crate::splice::{Operation, Splice};

/// FBL 0.11 section 9.3's sentence for a placement in a view that is not in the body.
pub const VIEW_NOT_IN_DOCUMENT: &str = "This view is not in the document, so its positions cannot be stored.";

/// The file an edit of a document was written to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditedFile {
    Registration,
    Body,
    Sidecar,
    /// *(FBL 0.12)* The legacy identities file.
    Identities,
}

/// The files of a document, each by its path as an edit names it, with their bytes: what [`OpenDocument::of_files`]
/// opens.
#[derive(Debug, Clone, Default)]
pub struct DocumentFiles {
    pub registration: Vec<u8>,
    pub registration_name: String,
    pub body: Option<(String, Vec<u8>)>,
    pub legacy_layout: Option<(String, Vec<u8>)>,
    pub legacy_identities: Option<(String, Vec<u8>)>,
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
    /// *(FBL 0.12)* The legacy identities, by their path as the body's name gives it, which an `identities:` block stands
    /// in for.
    identities: Option<(String, LegacySidecar)>,
    /// The files of each step of the history.
    done: Vec<Vec<EditedFile>>,
    undone: Vec<Vec<EditedFile>>,
    /// What the last change, undo or redo wrote: each file with its splices, the file that holds what it changes first.
    last: Vec<(EditedFile, Vec<Splice>)>,
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
        let registration = OpenRegistration::open_with(bytes.clone(), &binding);
        let name = |p: &Path| {
            p.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        };
        let location = locate_body(path, registration.document(), &binding, workspace_root);
        let mut files = DocumentFiles {
            registration: bytes,
            registration_name: name(path),
            body: None,
            legacy_layout: None,
            legacy_identities: None,
        };
        if let Some(body_path) = location.path.filter(|_| location.exists) {
            let document = registration.document();
            let body_name = document.body().map(str::to_owned).unwrap_or_else(|| name(&body_path));
            let beside = |pattern: &Option<String>| {
                let path = LegacySidecar::path_for(pattern.as_deref()?, &body_path);
                let folder = Path::new(&body_name).parent().unwrap_or(Path::new(""));
                let file = folder.join(name(&path)).to_string_lossy().replace('\\', "/");
                Some((file, std::fs::read(&path).ok()?))
            };
            files.legacy_layout = beside(&binding.registration.legacy_layout);
            files.legacy_identities = beside(&binding.registration.legacy_identities);
            files.body = std::fs::read(&body_path).ok().map(|bytes| (body_name.clone(), bytes));
        }
        OpenDocument::of_files(files, binding, options)
    }

    /// Opens a document whose files are in `files`, as [`OpenDocument::open`] finds them on disk, so that a host that
    /// holds its documents in memory reads and plans them as FBL says. A legacy file is used only while the registration
    /// has no block that stands in for it (FBL section 8.7).
    pub fn of_files(files: DocumentFiles, binding: Rc<FblBinding>, options: FblOptions) -> OpenDocument {
        let mut registration = OpenRegistration::open_with(files.registration, &binding);
        let mut body = None;
        let mut sidecar = None;
        let mut identities = None;
        if let Some((body_name, body_bytes)) = files.body {
            let document = registration.document();
            if document.identities.is_none() {
                identities = files
                    .legacy_identities
                    .map(|(name, bytes)| (name, LegacySidecar::open(bytes)));
            }
            let options = FblOptions {
                file_name: body_name.clone(),
                origin: Some(document.origin.clone()),
                view: document.view().map(str::to_owned),
                resource: document.resource().map(str::to_owned),
                identities: stored_ids(&registration, identities.as_ref()),
                ..options
            };
            let open = OpenBody::open(body_bytes, binding.clone(), Rc::new(options));
            registration.known_ids = Some(open.model().elements.iter().map(|e| e.id.clone()).collect());
            body = Some((body_name, open));
            if registration.document().layout.is_none() {
                sidecar = files
                    .legacy_layout
                    .map(|(name, bytes)| (name, LegacySidecar::open(bytes)));
            }
        }
        OpenDocument {
            registration,
            registration_name: files.registration_name,
            body,
            sidecar,
            identities,
            done: Vec::new(),
            undone: Vec::new(),
            last: Vec::new(),
        }
    }

    pub fn registration(&self) -> &OpenRegistration {
        &self.registration
    }

    pub fn body(&self) -> Option<&OpenBody> {
        self.body.as_ref().map(|(_, body)| body)
    }

    /// The positions a legacy layout holds for the view the registration selects, by the keys it writes them under;
    /// none while no legacy layout stands in for the `layout:` block (FBL section 8.7).
    pub fn legacy_positions(&self) -> Option<BTreeMap<String, (f64, f64)>> {
        let (_, sidecar) = self.sidecar.as_ref()?;
        let (_, body) = self.body.as_ref()?;
        let model = body.model();
        let header = self.registration.document().view();
        let view = model.select_view(header).map(|v| v.name.clone());
        Some(match view {
            Some(view) => sidecar.positions(Some(&view)),
            None => BTreeMap::new(),
        })
    }

    /// The bytes of the file a change or an undo can write, by its path as an edit names it.
    pub fn file(&self, name: &str) -> Option<&[u8]> {
        if let Some((_, body)) = self.body.as_ref().filter(|(n, _)| n == name) {
            return Some(body.bytes());
        }
        if let Some((_, sidecar)) = self.sidecar.as_ref().filter(|(n, _)| n == name) {
            return Some(sidecar.bytes());
        }
        if let Some((_, identities)) = self.identities.as_ref().filter(|(n, _)| n == name) {
            return Some(identities.bytes());
        }
        (self.registration_name == name).then(|| self.registration.bytes())
    }

    /// The name of the file the last change, undo or redo wrote, the one that holds what it changes when it wrote
    /// several; none for the registration, which an edit does not name when it is the document.
    pub fn last_file(&self) -> Option<&str> {
        match self.last.first()?.0 {
            EditedFile::Registration => None,
            file => self.name_of(file),
        }
    }

    /// The splices the last change, undo or redo wrote to the file `name`; none when it did not write that file.
    pub fn last_splices(&self, name: &str) -> Option<&[Splice]> {
        self.last
            .iter()
            .find(|(file, _)| self.name_of(*file) == Some(name))
            .map(|(_, splices)| splices.as_slice())
    }

    /// The names of the files the last change, undo or redo wrote.
    pub fn last_files(&self) -> Vec<&str> {
        self.last.iter().filter_map(|(file, _)| self.name_of(*file)).collect()
    }

    fn name_of(&self, file: EditedFile) -> Option<&str> {
        match file {
            EditedFile::Registration => Some(&self.registration_name),
            EditedFile::Body => self.body.as_ref().map(|(n, _)| n.as_str()),
            EditedFile::Sidecar => self.sidecar.as_ref().map(|(n, _)| n.as_str()),
            EditedFile::Identities => self.identities.as_ref().map(|(n, _)| n.as_str()),
        }
    }

    /// The body's reading, with what reading the registration and the legacy layout reports; none without a body.
    pub fn model(&self) -> Option<FblModel> {
        let (_, body) = self.body.as_ref()?;
        let mut model = body.model();
        model
            .findings
            .extend(self.registration.findings(&self.registration_name));
        if body.reading().missing_resource().is_some() {
            model
                .findings
                .extend(self.registration.document().missing_resource(&self.registration_name));
        }
        if let Some((name, sidecar)) = &self.sidecar {
            let view = model
                .select_view(self.registration.document().view())
                .map(|v| v.name.clone());
            model
                .findings
                .extend(sidecar.findings(view.as_deref(), &ids(&model), name));
        }
        let keys: HashSet<String> = body.reading().identities().iter().map(|e| e.key.entry()).collect();
        if let Some((name, identities)) = &self.identities {
            let declared = declared_kinds(body.reading().binding_rc());
            model
                .findings
                .extend(identities.identity_findings(&declared, &keys, name));
        }
        model.findings.extend(
            self.registration
                .document()
                .stale_identities(&keys, &self.registration_name),
        );
        Some(model)
    }

    /// Plans and applies `change` on the file that holds what it changes.
    pub fn change(&mut self, change: &ModelChange) -> PlanResult {
        let (result, file, before) = match change {
            ModelChange::Place { id, x, y } => match self.place(id, *x, *y) {
                Ok((result, file)) => (result, file, Vec::new()),
                Err(reason) => return PlanResult::Refused(reason),
            },
            ModelChange::Save | ModelChange::Identify { .. } | ModelChange::Setting(_) => {
                (self.registration.change(change), EditedFile::Registration, Vec::new())
            }
            _ => {
                let Some((_, body)) = &mut self.body else {
                    return PlanResult::Refused("The registration's body is not open, so it cannot be changed.".into());
                };
                let before = body.reading().identities();
                let result = body.change(change);
                self.registration.known_ids = Some(ids(&body.model()));
                (result, EditedFile::Body, before)
            }
        };
        if let PlanResult::Planned(edit) = &result
            && !edit.splices.is_empty()
            && !self.outside_history(change)
        {
            let mut wrote = vec![(file, edit.splices.clone())];
            if file == EditedFile::Body {
                let added = match change {
                    ModelChange::Add { id, .. } => id.as_deref(),
                    _ => None,
                };
                wrote.extend(self.follow_identities(&before, &edit.splices, added));
                self.sync_identities();
            }
            self.done.push(wrote.iter().map(|(file, _)| *file).collect());
            self.undone.clear();
            self.last = wrote;
        }
        result
    }

    /// What an edit of the body writes of the identities (FBL 0.12 section 8.6): an element whose key the edit changes
    /// keeps its entry under the new key, an entry whose key no longer names an element is pruned, and an element the
    /// edit adds has its entry written, all in that edit. An element is told across the edit by where its first byte
    /// goes, so that the later holders of a key keep their ids when an earlier one goes. An id the specification's
    /// strategy made for an element read without an entry is stored too, as no step of the history.
    fn follow_identities(
        &mut self,
        before: &[ElementIdentity],
        splices: &[Splice],
        added: Option<&str>,
    ) -> Option<(EditedFile, Vec<Splice>)> {
        let after = self.body.as_ref()?.1.reading().identities();
        let held = stored_ids(&self.registration, self.identities.as_ref());
        let mut matched = vec![false; after.len()];
        let mut moved: Vec<(String, String)> = Vec::new();
        let mut lost: Vec<&ElementIdentity> = Vec::new();
        for element in before {
            let now = moved_to(element.start, splices)
                .and_then(|start| (0..after.len()).find(|&a| after[a].start == start && !matched[a]));
            match now {
                Some(now) => {
                    matched[now] = true;
                    if element.stored && after[now].key.entry() != element.key.entry() {
                        moved.push((element.key.entry(), after[now].key.entry()));
                    }
                }
                None => lost.push(element),
            }
        }
        // An element a move rewrites elsewhere comes back with its key.
        let mut pruned: Vec<String> = Vec::new();
        for element in lost.into_iter().filter(|element| element.stored) {
            let key = element.key.entry();
            match (0..after.len()).find(|&a| !matched[a] && after[a].key.entry() == key) {
                Some(again) => matched[again] = true,
                None if !moved.iter().any(|(from, _)| *from == key) => pruned.push(key),
                None => {}
            }
        }
        // An entry that names no element any more goes too; one of a kind no rule declares stays as it is.
        let keys: HashSet<String> = after.iter().map(|a| a.key.entry()).collect();
        let declared = declared_kinds(self.body.as_ref()?.1.reading().binding_rc());
        let undeclared: HashSet<String> = self
            .identities
            .iter()
            .flat_map(|(_, identities)| {
                let entries = identities.identity_entries().unwrap_or_default();
                entries
                    .into_iter()
                    .filter(|entry| !entry.is_declared(&declared))
                    .map(|entry| entry.entry_key())
            })
            .collect();
        for key in held.keys() {
            if !keys.contains(key)
                && !undeclared.contains(key)
                && !pruned.contains(key)
                && !moved.iter().any(|(from, _)| from == key)
            {
                pruned.push(key.clone());
            }
        }
        // What the edit adds, stored with it; what was read without an entry, stored beside it.
        let kept = |key: &String| {
            moved.iter().any(|(_, to)| to == key)
                || (held.contains_key(key) && !pruned.contains(key) && !moved.iter().any(|(from, _)| from == key))
        };
        let mut new: Vec<&ElementIdentity> = Vec::new();
        let mut first_stored: Vec<&ElementIdentity> = Vec::new();
        for (place, element) in after.iter().enumerate() {
            if kept(&element.key.entry()) {
                continue;
            }
            if !matched[place] {
                new.push(element);
            } else if element.derived {
                first_stored.push(element);
            }
        }
        let id_of = |element: &ElementIdentity| -> Option<String> {
            added
                .map(str::to_owned)
                .or_else(|| element.derived.then(|| element.id.clone()))
        };
        let wrote = if let Some((_, identities)) = &mut self.identities {
            let new: Vec<NewIdentity> = new
                .iter()
                .filter_map(|element| {
                    Some(NewIdentity {
                        id: id_of(element)?,
                        kind: element.key.kind.clone(),
                        key: element.key.key.clone(),
                    })
                })
                .collect();
            let result = identities.change(|file| file.plan_identities(&moved, &pruned, &new));
            let first: Vec<NewIdentity> = first_stored
                .iter()
                .map(|element| NewIdentity {
                    id: element.id.clone(),
                    kind: element.key.kind.clone(),
                    key: element.key.key.clone(),
                })
                .collect();
            if let PlanResult::Planned(edit) = identities.plan_identities(&[], &[], &first) {
                identities.apply_outside_history(&edit);
            }
            match result {
                PlanResult::Planned(edit) if !edit.splices.is_empty() => Some((EditedFile::Identities, edit.splices)),
                _ => None,
            }
        } else if new.is_empty() && moved.is_empty() && pruned.is_empty() {
            None
        } else {
            let entry = new
                .first()
                .and_then(|element| Some((element.key.entry(), id_of(element)?)));
            let result = self.registration.change_identities(
                &moved,
                &pruned,
                entry.as_ref().map(|(key, id)| (key.as_str(), id.as_str())),
            );
            match result {
                PlanResult::Planned(edit) if !edit.splices.is_empty() => Some((EditedFile::Registration, edit.splices)),
                _ => None,
            }
        };
        if self.identities.is_none() {
            for element in first_stored {
                self.registration.store_identity(&element.key.entry(), &element.id);
            }
        }
        wrote
    }

    /// Reads the body again with the ids its files now store.
    fn sync_identities(&mut self) {
        let identities = stored_ids(&self.registration, self.identities.as_ref());
        if let Some((_, body)) = &mut self.body {
            body.set_identities(identities);
            self.registration.known_ids = Some(ids(&body.model()));
        }
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

    /// Undoes the last edit of the document, every file it wrote.
    pub fn undo(&mut self) -> UndoResult {
        let files = self
            .done
            .last()
            .cloned()
            .unwrap_or_else(|| vec![EditedFile::Registration]);
        let mut wrote = Vec::new();
        for file in files.iter().rev() {
            match self.undo_of(*file) {
                UndoResult::Done(splices) => wrote.push((*file, splices)),
                refused => return refused,
            }
        }
        wrote.reverse();
        self.sync_identities();
        self.done.pop();
        self.undone.push(files);
        let result = wrote.first().map_or_else(Vec::new, |(_, splices)| splices.clone());
        self.last = wrote;
        UndoResult::Done(result)
    }

    /// Redoes the last edit undone.
    pub fn redo(&mut self) -> UndoResult {
        let files = self
            .undone
            .last()
            .cloned()
            .unwrap_or_else(|| vec![EditedFile::Registration]);
        let mut wrote = Vec::new();
        for file in &files {
            match self.redo_of(*file) {
                UndoResult::Done(splices) => wrote.push((*file, splices)),
                refused => return refused,
            }
        }
        self.sync_identities();
        self.undone.pop();
        self.done.push(files);
        let result = wrote.first().map_or_else(Vec::new, |(_, splices)| splices.clone());
        self.last = wrote;
        UndoResult::Done(result)
    }

    fn undo_of(&mut self, file: EditedFile) -> UndoResult {
        match (file, &mut self.body, &mut self.sidecar) {
            (EditedFile::Body, Some((_, body)), _) => {
                let result = body.undo();
                self.registration.known_ids = Some(ids(&body.model()));
                result
            }
            (EditedFile::Sidecar, _, Some((_, sidecar))) => sidecar.undo(None),
            (EditedFile::Identities, ..) => match &mut self.identities {
                Some((_, identities)) => identities.undo(None),
                None => self.registration.undo(),
            },
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
            (EditedFile::Identities, ..) => match &mut self.identities {
                Some((_, identities)) => identities.redo(None),
                None => self.registration.redo(),
            },
            _ => self.registration.redo(),
        }
    }
}

/// The ids the registration or the legacy identities store, by each entry's key (FBL 0.12 sections 8.6, 8.7); of two
/// entries with one key, the first.
fn stored_ids(
    registration: &OpenRegistration,
    identities: Option<&(String, LegacySidecar)>,
) -> HashMap<String, String> {
    let Some((_, identities)) = identities else {
        return registration.document().identity_map();
    };
    let mut stored = HashMap::new();
    for entry in identities.identity_entries().unwrap_or_default() {
        stored.entry(entry.entry_key()).or_insert(entry.id);
    }
    stored
}

/// The kinds the rules of `binding` declare (FBL 0.12 section 8.6).
fn declared_kinds(binding: &FblBinding) -> Vec<String> {
    binding
        .all_rules()
        .iter()
        .filter_map(|rule| rule.id.as_ref()?.sidecar_kind.clone())
        .collect()
}

/// Where the byte at `start` stands after `splices`, in body order; none when a removal takes it.
fn moved_to(start: usize, splices: &[Splice]) -> Option<usize> {
    let mut shift = 0isize;
    for splice in splices {
        if splice.start <= start && start < splice.end {
            if matches!(splice.operation, Operation::RemoveEntry | Operation::RemoveKey) || start != splice.start {
                return None;
            }
            break;
        }
        if splice.end <= start {
            shift += splice.text.len() as isize - (splice.end - splice.start) as isize;
        }
    }
    Some((start as isize + shift) as usize)
}

fn ids(model: &FblModel) -> HashSet<String> {
    model.elements.iter().map(|e| e.id.clone()).collect()
}

#[cfg(test)]
#[path = "document_tests.rs"]
mod tests;
