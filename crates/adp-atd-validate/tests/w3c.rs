//! The four W3C descriptions against their cases, recorded from the RDF module (etalii.adp spec 067, T041, SC-003;
//! spec 035, T149), each read through its declared binding with no reader capability. What still differs is here by name:
//! a module's quirk with its reason, or what waits for a ruling, because no description can state it in the languages as
//! they are. Nothing else may differ, so a regression or a correction shows here. The SKOS description's rules read groups
//! (DISL 0.9, `groupBy`; etalii.adp spec 075), so its largest case runs with the rest.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use adp_atd_check::{FsFiles, check};
use adp_atd_runtime::RuntimeHost;
use adp_atd_validate::{cases_of, read_case, validate_description};

fn diagrams() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/definitions/diagrams")
}

/// Differences that are a module's quirk, by case: `description/case`, each place `step:part`, the reading's `read:part`.
/// Peter ruled the last three so on 2026-11-01 (spec 067).
const QUIRKS: [(&str, &[&str]); 8] = [
    // The hand-written parser's own sentence for a body that does not parse, which is not the grammar's: the same finding
    // at the same line.
    ("w3c-rdf/broken", &["read:findings"]),
    ("w3c-owl/broken", &["read:findings"]),
    ("w3c-shacl/broken", &["read:findings"]),
    ("w3c-skos/broken", &["read:findings"]),
    // The module's history holds a set to the value already there (step 24), which writes nothing and so is no edit of the
    // description's; the undos after it then undo one edit further.
    ("w3c-rdf/nobel-1903", &["26:drawing", "27:drawing"]),
    // The module removes `res:…#servings`, an id only its action reads, where the case and the description draw that
    // property's range as `dt:…#servings`, so that nothing has the id the step selects.
    (
        "w3c-owl/prov-o",
        &[
            "13:refused",
            "13:bytes",
            "13:drawing",
            "14:bytes",
            "14:drawing",
            "15:bytes",
            "15:drawing",
            "16:bytes",
            "16:drawing",
            "17:bytes",
            "17:drawing",
            "18:bytes",
            "18:drawing",
            "19:bytes",
            "19:drawing",
            "20:bytes",
            "20:drawing",
        ],
    ),
    // The row `skos.label:prefLabel:en` is the module's own, one per label and language, where the description has the one
    // field `prefLabel`.
    (
        "w3c-skos/turtle-edits-skos",
        &["3:refused", "3:bytes", "4:bytes", "5:bytes", "6:bytes", "7:bytes"],
    ),
    // The same row, which step 1 of the largest case sets, and the bytes of every later step that differ by it (spec 075,
    // R12; Peter, 2026-11-01). Step 15 writes the same bytes either way.
    (
        "w3c-skos/geographic-names",
        &[
            "1:refused",
            "1:bytes",
            "2:bytes",
            "3:bytes",
            "4:bytes",
            "5:bytes",
            "6:bytes",
            "7:bytes",
            "8:bytes",
            "9:bytes",
            "10:bytes",
            "11:bytes",
            "12:bytes",
            "13:bytes",
            "14:bytes",
            "16:bytes",
            "17:bytes",
        ],
    ),
];

/// Where `name`'s cases differ, by case.
fn differences_of(name: &str) -> BTreeMap<String, BTreeSet<String>> {
    let root = diagrams().join(format!("{name}.atd"));
    let checked = check(&FsFiles, &root);
    let cases: Vec<_> = cases_of(&root).iter().map(|dir| read_case(dir).unwrap()).collect();
    validate_description(&checked, &RuntimeHost::default(), &cases)
        .unwrap()
        .into_iter()
        .map(|result| {
            let places = result
                .differences
                .iter()
                .map(|d| match d.step {
                    Some(step) => format!("{step}:{}", d.part),
                    None => format!("read:{}", d.part),
                })
                .collect();
            (format!("{name}/{}", result.name), places)
        })
        .collect()
}

fn check_description(name: &str, count: usize) {
    let mut listed: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for (case, places) in QUIRKS.iter() {
        listed
            .entry(case)
            .or_default()
            .extend(places.iter().map(|p| (*p).to_owned()));
    }
    let found = differences_of(name);
    assert_eq!(found.len(), count, "{name} has its cases");
    let unexpected: Vec<String> = found
        .iter()
        .filter(|(case, places)| listed.get(case.as_str()).cloned().unwrap_or_default() != **places)
        .map(|(case, places)| format!("{case}: {places:?}\n  listed {:?}", listed.get(case.as_str())))
        .collect();
    assert!(unexpected.is_empty(), "{}", unexpected.join("\n"));
}

