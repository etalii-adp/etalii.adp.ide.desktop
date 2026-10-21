//! Relations between knowledge files (the standalone host's `KnowledgeRelations` at 8b6ccfc; knowledge-designer
//! Requirement 5): where a relation points, what its values are called, and the side of a two-way relation that
//! holds no values of its own.
//!
//! - **A relation stores the target's path relative to its own file, and its values are the target's row ids.**
//!   Neither the target's name nor a row's title is ever copied into the file that relates to it.
//! - **One side holds the values.** The other side of a two-way relation is a property marked computed, with no
//!   cells: what it shows is read from the side that holds them, each time, so the two sides cannot disagree.

use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::rc::Rc;

use adp_desl::table::content::{ContentCell, ContentRow};

use crate::model::{KnowledgeProperty, KnowledgeTable};

/// What a relation to the file itself names as its target.
pub const SELF: &str = ".";

/// What a relation property points at: the table of its target file, or why there is none to show.
#[derive(Debug, Clone)]
pub struct KnowledgeTarget {
    pub path: PathBuf,
    pub table: Option<KnowledgeTable>,
    /// Why there is no table, as a sentence; empty when there is one.
    pub problem: String,
}

/// What a relation property of a file points at, or none where relations are not resolved.
pub type TargetOf<'a> = &'a dyn Fn(&KnowledgeProperty) -> Option<Rc<KnowledgeTarget>>;

/// A path made absolute and without `.` and `..`, as `Path.GetFullPath` makes it, without asking the disk.
pub fn full_path(path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    };
    let mut normal = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normal.pop();
            }
            other => normal.push(other.as_os_str()),
        }
    }
    normal
}

/// Whether two paths name one file, ignoring case as the standalone host compares them.
pub fn same_path(left: &Path, right: &Path) -> bool {
    full_path(left).to_string_lossy().to_lowercase() == full_path(right).to_string_lossy().to_lowercase()
}

/// The full path a relation's target names, from the file the relation is in.
pub fn target_path(body_path: &Path, target_file: &str) -> PathBuf {
    let own = full_path(body_path);
    if target_file == SELF || target_file.is_empty() {
        return own;
    }
    full_path(&own.parent().unwrap_or(Path::new("")).join(target_file))
}

