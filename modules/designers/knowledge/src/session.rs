//! One open knowledge file on one connection (the standalone host's `KnowledgeSession` at 8b6ccfc): the table as
//! that connection sees it - its own view and its own window of lines - kept true to the file on disk, and the
//! author's edits of it.
//!
//! - **The file is the truth.** Everything the table shows is in the file; the session holds the last reading, and
//!   when the file changes it reads it again and gives what the connection is looking at.
//! - **An edit is shown at once and written behind.** A gesture is turned into the changes it means for the file;
//!   those are made at once to what the table shows ([`crate::projection`]) and queued for the file, where
//!   [`KnowledgeSession::write_pending`] writes them one edit at a time, in the order they were made. When a write
//!   lands the session reads the file again and says the edit is written. When a write is refused the edit is taken
//!   back, and every later edit still waiting is taken back with it: each was made on top of the one that failed.
//!
//! The standalone host writes on a task and watches the file; here the core decides when [`write_pending`] runs and
//! tells the session of a change from outside with [`KnowledgeSession::reload`] and of a file written through ADP
//! with [`KnowledgeSession::written`], so the session needs no thread of its own.
//!
//! [`write_pending`]: KnowledgeSession::write_pending

use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_cel::{Map, Value};
use adp_desl::table::engine::{self, LineOptions, Unwritten};
use adp_desl::table::model::{
    TableBaseline, TableChange, TableFinding, TableFindingSeverity, TableGesture, TableRow, TableStructure,
};
use adp_desl::table::session::{DesignerSession, NO_WRITER, TableWindow};
use adp_fbl::{FblModel, ModelChange};

use crate::body::{ANOTHER_VERSION, KnowledgeBody};
use crate::command::{KnowledgeEditCommand, KnowledgeRestore};
use crate::definition;
use crate::diagrams::{self, KnowledgeDiagrams};
use crate::edits::{self, KnowledgeOtherFile, KnowledgeSurroundings};
use crate::links::{self, KnowledgeFile};
use crate::mapper;
use crate::model::{self, KnowledgeProperty, KnowledgeTable};
use crate::project;
use crate::projection;
use crate::relations::{self, KnowledgeTarget};
use crate::store::{self, KnowledgeRead};
use crate::validator;

/// The pending edit that holds what loading the file changes, until the next edit takes it along.
const ON_LOAD: &str = "on-load";

/// How often a re-read is tried before the file is taken to be unreadable rather than mid-replace.
pub const READ_ATTEMPTS: usize = 3;

/// How a session reads a knowledge file.
pub type Reader = Box<dyn Fn(&Path) -> KnowledgeRead>;

/// How a session writes an edit: what puts it back, when it changed a byte, or why it was not written.
pub type Writer = Box<dyn FnMut(&KnowledgeEditCommand) -> Result<Option<KnowledgeRestore>, String>>;

/// An edit that is shown and not written yet.
struct PendingEdit {
    edit_id: String,
    changes: Vec<ModelChange>,
    /// The cell it sets, which is marked as not written yet; none for any other edit.
    cell: Option<(String, String)>,
}

/// A write still to come.
struct Queued {
    edit_id: String,
    generation: u64,
    command: KnowledgeEditCommand,
}

/// One open knowledge file on one connection.
pub struct KnowledgeSession {
    body_path: PathBuf,
    root_path: PathBuf,
    beside_its_rule: bool,
    read: Reader,
    write: Option<Writer>,
    pending: Vec<PendingEdit>,
    queue: VecDeque<Queued>,
    /// Raised by one when the pending edits are taken back, so the writes already queued for them do nothing.
    generation: u64,
    body: Option<KnowledgeBody>,
    /// What the table shows: the body's own model and table, or those with the pending edits made to them.
    shown_model: Rc<FblModel>,
    shown: KnowledgeTable,
    /// What is shown with the computed sides of its relations filled in: what the view is made of.
    view: KnowledgeTable,
    /// The tables this file's relations point at, by their paths, as they were last read.
    targets: RefCell<HashMap<String, Rc<KnowledgeTarget>>>,
    /// The file itself as a target, made once for what is shown now.
    itself: RefCell<Option<Rc<KnowledgeTarget>>>,
    /// The knowledge files of the workspace as they were last read, until one of them is written again.
    files: RefCell<Option<Rc<Vec<KnowledgeFile>>>>,
    /// The index the links of what is shown are resolved in (FBL section 17.3): the files, with this one as shown.
    workspace: adp_links::Workspace,
    /// The targets named by links, resolved once for what is shown now.
    linked: RefCell<HashMap<String, Rc<KnowledgeTarget>>>,
    view_id: String,
    /// The rows added while this view was open: they stay in sight until it is next opened.
    kept: HashSet<String>,
    /// What this connection searches the rows for; empty for all of them. It is in no file.
    search: String,
    window: TableWindow,
    /// Why the file could not be read from disk the last time it was tried, or empty.
    refusal: String,
    /// What the edits written through this session put back, the latest last.
    restores: Vec<KnowledgeRestore>,
    /// What the edits undone through this session make again, the latest last.
    undone: Vec<KnowledgeRestore>,
    changes: Vec<TableChange>,
    /// Where the ids of what an edit makes come from: a fresh ShortGuid each, unless a replay hands them out.
    new_id: Box<dyn FnMut() -> String>,
    /// What the host knows of the workspace's diagrams, which a relation may point at (etalii.adp spec 092, FR-025);
    /// none where relations point at knowledge files only.
    diagrams: Option<Rc<dyn KnowledgeDiagrams>>,
}