#[test]
fn the_rdf_cases_pass_but_for_what_is_listed() {
    check_description("w3c-rdf", 22);
}

#[test]
fn the_owl_cases_pass_but_for_what_is_listed() {
    check_description("w3c-owl", 23);
}

#[test]
fn the_shacl_cases_pass_but_for_what_is_listed() {
    check_description("w3c-shacl", 23);
}

#[test]
fn the_skos_cases_pass_but_for_what_is_listed() {
    check_description("w3c-skos", 25);
}

/// Setting a concept's documentation row `skos.doc:definition` writes its `skos:definition` triple with the bytes the
/// module writes (spec 075, FR-012): step 2 of the largest case, replayed on what its step 1 recorded.
#[test]
fn a_definition_set_through_its_row_writes_the_modules_bytes() {
    let root = diagrams().join("w3c-skos.atd");
    let checked = check(&FsFiles, &root);
    let mut case = read_case(&diagrams().join("conformance/w3c-skos/geographic-names")).unwrap();
    let step = case.fixture["steps"][1].clone();
    assert_eq!(step["property"]["attribute"], "skos.doc:definition");
    case.fixture["inputs"]["body"] = "after-1.ttl".into();
    case.fixture["steps"] = serde_json::json!([step]);
    let results = validate_description(&checked, &RuntimeHost::default(), &[case]).unwrap();
    let step: Vec<String> = results[0]
        .differences
        .iter()
        .filter(|d| d.step == Some(1) && matches!(d.part, "bytes" | "refused" | "splices"))
        .map(|d| d.to_string())
        .collect();
    assert!(step.is_empty(), "{step:#?}");
}

/// The two vocabularies of the STW Thesaurus draw through the description within its budgets (spec 075, SC-005): every
/// derived type computes, and no expression goes over its cost or a reading over its steps. The reading is compared with
/// an empty one, so that what it read is in the differences.
#[test]
fn the_stw_vocabularies_draw_within_the_budgets() {
    let root = diagrams().join("w3c-skos.atd");
    let checked = check(&FsFiles, &root);
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../modules/diagrams/rdf/tests/examples/stw");
    for (name, terms) in [("geographic-names.ttl", 436), ("business-economics.ttl", 1151)] {
        let body = std::fs::read(examples.join(name)).unwrap();
        let fixture = serde_json::json!({
            "tool": "w3c/skos",
            "inputs": { "body": name },
            "read": { "elements": [], "relations": [], "findings": [] },
            "steps": []
        });
        let case = adp_atd_validate::Case {
            name: name.to_owned(),
            fixture,
            files: [(name.to_owned(), body)].into_iter().collect(),
        };
        let results = validate_description(&checked, &RuntimeHost::default(), &[case]).unwrap();
        // The case states no finding, so any the reading gives is a difference.
        let findings: Vec<String> = results[0]
            .differences
            .iter()
            .filter(|d| d.part == "findings")
            .map(|d| d.actual.clone())
            .collect();
        assert!(findings.is_empty(), "{name}: {findings:?}");
        let read: Vec<serde_json::Value> = results[0]
            .differences
            .iter()
            .filter(|d| d.part == "reading")
            .filter_map(|d| serde_json::from_str::<Vec<serde_json::Value>>(&d.actual).ok())
            .flatten()
            .collect();
        let derived = read
            .iter()
            .filter(|e| matches!(e["type"].as_str(), Some("ConceptScheme" | "Concept" | "Collection")))
            .count();
        assert_eq!(derived, terms, "{name} derives every term");
    }
}

/// None of them requires a reader, and the Turtle's words are FBL's (`adp-atd-runtime`'s test holds the four bindings equal).
#[test]
fn no_reader_is_required() {
    for name in ["w3c-rdf", "w3c-owl", "w3c-shacl", "w3c-skos"] {
        let checked = check(&FsFiles, &diagrams().join(format!("{name}.atd")));
        assert!(
            !checked.description.members().any(|m| m["kind"] == "requires"
                && m["capability"]["text"]
                    .as_str()
                    .is_some_and(|c| c.starts_with("reader."))),
            "{name} requires no reader"
        );
    }
}
