//! Findings, validated and cached per project (etalii.adp spec 030, FR-019, task T028).
//!
//! Ported from the standalone host's `EtAlii.Adp.Problems` at `8b6ccfc`: one set of findings per project, kept outside
//! the project in the state folder, one JSON file each. A missing, unreadable or other-version cache reads as a set
//! never validated, not as an error. Validating a file, a folder or the whole project replaces exactly that scope's
//! findings. A finding whose file changed since it was validated is reported as stale until it is validated again. At
//! most 1000 findings are reported; the counts cover all of them.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use adp_core::state::StateFolder;
use serde::{Deserialize, Serialize};

/// The cache format; a cache written in any other is ignored, not migrated.
const CACHE_FORMAT: u32 = 1;
/// How many findings one answer carries at most.
pub const MAX_REPORTED: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

/// One finding: where it is, how bad it is, and what it says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub severity: Severity,
    pub message: String,
    /// The file, relative to the project with `/`.
    pub file: String,
    /// The line and column, from 1, when the finding has a place in the text.
    pub line: Option<u32>,
    pub column: Option<u32>,
    /// The element, when the finding is about one.
    pub element: Option<String>,
    /// The rule that produced it.
    pub rule: String,
}

/// A finding with whether its file changed since it was validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reported {
    pub finding: Finding,
    pub stale: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SetState {
    NeverValidated,
    Validated,
}

/// What the errors and warnings panel shows for a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProblemSet {
    pub state: SetState,
    pub problems: Vec<Reported>,
    pub errors: usize,
    pub warnings: usize,
    /// Counted apart from the warnings, so a note never makes a clean file look wrong.
    pub infos: usize,
    /// The number reported when there were more, otherwise 0.
    pub truncated_at: usize,
}

/// What a file looked like when it was validated: its length and modification time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct Stamp {
    length: u64,
    modified_nanos: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Cached {
    finding: Finding,
    stamp: Option<Stamp>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheFile {
    format: u32,
    state: SetState,
    problems: Vec<Cached>,
}

/// Validates one file; the project's validators (tool types, editors' schemas) implement it.
pub trait Validator {
    /// Whether this validator has anything to say about the file.
    fn applies_to(&self, file: &str) -> bool;
    fn validate(&self, root: &Path, file: &str) -> Vec<Finding>;
}

/// What to validate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    File(String),
    Folder(String),
    Project,
}

impl Scope {
    fn contains(&self, file: &str) -> bool {
        match self {
            Scope::File(f) => f == file,
            Scope::Folder(folder) => folder.is_empty() || file.starts_with(&format!("{folder}/")),
            Scope::Project => true,
        }
    }
}

/// One project's findings and their cache.
pub struct ProblemStore {
    state: StateFolder,
    project_id: String,
    root: PathBuf,
    set_state: SetState,
    problems: Vec<Cached>,
}

impl ProblemStore {
    /// Reads the project's cache; anything wrong with it reads as never validated.
    pub fn load(state: StateFolder, project_id: &str, root: &Path) -> Self {
        let cache: Option<CacheFile> = state.load(&state.problems(project_id)).ok().flatten();
        let (set_state, problems) = match cache {
            Some(c) if c.format == CACHE_FORMAT => (c.state, c.problems),
            _ => (SetState::NeverValidated, Vec::new()),
        };
        Self {
            state,
            project_id: project_id.into(),
            root: root.to_path_buf(),
            set_state,
            problems,
        }
    }

    pub fn get(&self) -> ProblemSet {
        let count = |s| self.problems.iter().filter(|p| p.finding.severity == s).count();
        let problems = self
            .problems
            .iter()
            .take(MAX_REPORTED)
            .map(|p| Reported {
                finding: p.finding.clone(),
                stale: p.stamp != stamp(&self.root, &p.finding.file),
            })
            .collect();
        ProblemSet {
            state: self.set_state,
            problems,
            errors: count(Severity::Error),
            warnings: count(Severity::Warning),
            infos: count(Severity::Info),
            truncated_at: if self.problems.len() > MAX_REPORTED {
                MAX_REPORTED
            } else {
                0
            },
        }
    }

    /// Validates every file in the scope with the validators that apply, replaces the scope's findings, and saves the
    /// cache. Files are those of the project below the scope, `.git` and its like excluded.
    pub fn validate(&mut self, scope: &Scope, validators: &[&dyn Validator]) -> std::io::Result<()> {
        let files = files_in(&self.root, scope)?;
        self.problems.retain(|p| !scope.contains(&p.finding.file));
        for file in files {
            let stamp = stamp(&self.root, &file);
            for validator in validators.iter().filter(|v| v.applies_to(&file)) {
                for finding in validator.validate(&self.root, &file) {
                    self.problems.push(Cached { finding, stamp });
                }
            }
        }
        self.problems.sort_by(|a, b| {
            a.finding
                .severity
                .cmp(&b.finding.severity)
                .then_with(|| a.finding.file.cmp(&b.finding.file))
                .then(a.finding.line.cmp(&b.finding.line))
        });
        if *scope == Scope::Project {
            self.set_state = SetState::Validated;
        }
        self.save()
    }

    fn save(&self) -> std::io::Result<()> {
        let cache = CacheFile {
            format: CACHE_FORMAT,
            state: self.set_state,
            problems: self.problems.clone(),
        };
        self.state.save(&self.state.problems(&self.project_id), &cache)
    }
}

fn stamp(root: &Path, file: &str) -> Option<Stamp> {
    let metadata = std::fs::metadata(root.join(file)).ok()?;
    let modified_nanos = metadata.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_nanos();
    Some(Stamp {
        length: metadata.len(),
        modified_nanos,
    })
}

fn files_in(root: &Path, scope: &Scope) -> std::io::Result<Vec<String>> {
    let mut files = Vec::new();
    match scope {
        Scope::File(file) => {
            if root.join(file).is_file() {
                files.push(file.clone());
            }
        }
        Scope::Folder(folder) => walk(root, folder, &mut files)?,
        Scope::Project => walk(root, "", &mut files)?,
    }
    Ok(files)
}

fn walk(root: &Path, relative: &str, files: &mut Vec<String>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(root.join(relative))? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = if relative.is_empty() {
            name.clone()
        } else {
            format!("{relative}/{name}")
        };
        if entry.file_type()?.is_dir() {
            if ![".git", ".hg", ".svn"].contains(&name.as_str()) {
                walk(root, &path, files)?;
            }
        } else {
            files.push(path);
        }
    }
    Ok(())
}
