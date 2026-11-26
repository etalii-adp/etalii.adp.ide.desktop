//! The three Databricks descriptions and the Wardley map description against their cases, recorded from the modules
//! (etalii.adp spec 067, T031, T033, SC-002), each read through its declared binding. What still differs is here by name:
//! a module's quirk with its reason. Nothing else may differ, so a regression or a correction shows here.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use adp_atd_check::{FsFiles, check};
use adp_atd_runtime::RuntimeHost;
use adp_atd_validate::{cases_of, read_case, validate_description};

fn diagrams() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/definitions/diagrams")
}

/// Differences that are a module's quirk, by case: `description/case`, each place `step:part`, the reading's `read:part`.
const QUIRKS: [(&str, &[&str]); 7] = [
    // The YAML parser's own sentence and line for a body that does not parse, which are not FBL's.
    ("databricks-bundle/unparseable", &["read:findings"]),
    ("databricks-job/unparseable", &["read:findings"]),
    ("databricks-pipeline/unreadable", &["read:findings"]),
    // The same findings in another order: the module reports task by task, DISL by rule. A quirk of the module's, by
    // Peter's ruling (spec 067), for which DISL does not change. With it the repeated task key, which the module reads
    // under one id and FBL's reading gives its place, the quirk of C4's `hierarchical-identifiers`.
    (
        "databricks-job/finding-order",
        &["read:findings", "read:drawing", "read:reading"],
    ),
    // Under the bundle and pipeline readings the module reports the findings of the whole file, every job's, the bundle's
    // and every pipeline's, whichever reading asked (the companion's gap 9), and runs the job reading's commands on the
    // file, which those readings' models cannot state: an operation of one reading writing another's types. The bundle's
    // case also orders its unknown nodes after the resources. A quirk of the module's, by Peter's ruling (spec 067), for
    // which no language changes.
    (
        "databricks-bundle/job-commands",
        &[
            "read:drawing",
            "read:findings",
            "1:drawing",
            "1:findings",
            "1:refused",
            "2:drawing",
            "2:findings",
            "2:refused",
            "3:drawing",
            "3:findings",
            "3:refused",
            "4:drawing",
            "4:findings",
            "4:refused",
        ],
    ),
    (
        "databricks-pipeline/job-commands",
        &[
            "read:findings",
            "1:bytes",
            "1:findings",
            "2:bytes",
            "2:findings",
            "3:bytes",
            "3:findings",
            "4:bytes",
            "4:findings",
            "4:refused",
        ],
    ),
    // A rename gives the module's links new ids where FBL 8.6 keeps them ("the ids stay", its fixture
    // `wardley-rename-identity`), a quirk of the module's by Peter's ruling (spec 067), and the case stays as recorded:
    // from step 11 on the drawing names those links by ids the description does not have, step 15 sets a property of
    // one of them, and the bytes differ from there. The case's moves are in canvas units and the module's zero-size
    // notes and accelerators are compared by presence (ATD §2.3, Peter's choices of 2026-11-25), so steps 1 to 10 pass.
    (
        "wardley-map/strategy-vocabulary",
        &[
            "11:drawing",
            "12:drawing",
            "13:drawing",
            "14:drawing",
            "15:bytes",
            "15:drawing",
            "15:refused",
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
            "21:bytes",
            "21:drawing",
            "21:refused",
            "22:bytes",
            "22:drawing",
            "23:bytes",
            "23:drawing",
            "23:refused",
            "24:bytes",
            "24:drawing",
            "25:bytes",
            "25:drawing",
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
fn the_bundle_cases_pass_but_for_what_is_listed() {
    check_description("databricks-bundle", 7);
}

#[test]
fn the_job_cases_pass_but_for_what_is_listed() {
    check_description("databricks-job", 7);
}

#[test]
fn the_pipeline_cases_pass_but_for_what_is_listed() {
    check_description("databricks-pipeline", 7);
}

#[test]
fn the_wardley_map_case_differs_only_as_listed() {
    check_description("wardley-map", 1);
}

/// Each of them requires no reader capability, and replaying says the same twice, as a host's validate must.
#[test]
fn no_reader_is_required_and_two_replays_say_the_same() {
    for name in [
        "databricks-bundle",
        "databricks-job",
        "databricks-pipeline",
        "wardley-map",
    ] {
        let root = diagrams().join(format!("{name}.atd"));
        let checked = check(&FsFiles, &root);
        assert!(
            !checked.description.members().any(|m| m["kind"] == "requires"
                && m["capability"]["text"]
                    .as_str()
                    .is_some_and(|c| c.starts_with("reader."))),
            "{name} requires no reader"
        );
        let cases: Vec<_> = cases_of(&root).iter().map(|dir| read_case(dir).unwrap()).collect();
        let replayed = || {
            validate_description(&checked, &RuntimeHost::default(), &cases)
                .unwrap()
                .into_iter()
                .flat_map(|result| result.differences)
                .map(|d| format!("{:?} {} {} {}", d.step, d.part, d.expected, d.actual))
                .collect::<Vec<_>>()
        };
        assert_eq!(replayed(), replayed(), "{name}");
    }
}
