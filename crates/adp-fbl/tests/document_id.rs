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
