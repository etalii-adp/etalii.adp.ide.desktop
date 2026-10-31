//! The six C4 descriptions against their cases, recorded from the module (etalii.adp spec 067, T029, SC-002): every case
//! passes on the declared binding, apart from the differences listed here by name, each with its reason. A regression
//! or a correction shows here.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use adp_atd_check::{FsFiles, check};
use adp_atd_runtime::RuntimeHost;
use adp_atd_validate::{cases_of, read_case, validate_description};

fn diagrams() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/definitions/diagrams")
}

/// The differences of the cases of the six descriptions, by case, each as `step:part`, the reading's as `read:part`.
const LISTED: [(&str, &[&str]); 8] = [
    // `c4.unknown-view-scope` and the line of `c4.empty-view`: a rule about the model cannot read the scope a view is
    // written with, nor where it stands, of a `View` (DISL 12.2, the companions' gap 13). A language's to answer.
    ("finding-order", &["read:findings"]),
    ("view-without-key", &["read:findings"]),
    // The same findings in another order: the module reports element by element, DISL by rule (the companions' gap 10).
    // A language's to answer.
    ("misplaced-container", &["read:findings"]),
    ("trailing-comments", &["read:findings"]),
    // The module reads two elements under the one id `api`, of which FBL's reading gives the second its place.
    (
        "hierarchical-identifiers",
        &["read:drawing", "read:findings", "read:reading"],
    ),
    // The module reads `web->api` without spaces, and writes an end in the case the element is declared in; the grammar
    // reads neither.
    ("lenient-spellings", &["read:drawing", "read:findings", "read:reading"]),
    // After a block that is never closed the module still reads the views; FBL's recovery reads them as a statement in
    // the model (its fixture `structurizr-recover-nested`).
    ("unbalanced", &["read:drawing", "read:findings"]),
    // An element declared without an identifier: the host's `person_User_1` against FBL's `person@3`.
    (
        "unidentified-elements",
        &["read:drawing", "read:findings", "read:reading"],
    ),
];

/// A gesture the module refuses for its target with a sentence of its own, "That element is no longer in this model.",
/// which the description words as what it is, "Adding a deployment node from the toolbox is not supported yet.".
const ADDED_FROM_THE_CANVAS: &str = "13:refused";

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
            (result.name, places)
        })
        .collect()
}

/// The differences `name`'s cases have: those listed, and those of its `operations`.
fn check_description(name: &str, operations: &[&str]) {
    let listed: BTreeMap<&str, &[&str]> = LISTED.into_iter().collect();
    let mut unexpected = Vec::new();
    let found = differences_of(name);
    assert!(found.len() >= 19, "{name} has its cases");
    for (case, places) in found {
        let expected: BTreeSet<String> = match case.as_str() {
            "operations" => operations.iter().map(|p| (*p).to_owned()).collect(),
            other => listed
                .get(other)
                .into_iter()
                .flat_map(|p| p.iter())
                .map(|p| (*p).to_owned())
                .collect(),
        };
        if places != expected {
            unexpected.push(format!("{case}: {places:?}\n  expected {expected:?}"));
        }
    }
    assert!(unexpected.is_empty(), "{name}:\n{}", unexpected.join("\n"));
}

#[test]
fn the_system_context_cases_pass_but_for_what_is_listed() {
    check_description("c4-context", &[ADDED_FROM_THE_CANVAS]);
}

#[test]
fn the_container_cases_pass_but_for_what_is_listed() {
    check_description("c4-container", &[ADDED_FROM_THE_CANVAS]);
}

#[test]
fn the_component_cases_pass_but_for_what_is_listed() {
    check_description("c4-component", &[ADDED_FROM_THE_CANVAS]);
}

#[test]
fn the_dynamic_cases_pass_but_for_what_is_listed() {
    check_description("c4-dynamic", &[ADDED_FROM_THE_CANVAS]);
}

#[test]
fn the_system_landscape_cases_pass_but_for_what_is_listed() {
    check_description("c4-system-landscape", &[ADDED_FROM_THE_CANVAS]);
}

/// The deployment view of `operations` holds elements without an identifier, named as `unidentified-elements` says, and
/// a node that hosts others, which the module draws as its boundary alone, in its chrome; the description draws the card
/// too. Every step's drawing shows it.
#[test]
fn the_deployment_cases_pass_but_for_what_is_listed() {
    let drawings: Vec<String> = (1..=15).map(|step| format!("{step}:drawing")).collect();
    let mut operations: Vec<&str> = drawings.iter().map(String::as_str).collect();
    operations.extend(["read:drawing", "read:reading", ADDED_FROM_THE_CANVAS]);
    check_description("c4-deployment", &operations);
}
