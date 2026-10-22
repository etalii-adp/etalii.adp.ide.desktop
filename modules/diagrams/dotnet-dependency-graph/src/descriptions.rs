//! A package's description, from the local NuGet cache and nowhere else (etalii.adp spec 030, task T082), ported from
//! the standalone host's `PackageDescriptionReader.cs` at `8b6ccfc`: the `.nuspec` beside the restored package in the
//! global packages folder, lowercased as NuGet restores it. No feed is queried and no network call is made: a package
//! never restored on this machine has no description, which is absence, never an error. An empty description is the
//! empty text, not absence. This reader never writes.

use std::path::{Path, PathBuf};

use crate::xml;

/// Reads descriptions from one global packages folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageDescriptionReader {
    root: PathBuf,
}

impl Default for PackageDescriptionReader {
    /// Reads from the machine's own global packages folder.
    fn default() -> Self {
        PackageDescriptionReader::new(Self::default_global_packages_folder())
    }
}

impl PackageDescriptionReader {
    /// Reads from the cache at `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        PackageDescriptionReader { root: root.into() }
    }

    /// Where NuGet keeps restored packages: `NUGET_PACKAGES` where it is set, else `~/.nuget/packages`.
    pub fn default_global_packages_folder() -> PathBuf {
        if let Some(configured) = std::env::var_os("NUGET_PACKAGES").filter(|v| !v.to_string_lossy().trim().is_empty())
        {
            return PathBuf::from(configured);
        }
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_default();
        home.join(".nuget").join("packages")
    }

    /// Where a cached package's `.nuspec` would be: the id and the version lowercased, as NuGet restores them.
    pub fn nuspec_path_for(root: &Path, package_id: &str, version: &str) -> PathBuf {
        let id = package_id.to_lowercase();
        root.join(&id).join(version.to_lowercase()).join(format!("{id}.nuspec"))
    }

    /// The description of `package_id` at the first of `versions` the cache holds one for; none when none does.
    pub fn read(&self, package_id: &str, versions: &[String]) -> Option<String> {
        versions.iter().find_map(|version| self.read_one(package_id, version))
    }

    fn read_one(&self, package_id: &str, version: &str) -> Option<String> {
        let nuspec = Self::nuspec_path_for(&self.root, package_id, version);
        if !nuspec.is_file() {
            return None;
        }
        let text = xml::load(&nuspec).ok()?;
        let document = xml::parse(&text).ok()?;
        xml::elements(&document, "description")
            .next()
            .map(|d| xml::value(d).trim().to_owned())
    }
}
