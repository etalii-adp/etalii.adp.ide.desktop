//! The Azure DevOps pipeline description against its case, recorded from the module (etalii.adp spec 035, T132), on
//! the shell's host, whose pipeline module contributes `reader.azurePipelinesYaml`: the file read and every edit
//! written by the module's own code, the stages laid out by DISL's `layered` layout, closed until a stage is opened,
//! and the default order's edges derived. Two differences are listed, by step and part.

use std::collections::BTreeSet;
use std::path::Path;

use adp_atd_check::{FsFiles, check};
use adp_atd_validate::{read_case, validate_description};

/// The differences of the case, each as `step:part`, the reading's as `read:part`.
const LISTED: &[&str] = &[
    // The module records the values a stage takes when the file states none: `enabled` as true and `displayName` as
    // its name. The description reads what the file states, which for these is nothing.
    "read:reading",
    // The module takes a depends-on that is none of the stages before it and refuses it as a cycle; the description's
    // field offers the stages before it only, so the host refuses the value as not one of them.
    "27:refused",
];

#[test]
fn the_azure_devops_pipeline_case_passes_but_for_what_is_listed() {
    let diagrams = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/definitions/diagrams");
    let checked = check(&FsFiles, &diagrams.join("azure-devops-pipeline.atd"));
    let case = read_case(&diagrams.join("conformance/azure-devops-pipeline/multi-stage")).unwrap();
    let results = validate_description(&checked, &adp_shell::runtime_host(), std::slice::from_ref(&case)).unwrap();
    let found: BTreeSet<String> = results[0]
        .differences
        .iter()
        .map(|d| match d.step {
            Some(step) => format!("{step}:{}", d.part),
            None => format!("read:{}", d.part),
        })
        .collect();
    let listed: BTreeSet<String> = LISTED.iter().map(|s| (*s).to_owned()).collect();
    assert_eq!(found, listed, "{:#?}", results[0].differences);
}