impl KnowledgeSession {
    /// A session that only shows the file.
    pub fn new(body_path: impl Into<PathBuf>) -> Self {
        KnowledgeSession::with(
            body_path,
            Box::new(|path: &Path| store::read(path)),
            None,
            PathBuf::new(),
        )
    }

    /// A session whose edits are written to the file, and which looks under `root` for the files that relate to it.
    pub fn open_for_editing(body_path: impl Into<PathBuf>, root: impl Into<PathBuf>) -> Self {
        KnowledgeSession::with(
            body_path,
            Box::new(|path: &Path| store::read(path)),
            Some(Box::new(|command: &KnowledgeEditCommand| command.execute())),
            root.into(),
        )
    }

    /// A session whose edits are written to the file, which looks under `root` for the files that relate to it, and
    /// whose relations may point at the diagrams `diagrams` knows of (etalii.adp spec 092, FR-025).
    pub fn open_for_editing_with(
        body_path: impl Into<PathBuf>,
        root: impl Into<PathBuf>,
        diagrams: Rc<dyn KnowledgeDiagrams>,
    ) -> Self {
        KnowledgeSession::with_diagrams(
            body_path,
            Box::new(|path: &Path| store::read(path)),
            Some(Box::new(|command: &KnowledgeEditCommand| command.execute())),
            root.into(),
            Some(diagrams),
        )
    }

    /// A session with its reading and writing given: a test refuses a write exactly when it chooses.
    pub fn with(body_path: impl Into<PathBuf>, read: Reader, write: Option<Writer>, root_path: PathBuf) -> Self {
        KnowledgeSession::with_diagrams(body_path, read, write, root_path, None)
    }

    /// [`KnowledgeSession::with`], with the diagrams its relations may point at known before the file is loaded, so
    /// that the links to them resolve as it is loaded.
    pub fn with_diagrams(
        body_path: impl Into<PathBuf>,
        read: Reader,
        write: Option<Writer>,
        root_path: PathBuf,
        diagrams: Option<Rc<dyn KnowledgeDiagrams>>,
    ) -> Self {
        let body_path = body_path.into();
        let extension = body_path
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();
        let opened = read(&body_path);
        let mut session = KnowledgeSession {
            beside_its_rule: definition::adds_beside_its_rule(&extension),
            body_path,
            root_path,
            read,
            write,
            pending: Vec::new(),
            queue: VecDeque::new(),
            generation: 0,
            body: None,
            shown_model: Rc::default(),
            shown: KnowledgeTable::default(),
            view: KnowledgeTable::default(),
            targets: RefCell::new(HashMap::new()),
            itself: RefCell::new(None),
            files: RefCell::new(None),
            workspace: adp_links::Workspace::default(),
            linked: RefCell::new(HashMap::new()),
            view_id: String::new(),
            kept: HashSet::new(),
            search: String::new(),
            window: TableWindow::default(),
            refusal: opened.refusal,
            restores: Vec::new(),
            undone: Vec::new(),
            changes: Vec::new(),
            new_id: Box::new(definition::new_id),
            diagrams,
        };
        session.show(opened.body);
        // The view the file was left in, by name from here on: another connection leaving the file in another view
        // does not move this one.
        session.view_id = session
            .shown
            .view_or_default("")
            .map(|v| v.id.clone())
            .unwrap_or_default();
        session.validate_on_load();
        session
    }