/// How a file names another as a relation's target: its path from the file's own folder, with forward slashes;
/// [`SELF`] for itself.
pub fn target_name(body_path: &Path, target: &Path) -> String {
    let (from, to) = (full_path(body_path), full_path(target));
    if same_path(&from, &to) {
        return SELF.into();
    }
    let folder: Vec<Component<'_>> = from.parent().map(|p| p.components().collect()).unwrap_or_default();
    let target: Vec<Component<'_>> = to.components().collect();
    let shared = folder.iter().zip(&target).take_while(|(a, b)| a == b).count();
    let mut parts: Vec<String> = std::iter::repeat_n("..".to_owned(), folder.len() - shared).collect();
    parts.extend(
        target[shared..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    parts.join("/")
}

/// What a row is called: the text of its title property, or its id when it has none.
pub fn title_of(table: &KnowledgeTable, row: &ContentRow) -> String {
    table
        .title_column()
        .and_then(|title| row.cells.iter().find(|c| c.column_id == title.id && c.key == "text"))
        .and_then(|c| c.values.first())
        .filter(|t| !t.is_empty())
        .cloned()
        .unwrap_or_else(|| row.id.clone())
}

/// What a relation's value is called: the title of the row it names in the target, or empty when it names none.
pub fn label_of(target: Option<&KnowledgeTarget>, row_id: &str) -> String {
    target
        .and_then(|t| t.table.as_ref())
        .and_then(|table| table.row(row_id).map(|row| title_of(table, row)))
        .unwrap_or_default()
}

/// Whether a relation shows both directions in its one property, as Notion's relation of a database to itself
/// does: a relation of the table to itself that has no other side of its own and is not the parent relation.
pub fn is_mutual(property: &KnowledgeProperty) -> bool {
    property.value_type == "relation"
        && property.target_file == SELF
        && property.counterpart.is_empty()
        && !property.is_parent
        && !property.is_computed
}

/// The table with each mutual relation shown from both ends: a row also shows the rows that name it.
fn with_mutual(table: KnowledgeTable) -> KnowledgeTable {
    let mutual: HashSet<String> = table
        .columns
        .iter()
        .filter(|p| is_mutual(p))
        .map(|p| p.id.clone())
        .collect();
    if mutual.is_empty() {
        return table;
    }
    // Per property: the rows that name each row.
    let mut named_by: HashMap<(String, String), Vec<String>> = HashMap::new();
    for row in &table.rows {
        for cell in row
            .cells
            .iter()
            .filter(|c| c.key == "rows" && mutual.contains(&c.column_id))
        {
            for value in cell.values.iter().filter(|v| **v != row.id) {
                named_by
                    .entry((cell.column_id.clone(), value.clone()))
                    .or_default()
                    .push(row.id.clone());
            }
        }
    }
    let rows = table
        .rows
        .iter()
        .map(|row| {
            let mut cells = row.cells.clone();
            for property in table.columns.iter().filter(|p| mutual.contains(&p.id)) {
                let Some(naming) = named_by.get(&(property.id.clone(), row.id.clone())) else {
                    continue;
                };
                match cells.iter().position(|c| c.column_id == property.id && c.key == "rows") {
                    None => cells.push(ContentCell::new(property.id.clone(), naming.clone(), "rows")),
                    Some(at) => {
                        let added: Vec<String> = naming
                            .iter()
                            .filter(|id| !cells[at].values.contains(id))
                            .cloned()
                            .collect();
                        cells[at].values.extend(added);
                    }
                }
            }
            ContentRow::new(row.id.clone(), cells)
        })
        .collect();
    KnowledgeTable { rows, ..table }
}

/// The table with the computed sides of its two-way relations filled in: for each row, the rows of the other table
/// whose relation names it. Nothing of this is in the file, and nothing of it is ever written.
pub fn with_computed(table: KnowledgeTable, target_of: TargetOf<'_>) -> KnowledgeTable {
    let table = with_mutual(table);
    let computed: Vec<KnowledgeProperty> = table
        .columns
        .iter()
        .filter(|p| p.value_type == "relation" && p.is_computed && !p.counterpart.is_empty())
        .cloned()
        .collect();
    if computed.is_empty() {
        return table;
    }
    // For each computed property: which rows over there name which rows here.
    let mut related: HashMap<String, HashMap<String, Vec<String>>> = HashMap::new();
    for property in &computed {
        // A relation to the file itself is read from the table as it is shown, edits not yet written included.
        let itself = property.target_file == SELF || property.target_file.is_empty();
        let target = if itself { None } else { target_of(property) };
        let other = if itself {
            Some(&table)
        } else {
            target.as_ref().and_then(|t| t.table.as_ref())
        };
        let mut by_row: HashMap<String, Vec<String>> = HashMap::new();
        for row in other.map(|t| t.rows.as_slice()).unwrap_or_default() {
            if let Some(cell) = row
                .cells
                .iter()
                .find(|c| c.column_id == property.counterpart && c.key == "rows")
            {
                for value in &cell.values {
                    by_row.entry(value.clone()).or_default().push(row.id.clone());
                }
            }
        }
        related.insert(property.id.clone(), by_row);
    }
    let rows = table
        .rows
        .iter()
        .map(|row| {
            let mut cells: Vec<ContentCell> = row
                .cells
                .iter()
                .filter(|c| !related.contains_key(&c.column_id))
                .cloned()
                .collect();
            for property in &computed {
                if let Some(naming) = related[&property.id].get(&row.id) {
                    cells.push(ContentCell::new(property.id.clone(), naming.clone(), "rows"));
                }
            }
            ContentRow::new(row.id.clone(), cells)
        })
        .collect();
    KnowledgeTable { rows, ..table }
}

/// Whether making `parent_id` the parent of `row_id` would make a row its own ancestor: the parent is the row
/// itself, or has it somewhere above.
pub fn would_cycle(table: &KnowledgeTable, parent: &KnowledgeProperty, row_id: &str, parent_id: &str) -> bool {
    let mut above = parent_id.to_owned();
    for _ in 0..=table.rows.len() {
        if above == row_id {
            return true;
        }
        let next = table
            .row(&above)
            .and_then(|r| r.cells.iter().find(|c| c.column_id == parent.id && c.key == "rows"))
            .and_then(|c| c.values.first());
        match next {
            None => return false,
            Some(next) => above = next.clone(),
        }
    }
    // A circle that was already there, and does not pass through this row.
    false
}
