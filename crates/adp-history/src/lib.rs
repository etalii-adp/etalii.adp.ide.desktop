//! Commands, composite commands and the history with its barriers (etalii.adp spec 030, FR-016, task T015).
//!
//! Ported from the standalone host's `EtAlii.Adp.History` at `8b6ccfc`: every change is a command that, when it
//! succeeds, returns the command that reverses it; the history keeps those inverses so they can be walked back and
//! forward. A rejected command changes nothing and keeps the redo side. The undo side holds at most
//! [`History::DEFAULT_CAPACITY`] changes and drops the oldest past that. A change from outside (another program
//! writing the body) ends the history: undo stops there and says why.

use std::collections::VecDeque;
use std::fmt;

/// Why a command, an undo or a redo did not happen: one sentence a person can read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal(pub String);

impl Refusal {
    pub fn new(reason: impl Into<String>) -> Self {
        Self(reason.into())
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Refusal {}

/// What a command that succeeded leaves behind.
pub struct Applied<M> {
    /// The command that reverses this one, or `None` when the change cannot be undone.
    pub inverse: Option<Box<dyn Command<M>>>,
    /// Something the person should know although the command succeeded; empty when there is nothing.
    pub warning: String,
}

impl<M> Applied<M> {
    pub fn undoable(inverse: impl Command<M> + 'static) -> Self {
        Self {
            inverse: Some(Box::new(inverse)),
            warning: String::new(),
        }
    }

    pub fn final_change() -> Self {
        Self {
            inverse: None,
            warning: String::new(),
        }
    }

    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warning = warning.into();
        self
    }
}

/// One change of a model of type `M`. Commands are `Send`, so a history can live on the core thread.
pub trait Command<M>: Send {
    /// What the change is called, as a person reads it in a menu ("Rename", "Move 3 elements").
    fn label(&self) -> String;

    /// Applies the change, or refuses it without changing the model.
    fn apply(&self, model: &mut M) -> Result<Applied<M>, Refusal>;
}

/// Several commands applied as one step: one entry in the history, undone in reverse order.
pub struct Composite<M> {
    label: String,
    parts: Vec<Box<dyn Command<M>>>,
}

impl<M> Composite<M> {
    pub fn new(label: impl Into<String>, parts: Vec<Box<dyn Command<M>>>) -> Self {
        Self {
            label: label.into(),
            parts,
        }
    }
}

impl<M: 'static> Command<M> for Composite<M> {
    fn label(&self) -> String {
        self.label.clone()
    }

    /// Applies every part in order. When one refuses, the parts already applied are reversed, so the model is left
    /// as it was and the refusal is the composite's.
    fn apply(&self, model: &mut M) -> Result<Applied<M>, Refusal> {
        let mut inverses: Vec<Box<dyn Command<M>>> = Vec::with_capacity(self.parts.len());
        let mut warnings = Vec::new();
        let mut undoable = true;
        for part in &self.parts {
            match part.apply(model) {
                Ok(applied) => {
                    if !applied.warning.is_empty() {
                        warnings.push(applied.warning);
                    }
                    match applied.inverse {
                        Some(inverse) => inverses.push(inverse),
                        None => undoable = false,
                    }
                }
                Err(refusal) => {
                    for inverse in inverses.iter().rev() {
                        // Best effort: an inverse that refuses leaves nothing better to do than report the first refusal.
                        let _ = inverse.apply(model);
                    }
                    return Err(refusal);
                }
            }
        }
        let warning = warnings.join(" ");
        if !undoable {
            return Ok(Applied::final_change().with_warning(warning));
        }
        inverses.reverse();
        Ok(Applied {
            inverse: Some(Box::new(Composite::new(self.label.clone(), inverses))),
            warning,
        })
    }
}

/// The result of an operation on the history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// It happened; `warning` is empty unless the person should know something.
    Done { label: String, warning: String },
    /// It did not happen, and why.
    Refused(Refusal),
}

impl Outcome {
    pub fn is_done(&self) -> bool {
        matches!(self, Outcome::Done { .. })
    }
}

/// What the history can do now, read in one moment (the standalone host's `HistoryAvailability`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Availability {
    pub can_undo: bool,
    pub can_redo: bool,
    pub undo_count: usize,
    pub redo_count: usize,
}

