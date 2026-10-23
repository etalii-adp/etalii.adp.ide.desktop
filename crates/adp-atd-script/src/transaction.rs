//! The gesture's transaction (ATD §12.4): each action's commands are applied as the script runs, so every expression
//! sees the actions before it; the whole lands as one change of the history with an exact undo, or is discarded.

use adp_history::{Applied, Command, Composite, History, Outcome, Refusal};

pub struct Transaction<M> {
    label: String,
    inverses: Vec<Box<dyn Command<M>>>,
    warnings: Vec<String>,
    undoable: bool,
}

impl<M: 'static> Transaction<M> {
    pub fn new(label: impl Into<String>) -> Self {
        Transaction {
            label: label.into(),
            inverses: Vec::new(),
            warnings: Vec::new(),
            undoable: true,
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    /// Whether no command has been applied.
    pub fn is_empty(&self) -> bool {
        self.inverses.is_empty() && self.undoable
    }

    /// Applies a command now and keeps its inverse. A refusal changes nothing; the caller stops the call.
    pub fn apply(&mut self, model: &mut M, command: &dyn Command<M>) -> Result<(), Refusal> {
        let applied = command.apply(model)?;
        if !applied.warning.is_empty() {
            self.warnings.push(applied.warning);
        }
        match applied.inverse {
            Some(inverse) => self.inverses.push(inverse),
            None => self.undoable = false,
        }
        Ok(())
    }

    /// Reverses every command applied, the last first, leaving the model as it was.
    pub fn discard(self, model: &mut M) {
        for inverse in self.inverses.iter().rev() {
            // Best effort, as a composite's own rollback: an inverse that refuses leaves nothing better to do.
            let _ = inverse.apply(model);
        }
    }

    /// The change the transaction made, as one step: its inverse undoes every command in reverse order.
    pub fn finish(mut self) -> Applied<M> {
        let warning = self.warnings.join(" ");
        if !self.undoable {
            return Applied::final_change().with_warning(warning);
        }
        self.inverses.reverse();
        Applied {
            inverse: Some(Box::new(Composite::new(self.label, self.inverses))),
            warning,
        }
    }

    /// Records the transaction on the history as one change; one that changed nothing records nothing.
    pub fn commit(self, history: &mut History<M>) -> Outcome {
        if self.is_empty() {
            return Outcome::Done {
                label: self.label,
                warning: self.warnings.join(" "),
            };
        }
        let label = self.label.clone();
        history.record(label, self.finish())
    }
}
