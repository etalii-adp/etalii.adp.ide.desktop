//! The `grammar` family (FBL 0.9 section 4.9; spec 035, T147): Turtle read by FBL's example binding, whose grammar is
//! in a file beside it, from the RDF type's own Turtle; an edit changes only its element's splice and is undone
//! exactly; a body that does not parse is read in part, with a finding at its line; a grammar with a conflict refuses
//! its binding. Its round-trip fixtures are run by `adp-conformance`.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_cel::{Map, Value};
use adp_fbl::documents::{self, ProblemSeverity};
use adp_fbl::{FblBinding, FblOptions, ModelChange, OpenBody, Operation, PlanResult, UndoResult};

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn turtle() -> Rc<FblBinding> {
    let path = repository().join("vendor/etalii.adp/specifications/fbl/w3c-turtle-grammar.fbl");
    let (problems, document) = documents::load_file(&path);
    assert!(
        !problems.iter().any(|p| p.severity == ProblemSeverity::Error),
        "{problems:?}"
    );
    Rc::new(document.unwrap().binding("turtle").unwrap().clone())
}

fn open(bytes: Vec<u8>, name: &str) -> OpenBody {
    OpenBody::open(
        bytes,
        turtle(),
        Rc::new(FblOptions {
            file_name: name.into(),
            ..FblOptions::default()
        }),
    )
}

/// The RDF type's Turtle: its parity fixtures and the examples and fixtures of its module that are well-formed.
fn rdf_turtle() -> Vec<PathBuf> {
    let root = repository();
    let mut files = vec![
        root.join("parity/fixtures/w3c/rdf/nobel-1903/nobel-1903.ttl"),
        root.join("modules/diagrams/rdf/tests/examples/nobel/laureates.ttl"),
        root.join("modules/diagrams/rdf/tests/examples/nobel/nobel-1903.ttl"),
    ];
    for name in [
        "constructs",
        "crlf-line-endings",
        "lf-line-endings",
        "no-trailing-newline",
        "tied-line-endings",
    ] {
        files.push(root.join(format!("modules/diagrams/rdf/tests/fixtures/{name}.ttl")));
    }
    files
}

#[test]
fn the_rdf_types_turtle_is_read_whole_with_every_prefix() {
    for file in rdf_turtle() {
        let bytes = std::fs::read(&file).unwrap();
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let body = open(bytes, "graph.ttl");

        let model = body.model();

        assert!(!model.unreadable, "{}", file.display());
        assert!(
            model.findings.iter().all(|f| f.code != "fbl.unparsed-text"),
            "{}: {:?}",
            file.display(),
            model.findings
        );
        let prefixes = text.lines().filter(|l| l.trim_start().starts_with("@prefix")).count();
        assert_eq!(
            model.elements.iter().filter(|e| e.type_name == "Prefix").count(),
            prefixes,
            "{}",
            file.display()
        );
        assert!(
            model.elements.iter().any(|e| e.type_name == "Resource"),
            "{}",
            file.display()
        );
    }
}

#[test]
fn a_set_changes_only_its_elements_value_and_its_undo_puts_the_bytes_back() {
    let file = repository().join("modules/diagrams/rdf/tests/examples/nobel/nobel-1903.ttl");
    let bytes = std::fs::read(&file).unwrap();
    let mut body = open(bytes.clone(), "nobel-1903.ttl");
    let model = body.model();
    let element = model
        .elements
        .iter()
        .find(|e| e.type_name == "Resource" && e.attributes.get("label").is_some_and(|l| !l.is_null()))
        .expect("a labelled resource");
    let mut attributes = Map::default();
    attributes.insert("label", Value::from("Renamed"));

    let result = body.change(&ModelChange::Set {
        id: element.id.clone(),
        attributes,
    });

    let PlanResult::Planned(edit) = result else {
        panic!("{result:?}");
    };
    assert_eq!(edit.splices.len(), 1);
    assert_eq!(edit.splices[0].operation, Operation::ReplaceValue);
    let changed = body.bytes().to_vec();
    let splice = &edit.splices[0];
    assert_eq!(&changed[..splice.start], &bytes[..splice.start]);
    assert_eq!(&changed[splice.start + "Renamed".len()..], &bytes[splice.end..]);
    let after = body.model();
    assert_eq!(
        after
            .elements
            .iter()
            .find(|e| e.id == element.id)
            .unwrap()
            .attributes
            .get("label"),
        Some(&Value::from("Renamed"))
    );
    assert!(matches!(body.undo(), UndoResult::Done(_)));
    assert_eq!(body.bytes(), bytes.as_slice());
}

