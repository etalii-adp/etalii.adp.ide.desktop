//! Which template files a pipeline may follow, and what they say (etalii.adp spec 030, task T078).
//!
//! Ported from the standalone host's `src/diagrams/azure-devops-pipeline/backend/EtAlii.Adp.Diagram.AzureDevOpsPipeline/
//! PipelineTemplates.cs` at `8b6ccfc`: a reference into another repository, one whose path holds an expression, and one
//! that leaves the workspace are refused rather than followed; a leading slash is the workspace's root; anything else
//! is relative to the file that wrote it. What was read is remembered until [`PipelineTemplates::forget`].

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::lines::LineDocument;
use crate::model::{TemplateReference, UnresolvedReason};

/// The bounds of what one workspace's pipelines may follow, and the templates read so far.
#[derive(Debug)]
pub struct PipelineTemplates {
    workspace_root: PathBuf,
    documents: RefCell<HashMap<PathBuf, Option<LineDocument>>>,
}

/// `path` absolute, with `.` and `..` resolved without touching the file system (`Path.GetFullPath`).
pub fn full_path(path: &Path) -> PathBuf {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut out = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn contains_expression(path: &str) -> bool {
    path.contains("${{") || path.contains("$(") || path.contains("$[")
}

fn is_windows_absolute(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    (bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':') || raw.starts_with("//")
}

impl PipelineTemplates {
    pub fn new(workspace_root: &Path) -> Self {
        PipelineTemplates {
            workspace_root: full_path(workspace_root),
            documents: RefCell::new(HashMap::new()),
        }
    }

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    /// The file `reference` names, written in the file at `referencing`, or why it is not followed.
    pub fn locate(&self, reference: &TemplateReference, referencing: &Path) -> Result<PathBuf, UnresolvedReason> {
        if !reference.resource.is_empty() {
            return Err(UnresolvedReason::OtherRepository);
        }
        if reference.path.is_empty() || contains_expression(&reference.path) {
            return Err(UnresolvedReason::ParameterDependent);
        }
        let raw = reference.path.replace('\\', "/");
        let combined = if let Some(rooted) = raw.strip_prefix('/') {
            self.workspace_root.join(rooted.trim_start_matches('/'))
        } else if Path::new(&raw).has_root() || is_windows_absolute(&raw) {
            return Err(UnresolvedReason::OutsideWorkspace);
        } else {
            referencing
                .parent()
                .map_or_else(|| self.workspace_root.clone(), Path::to_path_buf)
                .join(&raw)
        };
        let full = full_path(&combined);
        if !full.starts_with(&self.workspace_root) {
            return Err(UnresolvedReason::OutsideWorkspace);
        }
        if full.is_file() {
            Ok(full)
        } else {
            Err(UnresolvedReason::NotFound)
        }
    }

    /// The template at `path` as lines, read once; none when it cannot be read.
    pub fn read(&self, path: &Path) -> Option<LineDocument> {
        if let Some(cached) = self.documents.borrow().get(path) {
            return cached.clone();
        }
        let document = fs::read(path).ok().map(|bytes| {
            let text = String::from_utf8_lossy(&bytes);
            LineDocument::parse(text.strip_prefix('\u{feff}').unwrap_or(&text))
        });
        self.documents.borrow_mut().insert(path.to_path_buf(), document.clone());
        document
    }

    /// A template's path as the reader sees it: relative to the workspace, with forward slashes.
    pub fn display(&self, path: &Path) -> String {
        path.strip_prefix(&self.workspace_root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    }

    /// Drops what was read, so the next reading sees the files as they are now.
    pub fn forget(&self) {
        self.documents.borrow_mut().clear();
    }
}
