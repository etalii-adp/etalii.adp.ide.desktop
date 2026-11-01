//! The splice catalogue and the edit history (FBL sections 6.1, 6.5 and 7.1; etalii.adp spec 030, FR-031, task T039).
//!
//! Every write to a body is a splice: one byte range of the body before the edit, possibly empty, replaced by new
//! text, possibly empty. An edit is the splices of one transaction, applied together, recorded together and undone
//! together. Offsets are UTF-8 byte offsets into the body before the edit; the splices of an edit do not overlap and
//! apply in the order of their offsets, those at one offset in the order the edit lists them. Every byte outside the
//! splices stays as it was, the byte order mark and every line ending included.

use std::fmt;

use serde::{Deserialize, Serialize};

/// The twelve operations of FBL section 6.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Operation {
    ReplaceValue,
    InsertKey,
    RemoveKey,
    InsertEntry,
    RemoveEntry,
    EnsureContainer,
    RemoveContainer,
    RewriteReference,
    ReEmitLine,
    OpenBlock,
    SelfClose,
    /// *(FBL 0.16)* The last token of a statement an insert writes into.
    ReplaceSeparator,
}

impl Operation {
    pub const ALL: [Operation; 12] = [
        Operation::ReplaceValue,
        Operation::InsertKey,
        Operation::RemoveKey,
        Operation::InsertEntry,
        Operation::RemoveEntry,
        Operation::EnsureContainer,
        Operation::RemoveContainer,
        Operation::RewriteReference,
        Operation::ReEmitLine,
        Operation::OpenBlock,
        Operation::SelfClose,
        Operation::ReplaceSeparator,
    ];

    /// The name FBL gives the operation (`replace-value`, `insert-entry`, …).
    pub fn name(self) -> &'static str {
        match self {
            Operation::ReplaceValue => "replace-value",
            Operation::InsertKey => "insert-key",
            Operation::RemoveKey => "remove-key",
            Operation::InsertEntry => "insert-entry",
            Operation::RemoveEntry => "remove-entry",
            Operation::EnsureContainer => "ensure-container",
            Operation::RemoveContainer => "remove-container",
            Operation::RewriteReference => "rewrite-reference",
            Operation::ReEmitLine => "re-emit-line",
            Operation::OpenBlock => "open-block",
            Operation::SelfClose => "self-close",
            Operation::ReplaceSeparator => "replace-separator",
        }
    }
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// One replacement of a byte range by new text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Splice {
    pub operation: Operation,
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// Why splices could not be applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpliceError {
    /// A splice's range is backwards or reaches past the end of the body.
    Outside { start: usize, end: usize, length: usize },
    /// A splice starts inside the range of the one before it.
    Overlap { start: usize, end: usize },
    /// A splice cuts a UTF-8 character of the body in two.
    NotOnCharacter { offset: usize },
}

impl fmt::Display for SpliceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpliceError::Outside { start, end, length } => {
                write!(f, "the splice {start}..{end} is outside the body of {length} bytes")
            }
            SpliceError::Overlap { start, end } => write!(f, "the splice {start}..{end} overlaps the one before it"),
            SpliceError::NotOnCharacter { offset } => write!(f, "byte {offset} is inside a character"),
        }
    }
}

impl std::error::Error for SpliceError {}

/// An edit applied: the body after it, and the inverse splices, by offsets into that body, that give the body before
/// it back. Each inverse splice keeps the operation of the splice it inverts, as FBL's fixtures list undo steps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub before: Vec<u8>,
    pub after: Vec<u8>,
    pub splices: Vec<Splice>,
    pub inverse: Vec<Splice>,
}

/// Applies an edit's splices to a body. The splices are taken in the order of their offsets, stably, so those at one
/// offset keep the order the edit lists them in.
pub fn apply(body: &[u8], splices: &[Splice]) -> Result<Edit, SpliceError> {
    let mut ordered: Vec<&Splice> = splices.iter().collect();
    ordered.sort_by_key(|s| (s.start, s.end));
    let mut previous_end = 0;
    for splice in &ordered {
        if splice.start > splice.end || splice.end > body.len() {
            return Err(SpliceError::Outside {
                start: splice.start,
                end: splice.end,
                length: body.len(),
            });
        }
        if splice.start < previous_end {
            return Err(SpliceError::Overlap {
                start: splice.start,
                end: splice.end,
            });
        }
        for offset in [splice.start, splice.end] {
            if offset < body.len() && (body[offset] & 0b1100_0000) == 0b1000_0000 {
                return Err(SpliceError::NotOnCharacter { offset });
            }
        }
        previous_end = splice.end;
    }
    let mut after = Vec::with_capacity(body.len() + splices.iter().map(|s| s.text.len()).sum::<usize>());
    let mut inverse = Vec::with_capacity(ordered.len());
    let mut copied = 0;
    for splice in &ordered {
        after.extend_from_slice(&body[copied..splice.start]);
        let start = after.len();
        after.extend_from_slice(splice.text.as_bytes());
        inverse.push(Splice {
            operation: splice.operation,
            start,
            end: after.len(),
            text: String::from_utf8_lossy(&body[splice.start..splice.end]).into_owned(),
        });
        copied = splice.end;
    }
    after.extend_from_slice(&body[copied..]);
    Ok(Edit {
        before: body.to_vec(),
        after,
        splices: ordered.into_iter().cloned().collect(),
        inverse,
    })
}

