//! A file written only by splices, with the one history of its edits (FBL sections 7.1 and 7.2; the standalone
//! host's `History/EditHistory`, `SplicedFile` and `OpenBody`): a body, or a registration. Applying an edit applies
//! its splices and reads the file again. Undo and redo check for drift first and refuse with FBL section 7.2's
//! sentence, writing nothing. Re-reading after every edit, rather than patching the model, keeps one code path for
//! what a body means.

use std::rc::Rc;

use crate::documents::types::FblBinding;
use crate::model::{FblModel, FblOptions};
use crate::planning::{ModelChange, PlanResult, PlannedEdit, planner};
use crate::rules::reading::BodyReading;
use crate::splice::Splice;

#[cfg(test)]
mod tests;

/// FBL section 7.2's sentence for an undo refused because the file changed underneath it.
pub const DRIFT_UNDO: &str = "The file has changed since this edit, so it cannot be undone.";

pub const DRIFT_REDO: &str = "The file has changed since this edit was undone, so it cannot be redone.";

/// FBL 0.7 section 7.2's sentence for an edit refused because the file changed after it was read, before the edit was
/// applied.
pub const DRIFT_EDIT: &str = "The file has changed since this edit was made, so it was not written.";

/// What an undo or a redo did: the splices it applied, or why it was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UndoResult {
    Done(Vec<Splice>),
    Refused(String),
}

/// Applies `splices`, in order, offsets referring to the bytes before; they must be in body order and not overlap.
pub fn apply_in_order(bytes: &[u8], splices: &[Splice]) -> Result<Vec<u8>, String> {
    let mut output = Vec::with_capacity(bytes.len() + 64);
    let mut position = 0;
    for splice in splices {
        if splice.start < position || splice.end < splice.start || splice.end > bytes.len() {
            return Err(format!(
                "The splices of an edit overlap or leave the body: {} after offset {position}.",
                crate::planning::plan::describe(splice)
            ));
        }
        output.extend_from_slice(&bytes[position..splice.start]);
        output.extend_from_slice(splice.text.as_bytes());
        position = splice.end;
    }
    output.extend_from_slice(&bytes[position..]);
    Ok(output)
}

/// The splices that undo `splices` once applied to `before`: each puts its replaced bytes back, at its offset in
/// the body after the edit, under its own operation (FBL section 15.3).
pub fn inverse(before: &[u8], splices: &[Splice]) -> Vec<Splice> {
    let mut inverse = Vec::with_capacity(splices.len());
    let mut shift: isize = 0;
    for splice in splices {
        let start = (splice.start as isize + shift) as usize;
        let written = splice.text.len();
        let replaced = String::from_utf8_lossy(&before[splice.start..splice.end]).into_owned();
        inverse.push(Splice {
            operation: splice.operation,
            start,
            end: start + written,
            text: replaced,
        });
        shift += written as isize - (splice.end - splice.start) as isize;
    }
    inverse
}

#[derive(Debug, Clone)]
struct HistoryEntry {
    edit: PlannedEdit,
    inverse: Vec<Splice>,
    before: Vec<u8>,
    after: Vec<u8>,
}

/// The bytes of a file and the history of its edits.
#[derive(Debug, Clone, Default)]
pub struct SplicedFile {
    bytes: Vec<u8>,
    undo: Vec<HistoryEntry>,
    redo: Vec<HistoryEntry>,
}

