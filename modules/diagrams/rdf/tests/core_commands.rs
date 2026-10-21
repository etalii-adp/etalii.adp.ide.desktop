//! The family's commands over real files: each executes, its undo restores the bytes exactly (comments, formatting and
//! abbreviations included), and its redo lands the same edit again; a refusal writes nothing. Ported from the
//! standalone host's `RdfCommands.Tests.cs` and `RdfCommandHandlers.Tests.cs` at `8b6ccfc`, through the project's
//! history (`RestoreDocumentCommand` is the undo; its redo runs the edit again). The handlers' save warning has no
//! counterpart here: the desktop host's documents keep nothing beside the file that a save could lose.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use adp_diagram::DiagramDocuments;
use adp_diagram_rdf::commands::{ObjectValue, RdfCommand, RdfEdit};
use adp_diagram_rdf::shacl::writer::ShaclPropertyShapeOptions;
use adp_diagram_rdf::vocabulary::shacl;
use adp_history::{History, Outcome};

const CORPUS: &str =
    "@prefix ex: <http://example.org/> .\r\n\r\nex:alice ex:knows ex:bob ;\r\n    ex:name \"Alice\" . # who else\r\n";

const FAMILY: &str = "@prefix ex: <http://example.org/> .\r\n@prefix sh: <http://www.w3.org/ns/shacl#> .\r\n@prefix skos: <http://www.w3.org/2004/02/skos/core#> .\r\n\r\nex:alice ex:knows ex:bob ;\r\n    ex:name \"Alice\" . # who else\r\n\r\nex:PersonShape a sh:NodeShape ;\r\n    sh:targetClass ex:Person ;\r\n    sh:name \"Person\" .\r\n\r\nex:animals a skos:ConceptScheme .\r\nex:mammal a skos:Concept .\r\nex:cat a skos:Concept ;\r\n    skos:broader ex:mammal .\r\n";

const EX: &str = "http://example.org/";

fn ex(local: &str) -> String {
    format!("{EX}{local}")
}

struct Project {
    _folder: tempfile::TempDir,
    path: PathBuf,
    documents: DiagramDocuments,
    history: History<DiagramDocuments>,
}

