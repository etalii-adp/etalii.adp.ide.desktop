//! The five editors' descriptions against their 25 cases (etalii.adp spec 083, SC-001, SC-002): every case passes on
//! the desktop host's checks, read from the case's bytes alone (SC-004). A regression or a correction shows here.

use std::path::{Path, PathBuf};

use adp_atd_check::{FsFiles, check};
use adp_atd_runtime::{Capabilities, RuntimeHost};
use adp_atd_validate::{cases_of, read_case, validate_description};

fn editors() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/definitions/editors")
}

fn host() -> RuntimeHost {
    RuntimeHost {
        capabilities: adp_editor_checks::contribute(Capabilities::desktop()),
        ..RuntimeHost::default()
    }
}

/// The cases of `name` and how many passed.
fn passed(name: &str) -> (usize, usize) {
    let root = editors().join(format!("{name}.atd"));
    let checked = check(&FsFiles, &root);
    let cases: Vec<_> = cases_of(&root).iter().map(|dir| read_case(dir).unwrap()).collect();
    let results = validate_description(&checked, &host(), &cases).unwrap();
    for result in &results {
        assert!(result.passed(), "{name}/{}: {:?}", result.name, result.differences);
    }
    (results.len(), results.iter().filter(|r| r.passed()).count())
}

#[test]
fn every_editor_case_passes() {
    let counts: Vec<(usize, usize)> = ["plain-text", "markdown", "json", "xml", "yaml"].map(passed).to_vec();
    assert_eq!(counts, [(2, 2), (2, 2), (6, 6), (9, 9), (6, 6)]);
}

/// FR-005: a check the host has no source for is reported as missing, once per case, not as differing findings.
#[test]
fn a_missing_source_is_reported_once_per_case() {
    let root = editors().join("json.atd");
    let checked = check(&FsFiles, &root);
    let cases: Vec<_> = cases_of(&root).iter().map(|dir| read_case(dir).unwrap()).collect();
    let results = validate_description(&checked, &RuntimeHost::default(), &cases).unwrap();
    for result in results {
        assert_eq!(result.differences.len(), 1, "{}: {:?}", result.name, result.differences);
        assert_eq!(result.differences[0].actual, "missing from this host");
    }
}
