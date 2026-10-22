//! The generated inputs of the benchmarks (etalii.adp spec 030, research R12, task T090): a workspace of 5,000 files
//! and a canvas of 1,000 elements, written by code rather than kept as fixtures, the same bytes on every run.
//!
//! Each benchmark includes this file by path, so what it does not use of it is allowed to be unused.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

/// The files of the workspace SC-005, SC-007 and SC-008 name.
pub const FILES: usize = 5_000;
/// The elements of the canvas SC-006 names.
pub const ELEMENTS: usize = 1_000;

/// The canvas's elements in a grid of this many columns, each depending on the one below it.
const COLUMNS: usize = 40;

/// Writes a workspace of `files` files into `root`: ten areas of ten modules each, the files spread over them, in the
/// formats a workspace mixes (Markdown, JSON, YAML, XML and plain text), each a few hundred bytes. Returns the files'
/// paths in the order they were written.
pub fn workspace(root: &Path, files: usize) -> Vec<PathBuf> {
    let mut written = Vec::with_capacity(files);
    for index in 0..files {
        let area = index % 10;
        let module = (index / 10) % 10;
        let folder = root.join(format!("area-{area:02}")).join(format!("module-{module:02}"));
        if index < 100 {
            fs::create_dir_all(&folder).expect("the folder is created");
        }
        let (extension, text) = match index % 5 {
            0 => (
                "md",
                format!(
                    "# Note {index}\n\nWhat module {module} of area {area} keeps: item {index}, written by the benchmark.\n\n- one\n- two\n- three\n"
                ),
            ),
            1 => (
                "json",
                format!(
                    "{{\n  \"id\": {index},\n  \"area\": {area},\n  \"module\": {module},\n  \"tags\": [\"generated\", \"benchmark\"],\n  \"weight\": {}\n}}\n",
                    (index * 7919) % 1000
                ),
            ),
            2 => (
                "yaml",
                format!(
                    "id: {index}\narea: {area}\nmodule: {module}\ntags:\n  - generated\n  - benchmark\nweight: {}\n",
                    (index * 104_729) % 1000
                ),
            ),
            3 => (
                "xml",
                format!(
                    "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<item id=\"{index}\" area=\"{area}\" module=\"{module}\">\n  <tag>generated</tag>\n  <tag>benchmark</tag>\n</item>\n"
                ),
            ),
            _ => (
                "txt",
                format!(
                    "Item {index} of module {module} in area {area}.\nWritten by the benchmark, read by the explorer.\n"
                ),
            ),
        };
        let path = folder.join(format!("item-{index:04}.{extension}"));
        fs::write(&path, text).expect("the file is written");
        written.push(path);
    }
    written
}

/// The name of the canvas's element `index`.
pub fn element_label(index: usize) -> String {
    format!("Element {index:04}")
}

/// Writes a dependency graph of `elements` elements into `root` as `generated.dgr`, registered by `generated.adp`:
/// a grid of [`COLUMNS`] columns, each element placed (`x` and `row`, as the dependency graph keeps them) and depending
/// on the one below it, so every relation runs straight down and the space between two elements of a row is empty.
/// Returns the diagram's body.
pub fn canvas(root: &Path, elements: usize) -> PathBuf {
    let mut text =
        String::from("dependencies: 1\n# A generated canvas of the benchmarks (spec 030, T090).\nelements:\n");
    for index in 0..elements {
        let column = index % COLUMNS;
        let row = index / COLUMNS;
        text += &format!(
            "  - id: e{index:04}\n    label: {}\n    x: {}\n    row: {}\n",
            element_label(index),
            column * 320,
            row as i64 * 2
        );
    }
    text += "relations:\n";
    for index in 0..elements.saturating_sub(COLUMNS) {
        text += &format!(
            "  - id: r{index:04}\n    from: e{index:04}\n    to: e{:04}\n",
            index + COLUMNS
        );
    }
    let body = root.join("generated.dgr");
    fs::write(&body, text).expect("the diagram is written");
    fs::write(root.join("generated.adp"), "generic/dependencies\n").expect("the registration is written");
    body
}
