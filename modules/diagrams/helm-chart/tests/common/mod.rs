//! What the tests share (etalii.adp spec 030, task T080): the fixture charts the standalone host's module was tested
//! against (`EtAlii.Adp.Diagram.HelmChart.Tests/Fixtures` at `8b6ccfc`), its `hello-world` example, the `nginx`
//! example the parity script opens, a copy of one registered as the standalone host's transcript registers it, and the
//! chart and model of one.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use adp_core::catalogue::Catalogue;
use adp_diagram::{DiagramElement, DiagramSession, DiagramTools, DiagramWorkspace};
use adp_diagram_helm_chart::model::HelmChart;
use adp_diagram_helm_chart::store::options;
use adp_disl::{DislModel, DislPluginFunctions};
use adp_fbl::plugins::{PluginFile, PluginReadRequest};

/// The fixture chart `name`, or one of the module's examples (`examples/hello-world`), or the parity script's nginx.
pub fn fixture(name: &str) -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    match name {
        "examples/nginx" => manifest.join("../../../parity/fixtures/helm/chart/nginx"),
        _ if name.starts_with("examples/") => manifest.join("tests").join(name),
        _ => manifest.join("tests/fixtures").join(name),
    }
}

/// The empty directories a fixture is about, which git cannot store (`HelmTranscript.EmptyDirectories`).
pub fn empty_directories(name: &str) -> &'static [&'static str] {
    match name {
        "empty-directories" => &["crds", "charts/hollow", "charts/holder/charts/empty"],
        _ => &[],
    }
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

/// A copy of the fixture `name` in a temporary folder with its empty directories made, and its registration: the first
/// `.adp` in it, or `helm-chart.adp` holding the bare MIME line core writes.
pub fn registered(name: &str) -> (tempfile::TempDir, PathBuf) {
    let folder = tempfile::tempdir().unwrap();
    copy_tree(&fixture(name), folder.path());
    for directory in empty_directories(name) {
        fs::create_dir_all(folder.path().join(directory)).unwrap();
    }
    let mut registrations: Vec<PathBuf> = fs::read_dir(folder.path())
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.to_string_lossy().ends_with(".adp"))
        .collect();
    registrations.sort();
    let registration = registrations.into_iter().next().unwrap_or_else(|| {
        let registration = folder.path().join("helm-chart.adp");
        fs::write(&registration, "helm/chart\r\n").unwrap();
        registration
    });
    (folder, registration)
}

/// Every file under `folder` as the folder body would hand it, in ordinal order, with the directory entries.
pub fn request(folder: &Path) -> PluginReadRequest {
    let definition = adp_diagram_helm_chart::definition::HelmDefinition::get();
    let mut files: Vec<PluginFile> = adp_fbl::routing::folder_files(&definition.binding, folder, None)
        .into_iter()
        .map(|f| PluginFile::new(f.relative_path, fs::read(f.full_path).unwrap()))
        .collect();
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    PluginReadRequest {
        files,
        supplementary_entries: options().supplementary_entries.map(|of| of(folder)).unwrap_or_default(),
    }
}

/// The chart the fixture `name` reads to.
pub fn read(name: &str) -> HelmChart {
    adp_diagram_helm_chart::reader::read(&request(&fixture(name)))
}

/// The definition's model of the fixture `name`.
pub fn model(name: &str) -> DislModel {
    adp_diagram_helm_chart::plugin::model_of(&read(name))
}

pub fn tools() -> DiagramTools {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    adp_diagram_helm_chart::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
        DislPluginFunctions::none()
    }))
}

/// A registered chart opened in a project.
pub struct Opened {
    pub folder: tempfile::TempDir,
    pub root: PathBuf,
    pub registration: PathBuf,
    pub workspace: DiagramWorkspace,
    pub session: DiagramSession,
}

pub fn open_registered(folder: tempfile::TempDir, registration: PathBuf) -> Opened {
    let root = folder.path().to_path_buf();
    let mut workspace = DiagramWorkspace::new(&root);
    let name = registration.strip_prefix(&root).unwrap().to_owned();
    let (opened, session) = workspace.open(&mut tools(), &name).unwrap();
    assert_eq!(opened.origin, "helm/chart");
    Opened {
        folder,
        root,
        registration,
        workspace,
        session,
    }
}

/// The fixture `name`, registered and opened.
pub fn open(name: &str) -> Opened {
    let (folder, registration) = registered(name);
    open_registered(folder, registration)
}

impl Opened {
    /// What the session delivers at baseline.
    pub fn delivered(&mut self) -> Vec<DiagramElement> {
        match self
            .session
            .baseline(self.workspace.documents())
            .unwrap()
            .into_iter()
            .next()
        {
            Some(adp_diagram::DiagramDelta::Add(elements)) => elements,
            None => Vec::new(),
            other => panic!("{other:?}"),
        }
    }
}
