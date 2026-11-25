//! The SPARQL query description against its two cases, recorded from the module, through the shell's capabilities,
//! which contribute the module's reader as `reader.sparql` (etalii.adp spec 035, T132): every step passes, the scopes
//! laid out by `layout.nestedGrid` and each annotation drawn beside what it is attached to, its bounds the module's
//! painting (ATD §2.3), apart from the differences listed here by name.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use adp_atd_check::{FsFiles, check};
use adp_atd_validate::{cases_of, read_case, validate_description};

/// The differences of the cases, by case, each as `step:part`, the reading's as `read:part`.
const LISTED: &[(&str, &[&str])] = &[
    // The module's reading states no parent for a node inside a region: a variable, literal or term a group's
    // patterns mention is drawn in that group's frame, but read at the root. The description nests it in the scope it
    // is drawn in, as its drawing does.
    ("groups", &["read:reading"]),
];

fn differences() -> BTreeMap<String, BTreeSet<String>> {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/definitions/diagrams/sparql-query.atd");
    let checked = check(&FsFiles, &root);
    let cases: Vec<_> = cases_of(&root).iter().map(|dir| read_case(dir).unwrap()).collect();
    assert_eq!(cases.len(), 2, "the SPARQL query has its cases");
    validate_description(&checked, &adp_shell::runtime_host(), &cases)
        .expect("the description has a runtime")
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
        .filter(|(_, places): &(String, BTreeSet<String>)| !places.is_empty())
        .collect()
}

#[test]
fn the_sparql_query_cases_pass_but_for_what_is_listed() {
    let listed: BTreeMap<String, BTreeSet<String>> = LISTED
        .iter()
        .map(|(case, places)| ((*case).to_owned(), places.iter().map(|p| (*p).to_owned()).collect()))
        .collect();
    assert_eq!(differences(), listed);
}
