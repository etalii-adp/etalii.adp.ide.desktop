//! What the tests share (etalii.adp spec 030, task T082): the fixtures the standalone host's module was tested against
//! (`EtAlii.Adp.Diagram.DotNetDependencyGraph.Tests/Fixtures` at `8b6ccfc`), its shipped example, and a copy of a tree
//! to change.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

/// The fixture `relative` names, under `tests/fixtures`.
pub fn fixture(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(relative)
}

/// The module's shipped example, `examples/example 1/pipeline-toolkit`.
pub fn example() -> PathBuf {
    fixture("examples/example 1/pipeline-toolkit")
}

/// The fixture NuGet cache, so no test depends on what is restored on the machine it runs on.
pub fn cache() -> PathBuf {
    fixture("cache")
}

/// Copies the tree at `from` to `to`.
pub fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap().flatten() {
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// Every file under `root` with its bytes, sorted: to prove a read wrote nothing.
pub fn snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(folder) = pending.pop() {
        for entry in fs::read_dir(folder).unwrap().flatten() {
            if entry.path().is_dir() {
                pending.push(entry.path());
            } else {
                files.push((entry.path(), fs::read(entry.path()).unwrap()));
            }
        }
    }
    files.sort();
    files
}
