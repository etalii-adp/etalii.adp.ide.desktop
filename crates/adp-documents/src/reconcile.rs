//! A body changed on disk while it is open, reconciled with what the window holds (etalii.adp spec 030, FR-023 and
//! Story 2 scenario 5, task T058; FBL section 7.3).
//!
//! As the standalone host does at 8b6ccfc: a text tab without unsaved edits takes the new text over, and one with
//! edits that differ from the new text asks which side wins, "Load the disk version" or "Keep my changes"
//! (`TextEditorPanel.tsx`, modular-text-editors R6.4). Edits are compared with the new text in the editor's normal
//! form, every line ending as `\n`, so edits another program wrote back with other line endings are no conflict; a
//! clean tab takes over any new bytes, line endings included, so its next save keeps them. A diagram's
//! body is written behind every change, so it has no unsaved edits: it is read again and its history gets a barrier,
//! where undo stops (FR-016). The store's own save is not a change from outside; [`SelfWriteGuard`] tells it apart by
//! content, as the standalone host's `SelfWriteGuard` does, since a watcher reports a write after it lands.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// What a change on disk means for one open body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reconciliation {
    /// The body holds what the window already has: nothing to do.
    Unchanged,
    /// No unsaved edits stand in the way, or they equal the new text: the new text takes over.
    TakeOver,
    /// Unsaved edits differ from the new text: the person decides, with [`Resolution`].
    Conflict,
    /// The body is gone.
    Deleted,
}

/// How the person settles a [`Reconciliation::Conflict`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    /// "Load the disk version": the disk wins and the unsaved edits are let go.
    LoadDisk,
    /// "Keep my changes": the edits stay, unsaved, over the new text.
    KeepMine,
}

impl Resolution {
    /// The button's words, as the standalone host writes them.
    pub fn label(self) -> &'static str {
        match self {
            Resolution::LoadDisk => "Load the disk version",
            Resolution::KeepMine => "Keep my changes",
        }
    }
}

/// The notice over a text tab in conflict, as the standalone host writes it.
pub const CONFLICT_NOTICE: &str = "This file changed on disk while you were editing.";

/// Reconciles a text tab: `basis` is the text it last read or saved, `edited` its unsaved text when it has unsaved
/// edits, and `disk` what the file holds now, or `None` when it is gone.
pub fn reconcile_text(basis: &str, edited: Option<&str>, disk: Option<&str>) -> Reconciliation {
    let Some(disk) = disk else {
        return Reconciliation::Deleted;
    };
    if disk == basis {
        return Reconciliation::Unchanged;
    }
    match edited {
        Some(edited) if normal_form(edited) != normal_form(disk) => Reconciliation::Conflict,
        _ => Reconciliation::TakeOver,
    }
}

/// The text with every line ending as `\n`, the form the editor compares in.
fn normal_form(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Tells a store which changes on disk are its own saves (the standalone host's `SelfWriteGuard`): a save in flight,
/// or a file that holds exactly what it was last saved as. The remembered bytes are dropped the first time the file
/// is found to differ, so a later edit from outside that happens to restore them is not taken for a save.
#[derive(Debug, Default)]
pub struct SelfWriteGuard {
    writing: HashSet<PathBuf>,
    written: HashMap<PathBuf, Vec<u8>>,
}

impl SelfWriteGuard {
    /// Marks `path` as being written with `bytes`; pair with [`SelfWriteGuard::end`].
    pub fn begin(&mut self, path: &Path, bytes: &[u8]) {
        self.written.insert(key(path), bytes.to_vec());
        self.writing.insert(key(path));
    }

    /// The write to `path` is over, whether it landed or not.
    pub fn end(&mut self, path: &Path) {
        self.writing.remove(&key(path));
    }

    /// Whether a save to `path` is in flight.
    pub fn is_writing(&self, path: &Path) -> bool {
        self.writing.contains(&key(path))
    }

    /// Whether the change reported for `path` is this store's own, given what the file holds now (`None` when it
    /// cannot be read, which is never claimed).
    pub fn is_own_write(&mut self, path: &Path, on_disk: Option<&[u8]>) -> bool {
        let path = key(path);
        if self.writing.contains(&path) {
            return true;
        }
        let (Some(written), Some(on_disk)) = (self.written.get(&path), on_disk) else {
            return false;
        };
        if written.as_slice() == on_disk {
            return true;
        }
        self.written.remove(&path);
        false
    }

    /// Forgets what `path` was last saved as, when its document is dropped.
    pub fn forget(&mut self, path: &Path) {
        self.written.remove(&key(path));
    }
}

/// Paths compare without regard to case, as the standalone host's guard does on every platform.
fn key(path: &Path) -> PathBuf {
    PathBuf::from(path.to_string_lossy().to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tab_without_unsaved_edits_takes_the_new_text_over() {
        assert_eq!(reconcile_text("a\n", None, Some("b\n")), Reconciliation::TakeOver);
    }

    #[test]
    fn unsaved_edits_that_differ_from_the_new_text_are_a_conflict() {
        assert_eq!(
            reconcile_text("a\n", Some("mine\n"), Some("theirs\n")),
            Reconciliation::Conflict
        );
    }

    #[test]
    fn unsaved_edits_equal_to_the_new_text_are_no_conflict() {
        assert_eq!(
            reconcile_text("a\n", Some("b\n"), Some("b\r\n")),
            Reconciliation::TakeOver
        );
    }

    #[test]
    fn a_clean_tab_takes_over_other_line_endings_alone() {
        assert_eq!(reconcile_text("a\n", None, Some("a\r\n")), Reconciliation::TakeOver);
        assert_eq!(reconcile_text("a\n", None, Some("a\n")), Reconciliation::Unchanged);
    }

    #[test]
    fn a_body_that_is_gone_is_deleted() {
        assert_eq!(reconcile_text("a", None, None), Reconciliation::Deleted);
    }

    #[test]
    fn the_guard_claims_a_save_in_flight_and_a_file_that_holds_what_was_saved() {
        let mut guard = SelfWriteGuard::default();
        let path = Path::new("C:/Project/Ideas.mm");
        guard.begin(path, b"saved");
        assert!(guard.is_own_write(Path::new("c:/project/ideas.mm"), Some(b"anything")));
        guard.end(path);
        assert!(!guard.is_writing(path));
        assert!(
            guard.is_own_write(path, Some(b"saved")),
            "a late notice of the store's own save"
        );
        assert!(
            !guard.is_own_write(path, None),
            "a file that cannot be read is not claimed"
        );
    }

    #[test]
    fn the_guard_forgets_the_saved_bytes_once_the_file_differs() {
        let mut guard = SelfWriteGuard::default();
        let path = Path::new("ideas.mm");
        guard.begin(path, b"saved");
        guard.end(path);
        assert!(!guard.is_own_write(path, Some(b"theirs")));
        assert!(
            !guard.is_own_write(path, Some(b"saved")),
            "an edit from outside that restores the saved bytes is not the store's"
        );
        guard.begin(path, b"again");
        guard.end(path);
        guard.forget(path);
        assert!(!guard.is_own_write(path, Some(b"again")));
    }
}