impl Project {
    fn new(text: &str) -> Self {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("graph.ttl");
        fs::write(&path, text).unwrap();
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

fn round_trips(text: &str, edit: RdfEdit) {
    let mut project = Project::new(text);
    let outcome = project.execute(edit);
    assert!(matches!(outcome, Outcome::Done { .. }), "{outcome:?}");
    let edited = project.text();
    assert_ne!(edited, text);

    assert!(matches!(
        project.history.undo(&mut project.documents),
        Outcome::Done { .. }
    ));
    assert_eq!(project.text(), text);

    assert!(matches!(
        project.history.redo(&mut project.documents),
        Outcome::Done { .. }
    ));
    assert_eq!(project.text(), edited);
}

fn refusal(outcome: Outcome) -> String {
    match outcome {
        Outcome::Refused(refusal) => refusal.0,
        other => panic!("not refused: {other:?}"),
    }
}

#[test]
fn add_triple_execute_undo_redo_round_trips_byte_for_byte() {
    round_trips(
        CORPUS,
        RdfEdit::AddTriple {
            subject: ex("alice"),
            predicate: ex("knows"),
            object: ObjectValue::iri(ex("carol")),
        },
    );
}

#[test]
fn remove_triple_execute_undo_redo_round_trips_byte_for_byte() {
    round_trips(
        CORPUS,
        RdfEdit::RemoveTriple {
            subject: ex("alice"),
            predicate: ex("knows"),
            object: ObjectValue::iri(ex("bob")),
        },
    );
}

#[test]
fn remove_resource_execute_undo_redo_round_trips_byte_for_byte() {
    round_trips(CORPUS, RdfEdit::RemoveResource { iri: ex("bob") });
}

#[test]
fn rename_term_execute_undo_redo_round_trips_byte_for_byte() {
    round_trips(
        CORPUS,
        RdfEdit::RenameTerm {
            old_iri: ex("bob"),
            new_iri: ex("robert"),
        },
    );
}

#[test]
fn replace_object_literal_execute_undo_redo_round_trips_byte_for_byte() {
    round_trips(
        CORPUS,
        RdfEdit::ReplaceLiteral {
            subject: ex("alice"),
            predicate: ex("name"),
            old: ObjectValue::literal("Alice", "", ""),
            new_lexical: "Alicia".to_owned(),
            new_language: "en".to_owned(),
            new_datatype: String::new(),
        },
    );
}

#[test]
fn add_prefix_execute_undo_redo_round_trips_byte_for_byte() {
    round_trips(
        CORPUS,
        RdfEdit::AddPrefix {
            prefix: "foaf".to_owned(),
            iri: "http://xmlns.com/foaf/0.1/".to_owned(),
        },
    );
}

#[test]
fn a_refused_command_fails_with_the_sentence_and_the_file_is_untouched() {
    let mut project = Project::new(CORPUS);

    let refusal = refusal(project.execute(RdfEdit::RemoveTriple {
        subject: ex("nobody"),
        predicate: ex("knows"),
        object: ObjectValue::iri(ex("bob")),
    }));

    assert!(refusal.contains("nothing to remove"), "{refusal}");
    assert!(!project.history.availability().can_undo);
    assert_eq!(project.text(), CORPUS);
}

#[test]
fn an_unparseable_file_refuses_every_edit_naming_the_state() {
    let mut project = Project::new("this is ; not turtle @@@\r\n");

    let refusal = refusal(project.execute(RdfEdit::AddPrefix {
        prefix: "ex".to_owned(),
        iri: EX.to_owned(),
    }));

    assert!(refusal.contains("does not parse"), "{refusal}");
}

/// Every command of the family, by name (`RdfCommandHandlersTests.Commands`).
fn command_of(edit: &str) -> RdfEdit {
    match edit {
        "add-triple" => RdfEdit::AddTriple {
            subject: ex("alice"),
            predicate: ex("knows"),
            object: ObjectValue::iri(ex("carol")),
        },
        "remove-triple" => RdfEdit::RemoveTriple {
            subject: ex("alice"),
            predicate: ex("knows"),
            object: ObjectValue::iri(ex("bob")),
        },
        "remove-resource" => RdfEdit::RemoveResource { iri: ex("bob") },
        "rename" => RdfEdit::RenameTerm {
            old_iri: ex("bob"),
            new_iri: ex("robert"),
        },
        "replace-literal" => RdfEdit::ReplaceLiteral {
            subject: ex("alice"),
            predicate: ex("name"),
            old: ObjectValue::literal("Alice", "", ""),
            new_lexical: "Alicia".to_owned(),
            new_language: String::new(),
            new_datatype: String::new(),
        },
        "add-prefix" => RdfEdit::AddPrefix {
            prefix: "foaf".to_owned(),
            iri: "http://xmlns.com/foaf/0.1/".to_owned(),
        },
        "add-concept" => RdfEdit::AddSkosConcept {
            concept: ex("dog"),
            label: "Dog".to_owned(),
            language: "en".to_owned(),
            scheme: ex("animals"),
        },
        "disconnect" => RdfEdit::DisconnectSkosPair {
            a: ex("cat"),
            b: ex("mammal"),
            hierarchy: true,
        },
        "add-property-row" => RdfEdit::AddShaclPropertyRow {
            shape: ex("PersonShape"),
            path: ex("age"),
            options: ShaclPropertyShapeOptions {
                min_count: Some(0),
                max_count: Some(1),
                ..ShaclPropertyShapeOptions::default()
            },
        },
        "add-target" => RdfEdit::AddShaclTarget {
            shape: ex("PersonShape"),
            predicate: shacl::TARGET_NODE.to_owned(),
            term: ex("alice"),
        },
        "remove-target" => RdfEdit::RemoveShaclTarget {
            shape: ex("PersonShape"),
            predicate: shacl::TARGET_CLASS.to_owned(),
            term: ex("Person"),
        },
        "create-shape" => RdfEdit::CreateShaclNodeShape {
            shape: ex("AddressShape"),
        },
        "deactivate" => RdfEdit::SetShaclDeactivated {
            shape: ex("PersonShape"),
            deactivated: true,
        },
        "set-literal" => RdfEdit::SetShaclLiteral {
            shape: ex("PersonShape"),
            predicate: shacl::NAME.to_owned(),
            value: "Human".to_owned(),
        },
        "remove-shape" => RdfEdit::RemoveShaclShape {
            shape: ex("PersonShape"),
        },
        other => panic!("No such edit: {other}"),
    }
}

#[test]
fn every_handler_edits_and_can_be_undone() {
    for edit in [
        "add-triple",
        "remove-triple",
        "remove-resource",
        "rename",
        "replace-literal",
        "add-prefix",
        "add-concept",
        "disconnect",
        "add-property-row",
        "add-target",
        "remove-target",
        "create-shape",
        "deactivate",
        "set-literal",
        "remove-shape",
    ] {
        let mut project = Project::new(FAMILY);
        let outcome = project.execute(command_of(edit));
        assert!(matches!(outcome, Outcome::Done { .. }), "{edit}: {outcome:?}");
        assert_ne!(project.text(), FAMILY, "{edit}");
        assert!(
            matches!(project.history.undo(&mut project.documents), Outcome::Done { .. }),
            "{edit}"
        );
        assert_eq!(project.text(), FAMILY, "{edit}");
    }
}

#[test]
fn an_edit_of_a_file_that_does_not_parse_is_refused_and_writes_nothing() {
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join("broken.ttl");
    fs::copy(common::fixture_path("broken.ttl"), &body).unwrap();
    let before = fs::read(&body).unwrap();
    let mut documents = DiagramDocuments::default();
    let mut history = History::default();

    let outcome = history.execute(
        &mut documents,
        &RdfCommand::new(
            Path::new(&body),
            RdfEdit::AddPrefix {
                prefix: "foaf".to_owned(),
                iri: "http://xmlns.com/foaf/0.1/".to_owned(),
            },
        ),
    );

    let refusal = refusal(outcome);
    assert!(refusal.contains("could not be read"), "{refusal}");
    assert_eq!(fs::read(&body).unwrap(), before);
}
