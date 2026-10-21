//! What the ported tests share (the standalone host's `KnowledgeFiles` and `EditingTable` at 8b6ccfc): the module's
//! example documents, copies of them to change, a table as text, and one knowledge file open for editing as the host
//! opens it.
//!
//! `tests/examples/` holds the standalone host's `src/designers/knowledge/examples/` at 8b6ccfc byte for byte - the
//! cities in three formats, with the data its maintainer added, and `provinces.yaml`, the table their relation names -
//! because the ported expectations are about those files. The examples vendored from etalii.adp are read by
//! `vendored_examples.rs`.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use adp_designer_knowledge::{KnowledgeBody, KnowledgeSession, KnowledgeTable, store};
use adp_desl::table::content::{FilterGroup, FilterItem};
use adp_desl::table::model::{TableChange, TableGesture, TableRow};
use adp_desl::table::session::DesignerSession;

pub const EXTENSIONS: [&str; 3] = [".yaml", ".json", ".xml"];

/// The table the example's relation points at.
pub const RELATED: &str = "provinces.yaml";

pub fn examples_folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/examples")
}

pub fn vendor() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor/etalii.adp")
}

pub fn example_bytes(name: &str) -> Vec<u8> {
    std::fs::read(examples_folder().join(name)).expect("the example is there")
}

pub fn read_example(name: &str) -> KnowledgeBody {
    KnowledgeBody::read(example_bytes(name), name)
}

/// A copy of a shipped example in `folder`, under its own name, with the table its relation names beside it.
pub fn copy_example(name: &str, folder: &Path) -> PathBuf {
    let path = folder.join(name);
    std::fs::copy(examples_folder().join(name), &path).expect("the example is copied");
    let related = folder.join(RELATED);
    if name != RELATED && !related.exists() {
        std::fs::copy(examples_folder().join(RELATED), &related).expect("the related table is copied");
    }
    path
}

