//! The project list: add, validate, remove and open projects, kept across restarts (etalii.adp spec 030, FR-011,
//! task T025).
//!
//! As in the standalone host (`EtAlii.Adp.Projects`), a project is a folder the person added. Its path is validated
//! when it is added and again when it is opened; a folder that has gone is reported, never silently dropped. The list
//! lives in `projects.json` in the desktop host's own state folder, never in a workspace.

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use adp_core::state::StateFolder;
use serde::{Deserialize, Serialize};

/// Why something was not done: one sentence a person can read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal(pub String);

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Refusal {}

/// One project: a folder, its name and when it was added and last opened (milliseconds since the Unix epoch).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub added_at: u128,
    pub opened_at: Option<u128>,
}

#[derive(Default, Serialize, Deserialize)]
struct Stored {
    projects: Vec<Project>,
}

/// The list, read from and written to the state folder.
pub struct ProjectList {
    state: StateFolder,
    projects: Vec<Project>,
}

impl ProjectList {
    pub fn load(state: StateFolder) -> Result<Self, Refusal> {
        let stored: Stored = state
            .load(&state.projects())
            .map_err(|e| Refusal(format!("The project list could not be read: {e}")))?
            .unwrap_or_default();
        Ok(Self {
            state,
            projects: stored.projects,
        })
    }

    /// The projects, most recently opened first, then most recently added.
    pub fn projects(&self) -> Vec<Project> {
        let mut sorted = self.projects.clone();
        sorted.sort_by(|a, b| b.opened_at.cmp(&a.opened_at).then(b.added_at.cmp(&a.added_at)));
        sorted
    }

    /// Adds a folder after checking that it exists, is a folder, can be read and is not listed yet.
    pub fn add(&mut self, path: impl AsRef<Path>) -> Result<Project, Refusal> {
        let path = validate(path.as_ref())?;
        if self.projects.iter().any(|p| same_path(&p.path, &path)) {
            return Err(Refusal(format!("{} is already in the list.", path.display())));
        }
        let now = now();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        let project = Project {
            id: new_id(&path, now),
            name,
            path,
            added_at: now,
            opened_at: None,
        };
        self.projects.push(project.clone());
        self.save()?;
        Ok(project)
    }

    /// Removes a project from the list; its folder is not touched.
    pub fn remove(&mut self, id: &str) -> Result<(), Refusal> {
        let before = self.projects.len();
        self.projects.retain(|p| p.id != id);
        if self.projects.len() == before {
            return Err(Refusal("That project is not in the list.".into()));
        }
        self.save()
    }

    /// Opens a project: checks its folder again and records when it was opened. A folder that is gone is reported and
    /// stays in the list.
    pub fn open(&mut self, id: &str) -> Result<Project, Refusal> {
        let project = self
            .projects
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| Refusal("That project is not in the list.".into()))?;
        validate(&project.path)?;
        project.opened_at = Some(now());
        let opened = project.clone();
        self.save()?;
        Ok(opened)
    }

    fn save(&self) -> Result<(), Refusal> {
        self.state
            .save(
                &self.state.projects(),
                &Stored {
                    projects: self.projects.clone(),
                },
            )
            .map_err(|e| Refusal(format!("The project list could not be saved: {e}")))
    }
}

fn validate(path: &Path) -> Result<PathBuf, Refusal> {
    let shown = path.display();
    let metadata = std::fs::metadata(path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => Refusal(format!("The folder {shown} does not exist.")),
        std::io::ErrorKind::PermissionDenied => Refusal(format!("The folder {shown} cannot be read.")),
        _ => Refusal(format!("The folder {shown} cannot be opened: {e}")),
    })?;
    if !metadata.is_dir() {
        return Err(Refusal(format!("{shown} is not a folder.")));
    }
    std::fs::read_dir(path).map_err(|_| Refusal(format!("The folder {shown} cannot be read.")))?;
    Ok(std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf()))
}

fn same_path(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// A short id that stays stable for the project: a hash of its path and the moment it was added.
fn new_id(path: &Path, now: u128) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    now.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
