//! Links between knowledge files (etalii.adp spec 089; `knowledge.md` section 9; FBL section 17): a relation names
//! its target by a link to the target's document, `$<document-id>@<file name>`, and holds a link to each related
//! row, `$<document-id>@<file name>#<row id>.<title property's name>=<title>`. Links are resolved in the workspace,
//! the knowledge files of the folder the file is opened under; a relation's `backlinkName` shows, in its target, a
//! column computed from those links and stored in neither file.
//!
//! A file made before links names its target by a path and holds bare row ids; it is read as it is, and converted
//! when it is loaded for editing ([`load_changes`]).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use adp_cel::{Map, Value};
use adp_desl::table::content::{ContentCell, ContentRow};
use adp_fbl::{FblModel, ModelChange};
use adp_links::{IndexedDocument, IndexedElement, Link, Resolution, Workspace};

use crate::model::{KnowledgeProperty, KnowledgeTable};
use crate::project;
use crate::relations::{self, KnowledgeTarget};
use crate::store::KnowledgeRead;

/// The version of a knowledge file that holds links (DED 0.3).
pub const CURRENT_VERSION: &str = "0.3";

/// What separates a backlink column's id from the relation it shows: no id the designer makes holds it.
pub const BACKLINK_MARK: char = '~';

/// One knowledge file of the workspace, as it was last read.
#[derive(Debug, Clone)]
pub struct KnowledgeFile {
    pub path: PathBuf,
    pub table: KnowledgeTable,
}

/// The knowledge files under `folder` that read as knowledge files.
pub fn read_files(folder: &Path, read: &dyn Fn(&Path) -> KnowledgeRead) -> Vec<KnowledgeFile> {
    project::files(folder)
        .into_iter()
        .filter_map(|path| {
            let body = read(&path).body?;
            body.unreadable().is_empty().then(|| KnowledgeFile {
                path: relations::full_path(&path),
                table: body.table().clone(),
            })
        })
        .collect()
}

/// The knowledge file at `path` as the index of links of another kind of tool sees it (FBL section 17.3): a mind
/// map's, say, whose linked node stands for a row; none when the file is not a knowledge file that reads.
pub fn index_file(path: &Path) -> Option<IndexedDocument> {
    if !project::says_knowledge(path) {
        return None;
    }
    let body = crate::store::read(path).body?;
    body.unreadable()
        .is_empty()
        .then(|| indexed(&relations::full_path(path), body.table()))
}

/// What a row's link names it by: the text of the title property, or nothing.
fn title_text(table: &KnowledgeTable, row: &ContentRow) -> String {
    table
        .title_column()
        .and_then(|title| row.cells.iter().find(|c| c.column_id == title.id && c.key == "text"))
        .and_then(|c| c.values.first())
        .cloned()
        .unwrap_or_default()
}

/// A knowledge file as the index of links sees it (FBL section 17.3): its rows are what can be linked to, each named
/// by the title property.
pub fn indexed(path: &Path, table: &KnowledgeTable) -> IndexedDocument {
    let title = table.title_column().map(|t| t.name.clone());
    IndexedDocument {
        path: relations::full_path(path),
        document_id: (!table.document_id.is_empty()).then(|| table.document_id.clone()),
        name_property: title.clone(),
        elements: table
            .rows
            .iter()
            .map(|row| {
                IndexedElement::new(
                    row.id.clone(),
                    title.iter().map(|name| (name.clone(), title_text(table, row))),
                )
            })
            .collect(),
        links: Vec::new(),
    }
}

/// The workspace of a file: the knowledge files read, with the file itself as it is shown in place of its reading.
pub fn workspace(files: &[KnowledgeFile], body_path: &Path, own: &KnowledgeTable) -> Workspace {
    let mut workspace = Workspace::new(PathBuf::new(), files.iter().map(|f| indexed(&f.path, &f.table)));
    workspace.put(indexed(body_path, own));
    workspace
}

/// Whether a relation's target is written as a link rather than as a path.
pub fn is_link(target_file: &str) -> bool {
    target_file.starts_with('$')
}

