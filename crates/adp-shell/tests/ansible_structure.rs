//! The Ansible structure description against its case, recorded from the module (etalii.adp spec 035, T132), on the
//! shell's host, whose Ansible module contributes `reader.ansibleProject`: the folder read with its listing (FBL 0.26),
//! the ranks laid out by `layout.rankedColumns` at the widths the module measures, and the module's refusals. Every
//! step passes.

use std::path::Path;

use adp_atd_check::{FsFiles, check};
use adp_atd_validate::{read_case, validate_description};

#[test]
fn the_ansible_structure_case_passes() {
    let diagrams = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/definitions/diagrams");
    let checked = check(&FsFiles, &diagrams.join("ansible-structure.atd"));
    let case = read_case(&diagrams.join("conformance/ansible-structure/infrastructure")).unwrap();
    let results = validate_description(&checked, &adp_shell::runtime_host(), std::slice::from_ref(&case)).unwrap();
    let differences: Vec<String> = results[0]
        .differences
        .iter()
        .map(|d| format!("{:?} {}: {} | {}", d.step, d.part, d.expected, d.actual))
        .collect();
    assert!(differences.is_empty(), "{differences:#?}");
}
