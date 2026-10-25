//! One edit of a requirement document's body, carried out (the standalone host's `HierarchicalRequirementEditCommand`,
//! its handler, `RestoreRequirementFilesCommand` and its handler at a7fdf03): the body is read as it is now and
//! checked against the bytes the person's page was made from, the operation is made through the FBL library, and the
//! result is written. What it leaves behind is each file's content from before and after, so an undo gives the bytes
//! back exactly, and refuses once another program has written one of them since.
//!
//! The standalone host runs these as commands of the project's history; as the Knowledge designer's do here, the
//! session keeps the [`Restore`] an edit gives and calls it for undo, and redo runs the edit again.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::documents::Writer;
use crate::gestures::DRIFTED;
use crate::operations::lists::{ListEdit, ListFiles};
use crate::operations::{assessments, classification, columns, filters, structure, tags, text};

/// One edit of the body: an operation of the surface, with what it names and its params, planned against the bytes
/// the person's page was made from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditCommand {
    pub body_path: PathBuf,
    /// The operation's name in the specification, `setTitle` say.
    pub operation: String,
    /// The id the surface gives the entry it is for.
    pub target_id: String,
    /// Its params, in their written form.
    pub params: BTreeMap<String, String>,
    /// The body's bytes as the page was read from them. A file that holds other bytes now was changed since, and the
    /// edit is refused rather than made to a file the person did not see (FBL 0.7 section 7.2, spec 029 FR-024).
    pub expected: Vec<u8>,
    /// The registration, which an operation on a list writes in the same step: the tag list's colours and order are
    /// kept there (spec 029 R17). It is created with the first such change when there is none.
    pub registration_path: PathBuf,
}

/// One file of a step: what it held before the step, and what the step left it holding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileState {
    pub path: PathBuf,
    pub before: Vec<u8>,
    pub after: Vec<u8>,
}

/// What puts the files of one step back as they were, and the edit that makes it again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Restore {
    pub files: Vec<FileState>,
    pub redo: EditCommand,
}

/// Why an undo did not happen: a file another program wrote since, which ends the history, or a write that failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Undone {
    Outdated(String),
    Failed(String),
}

fn name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

impl EditCommand {
    /// Makes the edit: what it wrote, so that it can be undone, or why it was not written. An edit that changes no
    /// byte writes nothing and leaves nothing to undo.
    pub fn execute(&self, write: &mut Writer) -> Result<Option<Restore>, String> {
        let name = name_of(&self.body_path);
        let before = std::fs::read(&self.body_path).map_err(|e| format!("{name} could not be read: {e}"))?;
        if before != self.expected {
            return Err(DRIFTED.into());
        }
        if tags::handles(&self.operation)
            || classification::handles(&self.operation)
            || filters::handles(&self.operation)
            || columns::handles(&self.operation)
        {
            let files = self.lists(&before, &name)?;
            return self.write(files, write);
        }
        let after = if text::handles(&self.operation) {
            let value = self
                .params
                .get(text::attribute_of(&self.operation))
                .map_or("", String::as_str);
            text::apply(&before, &name, &self.operation, &self.target_id, value)?
        } else if structure::handles(&self.operation) {
            structure::apply(&before, &name, &self.operation, &self.target_id, &self.params)?
        } else if assessments::handles(&self.operation) {
            assessments::apply(&before, &name, &self.operation, &self.target_id, &self.params)?
        } else {
            return Err(format!("'{}' cannot be done here yet.", self.operation));
        };
        let file = FileState {
            path: self.body_path.clone(),
            before,
            after,
        };
        self.write(vec![file], write)
    }

    /// What an operation on a list the registration keeps writes, the tags, the categories, the states or the
    /// filters: the body when its entries change, and the registration when the list does, the body first. A
    /// registration the body has none of is created as the step's starting point, outside the step.
    fn lists(&self, body: &[u8], name: &str) -> Result<Vec<FileState>, String> {
        let registration_name = name_of(&self.registration_path);
        let registration = match std::fs::read(&self.registration_path) {
            Ok(bytes) => Some(bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(format!("{registration_name} could not be read: {e}")),
        };
        let files = ListFiles {
            body,
            body_name: name,
            registration: registration.as_deref(),
            registration_name: &registration_name,
        };
        let (operation, target, params) = (&self.operation, &self.target_id, &self.params);
        let edit: ListEdit = if classification::handles(operation) {
            classification::apply(files, operation, target, params)?
        } else if filters::handles(operation) {
            filters::apply(files, operation, target, params)?
        } else if columns::handles(operation) {
            columns::apply(files, operation, target, params)?
        } else {
            tags::apply(files, operation, target, params)?
        };
        let mut states = Vec::new();
        if let Some(after) = edit.body {
            states.push(FileState {
                path: self.body_path.clone(),
                before: body.to_vec(),
                after,
            });
        }
        if let (Some(before), Some(after)) = (edit.registration_before, edit.registration_after) {
            states.push(FileState {
                path: self.registration_path.clone(),
                before,
                after,
            });
        }
        Ok(states)
    }

    /// Writes each file the step changes, every one or none, and returns what puts them back. A step that changes no
    /// byte writes nothing and leaves nothing to undo.
    fn write(&self, files: Vec<FileState>, write: &mut Writer) -> Result<Option<Restore>, String> {
        let files: Vec<FileState> = files.into_iter().filter(|f| f.after != f.before).collect();
        if files.is_empty() {
            return Ok(None);
        }
        let mut written: Vec<&FileState> = Vec::new();
        for file in &files {
            if let Err(e) = write(&file.path, &file.after) {
                // What was written before this one is put back: a step is every file or none. A registration this
                // step created is put back as created, which is not part of the step (FBL 0.7 section 8.4).
                for done in written {
                    let _ = write(&done.path, &done.before);
                }
                return Err(format!("{} could not be written: {e}", name_of(&file.path)));
            }
            written.push(file);
        }
        Ok(Some(Restore {
            files,
            redo: self.clone(),
        }))
    }
}

impl Restore {
    /// Puts every file back to what it held before the step, or none. A file another program wrote since is not what
    /// the step left, so nothing is written and the history's own sentence says the record is spent.
    pub fn undo(&self, write: &mut Writer) -> Result<(), Undone> {
        for file in &self.files {
            if std::fs::read(&file.path).ok().as_ref() != Some(&file.after) {
                return Err(Undone::Outdated(format!(
                    "{} was changed by another program, so the edits made to it before that can no longer be undone.",
                    name_of(&file.path)
                )));
            }
        }
        let mut restored: Vec<&FileState> = Vec::new();
        for file in &self.files {
            if let Err(e) = write(&file.path, &file.before) {
                // What was restored before this one is written forward again: a step is every file or none.
                for done in restored {
                    let _ = write(&done.path, &done.after);
                }
                return Err(Undone::Failed(format!(
                    "Could not restore {}: {e}",
                    name_of(&file.path)
                )));
            }
            restored.push(file);
        }
        Ok(())
    }
}