    /// What loading a file for editing changes in it (FBL section 17.4; `knowledge.md` section 9): its document id,
    /// the conversion of a file made before links, and the links refreshed. Shown at once and written with the next
    /// edit of the file, in the same write: the first edit made in ADP also gives every entry without an id its id,
    /// which a write of its own would give under other ids than the next edit was planned with. Nothing when nothing
    /// differs, and nothing for a file opened only to be shown: opening writes nothing.
    fn validate_on_load(&mut self) {
        if self.write.is_none() || self.body.is_none() || !self.read_only_reason().is_empty() {
            return;
        }
        let changes = {
            let target_of = |p: &KnowledgeProperty| self.target_of(p);
            links::load_changes(
                &self.shown_model,
                &self.shown,
                &self.body_path,
                &self.workspace,
                &target_of,
                &mut definition::new_id,
            )
        };
        if !changes.is_empty() {
            self.project(&changes);
            self.pending.push(PendingEdit {
                edit_id: ON_LOAD.into(),
                changes,
                cell: None,
            });
        }
    }

    /// The folder whose knowledge files a link of this file is resolved among: the root it was opened under, or its
    /// own folder.
    fn folder(&self) -> PathBuf {
        if self.root_path.as_os_str().is_empty() {
            relations::full_path(&self.body_path)
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_default()
        } else {
            self.root_path.clone()
        }
    }

    /// The knowledge files of the workspace, read once and kept until one of them is written again.
    fn files(&self) -> Rc<Vec<KnowledgeFile>> {
        if let Some(files) = self.files.borrow().as_ref() {
            return files.clone();
        }
        let folder = self.folder();
        let mut read = links::read_files(&folder, &*self.read);
        if let Some(diagrams) = &self.diagrams {
            read.extend(diagrams.diagrams(&folder).into_iter().map(|diagram| KnowledgeFile {
                path: relations::full_path(&diagram.path),
                table: diagrams::table_of(&diagram),
                diagram: Some(Rc::new(diagram)),
            }));
        }
        let files = Rc::new(read);
        self.files.replace(Some(files.clone()));
        files
    }

    /// Hands out the ids of what later edits make - a row, a view, an option - from `ids` rather than as fresh
    /// ShortGuids: the parity check gives each the id the standalone host generated at the same step.
    pub fn set_id_source(&mut self, ids: Box<dyn FnMut() -> String>) {
        self.new_id = ids;
    }

    /// The file this session shows.
    pub fn body_path(&self) -> &Path {
        &self.body_path
    }

    /// How many edits are shown and not written yet; what loading the file changed is not one of them until an edit
    /// takes it along.
    pub fn pending_edits(&self) -> usize {
        self.pending.iter().filter(|p| p.edit_id != ON_LOAD).count()
    }

    /// Leaves the file in the form it was read in: what loading it changed is no longer shown and goes with no edit.
    /// For a replay of edits recorded by a host that predates links (etalii.adp spec 089), whose bytes are compared
    /// with that host's; a file a user opens is always validated on load (FBL section 17.4).
    pub fn discard_load_changes(&mut self) {
        let before = self.pending.len();
        self.pending.retain(|p| p.edit_id != ON_LOAD);
        if self.pending.len() != before {
            let body = self.body.take();
            self.show(body);
        }
    }

    /// What loading the file changed and is shown, not yet written: none when nothing differed (FBL section 17.4).
    pub fn load_changes(&self) -> Option<&[ModelChange]> {
        self.pending
            .iter()
            .find(|p| p.edit_id == ON_LOAD)
            .map(|p| p.changes.as_slice())
    }

    /// Why the table cannot be edited, or empty when it can: the reason of a file of another version, or of one that
    /// cannot be read.
    pub fn read_only(&self) -> String {
        self.read_only_reason()
    }

    /// Whether an edit undone through this session can be made again.
    pub fn can_redo(&self) -> bool {
        !self.undone.is_empty()
    }

    /// Why the file could not be read from disk the last time it was tried, or empty.
    pub fn refusal(&self) -> &str {
        &self.refusal
    }

    /// The table as it is shown now, edits not yet written included.
    pub fn shown(&self) -> &KnowledgeTable {
        &self.shown
    }

    /// The FBL model the table shown now is read from, edits not yet written included: its elements, as `atd record`
    /// states a reading (etalii.adp spec 076, T067).
    pub fn model(&self) -> &FblModel {
        &self.shown_model
    }

    /// The view this connection shows.
    pub fn active_view_id(&self) -> &str {
        &self.view_id
    }

    /// Whether edits are waiting to be written.
    pub fn has_pending_writes(&self) -> bool {
        !self.queue.is_empty()
    }

    /// Writes the edits that wait, one at a time, in the order they were made; after each, the file is read again
    /// and the connection told what became of the edit. A write that is refused takes back every edit still waiting.
    pub fn write_pending(&mut self) {
        while let Some(queued) = self.queue.pop_front() {
            if queued.generation != self.generation {
                // Taken back with an earlier edit that was refused: there is nothing to write.
                continue;
            }
            let result = match self.write.as_mut() {
                Some(write) => write(&queued.command),
                None => Err(NO_WRITER.into()),
            };
            self.settle(&queued.edit_id, result);
        }
    }

