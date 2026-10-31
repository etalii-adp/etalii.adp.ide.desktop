//! What FBL 0.12 section 8.6 says of identities that the fixtures do not pin down: the `identities:` block, an id first
//! stored, and the later holders of a key when an earlier one goes.

use std::cell::Cell;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use adp_cel::{Map, Value};
use serde_json::json;

use super::OpenDocument;
use crate::documents::loader::load;
use crate::documents::types::FblBinding;
use crate::history::UndoResult;
use crate::model::{FblOptions, IdRequest};
use crate::planning::{ModelChange, PlanResult};

const GRAMMAR: &str = r#"start: unit*
?unit: thing
     | link
     | _NL
thing: _THING NAME
link: ">" NAME
_THING.2: "thing"
NAME: /[a-z]+/
_NL: /(\r?\n[ \t]*)+/
WHITESPACE: /[ \t]+/
%ignore WHITESPACE
"#;

fn binding(legacy: bool, ordinal: bool) -> Rc<FblBinding> {
    let mut sidecar = json!({ "key": "entry.children[0].text", "kind": "thing" });
    if ordinal {
        sidecar["ordinal"] = json!("#{n}");
    }
    let document = json!({
        "fbl": "0.13",
        "bindings": { "things": {
            "claims": { "extensions": [".things"], "origins": ["a/b"] },
            "body": {
                "kind": "file",
                "family": "grammar",
                "grammar": GRAMMAR,
                "recover": { "unit": "unit", "sync": ["_NL"] }
            },
            "reader": "declared",
            "elements": [
                {
                    "name": "thing",
                    "type": "Thing",
                    "at": "/thing",
                    "id": { "sidecar": sidecar },
                    "attributes": { "name": { "node": "NAME" } },
                    "insert": { "place": "end-of-document", "emit": "thing {name}" },
                    "remove": {}
                },
                {
                    "name": "link",
                    "type": "Link",
                    "at": "/link",
                    "id": { "sidecar": { "key": "entry.children[0].text", "kind": "link" } },
                    "attributes": {
                        "to": { "node": "NAME", "reference": { "to": ["thing"], "by": "name" } }
                    },
                    "remove": {}
                }
            ],
            "registration": if legacy { json!({ "legacyIdentities": "{base}.identities.json" }) } else { json!({}) }
        }}
    });
    let (problems, document) = load(document.to_string().as_bytes());
    Rc::new(document.unwrap_or_else(|| panic!("{problems:?}")).bindings.remove(0).1)
}

/// A folder of its own for the files of a document, removed when the test ends.
struct Folder(PathBuf);

