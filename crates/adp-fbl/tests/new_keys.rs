//! *(FBL 0.24)* Where a set writes a key an existing entry of the yaml family does not have (FBL section 6.2,
//! `insert.newKeys`): at its place in `keys` by default, and with `after-first` on the line right after the entry's
//! first key, as the standalone host's line splicer writes it. The supply chain binding of the specification's
//! examples, with and without the setting.

use std::path::Path;
use std::rc::Rc;

use adp_cel::{Map, Value};
use adp_fbl::documents;
use adp_fbl::{FblOptions, ModelChange, OpenBody, PlanResult};

const BODY: &str = "supply-chain: 1\r\ngroups: []\r\nnodes:\r\n  - id: mine\r\n    type: source\r\n    name: Mine\r\n    quantity: 5\r\nflows: []\r\n";

fn body(after_first: bool) -> OpenBody {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/specifications/fbl/supply-chain.fbl");
    let mut json: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    if after_first {
        for rule in json["bindings"]["supply"]["elements"].as_array_mut().unwrap() {
            if rule.get("insert").is_some() {
                rule["insert"]["newKeys"] = serde_json::json!("after-first");
            }
        }
    }
    let (problems, document) = documents::load(json.to_string().as_bytes());
    let document = document.unwrap_or_else(|| panic!("{problems:?}"));
    let binding = document.binding("supply").unwrap().clone();
    OpenBody::open(
        BODY.as_bytes().to_vec(),
        Rc::new(binding),
        Rc::new(FblOptions::default()),
    )
}

fn set(body: &mut OpenBody, name: &str, value: &str) {
    let id = body
        .model()
        .elements
        .iter()
        .find(|e| e.type_name == "Node")
        .unwrap()
        .id
        .clone();
    let mut attributes = Map::default();
    attributes.insert(name.to_owned(), Value::from(value));
    let planned = body.change(&ModelChange::Set { id, attributes });
    assert!(matches!(planned, PlanResult::Planned(_)), "{planned:?}");
}

fn text(body: &OpenBody) -> String {
    String::from_utf8(body.bytes().to_vec()).unwrap()
}

#[test]
fn a_new_key_goes_at_its_place_in_keys_by_default() {
    let mut body = body(false);
    set(&mut body, "description", "Copper.");
    assert_eq!(
        BODY.replace("    name: Mine\r\n", "    name: Mine\r\n    description: Copper.\r\n"),
        text(&body)
    );
}

#[test]
fn with_after_first_a_new_key_goes_right_after_the_entrys_first_key() {
    let mut body = body(true);
    set(&mut body, "description", "Copper.");
    set(&mut body, "unit", "kt");
    assert_eq!(
        BODY.replace(
            "  - id: mine\r\n",
            "  - id: mine\r\n    unit: kt\r\n    description: Copper.\r\n"
        ),
        text(&body)
    );
}

/// *(FBL 0.25)* With `text.keepStyle: false`, a replaced value is written as a new value is: plain where it can be.
#[test]
fn without_keep_style_a_replaced_value_is_written_plain_where_it_can_be() {
    let quoted = BODY.replace("    name: Mine\r\n", "    name: \"Old mine\"\r\n");
    for (keep, written) in [(true, "\"New mine\""), (false, "New mine")] {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/specifications/fbl/supply-chain.fbl");
        let mut json: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        json["bindings"]["supply"]["text"]["keepStyle"] = serde_json::json!(keep);
        let (_, document) = documents::load(json.to_string().as_bytes());
        let binding = document.unwrap().binding("supply").unwrap().clone();
        let mut body = OpenBody::open(
            quoted.clone().into_bytes(),
            Rc::new(binding),
            Rc::new(FblOptions::default()),
        );
        set(&mut body, "name", "New mine");
        assert_eq!(quoted.replace("\"Old mine\"", written), text(&body), "keepStyle {keep}");
    }
}
