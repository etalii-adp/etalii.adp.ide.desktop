//! One open requirement document on one connection, as a hierarchy surface (the standalone host's
//! `HierarchicalRequirementSession` at a7fdf03): the whole read model, kept true to the Markdown file and its
//! registration on disk, and the person's gestures.
//!
//! - **The files are the truth.** Opening writes nothing. The session reads the body and the registration, when there
//!   is one, and reads them again whenever either changes; it queues the whole model each time, and nothing when the
//!   bytes are those it already holds.
//! - **A file that stops being readable keeps the last page**, read-only and saying why, until it can be read again:
//!   what the person was looking at does not vanish under them. A file changed from outside that cannot be read is
//!   read again first, [`READ_ATTEMPTS`] times in all, since another program may be halfway through writing it.
//! - **The registration is created with the first setting the person changes**, outside the history, as the
//!   binding's `createOnFirstPlacement` says (FBL 0.7 section 8.4). Collapsing is such a setting, and is no undo step
//!   either: the `collapsed` block is declared outside the history.
//!
//! - **An edit of the body, of a text or of the hierarchy, is one step of the session's history**, planned against
//!   the bytes this session's page was made from; a file that holds others by the time it runs is not written, and
//!   the page is read again (FR-024). Undo gives the bytes back exactly; once another program has written the file,
//!   it says so once and the steps before are gone.
//!
//! The standalone host writes on a task and watches the files; here a gesture is written when it is taken, and the
//! core tells the session of a change from outside with [`HierarchySurfaceSession::file_changed`], so the session
//! needs no thread of its own. Of the operations, etalii.adp spec 036 has built `toggleCollapsed`, the text and the
//! structure operations so far; the others are refused with the reason until theirs.

use std::path::{Path, PathBuf};

use adp_desl::hierarchy::model::{EditOutcome, Gesture, SurfaceAction, SurfaceChange, SurfaceModel, causes, roles};
use adp_desl::hierarchy::session::HierarchySurfaceSession;

use crate::body::RequirementBody;
use crate::commands::{EditCommand, Restore, Undone};
use crate::documents::{self, Files, Writer};
use crate::gestures;
use crate::mapper;
use crate::operations::{structure, text};

/// What undo and redo are refused with while no gesture of this page is a step of the history.
const NOTHING_TO_UNDO: &str = "There is nothing to undo.";
const NOTHING_TO_REDO: &str = "There is nothing to redo.";
/// How often a file changed from outside is read before it is taken as unreadable, and how long apart: the standalone
/// host's `WatchedFile.ReadAttempts` and `BetweenReadAttempts` at a7fdf03.
pub const READ_ATTEMPTS: usize = 3;
pub const BETWEEN_READ_ATTEMPTS: std::time::Duration = std::time::Duration::from_millis(20);

/// What is done between two reads of a file changed from outside.
pub type Wait = Box<dyn FnMut()>;

/// The ids of the actions a selected entry offers, the standalone host's.
pub const ADD_BELOW: &str = "hierarchical-requirement.add-below";
pub const ADD_REQUIREMENT: &str = "hierarchical-requirement.add-requirement";

/// One reading of the two files and the model made of it.
struct Reading {
    files: Files,
    model: SurfaceModel,
    /// Whether the body could not be read as a requirement document.
    unreadable: bool,
}

/// One open requirement document on one connection.
pub struct RequirementSession {
    body_path: PathBuf,
    registration_path: PathBuf,
    write: Writer,
    wait: Wait,
    reading: Reading,
    changes: Vec<SurfaceChange>,
    /// The steps written through this session, the latest last, and those undone, the latest undone last.
    restores: Vec<Restore>,
    undone: Vec<EditCommand>,
}

impl RequirementSession {
    /// Opens the Markdown file `body_path`, with its registration at `registration_path`, or, when it has none yet,
    /// one beside it named after it, created with the first setting.
    pub fn open(body_path: impl Into<PathBuf>, registration_path: Option<PathBuf>) -> Self {
        RequirementSession::with(body_path, registration_path, documents::disk_writer())
    }

    /// A session with its writer given: a test refuses a write exactly when it chooses.
    pub fn with(body_path: impl Into<PathBuf>, registration_path: Option<PathBuf>, write: Writer) -> Self {
        let body_path = body_path.into();
        let registration_path = registration_path.unwrap_or_else(|| documents::registration_beside(&body_path));
        let files = documents::read(&body_path, &registration_path);
        let mut session = RequirementSession {
            reading: Reading {
                files: Files::default(),
                model: SurfaceModel::default(),
                unreadable: true,
            },
            body_path,
            registration_path,
            write,
            wait: Box::new(|| std::thread::sleep(BETWEEN_READ_ATTEMPTS)),
            changes: Vec::new(),
            restores: Vec::new(),
            undone: Vec::new(),
        };
        session.reading = session
            .build(&files, causes::OPEN)
            .unwrap_or_else(|| session.unreadable(files));
        session
    }