impl Folder {
    fn new(name: &str) -> Folder {
        let folder = std::env::temp_dir().join(format!("adp-identities-{}-{name}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        Folder(folder)
    }

    fn open(&self, files: &[(&str, &str)], binding: Rc<FblBinding>, options: FblOptions) -> OpenDocument {
        for (name, text) in files {
            fs::write(self.0.join(name), text).unwrap();
        }
        let registration = fs::read(self.0.join("m.adp")).unwrap();
        OpenDocument::open(registration, &self.0.join("m.adp"), binding, options, &self.0)
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).ok();
    }
}

fn text(document: &OpenDocument, name: &str) -> String {
    String::from_utf8_lossy(document.file(name).unwrap()).into_owned()
}

fn ids(document: &OpenDocument) -> Vec<String> {
    document
        .model()
        .unwrap()
        .elements
        .iter()
        .map(|e| e.id.clone())
        .collect()
}

fn rename(id: &str, name: &str) -> ModelChange {
    let mut attributes = Map::default();
    attributes.insert("name", Value::from(name));
    ModelChange::Set {
        id: id.to_owned(),
        attributes,
    }
}

const REGISTRATION: &str = "a/b\nbody: m.things\nidentities:\n  link:tea: l1\n  thing:pot: t2\n  thing:tea: t1\n";

/// The block's keys follow a rename and its references in one edit, and one undo moves them back.
#[test]
fn an_entry_of_the_block_follows_its_key_in_one_edit() {
    let folder = Folder::new("block");
    let mut document = folder.open(
        &[("m.adp", REGISTRATION), ("m.things", "thing tea\nthing pot\n> tea\n")],
        binding(false, false),
        FblOptions::default(),
    );
    assert_eq!(ids(&document), ["t1", "t2", "l1"]);

    let result = document.change(&rename("t1", "cup"));

    assert!(matches!(result, PlanResult::Planned(_)), "{result:?}");
    assert_eq!(text(&document, "m.things"), "thing cup\nthing pot\n> cup\n");
    assert_eq!(
        text(&document, "m.adp"),
        "a/b\nbody: m.things\nidentities:\n  link:cup: l1\n  thing:pot: t2\n  thing:cup: t1\n"
    );
    assert_eq!(ids(&document), ["t1", "t2", "l1"]);
    assert_eq!(document.last_files(), ["m.things", "m.adp"]);

    assert!(matches!(document.undo(), UndoResult::Done(_)));
    assert_eq!(text(&document, "m.adp"), REGISTRATION);
    assert_eq!(text(&document, "m.things"), "thing tea\nthing pot\n> tea\n");
    assert_eq!(ids(&document), ["t1", "t2", "l1"]);

    assert!(matches!(document.redo(), UndoResult::Done(_)));
    assert_eq!(text(&document, "m.things"), "thing cup\nthing pot\n> cup\n");
    assert_eq!(ids(&document), ["t1", "t2", "l1"]);
}

/// An add writes its entry in the order of the block's keys, and a removal prunes it.
#[test]
fn an_add_and_a_removal_write_the_block_in_one_edit() {
    let folder = Folder::new("add");
    let mut document = folder.open(
        &[("m.adp", REGISTRATION), ("m.things", "thing tea\nthing pot\n> tea\n")],
        binding(false, false),
        FblOptions::default(),
    );

    let add = ModelChange::Add {
        type_name: "Thing".into(),
        id: Some("t3".into()),
        attributes: {
            let mut attributes = Map::default();
            attributes.insert("name", Value::from("mug"));
            attributes
        },
        parent_id: None,
        index: None,
        slot: None,
    };
    assert!(matches!(document.change(&add), PlanResult::Planned(_)));
    assert_eq!(
        text(&document, "m.adp"),
        "a/b\nbody: m.things\nidentities:\n  link:tea: l1\n  thing:mug: t3\n  thing:pot: t2\n  thing:tea: t1\n"
    );
    assert_eq!(ids(&document), ["t1", "t2", "l1", "t3"]);

    let remove = ModelChange::Remove { id: "t2".into() };
    assert!(matches!(document.change(&remove), PlanResult::Planned(_)));
    assert_eq!(
        text(&document, "m.adp"),
        "a/b\nbody: m.things\nidentities:\n  link:tea: l1\n  thing:mug: t3\n  thing:tea: t1\n"
    );
    assert!(matches!(document.undo(), UndoResult::Done(_)));
    assert!(matches!(document.undo(), UndoResult::Done(_)));
    assert_eq!(text(&document, "m.adp"), REGISTRATION);
}

/// The later holders of a key keep their ids when an earlier one goes: each entry moves to the place its holder now has.
#[test]
fn the_later_holders_of_a_key_keep_their_ids_when_an_earlier_one_goes() {
    let folder = Folder::new("holders");
    let registration = "a/b\nbody: m.things\n";
    let identities = "[\n  { \"id\": \"t1\", \"kind\": \"thing\", \"key\": \"tea\" },\n  { \"id\": \"t2\", \"kind\": \"thing\", \"key\": \"tea#2\" },\n  { \"id\": \"t3\", \"kind\": \"thing\", \"key\": \"tea#3\" }\n]\n";
    let mut document = folder.open(
        &[
            ("m.adp", registration),
            ("m.things", "thing tea\nthing tea\nthing tea\n"),
            ("m.identities.json", identities),
        ],
        binding(true, true),
        FblOptions::default(),
    );
    assert_eq!(ids(&document), ["t1", "t2", "t3"]);

    let remove = ModelChange::Remove { id: "t1".into() };
    assert!(matches!(document.change(&remove), PlanResult::Planned(_)));

    assert_eq!(ids(&document), ["t2", "t3"]);
    assert_eq!(
        text(&document, "m.identities.json"),
        "[\n  { \"id\": \"t2\", \"kind\": \"thing\", \"key\": \"tea\" },\n  { \"id\": \"t3\", \"kind\": \"thing\", \"key\": \"tea#2\" }\n]\n"
    );
    assert!(matches!(document.undo(), UndoResult::Done(_)));
    assert_eq!(text(&document, "m.identities.json"), identities);
    assert_eq!(ids(&document), ["t1", "t2", "t3"]);
}

/// An id read without an entry is stored with the next write of the body, and is no step of the history.
#[test]
fn an_id_first_stored_is_no_step_of_the_history() {
    let folder = Folder::new("first");
    let made = Rc::new(Cell::new(0));
    let derive = made.clone();
    let options = FblOptions {
        derive_id: Some(Rc::new(move |request: &IdRequest<'_>| {
            derive.set(derive.get() + 1);
            Some(format!("made-{}", request.line))
        })),
        ..FblOptions::default()
    };
    let mut document = folder.open(
        &[
            ("m.adp", "a/b\nbody: m.things\n"),
            ("m.things", "thing tea\nthing pot\n"),
            (
                "m.identities.json",
                "[\n  {\n    \"id\": \"t1\",\n    \"kind\": \"thing\",\n    \"key\": \"tea\"\n  }\n]\n",
            ),
        ],
        binding(true, false),
        options,
    );
    assert!(made.get() > 0);
    assert_eq!(ids(&document), ["t1", "made-2"]);

