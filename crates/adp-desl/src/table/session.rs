//! One open table on one connection (the standalone host's `IDesignerSession` at 8b6ccfc, and the window the
//! workspace's `DesignerService.SetTableWindow` and `SetTableView` set): what the table is, which lines the
//! connection has in sight and which view it shows, and the author's gestures. A session is per connection: two
//! tabs on one document are two sessions, each with its own window and its own active view.
//!
//! The standalone host raises a `Changed` event; here a session queues its changes, and the core drains them with
//! [`DesignerSession::take_changes`] onto the window's event channel (contract core-shell, `TableWindowChanged`),
//! so a session needs no thread and no lock of its own.

use std::collections::HashSet;

use super::content::TableContent;
use super::engine::{self, LineOptions};
use super::mapper;
use super::model::{TableBaseline, TableChange, TableFinding, TableGesture, TableRow};

/// One open table on one connection.
pub trait DesignerSession {
    /// The table as this connection first sees it. The rows follow as [`TableChange::Rows`].
    fn baseline(&self) -> TableBaseline;

    /// Which lines the connection has in sight, by index in the active view's order. The session answers with those
    /// lines, and keeps them current.
    fn set_window(&mut self, first: usize, count: usize);

    /// Which view this connection shows. A view that is not there - deleted while the call was on its way - changes
    /// nothing.
    fn set_active_view(&mut self, view_id: &str);

    /// Takes one gesture. Returns an empty string when it is accepted - it is then shown at once, and a
    /// [`TableChange::EditSettled`] says what became of it - or the sentence saying why it is refused, in which case
    /// nothing changed.
    fn edit(&mut self, edit_id: &str, gesture: &TableGesture) -> String;

    /// What changed since the last call, in order: the window's rows, the structure, the findings, or what became
    /// of an edit.
    fn take_changes(&mut self) -> Vec<TableChange>;
}

/// The lines a connection has in sight: none until it says which.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TableWindow {
    window: Option<(usize, usize)>,
}

impl TableWindow {
    pub fn set(&mut self, first: usize, count: usize) {
        self.window = Some((first, count));
    }

    pub fn is_set(&self) -> bool {
        self.window.is_some()
    }

    /// The window's lines out of all of a view's lines, or none while the connection has not said which it has in
    /// sight. A window past the end gives no line, at the end.
    pub fn rows_of(&self, lines: &[TableRow]) -> Option<TableChange> {
        let (first, count) = self.window?;
        let first = first.min(lines.len());
        let last = first + count.min(lines.len() - first);
        Some(TableChange::Rows {
            first,
            rows: lines[first..last].to_vec(),
            row_count: lines.len(),
        })
    }
}

/// What a session that only shows refuses an edit with.
pub const NO_WRITER: &str = "This table cannot be changed here.";

/// A session over a table's content that is held in memory and never written: it windows, switches views and
/// searches, and refuses every other gesture with its read-only reason. A host shows a document it cannot write
/// through it, and a designer module's own session builds on the same [`TableWindow`] and view engine.
pub struct ContentSession {
    content: TableContent,
    findings: Vec<TableFinding>,
    read_only_reason: String,
    view_id: String,
    search: String,
    window: TableWindow,
    changes: Vec<TableChange>,
}

impl ContentSession {
    pub fn new(content: TableContent, findings: Vec<TableFinding>, read_only_reason: impl Into<String>) -> Self {
        let view_id = content.view_or_default("").map(|v| v.id.clone()).unwrap_or_default();
        ContentSession {
            content,
            findings,
            read_only_reason: read_only_reason.into(),
            view_id,
            search: String::new(),
            window: TableWindow::default(),
            changes: Vec::new(),
        }
    }

    /// The view this connection shows.
    pub fn active_view_id(&self) -> &str {
        &self.view_id
    }

    fn lines(&self) -> Vec<TableRow> {
        let kept = HashSet::new();
        engine::lines(
            &self.content,
            self.content.view_or_default(&self.view_id),
            &LineOptions {
                kept: Some(&kept),
                search: &self.search,
                ..LineOptions::default()
            },
        )
    }

    fn structure(&self) -> TableChange {
        TableChange::Structure(Box::new(mapper::structure(
            &self.content,
            &self.read_only_reason,
            &self.view_id,
            self.lines().len(),
            None,
            None,
            &self.search,
        )))
    }
}

impl DesignerSession for ContentSession {
    fn baseline(&self) -> TableBaseline {
        let TableChange::Structure(structure) = self.structure() else {
            unreachable!("a structure is a structure")
        };
        let structure = *structure;
        TableBaseline {
            title: structure.title,
            columns: structure.columns,
            views: structure.views,
            settings: structure.settings,
            row_count: structure.row_count,
            findings: self.findings.clone(),
            read_only_reason: structure.read_only_reason,
        }
    }

    fn set_window(&mut self, first: usize, count: usize) {
        self.window.set(first, count);
        if let Some(rows) = self.window.rows_of(&self.lines()) {
            self.changes.push(rows);
        }
    }

    fn set_active_view(&mut self, view_id: &str) {
        if self.content.view(view_id).is_none() {
            return;
        }
        self.view_id = view_id.to_owned();
        self.changes.push(self.structure());
        if let Some(rows) = self.window.rows_of(&self.lines()) {
            self.changes.push(rows);
        }
    }

    fn edit(&mut self, edit_id: &str, gesture: &TableGesture) -> String {
        // A search is this connection's own and changes no document: a table that cannot be edited is searched all
        // the same.
        if gesture.kind == "search" {
            self.search = gesture.values.first().map(|v| v.trim().to_owned()).unwrap_or_default();
            self.changes.push(TableChange::EditSettled {
                edit_id: edit_id.to_owned(),
                written: true,
                error: String::new(),
                taken_back: Vec::new(),
            });
            self.changes.push(self.structure());
            self.changes.push(TableChange::Findings(self.findings.clone()));
            if let Some(rows) = self.window.rows_of(&self.lines()) {
                self.changes.push(rows);
            }
            return String::new();
        }
        if self.read_only_reason.is_empty() {
            NO_WRITER.to_owned()
        } else {
            self.read_only_reason.clone()
        }
    }

    fn take_changes(&mut self) -> Vec<TableChange> {
        std::mem::take(&mut self.changes)
    }
}