    /// What became of one edit's write: the file is read again, and the connection is told.
    fn settle(&mut self, edit_id: &str, result: Result<Option<KnowledgeRestore>, String>) {
        let read = self.read_retried();
        self.pending.retain(|p| p.edit_id != edit_id);
        let mut taken_back = Vec::new();
        let error = match result {
            Ok(restore) => {
                if restore.is_some() {
                    self.undone.clear();
                }
                self.restores.extend(restore);
                String::new()
            }
            Err(error) => {
                // Every edit still waiting was made on top of this one, so it goes with it.
                taken_back = self.pending.drain(..).map(|p| p.edit_id).collect();
                self.generation += 1;
                error
            }
        };
        match read.body {
            Some(body) => {
                self.refusal.clear();
                self.show(Some(body));
            }
            None => {
                self.refusal = read.refusal;
                let body = self.body.take();
                self.show(body);
            }
        }
        // A relation's target this step wrote is read again.
        self.targets.borrow_mut().clear();
        self.files.replace(None);
        self.relate();
        self.changes.push(TableChange::EditSettled {
            edit_id: edit_id.to_owned(),
            written: error.is_empty(),
            error,
            taken_back,
        });
        let everything = self.everything();
        self.changes.extend(everything);
    }

    /// Whether an edit written through this session can be undone.
    pub fn can_undo(&self) -> bool {
        !self.restores.is_empty()
    }

    /// Undoes the latest edit written through this session: every file it wrote is put back, or none.
    pub fn undo(&mut self) -> Result<(), String> {
        let Some(restore) = self.restores.pop() else {
            return Err("There is nothing to undo.".into());
        };
        if let Err(refusal) = restore.undo() {
            self.restores.push(restore);
            return Err(refusal);
        }
        self.undone.push(restore);
        // A diagram the step wrote is read again: an element its *Create* added is gone.
        self.files.replace(None);
        self.reload();
        Ok(())
    }

    /// Makes the latest undone edit again.
    pub fn redo(&mut self) -> Result<(), String> {
        let Some(restore) = self.undone.pop() else {
            return Err("There is nothing to redo.".into());
        };
        if let Err(refusal) = restore.redo() {
            self.undone.push(restore);
            return Err(refusal);
        }
        self.restores.push(restore);
        self.files.replace(None);
        self.reload();
        Ok(())
    }

    /// Ends the session once the edits still waiting have been written: closing a table does not lose what its
    /// author just typed.
    pub fn close(mut self) {
        self.write_pending();
    }

    /// The file changed outside this session: it is read again, and what the connection is looking at given anew.
    /// The same bytes are the same table: a write that changed nothing gives nothing.
    pub fn reload(&mut self) {
        let read = self.read_retried();
        let Some(body) = read.body else {
            // The last good reading stays, and the session stops claiming the file opens.
            self.refusal = read.refusal;
            return;
        };
        if self.refusal.is_empty() && self.body.as_ref().is_some_and(|b| b.bytes() == body.bytes()) {
            return;
        }
        self.refusal.clear();
        self.show(Some(body));
        let everything = self.everything();
        self.changes.extend(everything);
    }

    /// The file read until it opens, at most [`READ_ATTEMPTS`] times: a publish that replaces the file through a
    /// temporary one presents the name as absent for a moment, and a single refused read would throw the only
    /// notification of that write away (the standalone host's `WatchedFile.ReadAsync`, which also waits between
    /// attempts; the core that calls this decides when to).
    fn read_retried(&self) -> KnowledgeRead {
        let mut read = (self.read)(&self.body_path);
        for _ in 1..READ_ATTEMPTS {
            if read.body.is_some() {
                break;
            }
            read = (self.read)(&self.body_path);
        }
        read
    }

    /// A file was written through ADP: this one is read again, and a table one of its relations points at is read
    /// again and shown.
    pub fn written(&mut self, path: &Path) {
        if relations::same_path(path, &self.body_path) {
            self.reload();
            return;
        }
        let key = relations::full_path(path).to_string_lossy().to_lowercase();
        let targeted = self.targets.borrow_mut().remove(&key).is_some();
        // Any knowledge file of the workspace may hold links to this one, or be what one of its links names.
        let in_workspace = relations::full_path(path).starts_with(self.folder());
        if in_workspace {
            self.files.replace(None);
        }
        if !(targeted || in_workspace) || self.body.is_none() {
            return;
        }
        self.relate();
        let everything = self.everything();
        self.changes.extend(everything);
    }

