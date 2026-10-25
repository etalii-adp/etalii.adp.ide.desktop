//! What the ported tests share (the standalone host's `HierarchicalRequirementFiles` and `WebShopFolder` at
//! a7fdf03): the module's examples, the vendored definition and fixtures of its tool type, and a folder holding a
//! copy of the web shop to change.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use adp_designer_hierarchical_requirement::body::RequirementBody;

/// The module's examples, copied from etalii.adp byte for byte.
pub fn examples_folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/examples")
}

pub fn vendor() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor/etalii.adp")
}

/// The vendored definition: the specification, the binding and the companion.
pub fn definition_folder() -> PathBuf {
    vendor().join("definitions/designers")
}

/// The tool type's fixtures as the FBL corpus holds them.
pub fn fixtures_folder() -> PathBuf {
    vendor().join("specifications/fbl/fixtures")
}

pub fn fixture(path: &str) -> PathBuf {
    fixtures_folder().join(path)
}

/// Reads a Markdown file alone, without a registration.
pub fn read(path: &Path) -> RequirementBody {
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    RequirementBody::read(fs::read(path).unwrap(), &name, None)
}

/// Reads a Markdown file and the registration beside it.
pub fn read_with(body: &Path, registration: &Path) -> RequirementBody {
    let name = body.file_name().unwrap().to_string_lossy().into_owned();
    let registration_name = registration.file_name().unwrap().to_string_lossy().into_owned();
    RequirementBody::read(
        fs::read(body).unwrap(),
        &name,
        Some((fs::read(registration).unwrap(), registration_name.as_str())),
    )
}

/// The web shop and its registration, read.
pub fn web_shop() -> RequirementBody {
    read_with(
        &examples_folder().join("web-shop.md"),
        &examples_folder().join("web-shop.adp"),
    )
}

/// A folder holding a copy of the web shop example and its registration.
pub struct WebShopFolder {
    folder: tempfile::TempDir,
}

impl WebShopFolder {
    pub fn new() -> Self {
        let folder = tempfile::tempdir().unwrap();
        for name in ["web-shop.md", "web-shop.adp"] {
            fs::copy(examples_folder().join(name), folder.path().join(name)).unwrap();
        }
        WebShopFolder { folder }
    }

    pub fn path(&self) -> &Path {
        self.folder.path()
    }

    pub fn body(&self) -> PathBuf {
        self.folder.path().join("web-shop.md")
    }

    pub fn registration(&self) -> PathBuf {
        self.folder.path().join("web-shop.adp")
    }
}