/// A copy of a shipped example left in `view` whichever view its file names.
fn copy_in_view(name: &str, folder: &Path, view: &str) -> PathBuf {
    let path = copy_example(name, folder);
    let text = std::fs::read_to_string(&path).expect("UTF-8");
    let active = regex::Regex::new(r#"(activeView"?[:=] ?"?)v[0-9]+"#).expect("the pattern compiles");
    // The pattern has to have found the one place the file names its view, or the copy is left as it was.
    assert_eq!(1, active.find_iter(&text).count());
    std::fs::write(&path, active.replace(&text, format!("${{1}}{view}")).as_bytes()).expect("written");
    path
}

/// A copy left in its second view, which filters, sorts and groups nothing: every row, in the file's order.
pub fn copy_plain(name: &str, folder: &Path) -> PathBuf {
    copy_in_view(name, folder, "v2")
}

/// A copy left in its first view, which filters, sorts and groups its rows.
pub fn copy_grouped(name: &str, folder: &Path) -> PathBuf {
    copy_in_view(name, folder, "v1")
}

fn describe_filter(item: &FilterItem) -> String {
    match item {
        FilterItem::Group(group) => describe_group(group),
        FilterItem::Condition(c) => format!("{} {} '{}'", c.column_id, c.operator, c.value),
    }
}

fn describe_group(group: &FilterGroup) -> String {
    let items: Vec<String> = group.items.iter().map(describe_filter).collect();
    format!("{}({})", if group.any { "any" } else { "all" }, items.join(", "))
}

/// A table as text, so that two that differ say where.
pub fn describe(table: &KnowledgeTable) -> String {
    let mut text = format!("table {} active={}\n", table.name, table.active_view_id);
    for p in &table.columns {
        let options: Vec<String> = p
            .options
            .iter()
            .map(|o| format!("{}:{}:{}", o.id, o.name, o.colour))
            .collect();
        text += &format!(
            "property {} {} {} title={} target={} limit={} options=[{}]\n",
            p.id,
            p.name,
            p.value_type,
            p.is_title,
            p.target_file,
            p.limit,
            options.join(", ")
        );
    }
    for v in &table.views {
        text += &format!(
            "view {} {} groupBy={} hideEmpty={} freeze={}\n",
            v.id, v.name, v.group_by, v.hide_empty_groups, v.freeze
        );
        text += &format!("  columns {:?}\n", v.columns);
        text += &format!("  sorts {:?}\n", v.sorts);
        text += &format!("  filter {}\n", describe_group(&v.filter));
        text += &format!(
            "  order=[{}] hidden=[{}] collapsed=[{}]\n",
            v.group_order.join(", "),
            v.hidden_groups.join(", "),
            v.collapsed.join(", ")
        );
    }
    for r in &table.rows {
        let cells: Vec<String> = r
            .cells
            .iter()
            .map(|c| format!("{}=[{}]", c.column_id, c.values.join(", ")))
            .collect();
        text += &format!("row {} {}\n", r.id, cells.join("; "));
    }
    text
}

/// What became of an edit.
#[derive(Debug, Clone)]
pub struct Settled {
    pub written: bool,
    pub error: String,
    pub taken_back: Vec<String>,
}

/// One knowledge file open for editing as the host opens it: a session whose edits are written to the file. What
/// the session gives is kept, so a test can look at what became of an edit and at what was shown meanwhile.
pub struct EditingTable {
    pub path: PathBuf,
    pub session: KnowledgeSession,
    pub given: Vec<TableChange>,
    next: u64,
}

impl EditingTable {
    pub fn new(path: PathBuf) -> Self {
        EditingTable::under(path, PathBuf::new())
    }

    /// With the project the file is in: how far is looked for rows that relate to a row of this file.
    pub fn under(path: PathBuf, root: PathBuf) -> Self {
        let session = KnowledgeSession::open_for_editing(&path, root);
        EditingTable {
            path,
            session,
            given: Vec::new(),
            next: 0,
        }
    }

    /// With the writing given: a test refuses a write exactly when it chooses.
    pub fn with_writer(path: PathBuf, write: adp_designer_knowledge::session::Writer) -> Self {
        let session = KnowledgeSession::with(
            &path,
            Box::new(|at: &Path| store::read(at)),
            Some(write),
            PathBuf::new(),
        );
        EditingTable {
            path,
            session,
            given: Vec::new(),
            next: 0,
        }
    }

    /// The file as it is on disk now, read as the module reads it.
    pub fn on_disk(&self) -> KnowledgeTable {
        store::read(&self.path).body.expect("the file reads").table().clone()
    }

    pub fn bytes(&self) -> Vec<u8> {
        std::fs::read(&self.path).expect("the file reads")
    }

    fn take(&mut self) {
        let changes = self.session.take_changes();
        self.given.extend(changes);
    }

    /// The table's lines as the session shows them now, all of them.
    pub fn lines(&mut self) -> Vec<TableRow> {
        self.take();
        self.session.set_window(0, usize::MAX);
        let mut rows = Vec::new();
        for change in self.session.take_changes() {
            if let TableChange::Rows { rows: given, .. } = change {
                rows = given;
            }
        }
        rows
    }

    /// Sets the window the connection has in sight, keeping what it gives.
    pub fn window(&mut self, first: usize, count: usize) {
        self.session.set_window(first, count);
        self.take();
    }

    /// The last lines the session gave.
    pub fn last_rows(&self) -> (usize, Vec<TableRow>, usize) {
        self.given
            .iter()
            .rev()
            .find_map(|change| match change {
                TableChange::Rows { first, rows, row_count } => Some((*first, rows.clone(), *row_count)),
                _ => None,
            })
            .expect("lines were given")
    }

    /// The baseline as the session gives it now.
    pub fn baseline(&self) -> adp_desl::table::model::TableBaseline {
        self.session.baseline()
    }

    /// Makes a gesture and answers at once, as the service does: the edit's id and the session's answer. Nothing is
    /// written yet.
    pub fn begin(&mut self, gesture: TableGesture) -> (String, String) {
        self.next += 1;
        let id = format!("edit-{}", self.next);
        let answer = self.session.edit(&id, &gesture);
        self.take();
        (id, answer)
    }

    /// Writes what waits, and says what became of the edit.
    pub fn settled(&mut self, edit_id: &str) -> Settled {
        self.session.write_pending();
        self.take();
        self.given
            .iter()
            .rev()
            .find_map(|change| match change {
                TableChange::EditSettled {
                    edit_id: id,
                    written,
                    error,
                    taken_back,
                } if id == edit_id => Some(Settled {
                    written: *written,
                    error: error.clone(),
                    taken_back: taken_back.clone(),
                }),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{edit_id} was not settled"))
    }

    /// Another table of the application wrote these files: this one reads what it has to again.
    pub fn told(&mut self, written: &[&Path]) {
        for path in written {
            self.session.written(path);
        }
        self.take();
    }

    /// Makes a gesture that is expected to be accepted, and waits for what became of its write.
    pub fn edit(&mut self, gesture: TableGesture) -> Settled {
        let kind = gesture.kind.clone();
        let (id, answer) = self.begin(gesture);
        assert_eq!("", answer, "{kind} was refused");
        let settled = self.settled(&id);
        assert!(settled.written, "{kind} was not written: {}", settled.error);
        settled
    }
}

/// A folder of its own for a test, removed when it is dropped.
pub fn folder() -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix("adp-knowledge-")
        .tempdir()
        .expect("a temporary folder")
}