    /// Takes a reading as what the table shows, with the edits still waiting made to it again.
    fn show(&mut self, body: Option<KnowledgeBody>) {
        self.shown_model = body.as_ref().map(KnowledgeBody::shared_model).unwrap_or_default();
        self.shown = body.as_ref().map(|b| b.table().clone()).unwrap_or_default();
        self.body = body;
        let pending: Vec<Vec<ModelChange>> = self.pending.iter().map(|p| p.changes.clone()).collect();
        for changes in &pending {
            self.shown_model = Rc::new(projection::apply(&self.shown_model, changes, self.beside_its_rule));
        }
        if !pending.is_empty() {
            self.shown = model::read(&self.shown_model);
        }
        self.relate();
    }

    fn project(&mut self, changes: &[ModelChange]) {
        self.shown_model = Rc::new(projection::apply(&self.shown_model, changes, self.beside_its_rule));
        self.shown = model::read(&self.shown_model);
        self.relate();
    }

    /// Fills in the computed sides of the relations of what is shown, whenever what is shown changed.
    fn relate(&mut self) {
        self.itself.replace(None);
        self.linked.borrow_mut().clear();
        let files = self.files();
        self.workspace = links::workspace(&files, &self.body_path, &self.shown);
        let view = relations::with_computed(self.shown.clone(), &|p| self.target_of(p));
        self.view = links::with_backlinks(view, &self.body_path, &self.workspace, &files);
    }

    /// What a relation property of this file points at, read once and kept until the target is written again.
    fn target_of(&self, property: &KnowledgeProperty) -> Option<Rc<KnowledgeTarget>> {
        (property.value_type == "relation").then(|| self.target_at(&property.target_file))
    }

    /// What a target named from this file is: a relative path, or `.` for the file itself.
    fn target_at(&self, target_file: &str) -> Rc<KnowledgeTarget> {
        if links::is_link(target_file) {
            if let Some(target) = self.linked.borrow().get(target_file) {
                return target.clone();
            }
            let target = Rc::new(links::target_of_link(
                &self.workspace,
                &self.files(),
                &self.body_path,
                &self.shown,
                target_file,
            ));
            self.linked.borrow_mut().insert(target_file.to_owned(), target.clone());
            return target;
        }
        let path = relations::target_path(&self.body_path, target_file);
        if relations::same_path(&path, &self.body_path) {
            if let Some(itself) = self.itself.borrow().as_ref() {
                return itself.clone();
            }
            let itself = Rc::new(KnowledgeTarget {
                path,
                table: Some(self.shown.clone()),
                ..KnowledgeTarget::default()
            });
            self.itself.replace(Some(itself.clone()));
            return itself;
        }
        let key = path.to_string_lossy().to_lowercase();
        if let Some(target) = self.targets.borrow().get(&key) {
            return target.clone();
        }
        // A diagram chosen as a new relation's target is named by its path until the relation names it by a link.
        let full = relations::full_path(&path);
        if let Some(file) = self.files().iter().find(|f| f.diagram.is_some() && f.path == full) {
            return Rc::new(KnowledgeTarget {
                path: file.path.clone(),
                table: Some(file.table.clone()),
                diagram: file.diagram.clone(),
                ..KnowledgeTarget::default()
            });
        }
        let read = (self.read)(&path);
        let target = Rc::new(match read.body {
            None => KnowledgeTarget {
                path,
                table: None,
                problem: format!("'{target_file}' is not there."),
                ..KnowledgeTarget::default()
            },
            Some(body) if !body.unreadable().is_empty() => KnowledgeTarget {
                path,
                table: None,
                problem: format!("'{target_file}' cannot be read as a knowledge file."),
                ..KnowledgeTarget::default()
            },
            Some(body) => KnowledgeTarget {
                path,
                table: Some(body.table().clone()),
                ..KnowledgeTarget::default()
            },
        });
        self.targets.borrow_mut().insert(key, target.clone());
        target
    }

    /// The files a new relation can point at, as the answer to a relation asked for without one: this table, the
    /// project's other knowledge files and, where the host knows of them, its diagrams (etalii.adp spec 092, FR-025).
    fn targets(&self) -> Vec<(String, String)> {
        let mut targets = project::targets(&self.root_path, &self.body_path);
        if self.root_path.as_os_str().is_empty() {
            return targets;
        }
        let mut diagrams: Vec<(String, String)> = self
            .files()
            .iter()
            .filter(|f| f.diagram.is_some())
            .map(|f| (relations::target_name(&self.body_path, &f.path), f.table.name.clone()))
            .collect();
        diagrams.sort_by_key(|(name, _)| name.to_lowercase());
        targets.extend(diagrams);
        targets
    }