    /// The session with `wait` done between two reads of a file changed from outside: a test changes the file there
    /// exactly when it chooses, and waits for nothing.
    pub fn waiting(mut self, wait: Wait) -> Self {
        self.wait = wait;
        self
    }

    fn file_name(path: &Path) -> String {
        path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// The reading the two files give, as following `cause`; none when the body could not be read at all.
    fn build(&self, files: &Files, cause: &str) -> Option<Reading> {
        let bytes = files.body.clone()?;
        let name = Self::file_name(&self.body_path);
        let registration_name = Self::file_name(&self.registration_path);
        let title = mapper::title(&String::from_utf8_lossy(&bytes), &name);
        let read = RequirementBody::read(
            bytes,
            &name,
            files.registration.clone().map(|r| (r, registration_name.as_str())),
        );
        let registration = read.registration.as_ref();
        let stored = |block: &str| -> Option<Vec<String>> {
            registration?
                .block_entries(block)
                .map(|entries| entries.into_iter().map(|(key, _)| key).collect())
        };
        let model = mapper::map(&read.document, &title, cause, &stored);
        Some(Reading {
            files: files.clone(),
            model,
            unreadable: read.body.model().unreadable,
        })
    }

    fn unreadable(&self, files: Files) -> Reading {
        let model = SurfaceModel {
            title: self
                .body_path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            read_only_reason: format!(
                "'{}' cannot be read: {}",
                Self::file_name(&self.body_path),
                files.refusal
            ),
            cause: causes::OPEN.into(),
            ..SurfaceModel::default()
        };
        Reading {
            files,
            model,
            unreadable: true,
        }
    }

    /// Takes `reading` as the page's when its files differ from those the page was made from, and returns whether
    /// it did. A reading of a file that cannot be read keeps the last page's nodes, read-only with the reason, for
    /// as long as it cannot.
    fn take(&mut self, reading: Option<Reading>) -> bool {
        let Some(mut reading) = reading else {
            return false;
        };
        if reading.files == self.reading.files {
            return false;
        }
        if reading.unreadable && !self.reading.model.nodes.is_empty() {
            reading.model = SurfaceModel {
                read_only_reason: reading.model.read_only_reason,
                cause: reading.model.cause,
                findings: reading.model.findings,
                ..self.reading.model.clone()
            };
        }
        self.reading = reading;
        true
    }

    /// Reads both files again, as following `cause`, and queues the model when they changed.
    fn read_again(&mut self, cause: &str) -> bool {
        let files = documents::read(&self.body_path, &self.registration_path);
        let reading = self.build(&files, cause);
        let taken = self.take(reading);
        if taken {
            self.changes
                .push(SurfaceChange::Model(Box::new(self.reading.model.clone())));
        }
        taken
    }

    /// Reads both files again after another program changed one, and queues the model when they changed. A body
    /// that cannot be read as a requirement document is read again, up to [`READ_ATTEMPTS`] times, before the page is
    /// made read-only; one that cannot be read at all by then keeps the last page as it is.
    fn read_from_outside(&mut self) {
        let mut attempt = 1;
        let reading = loop {
            let files = documents::read(&self.body_path, &self.registration_path);
            let reading = self.build(&files, causes::OUTSIDE);
            if reading.as_ref().is_some_and(|r| !r.unreadable) || attempt == READ_ATTEMPTS {
                break reading;
            }
            attempt += 1;
            (self.wait)();
        };
        if self.take(reading) {
            self.changes
                .push(SurfaceChange::Model(Box::new(self.reading.model.clone())));
        }
    }

    /// A write of this session's has ended: the model as the files now are, as following `cause`, then what became
    /// of it.
    fn settle(&mut self, outcome: EditOutcome, cause: &str) {
        if !self.read_again(cause) {
            self.changes
                .push(SurfaceChange::Model(Box::new(self.reading.model.clone())));
        }
        self.changes.push(SurfaceChange::Outcome(outcome));
    }
}

impl RequirementSession {
    /// Takes a change of the body, of an entry's text or of the hierarchy: refused at once when the page cannot be
    /// changed, the entry is gone or a text cannot be written; otherwise made as one step, against the bytes the page
    /// was made from. A refused step is followed by the page read again: it was refused because the file is not what
    /// the page showed.
    fn body_gesture(&mut self, gesture: Gesture) -> Result<(), String> {
        if text::handles(&gesture.operation) {
            let attribute = text::attribute_of(&gesture.operation);
            let Some(value) = gesture.params.get(attribute) else {
                return Err(format!("'{}' needs its {attribute}.", gesture.operation));
            };
            let refusal = text::refusal(&gesture.operation, value);
            if !refusal.is_empty() {
                return Err(refusal.into());
            }
        }
        let model = &self.reading.model;
        if !model.read_only_reason.is_empty() {
            return Err(model.read_only_reason.clone());
        }
        let Some(expected) = self.reading.files.body.clone() else {
            return Err(gestures::DRIFTED.into());
        };
        let gone =
            gesture.operation != structure::ADD_TOP_ITEM && model.nodes.iter().all(|n| n.id != gesture.target_id);
        if gone {
            return Err(gestures::DRIFTED.into());
        }
        let command = EditCommand {
            body_path: self.body_path.clone(),
            operation: gesture.operation.clone(),
            target_id: gesture.target_id.clone(),
            params: gesture.params.clone(),
            expected,
        };
        let written = command.execute(&mut self.write);
        let (written, error) = match written {
            Ok(restore) => {
                if let Some(restore) = restore {
                    self.restores.push(restore);
                    self.undone.clear();
                }
                (true, String::new())
            }
            Err(error) => (false, error),
        };
        let cause = if written { causes::EDIT } else { causes::OUTSIDE };
        self.settle(
            EditOutcome {
                edit_id: gesture.edit_id,
                written,
                error,
            },
            cause,
        );
        Ok(())
    }

