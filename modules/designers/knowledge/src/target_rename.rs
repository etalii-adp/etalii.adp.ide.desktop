//! Keeps relations pointing at a table that was renamed (the standalone host's `KnowledgeTargetRename` at 8b6ccfc;
//! knowledge-designer Requirement 5.8): every knowledge file in the project whose relation names the renamed file, or
//! a file in the renamed folder, names it by its new path from the moment it has it. An undo of the rename is a rename
//! back, which comes through here again and puts each relation back as it was.

use std::path::{Path, PathBuf};

use adp_cel::{Map, Value};
use adp_documents::text::write_atomically;
use adp_fbl::ModelChange;

use crate::project;
use crate::relations::{self, SELF};
use crate::store;

/// Where a path is after the rename, or none when the rename did not move it.
fn now(path: &Path, from: &Path, to: &Path) -> Option<PathBuf> {
    if relations::same_path(path, from) {
        return Some(to.to_path_buf());
    }
    let (path, from) = (relations::full_path(path), relations::full_path(from));
    let inside = path
        .strip_prefix(&from)
        .ok()
        .filter(|rest| !rest.as_os_str().is_empty())?;
    Some(to.join(inside))
}

/// Follows a rename of `from` to `to` in the project at `root`: the files whose relations now name the new path,
/// each written and given back so that the open tables read it again.
pub fn renamed(root: &Path, from: &Path, to: &Path) -> Vec<PathBuf> {
    let mut written = Vec::new();
    for path in project::files(root) {
        let Some(body) = store::read(&path).body.filter(|b| b.read_only_reason().is_empty()) else {
            continue;
        };
        let changes: Vec<ModelChange> = body
            .table()
            .columns
            .iter()
            .filter(|p| p.value_type == "relation" && !p.target_file.is_empty() && p.target_file != SELF)
            .filter_map(|p| {
                let target = now(&relations::target_path(&path, &p.target_file), from, to)?;
                let attributes: Map = [("targetFile", Value::String(relations::target_name(&path, &target)))]
                    .into_iter()
                    .collect();
                Some(ModelChange::Set {
                    id: p.id.clone(),
                    attributes,
                })
            })
            .collect();
        if changes.is_empty() {
            continue;
        }
        // A file whose relations cannot follow is left as it is: the rename itself has happened.
        if let Ok(after) = body.change(&changes)
            && write_atomically(&path, &after).is_ok()
        {
            written.push(path);
        }
    }
    written
}