    /// *Create* on a relation to a diagram (etalii.adp spec 092, FR-025): the element named in `gesture` added to the
    /// diagram through its own tool, offered at once, and the `setCell` that relates it with the diagram's new body to
    /// write in the same step; or why not.
    fn create_element(&mut self, gesture: &TableGesture) -> Result<(TableGesture, KnowledgeOtherFile), String> {
        let property = self
            .shown
            .column(&gesture.column_id)
            .filter(|p| p.value_type == "relation" && !p.is_computed)
            .ok_or(diagrams::NO_DIAGRAM)?;
        let one = property.limit == "one";
        let target = self.target_of(property).ok_or(diagrams::NO_DIAGRAM)?;
        let diagram = target.diagram.clone().ok_or(diagrams::NO_DIAGRAM)?;
        let host = self.diagrams.clone().ok_or(diagrams::NO_DIAGRAM)?;
        let asked = gesture.setting_of(diagrams::ELEMENT_KIND);
        let kind = match diagram.kinds.as_slice() {
            [] => return Err(diagrams::NOTHING_TO_CREATE.into()),
            [only] if asked.is_empty() => only.type_name.clone(),
            kinds => kinds
                .iter()
                .find(|k| k.type_name == asked)
                .map(|k| k.type_name.clone())
                .ok_or(diagrams::PICK_A_KIND)?,
        };
        let name = gesture.setting_of(diagrams::NEW_ELEMENT).trim().to_owned();
        let created = host.create(&target.path, &kind, &name)?;
        // The new element is offered and its link written before the diagram is read again.
        if let Some(files) = self.files.borrow_mut().as_mut() {
            let mut grown: Vec<KnowledgeFile> = files.as_ref().clone();
            if let Some(file) = grown.iter_mut().find(|f| f.path == target.path) {
                file.table = diagrams::with_row(file.table.clone(), &created.id, &name);
            }
            *files = Rc::new(grown);
        }
        self.relate();
        let mut relating = gesture.clone();
        relating.settings.shift_remove(diagrams::NEW_ELEMENT);
        relating.settings.shift_remove(diagrams::ELEMENT_KIND);
        relating.values = if one {
            vec![created.id.clone()]
        } else {
            gesture.values.iter().cloned().chain([created.id.clone()]).collect()
        };
        Ok((
            relating,
            KnowledgeOtherFile {
                body_path: target.path.clone(),
                changes: Vec::new(),
                body: Some(created.body),
            },
        ))
    }

    /// What a related row is called, for the cells and the groups of a relation.
    fn label_of(&self, column_id: &str, row_id: &str) -> String {
        match self.view.column(column_id) {
            Some(property) => relations::label_of(self.target_of(property).as_deref(), row_id),
            None => String::new(),
        }
    }

    fn read_only_reason(&self) -> String {
        self.body
            .as_ref()
            .map(KnowledgeBody::read_only_reason)
            .unwrap_or_default()
    }

    /// The lines of the view this connection shows, in order.
    fn lines(&self) -> Vec<TableRow> {
        let unwritten: Unwritten = self.pending.iter().filter_map(|p| p.cell.clone()).collect();
        let label_of = |c: &str, r: &str| self.label_of(c, r);
        let editable = self.body.is_some() && self.read_only_reason().is_empty();
        engine::lines(
            &self.view,
            self.view.view_or_default(&self.view_id),
            &LineOptions {
                kept: Some(&self.kept),
                unwritten: Some(&unwritten),
                editable,
                label_of: Some(&label_of),
                search: &self.search,
            },
        )
    }

    fn structure(&self) -> TableStructure {
        let label_of = |c: &str, r: &str| self.label_of(c, r);
        let target_of = |p: &KnowledgeProperty| self.target_of(p);
        let options = LineOptions {
            kept: Some(&self.kept),
            label_of: Some(&label_of),
            search: &self.search,
            ..LineOptions::default()
        };
        let in_sight = engine::in_sight(&self.view, self.view.view_or_default(&self.view_id), &options);
        mapper::structure(
            &self.view,
            &self.read_only_reason(),
            &self.view_id,
            self.lines().len(),
            Some(&target_of),
            Some(&in_sight),
            Some(&label_of),
            &self.search,
        )
    }

