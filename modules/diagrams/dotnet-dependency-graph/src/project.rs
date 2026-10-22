//! Reads one project file for what the graph draws (etalii.adp spec 030, task T082), ported from the standalone host's
//! `ProjectReader.cs` at `8b6ccfc`: its `ProjectReference`s, a wildcard one expanded segment by segment as MSBuild
//! expands it; its `PackageReference`s, a version written on the reference or, the normal case under central package
//! management, the first `PackageVersion` naming the package in a `Directory.Packages.props` walking upward from the
//! project's folder; and the frameworks it targets, singular and plural.
//!
//! The file is read as written: no MSBuild condition is evaluated, no `Import` followed, no property expanded, and an
//! `Exclude` beside a wildcard is not applied. A reference whose version cannot be found is kept without one. This
//! reader never writes.

use std::fs;
use std::path::{Path, PathBuf};

use crate::graph::SolutionFailure;
use crate::{paths, xml};

const CENTRAL_PACKAGE_VERSIONS_FILE: &str = "Directory.Packages.props";

/// One `ProjectReference` as the project file declares it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectReferenceReading {
    /// The declaration verbatim.
    pub include: String,
    /// Where it points, resolved.
    pub absolute_path: PathBuf,
}

/// One `PackageReference`, with a version where one could be found; none for not discoverable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageReferenceReading {
    pub package_id: String,
    pub version: Option<String>,
}

/// What one read of a project file produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectReading {
    pub target_frameworks: Vec<String>,
    pub project_references: Vec<ProjectReferenceReading>,
    pub package_references: Vec<PackageReferenceReading>,
    pub failures: Vec<SolutionFailure>,
}

/// Reads the project file at `path`; never panics for a bad project file.
pub fn read(path: &Path) -> ProjectReading {
    let text = match xml::load(path) {
        Ok(text) => text,
        Err(error) => {
            return ProjectReading {
                failures: vec![SolutionFailure {
                    path: path.to_string_lossy().into_owned(),
                    reason: format!("The project file could not be read: {error}"),
                }],
                ..ProjectReading::default()
            };
        }
    };
    let Ok(document) = xml::parse(&text) else {
        return ProjectReading::default();
    };
    let folder = paths::normalise(path)
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    ProjectReading {
        target_frameworks: target_frameworks(&document),
        project_references: project_references(&document, &folder),
        package_references: package_references(&document, &folder),
        failures: Vec::new(),
    }
}

/// `TargetFramework` and `TargetFrameworks`, split on `;`, an unexpanded property left out, duplicates ignoring case.
fn target_frameworks(document: &roxmltree::Document<'_>) -> Vec<String> {
    let mut values: Vec<String> = Vec::new();
    for element in document
        .descendants()
        .filter(|n| n.is_element() && matches!(n.tag_name().name(), "TargetFramework" | "TargetFrameworks"))
    {
        for value in xml::value(element).split(';').map(str::trim).filter(|v| !v.is_empty()) {
            if value.contains('$') || values.iter().any(|v| paths::equal_ignoring_case(v, value)) {
                continue;
            }
            values.push(value.to_owned());
        }
    }
    values
}

/// Project-to-project references, each resolved to a full path.
fn project_references(document: &roxmltree::Document<'_>, folder: &Path) -> Vec<ProjectReferenceReading> {
    let mut references = Vec::new();
    for element in xml::elements(document, "ProjectReference") {
        let Some(include) = element.attribute("Include").filter(|i| !i.trim().is_empty()) else {
            continue;
        };
        let normalised = paths::with_platform_separators(include);
        if normalised.contains('*') {
            for found in expand_wildcard(folder, &normalised) {
                references.push(ProjectReferenceReading {
                    include: include.to_owned(),
                    absolute_path: found,
                });
            }
            continue;
        }
        references.push(ProjectReferenceReading {
            include: include.to_owned(),
            absolute_path: paths::normalise(&folder.join(normalised)),
        });
    }
    references
}

/// Every file a wildcard `Include` names, expanded segment by segment so a `*` at any depth works. The entries of a
/// folder are taken in ordinal order of their names, so a reading is the same on every run.
fn expand_wildcard(folder: &Path, pattern: &str) -> Vec<PathBuf> {
    let segments: Vec<&str> = pattern
        .split(std::path::MAIN_SEPARATOR)
        .filter(|s| !s.is_empty())
        .collect();
    let mut roots = vec![folder.to_path_buf()];
    for (index, segment) in segments.iter().enumerate() {
        let last = index == segments.len() - 1;
        let mut next = Vec::new();
        for root in &roots {
            if !segment.contains('*') {
                next.push(paths::normalise(&root.join(segment)));
                continue;
            }
            let Ok(entries) = fs::read_dir(root) else {
                continue;
            };
            let mut found: Vec<PathBuf> = entries
                .flatten()
                .filter(|e| if last { e.path().is_file() } else { e.path().is_dir() })
                .filter(|e| paths::matches(segment, &e.file_name().to_string_lossy()))
                .map(|e| e.path())
                .collect();
            found.sort();
            next.extend(found);
        }
        roots = next;
    }
    roots
        .into_iter()
        .filter(|p| p.is_file())
        .map(|p| paths::normalise(&p))
        .collect()
}

/// Package references, with a version where one can be found.
fn package_references(document: &roxmltree::Document<'_>, folder: &Path) -> Vec<PackageReferenceReading> {
    let mut references = Vec::new();
    for element in xml::elements(document, "PackageReference") {
        let Some(id) = element
            .attribute("Include")
            .or_else(|| element.attribute("Update"))
            .filter(|i| !i.trim().is_empty())
        else {
            continue;
        };
        let written = element.attribute("Version").map(str::to_owned).or_else(|| {
            element
                .children()
                .find(|c| c.is_element() && c.tag_name().name() == "Version")
                .map(xml::value)
        });
        let version = match written {
            Some(v) if !v.trim().is_empty() => Some(v),
            _ => centrally_managed_version_of(id, folder),
        };
        references.push(PackageReferenceReading {
            package_id: id.trim().to_owned(),
            version: version.filter(|v| !v.trim().is_empty()).map(|v| v.trim().to_owned()),
        });
    }
    references
}

/// The version the nearest `Directory.Packages.props` naming `package_id` gives, walking upward from `start`.
fn centrally_managed_version_of(package_id: &str, start: &Path) -> Option<String> {
    let mut folder = Some(start);
    while let Some(current) = folder {
        let candidate = current.join(CENTRAL_PACKAGE_VERSIONS_FILE);
        if candidate.is_file()
            && let Some(version) = package_version_in(&candidate, package_id)
        {
            return Some(version);
        }
        folder = current.parent();
    }
    None
}

/// The `PackageVersion` entry naming `package_id` in the props file at `path`, if any; none when it cannot be read.
fn package_version_in(path: &Path, package_id: &str) -> Option<String> {
    let text = xml::load(path).ok()?;
    let document = xml::parse(&text).ok()?;
    xml::elements(&document, "PackageVersion")
        .filter(|e| {
            e.attribute("Include")
                .is_some_and(|i| paths::equal_ignoring_case(i, package_id))
        })
        .filter_map(|e| e.attribute("Version"))
        .find(|v| !v.trim().is_empty())
        .map(str::to_owned)
}

/// Every `Directory.Packages.props` above the project at `path`, nearest first: what a version bump lands in.
pub fn central_package_files(path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut folder = path.parent();
    while let Some(current) = folder {
        let candidate = current.join(CENTRAL_PACKAGE_VERSIONS_FILE);
        if candidate.is_file() {
            files.push(candidate);
        }
        folder = current.parent();
    }
    files
}