    assert!(matches!(document.change(&rename("t1", "cup")), PlanResult::Planned(_)));

    let stored = "[\n  {\n    \"id\": \"t1\",\n    \"kind\": \"thing\",\n    \"key\": \"cup\"\n  },\n  {\n    \"id\": \"made-2\",\n    \"kind\": \"thing\",\n    \"key\": \"pot\"\n  }\n]\n";
    assert_eq!(text(&document, "m.identities.json"), stored);

    assert!(matches!(document.undo(), UndoResult::Done(_)));
    assert_eq!(text(&document, "m.things"), "thing tea\nthing pot\n");
    assert_eq!(
        text(&document, "m.identities.json"),
        "[\n  {\n    \"id\": \"t1\",\n    \"kind\": \"thing\",\n    \"key\": \"tea\"\n  },\n  {\n    \"id\": \"made-2\",\n    \"kind\": \"thing\",\n    \"key\": \"pot\"\n  }\n]\n"
    );
    assert!(matches!(document.undo(), UndoResult::Refused(_)));
}

/// A rule without `ordinal`: the second holder is reported as a duplicate and its id is not stored (FBL section 8.6).
#[test]
fn without_an_ordinal_the_second_holder_is_a_duplicate() {
    let folder = Folder::new("duplicate");
    let document = folder.open(
        &[
            ("m.adp", "a/b\nbody: m.things\nidentities:\n  thing:tea: t1\n"),
            ("m.things", "thing tea\nthing tea\n"),
        ],
        binding(false, false),
        FblOptions::default(),
    );

    let model = document.model().unwrap();

    assert_eq!(model.elements[0].id, "t1");
    assert_ne!(model.elements[1].id, "t1");
    assert!(!model.elements[1].id_is_stored);
    assert!(
        model
            .findings
            .iter()
            .any(|f| f.code == crate::model::codes::DUPLICATE_ID)
    );
}

/// FBL section 8.6: `ordinal` holds `{n}` once.
#[test]
fn an_ordinal_without_its_place_is_refused_when_loaded() {
    let document = json!({
        "fbl": "0.13",
        "bindings": { "things": {
            "claims": { "extensions": [".things"] },
            "body": { "kind": "file", "family": "grammar", "grammar": GRAMMAR },
            "reader": "declared",
            "elements": [{
                "name": "thing",
                "type": "Thing",
                "at": "/thing",
                "id": { "sidecar": { "key": "entry.children[0].text", "ordinal": "#" } }
            }]
        }}
    });

    let (problems, document) = load(document.to_string().as_bytes());

    assert!(document.is_none(), "{problems:?}");
    assert!(
        problems.iter().any(|p| p.message.contains("'ordinal' holds {n} once")),
        "{problems:?}"
    );
}