#[test]
fn a_body_that_does_not_parse_is_read_in_part_with_a_finding_at_its_line() {
    let text = "@prefix ex: <http://example.org/> .\n@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\nex:a rdfs:label \"A\" .\nex:b rdfs:label \"open .\nex:c rdfs:label \"C\" .\n";
    let body = open(text.as_bytes().to_vec(), "part.ttl");

    let model = body.model();

    assert!(!model.unreadable);
    let ids: Vec<&str> = model
        .elements
        .iter()
        .filter(|e| e.type_name == "Resource")
        .map(|e| e.id.as_str())
        .collect();
    assert_eq!(ids, ["ex:a", "ex:c"]);
    let unparsed: Vec<usize> = model
        .findings
        .iter()
        .filter(|f| f.code == "fbl.unparsed-text")
        .map(|f| f.location.as_ref().unwrap().line)
        .collect();
    assert_eq!(unparsed, [4]);
    // The text that did not parse is kept: saving writes the bytes that were read.
    assert_eq!(body.bytes(), text.as_bytes());
}

#[test]
fn a_binding_whose_grammar_has_a_conflict_is_refused_when_it_is_loaded() {
    let document = r#"{
      "fbl": "0.9",
      "bindings": {
        "items": {
          "claims": { "extensions": [".items"] },
          "body": {
            "kind": "file",
            "family": "grammar",
            "grammar": "doc: item*\nitem: NAME \";\" | other\nother: NAME \";\"\nNAME: /[a-z]+/\nWS: / +/\n%ignore WS\n",
            "start": "doc"
          },
          "reader": "declared",
          "elements": [{ "name": "item", "type": "Item", "at": "/item", "attributes": { "name": { "node": "NAME" } } }]
        }
      }
    }"#;

    let (problems, loaded) = documents::load(document.as_bytes());

    assert!(loaded.is_none());
    let problem = problems.iter().find(|p| p.severity == ProblemSeverity::Error).unwrap();
    assert!(problem.to_string().contains("not LALR(1)"), "{problem}");
    assert!(problem.to_string().contains("Reduce/Reduce"), "{problem}");
}

#[test]
fn a_grammar_given_as_text_reads_and_writes_its_body() {
    let document = r#"{
      "fbl": "0.9",
      "bindings": {
        "items": {
          "claims": { "extensions": [".items"] },
          "body": {
            "kind": "file",
            "family": "grammar",
            "grammar": "doc: item*\nitem: NAME \"=\" VALUE \";\"\nNAME: /[a-z]+/\nVALUE: /[0-9]+/\nWS: /[ \\n]+/\n%ignore WS\n",
            "start": "doc"
          },
          "reader": "declared",
          "elements": [{
            "name": "item", "type": "Item", "at": "/item",
            "id": { "from": { "node": "NAME" } },
            "attributes": { "value": { "node": "VALUE" } },
            "insert": { "place": "after-last", "emit": "{id} = {value};" },
            "remove": {}
          }]
        }
      }
    }"#;
    let (problems, loaded) = documents::load(document.as_bytes());
    let binding = Rc::new(
        loaded
            .unwrap_or_else(|| panic!("{problems:?}"))
            .binding("items")
            .unwrap()
            .clone(),
    );
    let mut body = OpenBody::open(
        b"a = 1;\nb = 2;\n".to_vec(),
        binding,
        Rc::new(FblOptions {
            file_name: "x.items".into(),
            ..FblOptions::default()
        }),
    );
    let mut values = Map::default();
    values.insert("value", Value::from("3"));

    let added = body.change(&ModelChange::Add {
        type_name: "Item".into(),
        id: Some("c".into()),
        attributes: values,
        parent_id: None,
        index: None,
        slot: None,
    });
    assert!(matches!(added, PlanResult::Planned(_)), "{added:?}");
    assert_eq!(String::from_utf8_lossy(body.bytes()), "a = 1;\nb = 2;\nc = 3;\n");

    // A value that is not a token of its terminal would not read back, and is refused.
    let mut letters = Map::default();
    letters.insert("value", Value::from("x"));
    let refused = body.change(&ModelChange::Set {
        id: "a".into(),
        attributes: letters,
    });
    assert!(matches!(refused, PlanResult::Refused(_)), "{refused:?}");

    let removed = body.change(&ModelChange::Remove { id: "b".into() });
    assert!(matches!(removed, PlanResult::Planned(_)), "{removed:?}");
    assert_eq!(String::from_utf8_lossy(body.bytes()), "a = 1;\nc = 3;\n");
}
