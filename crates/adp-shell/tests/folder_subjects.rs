//! A description whose binding reads a folder subject, validated in memory (etalii.adp spec 035, T132): the Helm
//! chart's case is read through `reader.helmChart` from the case's own files, and a drag on its read-only diagram
//! writes the registration's `layout:` as the module does, since a read-only binding keeps its body unwritten, not
//! where its elements stand.

use std::path::Path;

use adp_atd_check::{FsFiles, check};
use adp_atd_validate::{read_case, validate_description};

fn diagrams() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/definitions/diagrams")
}

#[test]
fn a_folder_subject_is_read_from_the_cases_files_and_a_drag_on_it_is_made() {
    let checked = check(&FsFiles, &diagrams().join("helm-chart.atd"));
    let case = read_case(&diagrams().join("conformance/helm-chart/nginx")).unwrap();
    let results = validate_description(&checked, &adp_shell::runtime_host(), std::slice::from_ref(&case)).unwrap();
    let differences = &results[0].differences;

    // The reading: the chart's elements are there, and no plugin is missing.
    assert!(
        !differences
            .iter()
            .any(|d| d.step.is_none() && d.part == "reading" && d.actual.contains("nothing")),
        "{differences:#?}"
    );
    assert!(!differences.iter().any(|d| d.actual.contains("std.pluginMissing")));
    // The first step drags a values file: it is made, not refused as a change to a read-only diagram.
    assert!(
        !differences.iter().any(|d| d.step == Some(1) && d.part == "refused"),
        "{differences:#?}"
    );
}

/// The Helm chart's module records its attributes under payload names (`chart.version`), which the description's trace
/// maps with `recordedNames` (ATD §2.3): the reading is compared through the map and differs nowhere.
#[test]
fn a_reading_recorded_under_a_modules_own_names_is_compared_through_the_traces_map() {
    let checked = check(&FsFiles, &diagrams().join("helm-chart.atd"));
    assert!(checked.description.recorded_names.contains_key("Chart"));
    let case = read_case(&diagrams().join("conformance/helm-chart/nginx")).unwrap();
    let results = validate_description(&checked, &adp_shell::runtime_host(), std::slice::from_ref(&case)).unwrap();
    let reading: Vec<_> = results[0]
        .differences
        .iter()
        .filter(|d| d.step.is_none() && d.part == "reading")
        .collect();
    assert!(reading.is_empty(), "{reading:#?}");
}

/// The SPARQL query's module records a variable's display under `name` and its bare name not at all, which its trace
/// maps (ATD §2.3, the empty name standing for an attribute the module records no value for): no variable's names
/// differ.
#[test]
fn an_attribute_mapped_to_the_empty_name_is_not_compared() {
    let checked = check(&FsFiles, &diagrams().join("sparql-query.atd"));
    let case = read_case(&diagrams().join("conformance/sparql-query/groups")).unwrap();
    let results = validate_description(&checked, &adp_shell::runtime_host(), std::slice::from_ref(&case)).unwrap();
    let names: Vec<_> = results[0]
        .differences
        .iter()
        .filter(|d| d.step.is_none() && (d.expected.contains(" name: ") || d.expected.contains(" display: ")))
        .filter(|d| d.expected.starts_with("var:"))
        .collect();
    assert!(names.is_empty(), "{names:#?}");
}
