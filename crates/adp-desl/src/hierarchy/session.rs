//! One open hierarchy surface on one connection (the standalone host's `IHierarchySurfaceSession` at a7fdf03): a
//! designer document drawn as one page of nested boxes. As the table's session does, it queues its changes and the
//! core drains them with [`HierarchySurfaceSession::take_changes`], so it needs no thread and no lock of its own.

use std::path::Path;

use super::model::{Gesture, SurfaceChange, SurfaceModel};

/// One open hierarchy surface on one connection.
pub trait HierarchySurfaceSession {
    /// The whole read model as it stands now.
    fn model(&self) -> SurfaceModel;

    /// Takes one gesture. `Ok` when it is accepted - a [`SurfaceChange::Model`] and a [`SurfaceChange::Outcome`] for
    /// its `edit_id` follow, in that order - or the sentence saying why it is refused, in which case nothing changed.
    fn gesture(&mut self, gesture: Gesture) -> Result<(), String>;

    /// What changed since the last call, in order.
    fn take_changes(&mut self) -> Vec<SurfaceChange>;

    /// Takes back the last written gesture, or says why it cannot be.
    fn undo(&mut self) -> Result<(), String>;

    /// Writes again the last gesture taken back, or says why it cannot be.
    fn redo(&mut self) -> Result<(), String>;

    /// The watcher's news that a file of the document changed: read it again, keeping what is collapsed by number.
    fn file_changed(&mut self, path: &Path);
}
