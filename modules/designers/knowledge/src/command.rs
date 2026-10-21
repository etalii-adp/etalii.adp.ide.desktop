//! One edit of a knowledge file, carried out (the standalone host's `KnowledgeEditCommand`,
//! `KnowledgeEditCommandHandler` and `RestoreKnowledgeFilesCommandHandler` at 8b6ccfc): the file is read as it is
//! now, the changes are made through the FBL runtime with the binding its format selects, and the result is saved
//! through the workspace's atomic writer. What it leaves behind is the content of every file it wrote from before
//! and after, so an undo gives the bytes back exactly.
//!
//! - **All or nothing.** The changes are made to a copy of the open body; the first one the runtime refuses ends the
//!   edit with its reason and the file is not written.
//! - **And so for a step over several files.** Every file's new content is made before any file is written, so a
//!   refusal in the second leaves the first as it was; and when a write itself fails after another has landed, what
//!   landed is put back.
//!
//! The standalone host runs these as commands of the project's history; the history here holds `Send` commands and
//! FBL's changes are not, so the core keeps the [`KnowledgeRestore`] an edit gives and calls it for undo.

use std::path::{Path, PathBuf};

use adp_documents::text::write_atomically;
use adp_fbl::ModelChange;

use crate::edits::KnowledgeOtherFile;
use crate::store;

/// One edit of a knowledge file: the changes a gesture came to, made to the file as one step, and what the same step
/// changes in other files.
#[derive(Debug, Clone)]
pub struct KnowledgeEditCommand {
    pub body_path: PathBuf,
    pub changes: Vec<ModelChange>,
    pub others: Vec<KnowledgeOtherFile>,
}

/// One file of a step that wrote several: what it held before, and what the step left it holding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeFileState {
    pub body_path: PathBuf,
    pub before: Vec<u8>,
    pub after: Vec<u8>,
}

/// What puts the files of one step back as they were, or makes the step again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeRestore {
    pub files: Vec<KnowledgeFileState>,
}

fn name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

impl KnowledgeEditCommand {
    /// Makes the edit: what it wrote, so that it can be undone, or why it was not written. An edit that changes no
    /// byte writes nothing and leaves nothing to undo.
    pub fn execute(&self) -> Result<Option<KnowledgeRestore>, String> {
        // Plan every file before writing any.
        let mut planned = Vec::new();
        let first = KnowledgeOtherFile {
            body_path: self.body_path.clone(),
            changes: self.changes.clone(),
        };
        for file in std::iter::once(&first).chain(&self.others) {
            let name = name_of(&file.body_path);
            let read = store::read(&file.body_path);
            let Some(body) = read.body else {
                return Err(format!("{name} could not be changed. {}", read.refusal));
            };
            let reason = body.read_only_reason();
            if !reason.is_empty() {
                return Err(if file.body_path == self.body_path {
                    reason
                } else {
                    format!("{name} cannot be changed: {reason}")
                });
            }
            let after = body.change(&file.changes)?;
            planned.push(KnowledgeFileState {
                body_path: file.body_path.clone(),
                before: body.bytes().to_vec(),
                after,
            });
        }
        let changed: Vec<KnowledgeFileState> = planned.into_iter().filter(|f| f.after != f.before).collect();
        if changed.is_empty() {
            // Nothing to write and nothing to undo.
            return Ok(None);
        }
        let mut written: Vec<&KnowledgeFileState> = Vec::new();
        for file in &changed {
            if let Err(e) = write_atomically(&file.body_path, &file.after) {
                // What landed before this one is put back: a step is every file or none.
                for landed in written {
                    let _ = write_atomically(&landed.body_path, &landed.before);
                }
                return Err(format!("{} could not be written: {e}", name_of(&file.body_path)));
            }
            written.push(file);
        }
        Ok(Some(KnowledgeRestore { files: changed }))
    }
}

impl KnowledgeRestore {
    /// Puts every file back to what it held before the step, or none: a file another program wrote since is not
    /// what the step left, and restoring over it would take that change with it.
    pub fn undo(&self) -> Result<(), String> {
        self.put(|f| &f.after, |f| &f.before)
    }

    /// Makes the step again, on the files as the undo left them.
    pub fn redo(&self) -> Result<(), String> {
        self.put(|f| &f.before, |f| &f.after)
    }

    fn put(
        &self,
        expected: impl Fn(&KnowledgeFileState) -> &Vec<u8>,
        wanted: impl Fn(&KnowledgeFileState) -> &Vec<u8>,
    ) -> Result<(), String> {
        for file in &self.files {
            if std::fs::read(&file.body_path).ok().as_ref() != Some(expected(file)) {
                return Err(format!(
                    "{} was changed by another program, so the edits made to it before that can no longer be undone.",
                    name_of(&file.body_path)
                ));
            }
        }
        let mut restored: Vec<&KnowledgeFileState> = Vec::new();
        for file in &self.files {
            if let Err(e) = write_atomically(&file.body_path, wanted(file)) {
                for done in restored {
                    let _ = write_atomically(&done.body_path, expected(done));
                }
                return Err(format!("Could not restore {}: {e}", name_of(&file.body_path)));
            }
            restored.push(file);
        }
        Ok(())
    }
}
