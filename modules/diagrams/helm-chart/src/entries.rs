//! The directories of a chart folder the plugin needs and no file rule can select, handed to the folder body as
//! supplementary entries: each a path relative to the folder ending in `/`, with no bytes (etalii.adp spec 030, task
//! T080).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/helm-chart/backend/EtAlii.Adp.Diagram.HelmChart/
//! HelmFolderEntries.cs`, a stand-in for gap 3 of `helm-chart.md`. File rules select files, so a `crds/` with no file
//! in it, a directory under `charts/` with no `Chart.yaml`, and a directory counted among a subchart's deeper entries
//! would be lost. It lists the `crds` folder itself, every directory directly under `charts/`, and every directory
//! directly under each of those subcharts' own `charts/`; it never goes through a symbolic link and reads no file.

use std::fs;
use std::path::Path;

/// Whether `path` is a directory that is not a symbolic link (a reparse point).
fn folder(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.is_dir())
}

/// The directories directly in `path`, by name in ordinal order, never a symbolic link; none when it is not a
/// directory one may enter.
fn directories(path: &Path) -> Vec<String> {
    if !folder(path) {
        return Vec::new();
    }
    let mut names: Vec<String> = fs::read_dir(path)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// The directory entries of the chart folder `chart`, in ordinal order.
pub fn of(chart: &Path) -> Vec<String> {
    let mut entries = Vec::new();
    if folder(&chart.join("crds")) {
        entries.push("crds/".to_owned());
    }
    for subchart in directories(&chart.join("charts")) {
        entries.push(format!("charts/{subchart}/"));
        for deeper in directories(&chart.join("charts").join(&subchart).join("charts")) {
            entries.push(format!("charts/{subchart}/charts/{deeper}/"));
        }
    }
    entries.sort();
    entries
}
