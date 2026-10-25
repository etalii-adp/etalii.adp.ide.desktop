//! The per-user state folder and its JSON files (etalii.adp spec 030, research R8, FR-036, task T017).
//!
//! The desktop host keeps its own state (the project list, the problem cache per project, the window layout) in the
//! platform's per-user application data folder, apart from the standalone host's, and never in a workspace. Every save
//! writes a temporary file beside the target and renames it over the target, so a crash leaves the old file or the
//! new one, never half of either.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

/// The folder the desktop host keeps its state in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateFolder {
    root: PathBuf,
}

impl StateFolder {
    /// The platform's folder: `%AppData%\EtAlii\ADP Desktop` on Windows,
    /// `~/Library/Application Support/net.etalii.adp.desktop` on macOS and `$XDG_DATA_HOME/etalii-adp-desktop` (by
    /// default `~/.local/share/etalii-adp-desktop`) on Linux.
    #[cfg(feature = "platform-state")]
    pub fn for_platform() -> io::Result<Self> {
        let base = directories::BaseDirs::new()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "The home folder could not be found."))?;
        Ok(Self::at(platform_folder(base.data_dir())))
    }

    /// A folder of the caller's choosing, for tests and for a portable install.
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The file of the project list.
    pub fn projects(&self) -> PathBuf {
        self.root.join("projects.json")
    }

    /// The cache of one project's findings.
    pub fn problems(&self, project_id: &str) -> PathBuf {
        self.root.join("problems").join(format!("{project_id}.json"))
    }

    /// The window layout: pane sizes and open tabs.
    pub fn layout(&self) -> PathBuf {
        self.root.join("layout.json")
    }

    /// Reads a JSON file of the state folder; a missing file reads as `None`.
    pub fn load<T: DeserializeOwned>(&self, path: &Path) -> io::Result<Option<T>> {
        match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{}: {e}", path.display()))),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Writes a JSON file of the state folder atomically, creating its folder when needed.
    pub fn save<T: Serialize>(&self, path: &Path, value: &T) -> io::Result<()> {
        let folder = path.parent().unwrap_or(&self.root);
        fs::create_dir_all(folder)?;
        let mut json = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
        json.push(b'\n');
        let temporary = folder.join(format!(
            ".{}.{}.tmp",
            path.file_name().and_then(|n| n.to_str()).unwrap_or("state"),
            std::process::id()
        ));
        {
            let mut file = fs::File::create(&temporary)?;
            file.write_all(&json)?;
            file.sync_all()?;
        }
        fs::rename(&temporary, path).inspect_err(|_| {
            let _ = fs::remove_file(&temporary);
        })
    }
}

#[cfg(all(feature = "platform-state", target_os = "windows"))]
fn platform_folder(data: &Path) -> PathBuf {
    data.join("EtAlii").join("ADP Desktop")
}

#[cfg(all(feature = "platform-state", target_os = "macos"))]
fn platform_folder(data: &Path) -> PathBuf {
    data.join("net.etalii.adp.desktop")
}

#[cfg(all(feature = "platform-state", not(any(target_os = "windows", target_os = "macos"))))]
fn platform_folder(data: &Path) -> PathBuf {
    data.join("etalii-adp-desktop")
}
