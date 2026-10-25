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
use crate::operations::{structure, text};

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
        let after = if text::handles(&self.operation) {
            let value = self
                .params
                .get(text::attribute_of(&self.operation))
                .map_or("", String::as_str);
            text::apply(&before, &name, &self.operation, &self.target_id, value)?
        } else if structure::handles(&self.operation) {
            structure::apply(&before, &name, &self.operation, &self.target_id, &self.params)?
        } else {
            return Err(format!("'{}' cannot be done here yet.", self.operation));
        };
        if after == before {
            // Nothing to write and nothing to undo.
            return Ok(None);
        }
        write(&self.body_path, &after).map_err(|e| format!("{name} could not be written: {e}"))?;
        Ok(Some(Restore {
            files: vec![FileState {
                path: self.body_path.clone(),
                before,
                after,
            }],
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
