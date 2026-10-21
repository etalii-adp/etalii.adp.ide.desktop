//! What the tests share (etalii.adp spec 030, task T079): the fixture trees the standalone host's module was tested
//! against (`EtAlii.Adp.Diagram.AnsibleStructure.Tests/Fixtures` at `8b6ccfc`), a copy of one to change, and the
//! project and model of one (`AnsibleModels.cs`).

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use adp_diagram_ansible_structure::definition::AnsibleDefinition;
use adp_diagram_ansible_structure::model::AnsibleProject;
use adp_diagram_ansible_structure::{plugin, reader};
use adp_disl::DislModel;

/// The fixture tree `name`.
pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

/// The project the fixture tree `name` reads to.
pub fn read(name: &str) -> AnsibleProject {
    reader::read_folder(&fixture(name), &AnsibleDefinition::get().binding)
}

/// The definition's model of the fixture tree `name`.
pub fn model(name: &str) -> DislModel {
    plugin::model_of(&read(name))
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

/// A copy of the fixture tree `name` in a temporary folder, registered as the standalone host's tests register it.
pub fn registered(name: &str) -> (tempfile::TempDir, PathBuf) {
    let folder = tempfile::tempdir().unwrap();
    copy_tree(&fixture(name), folder.path());
    let registration = folder.path().join("infrastructure.adp");
    fs::write(&registration, "ansible/structure\n").unwrap();
    (folder, registration)
}
