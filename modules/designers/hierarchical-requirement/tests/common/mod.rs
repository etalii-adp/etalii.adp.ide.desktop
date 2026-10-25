//! What the ported tests share (the standalone host's `HierarchicalRequirementFiles` and `WebShopFolder` at
//! a7fdf03): the module's examples, the vendored definition and fixtures of its tool type, and a folder holding a
//! copy of the web shop to change.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use adp_designer_hierarchical_requirement::body::RequirementBody;
use adp_desl::hierarchy::model::{Gesture, SurfaceChange, SurfaceModel};

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

impl WebShopFolder {
    /// Another program changes the body, as an agent does: the text read, changed by `change` and written back.
    pub fn write_as_an_agent(&self, change: impl FnOnce(&str) -> String) {
        let text = fs::read_to_string(self.body()).unwrap();
        fs::write(self.body(), change(&text)).unwrap();
    }
}

/// The line ending `text` was written with, so an inserted line keeps it.
pub fn newline_of(text: &str) -> &'static str {
    if text.contains("\r\n") { "\r\n" } else { "\n" }
}

/// A gesture of `operation` on `target`, with its one param.
pub fn set(operation: &str, target: &str, param: &str, value: &str) -> Gesture {
    Gesture {
        edit_id: format!("edit-{operation}-{target}"),
        operation: operation.into(),
        target_id: target.into(),
        params: [(param.to_owned(), value.to_owned())].into_iter().collect(),
    }
}

/// The outcome among `changes`: whether the gesture was written, and why not.
pub fn outcome(changes: &[SurfaceChange]) -> Option<(bool, String)> {
    changes.iter().find_map(|change| match change {
        SurfaceChange::Outcome(outcome) => Some((outcome.written, outcome.error.clone())),
        SurfaceChange::Model(_) => None,
    })
}

/// The models among `changes`, in order.
pub fn models(changes: &[SurfaceChange]) -> Vec<SurfaceModel> {
    changes
        .iter()
        .filter_map(|change| match change {
            SurfaceChange::Model(model) => Some((**model).clone()),
            SurfaceChange::Outcome(_) => None,
        })
        .collect()
}

/// Params in their written form, `name=value|name=value`, none for an empty text.
pub fn params(written: &str) -> std::collections::BTreeMap<String, String> {
    written
        .split('|')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (name, value) = pair.split_once('=').unwrap();
            (name.to_owned(), value.to_owned())
        })
        .collect()
}

/// A gesture of `operation` on `target`, with its params written as [`params`] reads them.
pub fn gesture(operation: &str, target: &str, written: &str) -> Gesture {
    Gesture {
        edit_id: format!("edit-{operation}-{target}"),
        operation: operation.into(),
        target_id: target.into(),
        params: params(written),
    }
}

/// What step `step` of the conformance fixture `fixture` expects.
pub fn expect(fixture: &str, step: usize) -> String {
    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(self::fixture(fixture).join("fixture.json")).unwrap()).unwrap();
    json["steps"][step]["expect"].as_str().unwrap().to_owned()
}

/// Runs `operation` on `target` through `session` and checks it was written.
pub fn run(
    session: &mut impl adp_desl::hierarchy::session::HierarchySurfaceSession,
    operation: &str,
    target: &str,
    written: &str,
) {
    session.gesture(gesture(operation, target, written)).unwrap();
    assert_eq!(
        Some((true, String::new())),
        outcome(&session.take_changes()),
        "{operation} {target}"
    );
}
