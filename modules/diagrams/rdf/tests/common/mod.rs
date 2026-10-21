//! What the tests share (etalii.adp spec 030, task T083): the fixtures the standalone host's module was tested against
//! (`EtAlii.Adp.Diagram.Rdf.Tests/Fixtures` at `8b6ccfc`), its examples (`src/diagrams/rdf/examples`), and a document,
//! a model and a target made from a text.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use adp_diagram_rdf::family::Target;
use adp_diagram_rdf::lines::LineDocument;
use adp_diagram_rdf::model::RdfModel;
use adp_diagram_rdf::parser;
use adp_diagram_rdf::store::RdfDocument;

/// The path of the fixture `name` (`Fixtures/{name}`).
pub fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

/// The text of the fixture `name`, as committed.
pub fn fixture(name: &str) -> String {
    String::from_utf8(fs::read(fixture_path(name)).unwrap()).unwrap()
}

/// The path of the example `relative` (`examples/{relative}`).
pub fn example_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/examples").join(relative)
}

/// The text of the example `relative`, as committed.
pub fn example(relative: &str) -> String {
    String::from_utf8(fs::read(example_path(relative)).unwrap()).unwrap()
}

/// What `text` states; panics when it does not parse.
pub fn model(text: &str) -> RdfModel {
    parser::parse(&LineDocument::parse(text)).unwrap()
}

/// The document `text` reads as, at a made-up body path.
pub fn document(text: &str) -> RdfDocument {
    RdfDocument::read(Path::new("/project/test.ttl"), text)
}

/// A target of `element_id` in the made-up body, opened as `origin`.
pub fn target<'a>(element_id: &'a str, origin: &'a str) -> Target<'a> {
    Target {
        element_id,
        body_path: Path::new("/project/test.ttl"),
        origin,
    }
}