    /// What the table has to report: what the reading of the file found, and what the designer's own rules find in
    /// the table as it is shown - so an edit that puts something right is seen to at once.
    fn findings(&self) -> Vec<TableFinding> {
        let Some(body) = &self.body else {
            return Vec::new();
        };
        // A file of a version this application does not know is one finding of its own, not the reader's complaint
        // about its first line.
        if body.unreadable() == ANOTHER_VERSION {
            return vec![TableFinding::new(
                validator::NEWER_VERSION,
                TableFindingSeverity::Warning,
                ANOTHER_VERSION,
            )];
        }
        let mut findings = mapper::findings(body.model());
        if body.unreadable().is_empty() {
            let target_of = |p: &KnowledgeProperty| self.target_of(p);
            findings.extend(validator::validate(&self.view, &self.shown_model, Some(&target_of)));
        }
        findings
    }

    fn rows_of_window(&self) -> Option<TableChange> {
        if self.body.is_none() || !self.window.is_set() {
            return None;
        }
        self.window.rows_of(&self.lines())
    }

    /// The structure, the findings and the window's lines: what a connection is given when the table changed.
    fn everything(&self) -> Vec<TableChange> {
        let mut changes = vec![TableChange::Structure(Box::new(self.structure()))];
        if self.body.is_some() {
            changes.push(TableChange::Findings(self.findings()));
        }
        changes.extend(self.rows_of_window());
        changes
    }

    /// The related rows an edit adds, written as links to them (FBL section 17.1) where their target has a document
    /// id; the edits themselves name a row by its id, as what is shown does.
    fn write_rows_as_links(&self, changes: &mut [ModelChange]) {
        for change in changes {
            let ModelChange::Add {
                type_name,
                attributes,
                parent_id: Some(cell),
                ..
            } = change
            else {
                continue;
            };
            if type_name != "CellItem" {
                continue;
            }
            let Some(Value::String(row)) = attributes.get("row").cloned() else {
                continue;
            };
            let property_id = cell.split_once('/').map(|(_, p)| p).unwrap_or_default();
            let Some(property) = self.shown.column(property_id) else {
                continue;
            };
            let target = self.target_of(property);
            let text = links::row_text(target.as_deref(), &row);
            if text != row {
                attributes.insert("row", Value::String(text));
            }
        }
    }

    /// Takes an edit as made: shown from now on, and queued to be written.
    fn accept(
        &mut self,
        edit_id: &str,
        changes: Vec<ModelChange>,
        cell: Option<(String, String)>,
        others: Vec<KnowledgeOtherFile>,
    ) {
        self.project(&changes);
        // What loading the file changed goes with the first edit, in its write and its step.
        let changes = match self.pending.iter().position(|p| p.edit_id == ON_LOAD) {
            Some(at) => {
                let mut all = self.pending.remove(at).changes;
                all.extend(changes);
                all
            }
            None => changes,
        };
        self.queue.push_back(Queued {
            edit_id: edit_id.to_owned(),
            generation: self.generation,
            command: KnowledgeEditCommand {
                body_path: self.body_path.clone(),
                changes: changes.clone(),
                others,
            },
        });
        self.pending.push(PendingEdit {
            edit_id: edit_id.to_owned(),
            changes,
            cell,
        });
    }
}

impl DesignerSession for KnowledgeSession {
    fn baseline(&self) -> TableBaseline {
        if self.body.is_none() {
            // Nothing was ever read: the table is its refusal, and nothing else.
            return TableBaseline {
                read_only_reason: if self.refusal.is_empty() {
                    "The file cannot be read.".into()
                } else {
                    self.refusal.clone()
                },
                ..TableBaseline::default()
            };
        }
        let structure = self.structure();
        TableBaseline {
            title: structure.title,
            columns: structure.columns,
            views: structure.views,
            settings: structure.settings,
            row_count: structure.row_count,
            findings: self.findings(),
            read_only_reason: structure.read_only_reason,
        }
    }

    fn set_window(&mut self, first: usize, count: usize) {
        self.window.set(first, count);
        if let Some(rows) = self.rows_of_window() {
            self.changes.push(rows);
        }
    }

    fn set_active_view(&mut self, view_id: &str) {
        if self.body.is_none() || self.shown.view(view_id).is_none() {
            // A view that is not there - deleted while this call was on its way - changes nothing.
            return;
        }
        self.view_id = view_id.to_owned();
        self.kept.clear();
        // The view a file is left in is in the file: switching is an edit, written and undone as any other.
        if self.write.is_some() && self.read_only_reason().is_empty() && self.shown.active_view_id != view_id {
            let attributes: Map = [("activeView", Value::String(view_id.to_owned()))]
                .into_iter()
                .collect();
            self.accept(
                &definition::new_id(),
                vec![ModelChange::Set {
                    id: edits::TABLE_ID.into(),
                    attributes,
                }],
                None,
                Vec::new(),
            );
        }
        self.changes.push(TableChange::Structure(Box::new(self.structure())));
        if let Some(rows) = self.rows_of_window() {
            self.changes.push(rows);
        }
    }