enum Entry<M> {
    Change {
        label: String,
        inverse: Box<dyn Command<M>>,
    },
    Barrier {
        reason: String,
    },
}

/// The undo and redo sides of one project's changes.
pub struct History<M> {
    undo: VecDeque<Entry<M>>,
    redo: Vec<(String, Box<dyn Command<M>>)>,
    capacity: usize,
}

impl<M> Default for History<M> {
    fn default() -> Self {
        Self::with_capacity(Self::DEFAULT_CAPACITY)
    }
}

impl<M> History<M> {
    /// As in the standalone host: generous rather than tight, so a long session can still undo far back.
    pub const DEFAULT_CAPACITY: usize = 100;

    /// The sentence undo gives when the history ends at a change from outside, unless the caller gives its own.
    pub const CHANGED_OUTSIDE: &'static str =
        "The file was changed outside ADP, so the changes before that cannot be undone.";

    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity >= 1, "a history holds at least one change");
        Self {
            undo: VecDeque::new(),
            redo: Vec::new(),
            capacity,
        }
    }

    pub fn availability(&self) -> Availability {
        Availability {
            can_undo: matches!(self.undo.back(), Some(Entry::Change { .. })),
            can_redo: !self.redo.is_empty(),
            undo_count: self.changes(),
            redo_count: self.redo.len(),
        }
    }

    /// Runs the command and, when it succeeds with an inverse, records it and discards the redo side, whose future no
    /// longer exists. A refusal changes nothing, the redo side included.
    pub fn execute(&mut self, model: &mut M, command: &dyn Command<M>) -> Outcome {
        let label = command.label();
        match command.apply(model) {
            Err(refusal) => Outcome::Refused(refusal),
            Ok(applied) => {
                if let Some(inverse) = applied.inverse {
                    self.redo.clear();
                    self.push(label.clone(), inverse);
                }
                Outcome::Done {
                    label,
                    warning: applied.warning,
                }
            }
        }
    }

    /// Reverses the most recent change and moves it to the redo side. At a barrier it refuses with the barrier's reason.
    pub fn undo(&mut self, model: &mut M) -> Outcome {
        match self.undo.pop_back() {
            None => Outcome::Refused(Refusal::new("There is nothing to undo.")),
            Some(Entry::Barrier { reason }) => {
                let refusal = Refusal::new(reason.clone());
                self.undo.push_back(Entry::Barrier { reason });
                Outcome::Refused(refusal)
            }
            Some(Entry::Change { label, inverse }) => match inverse.apply(model) {
                Err(refusal) => {
                    self.undo.push_back(Entry::Change { label, inverse });
                    Outcome::Refused(refusal)
                }
                Ok(applied) => {
                    if let Some(redo) = applied.inverse {
                        self.redo.push((label.clone(), redo));
                    }
                    Outcome::Done {
                        label,
                        warning: applied.warning,
                    }
                }
            },
        }
    }

    /// Re-applies the most recently undone change and moves it back to the undo side.
    pub fn redo(&mut self, model: &mut M) -> Outcome {
        let Some((label, command)) = self.redo.pop() else {
            return Outcome::Refused(Refusal::new("There is nothing to redo."));
        };
        match command.apply(model) {
            Err(refusal) => {
                self.redo.push((label, command));
                Outcome::Refused(refusal)
            }
            Ok(applied) => {
                if let Some(inverse) = applied.inverse {
                    self.push(label.clone(), inverse);
                }
                Outcome::Done {
                    label,
                    warning: applied.warning,
                }
            }
        }
    }

    /// Ends the history at a change from outside: what came before can no longer be undone, and redo is gone.
    pub fn mark_changed_outside(&mut self, reason: Option<&str>) {
        self.redo.clear();
        if !matches!(self.undo.back(), Some(Entry::Barrier { .. })) {
            let reason = reason.unwrap_or(Self::CHANGED_OUTSIDE).to_owned();
            self.undo.push_back(Entry::Barrier { reason });
        }
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    fn changes(&self) -> usize {
        self.undo.iter().filter(|e| matches!(e, Entry::Change { .. })).count()
    }

    fn push(&mut self, label: String, inverse: Box<dyn Command<M>>) {
        self.undo.push_back(Entry::Change { label, inverse });
        while self.changes() > self.capacity {
            self.undo.pop_front();
        }
    }
}
