//! The checker against the vendored corpus (spec 035, T036): every example checks without findings, and every refused
//! file gives exactly the findings its `.findings.json` states, by code, severity, file, line and column.

use std::path::{Path, PathBuf};

use adp_atd_check::{FsFiles, check};
use serde_json::Value;

fn spec_dir() -> PathBuf {
    match std::env::var_os("ATD_SPEC_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/specifications/atd"),
    }
}

fn atd_files(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "atd"))
        .collect();
    found.sort();
    found
}

fn basename(file: &str) -> &str {
    file.rsplit('/').next().unwrap_or(file)
}

#[test]
fn every_example_checks_without_findings() {
    let mut problems = Vec::new();
    for path in atd_files(&spec_dir()) {
        for finding in check(&FsFiles, &path).findings {
            problems.push(finding.to_line());
        }
    }
    assert!(
        problems.is_empty(),
        "the examples have findings:\n{}",
        problems.join("\n")
    );
}

#[test]
fn every_refused_file_gives_exactly_its_findings() {
    let mut problems = Vec::new();
    for path in atd_files(&spec_dir().join("refused")) {
        let expected: Value =
            serde_json::from_str(&std::fs::read_to_string(path.with_extension("findings.json")).unwrap()).unwrap();
        let mut expected: Vec<(String, String, String, u64, u64)> = expected["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                (
                    f["code"].as_str().unwrap().to_owned(),
                    f["severity"].as_str().unwrap().to_owned(),
                    f["file"].as_str().unwrap().to_owned(),
                    f["line"].as_u64().unwrap(),
                    f["column"].as_u64().unwrap(),
                )
            })
            .collect();
        let checked = check(&FsFiles, &path);
        let mut actual: Vec<(String, String, String, u64, u64)> = checked
            .findings
            .iter()
            .map(|f| {
                (
                    f.code.to_owned(),
                    f.severity.as_str().to_owned(),
                    basename(&f.file).to_owned(),
                    f.line as u64,
                    f.column as u64,
                )
            })
            .collect();
        expected.sort();
        actual.sort();
        if expected != actual {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            problems.push(format!(
                "{name}:\n  expected {expected:?}\n  actual   {actual:?}\n  messages {:#?}",
                checked.findings.iter().map(|f| f.to_line()).collect::<Vec<_>>()
            ));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