    fn edit(&mut self, edit_id: &str, gesture: &TableGesture) -> String {
        if self.body.is_none() {
            return if self.refusal.is_empty() {
                "The file cannot be read.".into()
            } else {
                self.refusal.clone()
            };
        }
        // A search is this connection's own and changes no file: a table that cannot be edited is searched all the
        // same.
        if gesture.kind == "search" {
            self.search = gesture.values.first().map(|v| v.trim().to_owned()).unwrap_or_default();
            self.changes.push(TableChange::EditSettled {
                edit_id: edit_id.to_owned(),
                written: true,
                error: String::new(),
                taken_back: Vec::new(),
            });
            let everything = self.everything();
            self.changes.extend(everything);
            return String::new();
        }
        // A file that cannot be read, or is of another version, refuses every edit and is never written.
        let reason = self.read_only_reason();
        if !reason.is_empty() {
            return reason;
        }
        if self.write.is_none() {
            return NO_WRITER.into();
        }
        // Rows of other tables may relate to a row that is about to go: its author is told how many first.
        if gesture.kind == "deleteRows"
            && !self.root_path.as_os_str().is_empty()
            && gesture.setting_of("confirmed") != "true"
        {
            let ids: Vec<String> = if gesture.values.is_empty() {
                vec![gesture.row_id.clone()]
            } else {
                gesture.values.clone()
            };
            let relating = project::rows_relating_to(&self.root_path, &self.body_path, &self.shown.document_id, &ids);
            if relating > 0 {
                return edits::ask_first(relating, ids.len());
            }
        }
        // A backlink column is computed from another relation's links: it is neither filled in nor changed here.
        if links::is_backlink_column(&gesture.column_id)
            && matches!(
                gesture.kind.as_str(),
                "setCell" | "setCells" | "deleteColumn" | "renameColumn" | "setColumnType" | "duplicateColumn"
            )
        {
            return if gesture.kind.starts_with("setCell") {
                edits::COMPUTED.into()
            } else {
                edits::BACKLINK_COLUMN.into()
            };
        }
        // A relation asked for without the table it relates to is answered with the tables there are to choose from.
        if gesture.kind == "addRelation" && gesture.setting_of("target").trim().is_empty() {
            return edits::choose_target(&self.targets());
        }
        // *Create* on a relation to a diagram adds the element to the diagram first, and relates it in the same step.
        let mut created = None;
        let made;
        let gesture = if gesture.kind == "setCell" && !gesture.setting_of(diagrams::NEW_ELEMENT).trim().is_empty() {
            match self.create_element(gesture) {
                Ok((relating, other)) => {
                    created = Some(other);
                    made = relating;
                    &made
                }
                Err(reason) => return reason,
            }
        } else {
            gesture
        };
        // The id source is taken out while the closures below borrow the session, and put back after.
        let mut ids = std::mem::replace(&mut self.new_id, Box::new(definition::new_id));
        let edit = {
            let target_of = |p: &KnowledgeProperty| self.target_of(p);
            let target_at = |name: &str| self.target_at(name);
            let files = KnowledgeSurroundings {
                body_path: &self.body_path,
                target_of: &target_of,
                target_at: &target_at,
            };
            edits::plan(
                &self.shown,
                &self.view_id,
                gesture,
                &mut *ids,
                self.beside_its_rule,
                Some(&files),
            )
        };
        self.new_id = ids;
        if edit.is_refused() {
            if created.is_some() {
                // The element offered at once was never written: the diagram is read again as it is.
                self.files.replace(None);
                self.relate();
            }
            return edit.refusal;
        }
        let mut edit = edit;
        edit.others.extend(created);
        self.write_rows_as_links(&mut edit.changes);
        if edit.changes.is_empty() {
            // Nothing to write: the edit is settled as it is made.
            self.changes.push(TableChange::EditSettled {
                edit_id: edit_id.to_owned(),
                written: true,
                error: String::new(),
                taken_back: Vec::new(),
            });
            return String::new();
        }
        if !edit.new_view_id.is_empty() {
            // A view just made is the view its maker looks at.
            self.view_id = edit.new_view_id.clone();
            self.kept.clear();
        }
        if !edit.new_row_id.is_empty() {
            self.kept.insert(edit.new_row_id.clone());
        }
        let cell = (gesture.kind == "setCell").then(|| (gesture.row_id.clone(), gesture.column_id.clone()));
        self.accept(edit_id, edit.changes, cell, edit.others);
        let everything = self.everything();
        self.changes.extend(everything);
        String::new()
    }

    fn take_changes(&mut self) -> Vec<TableChange> {
        std::mem::take(&mut self.changes)
    }
}