/// The undo and redo sides of a body's edits (FBL section 7.1): undo applies an edit's inverse and redo the edit
/// again, each only from the body the edit left or started from. A new edit discards what could be redone.
#[derive(Debug, Default)]
pub struct EditHistory {
    done: Vec<Edit>,
    undone: Vec<Edit>,
}

impl EditHistory {
    /// Applies an edit to `body` and records it.
    pub fn edit(&mut self, body: &[u8], splices: &[Splice]) -> Result<Vec<u8>, SpliceError> {
        let edit = apply(body, splices)?;
        let after = edit.after.clone();
        self.done.push(edit);
        self.undone.clear();
        Ok(after)
    }

    /// The splices that undo the last edit, and the body they give, when `body` is what that edit left.
    pub fn undo(&mut self, body: &[u8]) -> Option<(Vec<Splice>, Vec<u8>)> {
        let edit = self.done.last()?;
        if edit.after != body {
            return None;
        }
        let edit = self.done.pop()?;
        let result = (edit.inverse.clone(), edit.before.clone());
        self.undone.push(edit);
        Some(result)
    }

    /// The splices that redo the last undone edit, and the body they give, when `body` is what it started from.
    pub fn redo(&mut self, body: &[u8]) -> Option<(Vec<Splice>, Vec<u8>)> {
        let edit = self.undone.last()?;
        if edit.before != body {
            return None;
        }
        let edit = self.undone.pop()?;
        let result = (edit.splices.clone(), edit.after.clone());
        self.done.push(edit);
        Some(result)
    }

    pub fn can_undo(&self) -> bool {
        !self.done.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.undone.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn splice(operation: Operation, start: usize, end: usize, text: &str) -> Splice {
        Splice {
            operation,
            start,
            end,
            text: text.into(),
        }
    }

    #[test]
    fn splices_apply_by_offset_and_their_inverse_restores_the_body() {
        let body = "a: 1\r\nb: 2\r\n".as_bytes();
        let edit = apply(
            body,
            &[
                splice(Operation::InsertEntry, 12, 12, "c: 3\r\n"),
                splice(Operation::ReplaceValue, 3, 4, "10"),
            ],
        )
        .unwrap();
        assert_eq!(edit.after, b"a: 10\r\nb: 2\r\nc: 3\r\n");
        assert_eq!(apply(&edit.after, &edit.inverse).unwrap().after, body);
        assert_eq!(edit.inverse[0], splice(Operation::ReplaceValue, 3, 5, "1"));
    }

    #[test]
    fn splices_at_one_offset_keep_the_order_the_edit_lists_them_in() {
        let edit = apply(
            b"[]",
            &[
                splice(Operation::EnsureContainer, 1, 1, "{"),
                splice(Operation::InsertEntry, 1, 1, "x"),
                splice(Operation::EnsureContainer, 1, 1, "}"),
            ],
        )
        .unwrap();
        assert_eq!(edit.after, b"[{x}]");
    }

    #[test]
    fn overlapping_outside_and_split_character_splices_are_refused() {
        assert!(matches!(
            apply(b"abc", &[splice(Operation::RemoveKey, 2, 5, "")]),
            Err(SpliceError::Outside { .. })
        ));
        assert!(matches!(
            apply(
                b"abcdef",
                &[
                    splice(Operation::RemoveKey, 0, 3, ""),
                    splice(Operation::RemoveKey, 2, 4, "")
                ]
            ),
            Err(SpliceError::Overlap { .. })
        ));
        assert!(matches!(
            apply("é".as_bytes(), &[splice(Operation::ReplaceValue, 1, 2, "e")]),
            Err(SpliceError::NotOnCharacter { offset: 1 })
        ));
    }

    #[test]
    fn undo_and_redo_walk_the_edits_from_the_body_they_left() {
        let mut history = EditHistory::default();
        let body = b"x".to_vec();
        let edited = history.edit(&body, &[splice(Operation::InsertKey, 1, 1, "y")]).unwrap();
        assert!(
            history.undo(b"something else").is_none(),
            "undo refuses a body the edit did not leave"
        );
        let (_, undone) = history.undo(&edited).unwrap();
        assert_eq!(undone, body);
        let (_, redone) = history.redo(&undone).unwrap();
        assert_eq!(redone, edited);
        assert!(!history.can_redo());
    }
}