impl SplicedFile {
    pub fn new(bytes: Vec<u8>) -> Self {
        SplicedFile {
            bytes,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    /// The file's bytes after every applied edit.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Applies an edit planned against the current bytes and records it; true when the bytes changed.
    pub fn apply(&mut self, edit: &PlannedEdit) -> Result<bool, String> {
        if edit.splices.is_empty() {
            return Ok(false);
        }
        let before = self.bytes.clone();
        let after = apply_in_order(&before, &edit.splices)?;
        self.undo.push(HistoryEntry {
            edit: edit.clone(),
            inverse: inverse(&before, &edit.splices),
            before,
            after: after.clone(),
        });
        self.redo.clear();
        self.bytes = after;
        Ok(true)
    }

    /// Applies an edit that is no step of the history (FBL 0.7 section 7.1, a block declared `history: false`). The
    /// history is kept: every entry is carried over the edit, so an undo or a redo after it passes it by and writes
    /// its own splices at the offsets the edit moved. An entry whose splices the edit overlaps cannot be carried, and
    /// the history is cleared instead. True when the bytes changed.
    pub fn apply_outside_history(&mut self, edit: &PlannedEdit) -> Result<bool, String> {
        if edit.splices.is_empty() {
            return Ok(false);
        }
        let after = apply_in_order(&self.bytes, &edit.splices)?;
        self.carry_over(&after, &edit.splices);
        self.bytes = after;
        Ok(true)
    }

    /// Carries every entry over `through`, an edit outside the history already planned, which makes the file
    /// `current`. Each entry's splices are moved by the bytes the edit added or removed before them, and its bodies
    /// are those the entries now lead through. When the edit overlaps an entry's splices, the history is cleared:
    /// that entry cannot be undone without undoing the edit.
    fn carry_over(&mut self, current: &[u8], through: &[Splice]) {
        let mut undo = Vec::new();
        let mut state = current.to_vec();
        let mut outside = through.to_vec();
        for entry in self.undo.iter().rev() {
            let Some(inverse_splices) = moved(&entry.inverse, &outside) else {
                return self.clear();
            };
            let Ok(before) = apply_in_order(&state, &inverse_splices) else {
                return self.clear();
            };
            let edit = PlannedEdit {
                splices: inverse(&state, &inverse_splices),
                snapshot: false,
            };
            let Some(earlier) = moved(&outside, &entry.inverse) else {
                return self.clear();
            };
            undo.push(HistoryEntry {
                edit,
                inverse: inverse_splices,
                before: before.clone(),
                after: state,
            });
            outside = earlier;
            state = before;
        }
        let mut redo = Vec::new();
        let mut state = current.to_vec();
        let mut outside = through.to_vec();
        for entry in self.redo.iter().rev() {
            let Some(splices) = moved(&entry.edit.splices, &outside) else {
                return self.clear();
            };
            let Ok(after) = apply_in_order(&state, &splices) else {
                return self.clear();
            };
            let Some(later) = moved(&outside, &entry.edit.splices) else {
                return self.clear();
            };
            redo.push(HistoryEntry {
                inverse: inverse(&state, &splices),
                edit: PlannedEdit {
                    splices,
                    snapshot: false,
                },
                before: state,
                after: after.clone(),
            });
            outside = later;
            state = after;
        }
        undo.reverse();
        redo.reverse();
        self.undo = undo;
        self.redo = redo;
    }

    fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    /// Undoes the most recent edit. `current` is the file as it is now on disk or in another editor's buffer; when
    /// it differs from what the history expects, the undo is refused and nothing is written.
    pub fn undo(&mut self, current: Option<&[u8]>) -> UndoResult {
        let Some(entry) = self.undo.last() else {
            return UndoResult::Refused("There is nothing to undo.".into());
        };
        if current.unwrap_or(&self.bytes) != entry.after.as_slice() {
            return UndoResult::Refused(DRIFT_UNDO.into());
        }
        let entry = self.undo.pop().expect("an edit to undo");
        // A snapshot edit's undo gives the same bytes as its inverse splices: the body before it.
        self.bytes = entry.before.clone();
        let splices = entry.inverse.clone();
        self.redo.push(entry);
        UndoResult::Done(splices)
    }

    /// Redoes the most recently undone edit, refused on drift as `undo` is.
    pub fn redo(&mut self, current: Option<&[u8]>) -> UndoResult {
        let Some(entry) = self.redo.last() else {
            return UndoResult::Refused("There is nothing to redo.".into());
        };
        if current.unwrap_or(&self.bytes) != entry.before.as_slice() {
            return UndoResult::Refused(DRIFT_REDO.into());
        }
        let entry = self.redo.pop().expect("an edit to redo");
        self.bytes = entry.after.clone();
        let splices = entry.edit.splices.clone();
        self.undo.push(entry);
        UndoResult::Done(splices)
    }

    /// Replaces the bytes after an external change the host has accepted (FBL section 7.3) and clears the history.
    pub fn reload(&mut self, bytes: Vec<u8>) {
        self.clear();
        self.bytes = bytes;
    }
}

/// `splices` with their offsets moved over `through`, both in the offsets of one body (an edit outside the history is
/// carried past an entry by that entry's own splices, not by its moved ones): by the bytes every splice of `through`
/// that ends before one, or inserts where it starts, adds or removes. None when the two overlap.
fn moved(splices: &[Splice], through: &[Splice]) -> Option<Vec<Splice>> {
    let mut moved = Vec::with_capacity(splices.len());
    for splice in splices {
        let mut shift: isize = 0;
        for other in through {
            if other.start < splice.end && splice.start < other.end {
                return None;
            }
            if other.end <= splice.start {
                shift += other.text.len() as isize - (other.end - other.start) as isize;
            }
        }
        moved.push(Splice {
            start: (splice.start as isize + shift) as usize,
            end: (splice.end as isize + shift) as usize,
            ..splice.clone()
        });
    }
    Some(moved)
}

/// One open body (FBL section 9.1): its bytes, its reading through one binding, and the one history of its edits.
#[derive(Debug)]
pub struct OpenBody {
    file: SplicedFile,
    binding: Rc<FblBinding>,
    options: Rc<FblOptions>,
    reading: BodyReading,
}

impl OpenBody {
    pub fn open(bytes: Vec<u8>, binding: Rc<FblBinding>, options: Rc<FblOptions>) -> OpenBody {
        let reading = BodyReading::read(bytes.clone(), binding.clone(), options.clone());
        OpenBody {
            file: SplicedFile::new(bytes),
            binding,
            options,
            reading,
        }
    }

    pub fn bytes(&self) -> &[u8] {
        self.file.bytes()
    }

    pub fn model(&self) -> FblModel {
        self.reading.to_model()
    }

    pub fn reading(&self) -> &BodyReading {
        &self.reading
    }

    pub fn can_undo(&self) -> bool {
        self.file.can_undo()
    }

    /// An unreadable body (FBL section 7.5) and a read-only binding (FBL section 3.4) refuse every change.
    pub fn is_read_only(&self) -> bool {
        self.reading.unreadable().is_some() || self.binding.read_only.is_some()
    }

    /// Plans `change` against the current bytes without applying it.
    pub fn plan(&self, change: &ModelChange) -> PlanResult {
        planner::plan(&self.reading, change)
    }

    /// Plans and applies `change`: the planned edit, or the refusal with nothing written.
    pub fn change(&mut self, change: &ModelChange) -> PlanResult {
        let result = self.plan(change);
        if let PlanResult::Planned(edit) = &result
            && let Err(problem) = self.apply(edit)
        {
            return PlanResult::Invalid(problem);
        }
        result
    }

    /// Plans and applies `change` when `current`, the file as it is now on disk or in another editor's buffer, is the
    /// bytes it is planned against (FBL 0.7 section 7.2). When it is not, the edit is refused with [`DRIFT_EDIT`],
    /// nothing is written, and the body is read again from `current`, which ends the history (FBL section 7.3).
    pub fn change_against(&mut self, change: &ModelChange, current: &[u8]) -> PlanResult {
        if current == self.file.bytes() {
            return self.change(change);
        }
        self.reload(current.to_vec());
        PlanResult::Refused(DRIFT_EDIT.into())
    }

    /// Replaces the bytes after an external change the host has accepted (FBL section 7.3): reads again and clears
    /// the history.
    pub fn reload(&mut self, bytes: Vec<u8>) {
        self.file.reload(bytes);
        self.reread();
    }

    /// Applies an edit planned against the current bytes, records it, and reads the body again.
    pub fn apply(&mut self, edit: &PlannedEdit) -> Result<(), String> {
        if self.file.apply(edit)? {
            self.reread();
        }
        Ok(())
    }

    pub fn undo(&mut self) -> UndoResult {
        let result = self.file.undo(None);
        if let UndoResult::Done(_) = result {
            self.reread();
        }
        result
    }

    pub fn redo(&mut self) -> UndoResult {
        let result = self.file.redo(None);
        if let UndoResult::Done(_) = result {
            self.reread();
        }
        result
    }

    fn reread(&mut self) {
        self.reading = BodyReading::read(self.file.bytes().to_vec(), self.binding.clone(), self.options.clone());
    }
}
