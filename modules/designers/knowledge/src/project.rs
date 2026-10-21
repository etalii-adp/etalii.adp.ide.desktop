//! The knowledge files of a project, for what one of them has to know of the others: which name it by a relation
//! (the standalone host's `KnowledgeProject` at 8b6ccfc; knowledge-designer Requirement 5.8).

use std::path::{Path, PathBuf};

use crate::relations::{self, SELF};
use crate::store;

/// What a knowledge file says of itself in each of its three formats: a file without it is not read.
const MARK: &str = "etalii/knowledge";

const EXTENSIONS: [&str; 4] = ["yaml", "yml", "json", "xml"];

/// Whether a file's text says it is a knowledge file.
fn says(file: &Path) -> bool {
    std::fs::read(file).is_ok_and(|bytes| String::from_utf8_lossy(&bytes).contains(MARK))
}

/// The files under a folder that say they are knowledge files, folders of tools' own left out.
pub fn files(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let (mut found, mut folders) = (Vec::new(), Vec::new());
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            folders.push(path);
        } else if path
            .extension()
            .is_some_and(|e| EXTENSIONS.contains(&e.to_string_lossy().to_lowercase().as_str()))
            && says(&path)
        {
            found.push(path);
        }
    }
    for inner in folders {
        let name = inner
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !name.starts_with('.') && !matches!(name.as_str(), "node_modules" | "bin" | "obj") {
            found.extend(files(&inner));
        }
    }
    found
}

/// The tables a relation of one file can relate to: the file itself first, then the project's other knowledge files
/// that can be read, each as the file names it and under its table's name.
pub fn targets(root: &Path, body_path: &Path) -> Vec<(String, String)> {
    let mut targets = vec![(SELF.to_owned(), "This table".to_owned())];
    if root.as_os_str().is_empty() {
        return targets;
    }
    let mut others: Vec<PathBuf> = files(root)
        .into_iter()
        .filter(|p| !relations::same_path(p, body_path))
        .collect();
    others.sort_by_key(|p| p.to_string_lossy().to_lowercase());
    for path in others {
        if let Some(body) = store::read(&path).body.filter(|b| b.unreadable().is_empty()) {
            let name = if body.table().name.is_empty() {
                path.file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default()
            } else {
                body.table().name.clone()
            };
            targets.push((relations::target_name(body_path, &path), name));
        }
    }
    targets
}

/// How many rows of the project's other knowledge files relate to any of the given rows of one file: what deleting
/// those rows would leave pointing at nothing.
pub fn rows_relating_to(root: &Path, body_path: &Path, row_ids: &[String]) -> usize {
    let mut count = 0;
    for path in files(root).into_iter().filter(|p| !relations::same_path(p, body_path)) {
        let Some(body) = store::read(&path).body else {
            continue;
        };
        // The side of a two-way relation that holds no values of its own names nothing.
        let naming: Vec<&str> = body
            .table()
            .columns
            .iter()
            .filter(|p| {
                p.value_type == "relation"
                    && !p.is_computed
                    && !p.target_file.is_empty()
                    && p.target_file != SELF
                    && relations::same_path(&relations::target_path(&path, &p.target_file), body_path)
            })
            .map(|p| p.id.as_str())
            .collect();
        if naming.is_empty() {
            continue;
        }
        count += body
            .table()
            .rows
            .iter()
            .filter(|row| {
                row.cells.iter().any(|c| {
                    c.key == "rows"
                        && naming.contains(&c.column_id.as_str())
                        && c.values.iter().any(|v| row_ids.contains(v))
                })
            })
            .count();
    }
    count
}
