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
