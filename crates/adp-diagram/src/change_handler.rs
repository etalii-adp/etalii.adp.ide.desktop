//! How a session reacts to its document changing (etalii.adp spec 030, FR-023), written once: ported from the
//! standalone host's `EtAlii.Adp.Diagram/DiagramDocumentChangeHandler.cs` at `8b6ccfc`.
//!
//! The one shape: ignore a change to any other path, render, diff against what the connection was last given, and
//! hand back the deltas if there are any; a read failure costs the notification, never the session, and leaves the
//! record of what was delivered as it was so the next change diffs from the right place. The handler owns what was
//! delivered, because the baseline, a viewport report and a document change all move it, and a diff is only right
//! against the last of them. The standalone host raised the deltas into a channel under a lock; here the core thread
//! is the only caller, so the deltas are returned and the caller sends them.

use std::io;
use std::path::{Path, PathBuf};

use crate::element::{DiagramDelta, DiagramElement, diff};

/// What a change to a path meant for one session.
#[derive(Debug)]
pub enum DocumentChange {
    /// The change was to another document.
    OtherDocument,
    /// The document changed, but nothing the connection holds did.
    NothingVisible,
    /// What the connection holds changed by these deltas.
    Changed(Vec<DiagramDelta>),
    /// The document could not be read again; the sentence names its path. Nothing was delivered.
    Unreadable(String),
}

/// The record of what one connection was last given, and the diff of every later rendering against it.
#[derive(Debug, Clone)]
pub struct DiagramDocumentChangeHandler {
    body_path: PathBuf,
    delivered: Vec<DiagramElement>,
}

impl DiagramDocumentChangeHandler {
    /// A handler for the session showing the document at `body_path`.
    pub fn new(body_path: impl Into<PathBuf>) -> Self {
        DiagramDocumentChangeHandler {
            body_path: body_path.into(),
            delivered: Vec::new(),
        }
    }

    pub fn body_path(&self) -> &Path {
        &self.body_path
    }

    /// What the connection was last given.
    pub fn delivered(&self) -> &[DiagramElement] {
        &self.delivered
    }

    /// Records `rendered` as what the connection now holds: a session's baseline.
    pub fn deliver(&mut self, rendered: Vec<DiagramElement>) -> &[DiagramElement] {
        self.delivered = rendered;
        &self.delivered
    }

    /// The deltas from what the connection held to `rendered`, recording it: a session's answer to a viewport
    /// report, which it returns rather than raises.
    pub fn refresh(&mut self, rendered: Vec<DiagramElement>) -> Vec<DiagramDelta> {
        let deltas = diff(&self.delivered, &rendered);
        self.delivered = rendered;
        deltas
    }

    /// The session's reaction to `path` changing: `render` is asked only when it is this session's document, and
    /// the comparison ignores case, as every session's did.
    pub fn on_document_changed(
        &mut self,
        path: &Path,
        render: impl FnOnce() -> io::Result<Vec<DiagramElement>>,
    ) -> DocumentChange {
        if !same_path(path, &self.body_path) {
            return DocumentChange::OtherDocument;
        }
        match render() {
            Err(error) => DocumentChange::Unreadable(format!(
                "Could not re-read {} after a change: {error}",
                self.body_path.display()
            )),
            Ok(rendered) => {
                let deltas = self.refresh(rendered);
                if deltas.is_empty() {
                    DocumentChange::NothingVisible
                } else {
                    DocumentChange::Changed(deltas)
                }
            }
        }
    }
}

