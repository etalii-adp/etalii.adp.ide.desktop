//! The shapes reading's commands over real files: each lands its writer's splice through the family store, each undo
//! restores the document byte for byte - the removal that sweeps a blank subtree included - and each edit touches only
//! the lines it must. Ported from the standalone host's `ShaclCommands.Tests.cs` at `8b6ccfc`, through the project's
//! history (`RestoreDocumentCommand` is the undo; its redo runs the edit again).

use std::fs;
use std::path::PathBuf;

use adp_diagram::DiagramDocuments;
use adp_diagram_rdf::commands::{RdfCommand, RdfEdit};
use adp_diagram_rdf::shacl::refusals::NO_SUCH_SHAPE;
use adp_diagram_rdf::shacl::writer::ShaclPropertyShapeOptions;
use adp_diagram_rdf::vocabulary::shacl;
use adp_history::{History, Outcome};

const CORPUS: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\r\n@prefix ex: <http://example.org/> .\r\n\r\nex:PersonShape a sh:NodeShape ;\r\n    sh:targetClass ex:Person ;\r\n    sh:property [ sh:path ex:name ; sh:minCount 1 ] . # the one that matters\r\n";

const EX: &str = "http://example.org/";

struct Project {
    _folder: tempfile::TempDir,
    path: PathBuf,
    documents: DiagramDocuments,
    history: History<DiagramDocuments>,
}

impl Project {
    fn new() -> Self {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("shapes.ttl");
        fs::write(&path, CORPUS).unwrap();
        Project {
            _folder: folder,
            path,
            documents: DiagramDocuments::default(),
            history: History::default(),
        }
    }

    fn text(&self) -> String {
        fs::read_to_string(&self.path).unwrap()
    }

    fn execute(&mut self, edit: RdfEdit) -> Outcome {
        let command = RdfCommand::new(&self.path, edit);
        self.history.execute(&mut self.documents, &command)
    }
}

fn shape() -> String {
    format!("{EX}PersonShape")
}

fn round_trips(edit: RdfEdit) {
    let mut project = Project::new();
    assert!(matches!(project.execute(edit), Outcome::Done { .. }));
    let changed = project.text();
    assert_ne!(changed, CORPUS);
    assert!(matches!(
        project.history.undo(&mut project.documents),
        Outcome::Done { .. }
    ));
    assert_eq!(project.text(), CORPUS);
    assert!(matches!(
        project.history.redo(&mut project.documents),
        Outcome::Done { .. }
    ));
    assert_eq!(project.text(), changed);
}

fn age_row() -> RdfEdit {
    RdfEdit::AddShaclPropertyRow {
        shape: shape(),
        path: format!("{EX}age"),
        options: ShaclPropertyShapeOptions {
            min_count: Some(0),
            max_count: Some(1),
            ..ShaclPropertyShapeOptions::default()
        },
    }
}

fn alice_target() -> RdfEdit {
    RdfEdit::AddShaclTarget {
        shape: shape(),
        predicate: shacl::TARGET_NODE.to_owned(),
        term: format!("{EX}alice"),
    }
}

fn deactivate() -> RdfEdit {
    RdfEdit::SetShaclDeactivated {
        shape: shape(),
        deactivated: true,
    }
}

#[test]
fn adding_a_property_row_is_one_undo_away() {
    round_trips(age_row());
}

#[test]
fn adding_a_target_is_one_undo_away() {
    round_trips(alice_target());
}

#[test]
fn removing_a_target_is_one_undo_away() {
    round_trips(RdfEdit::RemoveShaclTarget {
        shape: shape(),
        predicate: shacl::TARGET_CLASS.to_owned(),
        term: format!("{EX}Person"),
    });
}

#[test]
fn creating_a_node_shape_is_one_undo_away() {
    round_trips(RdfEdit::CreateShaclNodeShape {
        shape: format!("{EX}AddressShape"),
    });
}

#[test]
fn deactivating_a_shape_is_one_undo_away() {
    round_trips(deactivate());
}

#[test]
fn removing_a_shape_with_its_subtree_is_one_undo_away() {
    round_trips(RdfEdit::RemoveShaclShape { shape: shape() });
}

#[test]
fn a_refused_edit_fails_with_its_sentence_and_leaves_the_file_untouched() {
    let mut project = Project::new();
    let outcome = project.execute(RdfEdit::AddShaclPropertyRow {
        shape: format!("{EX}NoSuchShape"),
        path: format!("{EX}p"),
        options: ShaclPropertyShapeOptions::default(),
    });
    match outcome {
        Outcome::Refused(refusal) => assert_eq!(refusal.0, NO_SUCH_SHAPE),
        other => panic!("{other:?}"),
    }
    assert_eq!(project.text(), CORPUS);
}

/// Every line the file had it still has, bar the one whose terminator the splice turned into a `;`, and it grew by
/// the one line the new triple takes; the trailing comment is the canary of a reserialization.
fn minimal_diff(edit: RdfEdit, expected: &str) {
    let mut project = Project::new();
    assert!(matches!(project.execute(edit), Outcome::Done { .. }));
    let text = project.text();
    let before: Vec<&str> = CORPUS.split("\r\n").collect();
    let after: Vec<&str> = text.split("\r\n").collect();
    assert_eq!(after.len(), before.len() + 1);
    assert!(text.contains(expected));
    assert!(after.iter().any(|l| l.contains("# the one that matters")));
    let missing: Vec<&&str> = before.iter().filter(|l| !after.contains(l)).collect();
    assert_eq!(missing.len(), 1, "{missing:?}");
    let changed = missing[0];
    let terminator = changed.rfind(" .").unwrap_or(0);
    assert!(terminator > 0, "the changed line is not a statement end: {changed}");
    assert!(after.iter().any(|l| l.starts_with(&changed[..terminator])));
}

#[test]
fn adding_a_target_touches_only_the_lines_it_must() {
    minimal_diff(alice_target(), "sh:targetNode");
}

#[test]
fn adding_a_property_row_touches_only_the_lines_it_must() {
    minimal_diff(age_row(), "sh:path ex:age");
}

#[test]
fn deactivating_touches_only_the_lines_it_must() {
    minimal_diff(deactivate(), "sh:deactivated");
}
