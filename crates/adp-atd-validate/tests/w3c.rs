//! The four W3C descriptions against their cases, recorded from the RDF module (etalii.adp spec 067, T041, SC-003;
//! spec 035, T149), each read through its declared binding with no reader capability. What still differs is here by name:
//! a module's quirk with its reason, or what waits for a ruling, because no description can state it in the languages as
//! they are. Nothing else may differ, so a regression or a correction shows here. Run with `--release` or the optimised
//! packages of the workspace's profile: the descriptions' rules are quadratic in the triples.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use adp_atd_check::{FsFiles, check};
use adp_atd_runtime::RuntimeHost;
use adp_atd_validate::{cases_of, read_case, validate_description};

fn diagrams() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/definitions/diagrams")
}

/// What is not replayed here: a body of 157 KB, over which the SKOS description's rules take 12 minutes in a release build
/// and go over their evaluation budget, so that no card is derived (`w3c-rdf.md`'s gap 6, a grouping function). Run by
/// name: `the_geographic_names_case`, which only says that it runs.
const TOO_LONG: &str = "geographic-names";

/// Differences that are a module's quirk, by case: `description/case`, each place `step:part`, the reading's `read:part`.
const QUIRKS: [(&str, &[&str]); 4] = [
    // The hand-written parser's own sentence for a body that does not parse, which is not the grammar's: the same finding
    // at the same line.
    ("w3c-rdf/broken", &["read:findings"]),
    ("w3c-owl/broken", &["read:findings"]),
    ("w3c-shacl/broken", &["read:findings"]),
    ("w3c-skos/broken", &["read:findings"]),
];

/// What waits for a ruling: no description can state it in the languages as they are (spec 067).
///
/// - A new statement after a body whose last line is blank. The module writes a blank line before a statement of its own
///   unless the last line is blank already, and `emit`'s line feed always writes one, so the bytes of `nobel-1903` from
///   step 8 and of `prov-o` from step 4 keep one blank line more. FBL states nothing that reads the body's last line.
/// - `nobel-1903`, steps 26 and 27: the module's history holds a set to the value already there (step 24), which writes
///   nothing and so is no edit of the description's; the undos after it then undo one edit further.
/// - `prov-o`, step 13 on: the module removes `res:…#servings`, an id only its action reads, where the case and the
///   description draw that property's range as `dt:…#servings`, so that nothing has the id the step selects.
/// - `turtle-edits-skos`, step 3 on: the row `skos.label:prefLabel:en` is the module's own, one per label and language,
///   where the description has the one field `prefLabel`.
const AWAITING_RULING: [(&str, &[&str]); 3] = [
    (
        "w3c-rdf/nobel-1903",
        &[
            "8:bytes",
            "9:bytes",
            "10:bytes",
            "11:bytes",
            "12:bytes",
            "13:bytes",
            "14:bytes",
            "15:bytes",
            "16:bytes",
            "17:bytes",
            "18:bytes",
            "19:bytes",
            "20:bytes",
            "21:bytes",
            "22:bytes",
            "23:bytes",
            "24:bytes",
            "26:bytes",
            "26:drawing",
            "27:bytes",
            "27:drawing",
        ],
    ),
    (
        "w3c-owl/prov-o",
        &[
            "4:bytes",
            "5:bytes",
            "6:bytes",
            "7:bytes",
            "8:bytes",
            "9:bytes",
            "10:bytes",
            "11:bytes",
            "12:bytes",
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
    (
        "w3c-skos/turtle-edits-skos",
        &["3:refused", "3:bytes", "4:bytes", "5:bytes", "6:bytes", "7:bytes"],
    ),
];

/// Where `name`'s cases differ, by case, but for the case that takes too long.
fn differences_of(name: &str) -> BTreeMap<String, BTreeSet<String>> {
    let root = diagrams().join(format!("{name}.atd"));
    let checked = check(&FsFiles, &root);
    let cases: Vec<_> = cases_of(&root)
        .iter()
        .filter(|dir| !dir.ends_with(TOO_LONG))
        .map(|dir| read_case(dir).unwrap())
        .collect();
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
    for (case, places) in QUIRKS.iter().chain(AWAITING_RULING.iter()) {
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
    check_description("w3c-skos", 24);
}

/// The one case of the four descriptions that is not run with the rest (`TOO_LONG`).
#[test]
#[ignore = "takes longer than a build should"]
fn the_geographic_names_case() {
    let root = diagrams().join("w3c-skos.atd");
    let checked = check(&FsFiles, &root);
    let case = read_case(&diagrams().join("conformance/w3c-skos").join(TOO_LONG)).unwrap();
    let results = validate_description(&checked, &RuntimeHost::default(), &[case]).unwrap();
    assert_eq!(results.len(), 1);
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