/// Whether two paths name one file, ignoring case as the standalone host does on every platform.
pub(crate) fn same_path(left: &Path, right: &Path) -> bool {
    left.to_string_lossy().to_lowercase() == right.to_string_lossy().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIRST: &str = r"C:\projects\one\first.sample";
    const SECOND: &str = r"C:\projects\one\second.sample";

    fn element(id: &str, x: f64) -> DiagramElement {
        DiagramElement {
            x,
            y: 20.0,
            ..DiagramElement::node(id, "Node")
        }
    }

    fn changed(change: DocumentChange) -> Vec<DiagramDelta> {
        match change {
            DocumentChange::Changed(deltas) => deltas,
            other => panic!("a change, not {other:?}"),
        }
    }

    #[test]
    fn two_sessions_on_two_paths_only_the_changed_one_hears() {
        let mut first = DiagramDocumentChangeHandler::new(FIRST);
        let mut second = DiagramDocumentChangeHandler::new(SECOND);
        first.deliver(Vec::new());
        second.deliver(Vec::new());

        let heard = first.on_document_changed(Path::new(FIRST), || Ok(vec![element("a", 10.0), element("b", 10.0)]));
        let other = second.on_document_changed(Path::new(FIRST), || Ok(vec![element("x", 10.0)]));

        assert_eq!(changed(heard).len(), 1);
        assert!(matches!(other, DocumentChange::OtherDocument));
    }

    #[test]
    fn the_path_comparison_ignores_case() {
        let mut handler = DiagramDocumentChangeHandler::new(FIRST);
        handler.deliver(Vec::new());
        let change = handler.on_document_changed(Path::new(&FIRST.to_uppercase()), || Ok(vec![element("a", 10.0)]));
        assert_eq!(changed(change).len(), 1);
    }

    #[test]
    fn a_change_gives_the_diff_against_what_was_delivered() {
        let mut handler = DiagramDocumentChangeHandler::new(FIRST);
        handler.deliver(vec![element("a", 10.0), element("b", 10.0)]);

        let deltas = changed(handler.on_document_changed(Path::new(FIRST), || Ok(vec![element("a", 99.0)])));

        assert_eq!(deltas[0], DiagramDelta::Remove(vec!["b".into()]));
        assert_eq!(deltas[1], DiagramDelta::Add(vec![element("a", 99.0)]));
        assert_eq!(handler.delivered(), [element("a", 99.0)]);
    }

    #[test]
    fn a_change_that_changes_nothing_visible_gives_nothing() {
        let mut handler = DiagramDocumentChangeHandler::new(FIRST);
        handler.deliver(vec![element("a", 10.0)]);
        let change = handler.on_document_changed(Path::new(FIRST), || Ok(vec![element("a", 10.0)]));
        assert!(matches!(change, DocumentChange::NothingVisible));
    }

    #[test]
    fn a_read_failure_names_its_path_and_leaves_what_was_delivered() {
        for kind in [io::ErrorKind::Other, io::ErrorKind::PermissionDenied] {
            let mut handler = DiagramDocumentChangeHandler::new(FIRST);
            handler.deliver(vec![element("a", 10.0)]);
            let change = handler.on_document_changed(Path::new(FIRST), || Err(io::Error::new(kind, "locked")));
            let DocumentChange::Unreadable(warning) = change else {
                panic!("unreadable, not {change:?}");
            };
            assert!(warning.contains(FIRST), "{warning}");
            assert_eq!(handler.delivered(), [element("a", 10.0)]);
        }
    }

    #[test]
    #[should_panic(expected = "a defect")]
    fn any_other_failure_reaches_the_caller() {
        let mut handler = DiagramDocumentChangeHandler::new(FIRST);
        handler.deliver(vec![element("a", 10.0)]);
        handler.on_document_changed(Path::new(FIRST), || panic!("a defect"));
    }

    #[test]
    fn a_viewport_refresh_returns_the_deltas() {
        let mut handler = DiagramDocumentChangeHandler::new(FIRST);
        handler.deliver(vec![element("a", 10.0)]);
        let deltas = handler.refresh(vec![element("a", 10.0), element("b", 10.0)]);
        assert_eq!(deltas, vec![DiagramDelta::Add(vec![element("b", 10.0)])]);
        assert_eq!(handler.delivered().len(), 2);
    }
}
