//! The module's examples: every document parses and validates clean, every one is registered, every folder carries
//! its provenance and licence, and the laureates file really exceeds the drawn-node budget. Ported from the standalone
//! host's `RdfExamples.Tests.cs` at `8b6ccfc`, over the copy of `src/diagrams/rdf/examples` in `tests/examples`.

mod common;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use adp_diagram_rdf::lines::LineDocument;
use adp_diagram_rdf::projection::{self, DEFAULT_BUDGET};
use adp_diagram_rdf::{ORIGINS, parser, validator};

fn walk(folder: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(folder).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, found);
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("ttl") || e.eq_ignore_ascii_case("nt"))
        {
            found.push(path);
        }
    }
}

fn documents() -> Vec<PathBuf> {
    let mut found = Vec::new();
    walk(&common::example_path(""), &mut found);
    found.sort();
    found
}

#[test]
fn the_walk_still_finds_the_documents() {
    assert!(documents().len() >= 5);
}

#[test]
fn every_document_parses_and_validates_clean() {
    for path in documents() {
        let text = String::from_utf8(fs::read(&path).unwrap()).unwrap();
        let model = parser::parse(&LineDocument::parse(&text)).unwrap_or_else(|e| panic!("{path:?}: {e:?}"));
        assert!(!model.triples.is_empty(), "{path:?}");
        let problems = validator::judge(&model);
        assert!(problems.is_empty(), "{path:?}: {problems:?}");
    }
}

#[test]
fn every_document_is_registered() {
    for path in documents() {
        let folder = path.parent().unwrap();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let registered = fs::read_dir(folder).unwrap().flatten().any(|entry| {
            let adp = entry.path();
            if adp.extension().is_none_or(|e| e != "adp") {
                return false;
            }
            let text = fs::read_to_string(&adp).unwrap();
            let mut lines = text.lines();
            lines.next().is_some_and(|first| ORIGINS.contains(&first.trim()))
                && lines.any(|line| line.trim() == format!("body: {name}"))
        });
        assert!(registered, "{path:?} has no registration of this family naming it.");
    }
}

#[test]
fn the_laureates_file_really_exceeds_the_budget() {
    let model = common::model(&common::example("nobel/laureates.ttl"));

    let projection = projection::project(&model, DEFAULT_BUDGET);

    assert!(
        projection.truncated(),
        "laureates.ttl draws only {} nodes",
        projection.total
    );
    assert_eq!(projection.shown, DEFAULT_BUDGET);
}

#[test]
fn every_folder_carries_its_provenance_and_licence() {
    let folders: BTreeSet<PathBuf> = documents().iter().map(|p| p.parent().unwrap().to_path_buf()).collect();
    for folder in folders {
        assert!(
            folder.join("readme.md").is_file(),
            "{folder:?} has no provenance readme."
        );
        assert!(
            folder.join("LICENSE.md").is_file(),
            "{folder:?} carries no licence text."
        );
        let readme = fs::read_to_string(folder.join("readme.md")).unwrap();
        assert!(readme.contains("Retrieved"), "{folder:?}");
        assert!(readme.contains("License"), "{folder:?}");
    }
}
