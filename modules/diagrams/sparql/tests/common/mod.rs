//! What the tests share (etalii.adp spec 030, task T084): the fixtures the standalone host's module was tested against
//! (`EtAlii.Adp.Diagram.Sparql.Tests/Fixtures` at `8b6ccfc`) and its example corpus (`src/diagrams/sparql/examples`),
//! committed byte for byte under `tests/fixtures`.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use adp_diagram_sparql::model::SparqlQueryModel;
use adp_diagram_sparql::parser;
use adp_diagram_sparql::projection::{self, SparqlProjectionResult};

/// The fixture `name`.
pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

/// The text of the fixture `name`.
pub fn text(name: &str) -> String {
    fs::read_to_string(fixture(name)).unwrap()
}

/// What the fixture `name` asks.
pub fn parse_fixture(name: &str) -> SparqlQueryModel {
    parser::parse(&text(name)).unwrap()
}

/// What `text` draws.
pub fn project(text: &str) -> SparqlProjectionResult {
    projection::project(&parser::parse(text).unwrap())
}

/// What the fixture `name` draws.
pub fn project_fixture(name: &str) -> SparqlProjectionResult {
    project(&text(name))
}

/// Every query of the example corpus.
pub fn examples() -> Vec<PathBuf> {
    let mut all = Vec::new();
    let root = fixture("examples");
    for folder in fs::read_dir(&root).unwrap().flatten() {
        for file in fs::read_dir(folder.path()).unwrap().flatten() {
            if file.path().extension().is_some_and(|e| e == "rq") {
                all.push(file.path());
            }
        }
    }
    all.sort();
    all
}
