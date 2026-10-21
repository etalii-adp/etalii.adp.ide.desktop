//! What the plugin learns from the registered folder itself, because FBL's contract hands a plugin files and nothing
//! else (etalii.adp spec 030, task T079): whether a folder is there, which folders a folder holds, how many files a
//! folder holds, and the files the rules select that are links.
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/ansible-structure/backend/
//! EtAlii.Adp.Diagram.AnsibleStructure/AnsibleFolderListing.cs`, the stand-in for gaps 1 and 2 of the definition's
//! companion (`ansible-structure.md`; findings B2, B3 and B11): a role is a folder that may hold no file, a role's
//! `meta/` counts by being there, a role's templates and files are counted and never opened, and a file that is a link
//! is read although the folder subject never selects it. It counts without reading; the only bytes it reads are a
//! linked file's. A folder named by the layout (`playbooks`, `roles`, a role's `tasks`) is entered whether or not it is
//! a link; a folder found by listing (a role, an environment) is skipped when it is one, so a link loop ends the
//! listing rather than the process. It holds the folder's path and nothing it read.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use adp_fbl::FblBinding;
use adp_fbl::plugins::PluginFile;
use adp_fbl::routing::{glob_matches, platform_ignores_case};

/// A listing of one folder, which need not exist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnsibleFolderListing {
    root: PathBuf,
}

impl AnsibleFolderListing {
    /// A listing of `folder`, made full against the current directory.
    pub fn new(folder: &Path) -> Self {
        AnsibleFolderListing {
            root: std::path::absolute(folder).unwrap_or_else(|_| folder.to_path_buf()),
        }
    }

    /// The registered folder's full path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Whether the folder at `relative` (`/`-separated, empty for the root) is there.
    pub fn folder_exists(&self, relative: &str) -> bool {
        self.full(relative).is_dir()
    }

    /// Whether a file is at `relative`, a linked one included. Nothing is opened.
    pub fn file_exists(&self, relative: &str) -> bool {
        self.full(relative).is_file()
    }

    /// The names of the folders directly in the folder at `relative`, in ordinal order, without those that are links;
    /// none when it is not there or cannot be listed.
    pub fn folders(&self, relative: &str) -> Vec<String> {
        let Ok(entries) = fs::read_dir(self.full(relative)) else {
            return Vec::new();
        };
        let mut names: Vec<String> = entries
            .flatten()
            .filter(|entry| entry.path().is_dir() && fs::symlink_metadata(entry.path()).is_ok_and(|m| !is_link(&m)))
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// How many files the folder at `relative` holds directly, linked ones included, even one whose target is gone; 0
    /// when it is not there or cannot be listed.
    pub fn file_count(&self, relative: &str) -> usize {
        let Ok(entries) = fs::read_dir(self.full(relative)) else {
            return 0;
        };
        // As a listing of the folder reports them: whatever is not a folder, a link that leads nowhere included.
        entries.flatten().filter(|entry| !entry.path().is_dir()).count()
    }

    /// The files `binding`'s file rules select that the folder subject does not, because the file or a folder on its
    /// way is a link, with their bytes, in ordinal order of their relative paths. A link is followed where a rule
    /// names a folder and not where a rule's `*` stands for one.
    pub fn linked_files(&self, binding: &FblBinding) -> Vec<PluginFile> {
        if !self.root.is_dir() {
            return Vec::new();
        }
        let caseless = platform_ignores_case();
        let mut found: BTreeMap<String, PathBuf> = BTreeMap::new();
        for rule in &binding.body.files {
            let segments: Vec<&str> = rule.glob.split('/').collect();
            let mut matches = Vec::new();
            collect(&segments, 0, &self.root, "", false, &mut matches);
            for (relative, full, linked) in matches {
                if !linked || binding.body.ignore.iter().any(|g| glob_matches(g, &relative, caseless)) {
                    continue;
                }
                found.entry(relative).or_insert(full);
            }
        }
        found
            .into_iter()
            .filter_map(|(relative, full)| fs::read(full).ok().map(|bytes| PluginFile::new(relative, bytes)))
            .collect()
    }

    fn full(&self, relative: &str) -> PathBuf {
        if relative.is_empty() {
            self.root.clone()
        } else {
            relative
                .split('/')
                .fold(self.root.clone(), |path, part| path.join(part))
        }
    }
}

/// Whether the entry is a link: a symbolic link, or on Windows any reparse point (a junction too).
fn is_link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        const REPARSE_POINT: u32 = 0x400;
        if metadata.file_attributes() & REPARSE_POINT != 0 {
            return true;
        }
    }
    metadata.file_type().is_symlink()
}

fn is_literal(segment: &str) -> bool {
    !segment.contains(['*', '?', '['])
}

/// The files below `directory` that the glob segments from `at` on match, and whether a link lies on the way.
fn collect(
    segments: &[&str],
    at: usize,
    directory: &Path,
    prefix: &str,
    linked: bool,
    out: &mut Vec<(String, PathBuf, bool)>,
) {
    let segment = segments[at];
    let last = at == segments.len() - 1;
    let mut entries: Vec<(String, PathBuf)> = if is_literal(segment) {
        let path = directory.join(segment);
        if (last && path.is_file()) || (!last && path.is_dir()) {
            vec![(segment.to_owned(), path)]
        } else {
            Vec::new()
        }
    } else {
        let Ok(listed) = fs::read_dir(directory) else {
            return;
        };
        listed
            .flatten()
            .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path()))
            .filter(|(name, _)| glob_matches(segment, name, platform_ignores_case()))
            .collect()
    };
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, path) in entries {
        let is_linked = fs::symlink_metadata(&path).is_ok_and(|m| is_link(&m));
        let relative = format!("{prefix}{name}");
        if last {
            // A file as a listing of its folder reports it: a link to a folder is a folder.
            if !path.is_dir() {
                out.push((relative, path, linked || is_linked));
            }
            continue;
        }
        if !path.is_dir() || (is_linked && !is_literal(segment)) {
            continue;
        }
        collect(
            segments,
            at + 1,
            &path,
            &format!("{relative}/"),
            linked || is_linked,
            out,
        );
    }
}
