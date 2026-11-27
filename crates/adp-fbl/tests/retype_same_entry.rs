//! *(FBL 0.27)* A retype between rules that read the same entry (FBL section 6.4): where a rule of the new type reads the entry, the
//! rules telling the two types apart by what it holds, the entry stays and the values the retype gives are set on it.
//! The timeline binding of the specification's examples: a moment is an entry without an `end`, a period one with.

use std::path::Path;
use std::rc::Rc;

use adp_cel::{Map, Value};
use adp_fbl::documents;
use adp_fbl::{FblOptions, ModelChange, OpenBody, PlanResult};

const BODY: &str = "timeline: 1\nelements:\n  - id: launch\n    label: Launch\n    begin: 2026-01-13\n    row: 2\n";

fn body() -> OpenBody {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/specifications/fbl/timeline.fbl");
    let (problems, document) = documents::load(&std::fs::read(path).unwrap());
    let document = document.unwrap_or_else(|| panic!("{problems:?}"));
    let binding = document.binding("timeline").unwrap().clone();
    OpenBody::open(
        BODY.as_bytes().to_vec(),
        Rc::new(binding),
        Rc::new(FblOptions::default()),
    )
}

#[test]
fn a_moment_retyped_to_a_period_with_its_end_keeps_its_entry_and_gains_the_end() {
    let mut body = body();
    let mut attributes = Map::default();
    attributes.insert("end", Value::from("2026-02-01"));
    let planned = body.change(&ModelChange::Retype {
        id: "launch".into(),
        type_name: "Period".into(),
        attributes,
    });
    assert!(matches!(planned, PlanResult::Planned(_)), "{planned:?}");
    let text = String::from_utf8(body.bytes().to_vec()).unwrap();
    assert!(
        text.contains("    begin: 2026-01-13\n    end: 2026-02-01\n    row: 2\n"),
        "{text}"
    );
    assert_eq!(body.model().elements[0].type_name, "Period");
}

#[test]
fn a_retype_that_gives_nothing_writes_nothing() {
    let mut body = body();
    let planned = body.change(&ModelChange::Retype {
        id: "launch".into(),
        type_name: "Period".into(),
        attributes: Map::default(),
    });
    assert!(matches!(planned, PlanResult::Planned(_)), "{planned:?}");
    assert_eq!(body.bytes(), BODY.as_bytes());
}

#[test]
fn a_retype_to_a_type_no_rule_reads_the_entry_as_is_refused() {
    let mut body = body();
    let planned = body.change(&ModelChange::Retype {
        id: "launch".into(),
        type_name: "Connection".into(),
        attributes: Map::default(),
    });
    assert!(matches!(planned, PlanResult::Refused(_)), "{planned:?}");
}
