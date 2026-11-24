//! *(FBL 0.23)* The document id a body's root holds in its binding's `document.id` slot (FBL section 17.2): a root key
//! of a `yaml` or `json` body, an attribute of an `xml` body's root element. The knowledge designer's examples hold the
//! same id in each of the three formats.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_fbl::documents::{self, ProblemSeverity};
use adp_fbl::{FblOptions, OpenBody};

fn vendor() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp")
}

fn document_id(bytes: Vec<u8>, binding: &str) -> Option<String> {
    let (problems, document) = documents::load_file(&vendor().join("definitions/designers/knowledge.fbl"));
    assert!(
        !problems.iter().any(|p| p.severity == ProblemSeverity::Error),
        "{problems:?}"
    );
    let binding = document.unwrap().binding(binding).unwrap().clone();
    OpenBody::open(bytes, Rc::new(binding), Rc::new(FblOptions::default()))
        .model()
        .document_id
}

fn example(name: &str) -> Vec<u8> {
    std::fs::read(vendor().join("definitions/designers/examples").join(name)).unwrap()
}

#[test]
fn the_three_formats_hold_their_id_where_their_binding_says() {
    assert_eq!(Some("c4m8x2tr".into()), document_id(example("cities.yaml"), "yaml"));
    assert_eq!(Some("c4m8x2tr".into()), document_id(example("cities.json"), "json"));
    assert_eq!(Some("c4m8x2tr".into()), document_id(example("cities.xml"), "xml"));
}

#[test]
fn a_body_without_an_id_or_with_an_empty_one_holds_none() {
    let yaml = String::from_utf8(example("cities.yaml")).unwrap();
    let without = yaml.replace("id: c4m8x2tr\r\n", "").replace("id: c4m8x2tr\n", "");
    assert_ne!(yaml, without);
    assert_eq!(None, document_id(without.into_bytes(), "yaml"));
    let empty = yaml.replace("id: c4m8x2tr", "id: \"\"");
    assert_eq!(None, document_id(empty.into_bytes(), "yaml"));
}

fn open(bytes: Vec<u8>, binding: &str) -> adp_fbl::OpenBody {
    let (_, document) = documents::load_file(&vendor().join("definitions/designers/knowledge.fbl"));
    let binding = document.unwrap().binding(binding).unwrap().clone();
    OpenBody::open(bytes, Rc::new(binding), Rc::new(FblOptions::default()))
}

#[test]
fn a_body_without_an_id_is_given_one_in_its_slot_and_keeps_every_other_byte() {
    for (name, binding, written) in [
        ("cities.yaml", "yaml", "id: c4m8x2tr"),
        ("cities.json", "json", "\"id\": \"c4m8x2tr\","),
        ("cities.xml", "xml", " id=\"c4m8x2tr\""),
    ] {
        let original = String::from_utf8(example(name)).unwrap();
        let without = original
            .replacen(&format!("{written}\r\n"), "", 1)
            .replacen(&format!("  {written}\r\n"), "", 1)
            .replacen(written, "", 1);
        assert_ne!(original, without, "{name}");
        let mut body = open(without.clone().into_bytes(), binding);
        assert_eq!(None, body.model().document_id, "{name}");
        let planned = body.change(&adp_fbl::ModelChange::SetDocumentId { id: "k3f9x2ma".into() });
        assert!(
            matches!(planned, adp_fbl::PlanResult::Planned(_)),
            "{name}: {planned:?}"
        );
        assert_eq!(Some("k3f9x2ma".into()), body.model().document_id, "{name}");
        let after = String::from_utf8(body.bytes().to_vec()).unwrap();
        // One insertion, holding the id, and every other byte kept.
        let prefix = without.bytes().zip(after.bytes()).take_while(|(a, b)| a == b).count();
        let suffix = without[prefix..]
            .bytes()
            .rev()
            .zip(after[prefix..].bytes().rev())
            .take_while(|(a, b)| a == b)
            .count();
        assert_eq!(without.len(), prefix + suffix, "{name}");
        let inserted = &after[prefix..after.len() - suffix];
        assert!(inserted.contains("k3f9x2ma"), "{name}: {inserted:?}");
        assert!(body.can_undo(), "{name}");
        body.undo();
        assert_eq!(without.as_bytes(), body.bytes(), "{name}");
    }
}

#[test]
fn a_document_id_cannot_be_set_through_the_attribute_that_reads_it() {
    let mut body = open(example("cities.yaml"), "yaml");
    let table = body
        .model()
        .elements
        .iter()
        .find(|e| e.type_name == "Table")
        .unwrap()
        .id
        .clone();
    let mut attributes = adp_cel::Map::default();
    attributes.insert("documentId", adp_cel::Value::from("zzzzzzzz"));
    let planned = body.change(&adp_fbl::ModelChange::Set { id: table, attributes });
    assert!(matches!(planned, adp_fbl::PlanResult::Refused(_)), "{planned:?}");
}

#[test]
fn a_conversion_writes_the_version_that_no_set_may_change() {
    for (name, binding, old, new) in [
        ("cities.yaml", "yaml", "ded: \"0.1\"", "ded: \"0.3\""),
        ("cities.json", "json", "\"ded\": \"0.1\"", "\"ded\": \"0.3\""),
        ("cities.xml", "xml", "version=\"0.1\"", "version=\"0.3\""),
    ] {
        let original = String::from_utf8(example(name)).unwrap();
        assert!(original.contains(new), "{name}");
        let before = original.replacen(new, old, 1);
        let mut body = open(before.clone().into_bytes(), binding);
        let table = body
            .model()
            .elements
            .iter()
            .find(|e| e.type_name == "Table")
            .unwrap()
            .id
            .clone();
        let mut version = adp_cel::Map::default();
        version.insert("version", adp_cel::Value::from("0.3"));
        let set = body.change(&adp_fbl::ModelChange::Set {
            id: table,
            attributes: version.clone(),
        });
        assert!(matches!(set, adp_fbl::PlanResult::Refused(_)), "{name}: {set:?}");
        let migrated = body.change(&adp_fbl::ModelChange::Migrate { attributes: version });
        assert!(
            matches!(migrated, adp_fbl::PlanResult::Planned(_)),
            "{name}: {migrated:?}"
        );
        assert_eq!(original.as_bytes(), body.bytes(), "{name}");
    }
}