    /// Whether a step written through this session can be undone.
    pub fn can_undo(&self) -> bool {
        !self.restores.is_empty()
    }
}

impl HierarchySurfaceSession for RequirementSession {
    fn model(&self) -> SurfaceModel {
        self.reading.model.clone()
    }

    fn gesture(&mut self, gesture: Gesture) -> Result<(), String> {
        if text::handles(&gesture.operation) || structure::handles(&gesture.operation) {
            return self.body_gesture(gesture);
        }
        let setting = gestures::plan(&self.reading.model, &gesture)?;
        let known_ids = self
            .reading
            .model
            .nodes
            .iter()
            .filter(|node| node.id == node.number)
            .map(|node| node.id.clone())
            .collect();
        let written = documents::write_setting(
            &self.body_path,
            &self.registration_path,
            setting,
            known_ids,
            &mut self.write,
        );
        self.settle(
            EditOutcome {
                edit_id: gesture.edit_id,
                written: written.is_ok(),
                error: written.err().unwrap_or_default(),
            },
            causes::EDIT,
        );
        Ok(())
    }

    /// The keys that add an entry directly below the selected one, with no dialog (the standalone host's
    /// `HierarchicalRequirementContextActionProvider`): Ctrl+Enter an item below an item and a requirement below a
    /// requirement, Ctrl+Shift+Enter a requirement to an item. The labels and the unavailable message are the
    /// specification's.
    fn actions(&self, node_id: &str) -> Vec<SurfaceAction> {
        let Some(node) = self.reading.model.nodes.iter().find(|n| n.id == node_id) else {
            return Vec::new();
        };
        let action = |id: &str, label: &str, icon: &str, shift: bool, operation: &str| SurfaceAction {
            id: id.into(),
            label: label.into(),
            icon: icon.into(),
            key: "Enter".into(),
            shift,
            unavailable: String::new(),
            operation: operation.into(),
        };
        if node.role == roles::LEVEL {
            return vec![
                action(
                    ADD_BELOW,
                    "New item below",
                    "mdi-plus-box-outline",
                    false,
                    structure::ADD_ITEM_BELOW,
                ),
                action(
                    ADD_REQUIREMENT,
                    "New requirement",
                    "mdi-plus",
                    true,
                    structure::ADD_REQUIREMENT,
                ),
            ];
        }
        let mut below = action(
            ADD_BELOW,
            "New requirement below",
            "mdi-plus",
            false,
            structure::ADD_REQUIREMENT_BELOW,
        );
        if node.parent_id.is_empty() {
            below.unavailable = structure::NO_ITEM.into();
        }
        vec![below]
    }

    fn take_changes(&mut self) -> Vec<SurfaceChange> {
        std::mem::take(&mut self.changes)
    }

    fn undo(&mut self) -> Result<(), String> {
        let Some(restore) = self.restores.pop() else {
            return Err(NOTHING_TO_UNDO.into());
        };
        match restore.undo(&mut self.write) {
            Ok(()) => {
                self.undone.push(restore.redo);
                self.read_again(causes::EDIT);
                Ok(())
            }
            // The record is spent, and so is every one before it: they were made against a file that is gone.
            Err(Undone::Outdated(reason)) => {
                self.restores.clear();
                self.undone.clear();
                Err(reason)
            }
            Err(Undone::Failed(reason)) => {
                self.restores.push(restore);
                Err(reason)
            }
        }
    }

    fn redo(&mut self) -> Result<(), String> {
        let Some(command) = self.undone.pop() else {
            return Err(NOTHING_TO_REDO.into());
        };
        match command.execute(&mut self.write) {
            Ok(restore) => {
                self.restores.extend(restore);
                self.read_again(causes::EDIT);
                Ok(())
            }
            Err(reason) => {
                self.undone.clear();
                self.read_again(causes::OUTSIDE);
                Err(reason)
            }
        }
    }

    fn file_changed(&mut self, path: &Path) {
        if path == self.body_path || path == self.registration_path {
            self.read_from_outside();
        }
    }
}