/// What a target written as a link is: the table it resolves to, or the finding that it does not.
pub fn target_of_link(
    workspace: &Workspace,
    files: &[KnowledgeFile],
    body_path: &Path,
    own: &KnowledgeTable,
    target_file: &str,
) -> KnowledgeTarget {
    match workspace.resolve(target_file) {
        Resolution::Resolved(resolved) => {
            let table = if relations::same_path(&resolved.path, body_path) {
                Some(own.clone())
            } else {
                files.iter().find(|f| f.path == resolved.path).map(|f| f.table.clone())
            };
            KnowledgeTarget {
                path: resolved.path,
                table,
                ..KnowledgeTarget::default()
            }
        }
        Resolution::Finding(finding) => {
            let name = finding
                .link
                .as_ref()
                .map(|l| l.document_name.clone())
                .unwrap_or_default();
            KnowledgeTarget {
                path: relations::full_path(body_path)
                    .parent()
                    .unwrap_or(Path::new(""))
                    .join(name),
                table: None,
                problem: finding.message,
                code: finding.code.to_owned(),
            }
        }
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// How a relation names its target: a link to the target's document, or, while the target has no document id, its
/// path from the file's own folder as a file made before links did.
pub fn target_text(body_path: &Path, target: &KnowledgeTarget) -> String {
    match target.table.as_ref().filter(|t| !t.document_id.is_empty()) {
        Some(table) => Link::to_document(table.document_id.clone(), file_name(&target.path)).text(),
        None => relations::target_name(body_path, &target.path),
    }
}

/// How a relation holds a related row: a link to it named by the target's title property, or, while the target has
/// no document id, the row's id.
pub fn row_text(target: Option<&KnowledgeTarget>, row_id: &str) -> String {
    let Some((target, table)) = target.and_then(|t| Some((t, t.table.as_ref()?))) else {
        return row_id.to_owned();
    };
    if table.document_id.is_empty() {
        return row_id.to_owned();
    }
    let bare = Link::to_element(
        table.document_id.clone(),
        file_name(&target.path),
        row_id,
        table.title_column().map(|t| t.name.clone()),
        None,
    );
    // Written as load-time validation would refresh it, so that loading the file again changes nothing.
    let one = Workspace::new(PathBuf::new(), [indexed(&target.path, table)]);
    match one.resolve(&bare.text()) {
        Resolution::Resolved(resolved) => resolved.refreshed.text(),
        Resolution::Finding(_) => bare.text(),
    }
}

/// Whether a column is a backlink column, computed from the links of a relation elsewhere.
pub fn is_backlink_column(column_id: &str) -> bool {
    column_id.contains(BACKLINK_MARK)
}

/// The table with a backlink column for each relation of the workspace whose `backlinkName` names one here: per row,
/// the rows of that relation's file that link to it (FBL section 17.5). Nothing of it is in either file.
pub fn with_backlinks(
    table: KnowledgeTable,
    body_path: &Path,
    workspace: &Workspace,
    files: &[KnowledgeFile],
) -> KnowledgeTable {
    let own = relations::full_path(body_path);
    let mut sources: Vec<(PathBuf, &KnowledgeTable)> = vec![(own.clone(), &table)];
    sources.extend(
        files
            .iter()
            .filter(|f| f.path != own)
            .map(|f| (f.path.clone(), &f.table)),
    );
    let mut added: Vec<(KnowledgeProperty, HashMap<String, Vec<String>>)> = Vec::new();
    for (path, source) in &sources {
        for property in source
            .columns
            .iter()
            .filter(|p| p.value_type == "relation" && !p.is_computed && !p.backlink_name.is_empty())
        {
            let resolves_here = if is_link(&property.target_file) {
                workspace
                    .resolve(&property.target_file)
                    .resolved()
                    .is_some_and(|r| r.path == own)
            } else {
                relations::same_path(&relations::target_path(path, &property.target_file), &own)
            };
            if !resolves_here {
                continue;
            }
            let mut by_row: HashMap<String, Vec<String>> = HashMap::new();
            for row in &source.rows {
                if let Some(cell) = row.cells.iter().find(|c| c.column_id == property.id && c.key == "rows") {
                    for value in &cell.values {
                        by_row.entry(value.clone()).or_default().push(row.id.clone());
                    }
                }
            }
            let source_name = if source.document_id.is_empty() {
                file_name(path)
            } else {
                source.document_id.clone()
            };
            let mut column = property.clone();
            column.id = format!("{}{BACKLINK_MARK}{source_name}", property.id);
            column.name = property.backlink_name.clone();
            column.is_title = false;
            column.is_parent = false;
            column.is_computed = true;
            column.limit = "none".into();
            column.counterpart = property.id.clone();
            column.backlink_name = String::new();
            column.target_file = if source.document_id.is_empty() {
                relations::target_name(body_path, path)
            } else {
                Link::to_document(source.document_id.clone(), file_name(path)).text()
            };
            added.push((column, by_row));
        }
    }
    if added.is_empty() {
        return table;
    }
    let rows = table
        .rows
        .iter()
        .map(|row| {
            let mut cells = row.cells.clone();
            for (column, by_row) in &added {
                if let Some(naming) = by_row.get(&row.id) {
                    cells.push(ContentCell::new(column.id.clone(), naming.clone(), "rows"));
                }
            }
            ContentRow::new(row.id.clone(), cells)
        })
        .collect();
    let mut columns = table.columns.clone();
    columns.extend(added.into_iter().map(|(c, _)| c));
    KnowledgeTable { rows, columns, ..table }
}

fn set(id: impl Into<String>, pairs: Vec<(&str, Value)>) -> ModelChange {
    ModelChange::Set {
        id: id.into(),
        attributes: pairs.into_iter().collect::<Map>(),
    }
}

fn text_of(element: &adp_fbl::FblElement, name: &str) -> String {
    crate::values::text(element.attributes.get(name))
}

/// What loading a file for editing changes in it, as one step (FBL section 17.4; `knowledge.md` section 9): its
/// document id when it has none, the conversion of a file made before links, and the refresh of every link that
/// resolved to something other than what it says. Empty when nothing differs. `target_of` is what a relation of
/// the file, as it is now, points at.
pub fn load_changes(
    model: &FblModel,
    table: &KnowledgeTable,
    body_path: &Path,
    workspace: &Workspace,
    target_of: &dyn Fn(&KnowledgeProperty) -> Option<std::rc::Rc<KnowledgeTarget>>,
    new_id: &mut dyn FnMut() -> String,
) -> Vec<ModelChange> {
    let mut changes = Vec::new();
    let mut document_id = table.document_id.clone();
    if document_id.is_empty() {
        document_id = new_id();
        changes.push(ModelChange::SetDocumentId {
            id: document_id.clone(),
        });
    }
    let by_id: HashMap<&str, &adp_fbl::FblElement> = model.elements.iter().map(|e| (e.id.as_str(), e)).collect();
    // What the file itself is as a target, now that it has an id.
    let mut itself = table.clone();
    itself.document_id = document_id.clone();
    let itself = KnowledgeTarget {
        path: relations::full_path(body_path),
        table: Some(itself),
        ..KnowledgeTarget::default()
    };
    // The old form's relations to convert: a target named by its path and found with a document id.
    let mut converted: HashMap<String, KnowledgeTarget> = HashMap::new();
    let mut still_old: Vec<&KnowledgeProperty> = Vec::new();
    for property in table
        .columns
        .iter()
        .filter(|p| p.value_type == "relation" && !p.is_computed)
    {
        if is_link(&property.target_file) {
            // Refreshed when its document was found under another name.
            if let Some(resolved) = workspace.resolve(&property.target_file).resolved() {
                let text = resolved.refreshed.text();
                if text != property.target_file {
                    changes.push(set(property.id.clone(), vec![("targetFile", Value::String(text))]));
                }
            }
            continue;
        }
        let target = if relations::is_itself(table, &property.target_file) {
            Some(itself.clone())
        } else {
            target_of(property).map(|t| (*t).clone())
        };
        let Some(target) = target.filter(|t| t.table.as_ref().is_some_and(|t| !t.document_id.is_empty())) else {
            still_old.push(property);
            continue;
        };
        let related = target.table.as_ref().expect("a table with an id");
        let mut pairs = vec![("targetFile", Value::String(target_text(body_path, &target)))];
        if !property.counterpart.is_empty() {
            let name = related
                .column(&property.counterpart)
                .map(|c| c.name.clone())
                .unwrap_or_default();
            pairs.push(("counterpart", Value::Null));
            if !name.is_empty() {
                pairs.push(("backlinkName", Value::String(name)));
            }
        }
        changes.push(set(property.id.clone(), pairs));
        converted.insert(property.id.clone(), target);
    }
    // Every row a relation holds: converted to a link, or refreshed.
    for item in model
        .elements
        .iter()
        .filter(|e| e.type_name == "CellItem" && e.attributes.contains_key("row"))
    {
        let written = text_of(item, "row");
        let Some(cell) = item.parent_id.as_deref().and_then(|p| by_id.get(p)) else {
            continue;
        };
        let property = text_of(cell, "property");
        let text = if let Some(target) = converted.get(&property) {
            if is_link(&written) {
                continue;
            }
            row_text(Some(target), &written)
        } else if is_link(&written) {
            match workspace.resolve(&written).resolved() {
                Some(resolved) => resolved.refreshed.text(),
                None => continue,
            }
        } else {
            continue;
        };
        if text != written {
            changes.push(set(item.id.clone(), vec![("row", Value::String(text))]));
        }
    }
    // A counterpart property no old relation names any more is the backlink column it already was: it goes.
    for property in table
        .columns
        .iter()
        .filter(|p| p.value_type == "relation" && p.is_computed && !p.counterpart.is_empty())
    {
        let named_by_old_here = still_old.iter().any(|o| o.counterpart == property.id);
        let named_elsewhere = target_of(property)
            .and_then(|t| {
                t.table.as_ref().map(|other| {
                    other
                        .column(&property.counterpart)
                        .is_some_and(|o| !is_link(&o.target_file) && o.counterpart == property.id)
                })
            })
            .unwrap_or(false);
        if !named_by_old_here && !named_elsewhere {
            changes.push(ModelChange::Remove {
                id: property.id.clone(),
            });
        }
    }
    // A file converted says it is of the version that holds links.
    let version = model
        .elements
        .iter()
        .find(|e| e.type_name == "Table")
        .map(|t| text_of(t, "version"))
        .unwrap_or_default();
    if !changes.is_empty() && version != CURRENT_VERSION {
        let attributes: Map = [("version", Value::String(CURRENT_VERSION.into()))]
            .into_iter()
            .collect();
        changes.insert(0, ModelChange::Migrate { attributes });
    }
    changes
}
