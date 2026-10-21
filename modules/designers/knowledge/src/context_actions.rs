//! What a knowledge file offers in the explorer: adding a relation to another table, or to itself (the standalone
//! host's `KnowledgeContextActionProvider.cs` at 8b6ccfc; knowledge-designer Requirements 5.1 and 7.6).
//!
//! - **One entry, and its questions in the order Notion asks them**: the table to relate to, chosen in the file
//!   dialog, which offers the knowledge files of the project and nothing else, with this table first; whether a cell
//!   holds one row or any number; whether the other table shows the relation too, and under which name there. Each
//!   answer leads to the next question, and nothing is written before the last one is given. What was answered so far
//!   travels in the id the next answer is committed under, so nothing is kept between them.
//! - **The edit is the table's own.** The relation is made by the same `addRelation` gesture and the same plan as an
//!   edit made in the table. The standalone host runs it on the project's history; here [`commit`] gives the gesture,
//!   which the shell sends through the table's session - one undoable step, over both files when it is two-way, shown
//!   at once - and [`make`] carries it out on the files directly for a table no tab shows.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_context::{Action, ActionGroup, Prompt, PromptOption, TextField};
use adp_desl::table::model::TableGesture;

use crate::command::{KnowledgeEditCommand, KnowledgeRestore};
use crate::definition;
use crate::edits::{self, KnowledgeSurroundings};
use crate::model::{KnowledgeProperty, KnowledgeTable};
use crate::project;
use crate::relations::{self, KnowledgeTarget};
use crate::store;

/// The entry, and the id its first answer - the table - is committed under.
pub const RELATE: &str = "knowledge.relate";

/// The id the second answer is committed under, followed by the table chosen.
const LIMIT_STEP: &str = "knowledge.relate.limit";

/// The id the third answer is committed under, followed by the limit and the table chosen.
const SIDE_STEP: &str = "knowledge.relate.side";

pub const ONE: &str = "one";
pub const MANY: &str = "many";
pub const ONE_WAY: &str = "one-way";
pub const TWO_WAY: &str = "two-way";

/// What keeps the parts of a step's id apart: no file name holds it.
const PART: char = '|';

const ICON: &str = "mdi-arrow-top-right";

pub const OTHER_SIDE_LABEL: &str = "Name in the other table";
pub const OTHER_SIDE_NEEDS_NAME: &str = "The relation needs a name in the other table.";
pub const NOT_A_TABLE: &str = "That file is not a table.";
pub const TABLE_GONE: &str = "That table is no longer there.";
pub const NOT_A_STEP: &str = "That is not a step of adding a relation.";

/// What answering one question of the entry comes to.
#[derive(Debug, Clone, PartialEq)]
pub enum Commit {
    /// The answer is taken, and the next question follows: answered, it is committed under `next_action_id`.
    Asks { prompt: Prompt, next_action_id: String },
    /// The last answer is taken: the relation is the gesture, made on the table at `body_path`.
    Make { body_path: PathBuf, gesture: TableGesture },
    /// The answer is refused, with the sentence why; nothing is written.
    Failed(String),
}

/// What the other side of a two-way relation is called until its author says otherwise: this table's name.
fn other_side_name(body_path: &Path) -> String {
    store::read(body_path)
        .body
        .map(|b| b.table().name.clone())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| {
            body_path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default()
        })
}

/// The file an answer names: a path from the project's root, as the dialog hands it over, or a full one.
fn chosen(root: &Path, value: &str) -> PathBuf {
    let named = Path::new(value);
    relations::full_path(&if named.is_absolute() {
        named.to_path_buf()
    } else {
        root.join(value.replace('/', std::path::MAIN_SEPARATOR_STR))
    })
}

/// A file as the dialog names it: its path from the project's root, with forward slashes.
fn in_project(root: &Path, path: &Path) -> String {
    let root = relations::full_path(root);
    let path = relations::full_path(path);
    path.strip_prefix(&root)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| path.to_string_lossy().into_owned())
}

/// What the entry offers on an explorer entry: the one entry on a knowledge file or its registration, nothing on
/// anything else.
pub fn discover(path: &Path, is_container: bool) -> Vec<ActionGroup> {
    if is_container || project::body_of(path).is_none() {
        return Vec::new();
    }
    vec![ActionGroup {
        actions: vec![Action::new(RELATE, "Add relation…").with_icon(ICON)],
    }]
}

/// Whether the file dialog takes a file: a knowledge file's data file, and nothing else.
pub fn accepts(path: &Path) -> bool {
    project::is_knowledge_file(path)
}

/// Runs the entry: the file dialog the table to relate to is chosen in, with this table pinned first.
pub fn execute(root: &Path, path: &Path) -> Result<Prompt, String> {
    let Some(body) = project::body_of(path) else {
        return Err(TABLE_GONE.into());
    };
    let option = |id: String, label: String| PromptOption {
        id,
        label,
        selectable: true,
        children: Vec::new(),
        suggested_value: String::new(),
        icon: String::new(),
        description: String::new(),
        name_suppressed_reason: String::new(),
        unavailable_reason: String::new(),
    };
    let mut others: Vec<PathBuf> = project::files(root)
        .into_iter()
        .filter(|p| !relations::same_path(p, &body) && accepts(p))
        .collect();
    others.sort_by_key(|p| in_project(root, p).to_lowercase());
    Ok(Prompt::File {
        title: "Relate to".into(),
        icon: ICON.into(),
        confirm_label: "Next".into(),
        files: others
            .iter()
            .map(|p| option(in_project(root, p), in_project(root, p)))
            .collect(),
        pinned: vec![option(in_project(root, &body), "This table".into())],
        empty_message: "There is no other table in this project.".into(),
        name_field: None,
    })
}

/// Checks an answer before it is committed: the table chosen is a knowledge file, and the name typed in the last
/// step is a name.
pub fn validate(root: &Path, action_id: &str, value: &str) -> Result<(), String> {
    if action_id.starts_with(SIDE_STEP) {
        return if value.trim().is_empty() {
            Err(OTHER_SIDE_NEEDS_NAME.into())
        } else {
            Ok(())
        };
    }
    if action_id != RELATE || accepts(&chosen(root, value)) {
        Ok(())
    } else {
        Err(NOT_A_TABLE.into())
    }
}

/// Commits one answer of the entry on the table at `path` (its data file or its registration): the next question,
/// or the gesture that makes the relation once the last is answered.
pub fn commit(root: &Path, path: &Path, action_id: &str, value: &str, text: &str) -> Commit {
    let Some(body_path) = project::body_of(path) else {
        return Commit::Failed(TABLE_GONE.into());
    };
    let Some(body) = store::read(&body_path).body else {
        return Commit::Failed(TABLE_GONE.into());
    };
    let reason = body.read_only_reason();
    if !reason.is_empty() {
        return Commit::Failed(reason);
    }
    let option = |id: &str, label: &str| PromptOption {
        id: id.into(),
        label: label.into(),
        selectable: true,
        children: Vec::new(),
        suggested_value: String::new(),
        icon: String::new(),
        description: String::new(),
        name_suppressed_reason: String::new(),
        unavailable_reason: String::new(),
    };
    let parts: Vec<&str> = action_id.split(PART).collect();
    match parts[0] {
        // The table is chosen: how many rows may a cell hold?
        RELATE => {
            if !accepts(&chosen(root, value)) {
                return Commit::Failed(NOT_A_TABLE.into());
            }
            Commit::Asks {
                prompt: Prompt::Choice {
                    title: "A cell holds".into(),
                    icon: ICON.into(),
                    confirm_label: "Next".into(),
                    options: vec![option(MANY, "Any number of rows"), option(ONE, "One row")],
                    empty_message: String::new(),
                    name_field: None,
                },
                next_action_id: [LIMIT_STEP, value].join(&PART.to_string()),
            }
        }
        // And is it shown in the other table too, under which name?
        LIMIT_STEP if parts.len() == 2 && (value == ONE || value == MANY) => {
            let name = other_side_name(&body_path);
            let mut one_way = option(ONE_WAY, "In this table only");
            one_way.name_suppressed_reason = "A relation shown in this table only has no name in the other.".into();
            let mut two_way = option(TWO_WAY, "In both tables");
            two_way.suggested_value = name.clone();
            Commit::Asks {
                prompt: Prompt::Choice {
                    title: "Show the relation".into(),
                    icon: ICON.into(),
                    confirm_label: "Add relation".into(),
                    options: vec![one_way, two_way],
                    empty_message: String::new(),
                    name_field: Some(TextField {
                        label: OTHER_SIDE_LABEL.into(),
                        initial_value: name,
                    }),
                },
                next_action_id: [SIDE_STEP, value, parts[1]].join(&PART.to_string()),
            }
        }
        SIDE_STEP if parts.len() == 3 && (value == ONE_WAY || value == TWO_WAY) => {
            let target = chosen(root, parts[2]);
            let two_way = value == TWO_WAY;
            let counterpart = if !two_way {
                String::new()
            } else if !text.trim().is_empty() {
                text.trim().to_owned()
            } else {
                other_side_name(&body_path)
            };
            Commit::Make {
                gesture: TableGesture::new("addRelation")
                    .setting("target", relations::target_name(&body_path, &target))
                    .setting("limit", if parts[1] == ONE { "one" } else { "none" })
                    // The other side has the name its author gave it; this table's name when none was given.
                    .setting("counterpart", counterpart),
                body_path,
            }
        }
        _ => Commit::Failed(NOT_A_STEP.into()),
    }
}

/// What a target named from a file is, read from disk, or the file's own table for itself.
fn target_at(body_path: &Path, own: &KnowledgeTable, target_file: &str) -> Rc<KnowledgeTarget> {
    let path = relations::target_path(body_path, target_file);
    if relations::same_path(&path, body_path) {
        return Rc::new(KnowledgeTarget {
            path,
            table: Some(own.clone()),
            problem: String::new(),
        });
    }
    let read = store::read(&path);
    Rc::new(match read.body {
        None => KnowledgeTarget {
            path,
            table: None,
            problem: format!("'{target_file}' is not there."),
        },
        Some(body) if !body.unreadable().is_empty() => KnowledgeTarget {
            path,
            table: None,
            problem: format!("'{target_file}' cannot be read as a knowledge file."),
        },
        Some(body) => KnowledgeTarget {
            path,
            table: Some(body.table().clone()),
            problem: String::new(),
        },
    })
}

/// Makes the relation a [`Commit::Make`] describes on the files directly, as one step: what puts both files back,
/// or why it was not made.
pub fn make(body_path: &Path, gesture: &TableGesture) -> Result<Option<KnowledgeRestore>, String> {
    let Some(body) = store::read(body_path).body else {
        return Err(TABLE_GONE.into());
    };
    let own = body.table().clone();
    let target_of = |p: &KnowledgeProperty| Some(target_at(body_path, &own, &p.target_file));
    let target_named = |name: &str| target_at(body_path, &own, name);
    let files = KnowledgeSurroundings {
        body_path,
        target_of: &target_of,
        target_at: &target_named,
    };
    let extension = body_path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let mut new_id = definition::new_id;
    let edit = edits::plan(
        &own,
        "",
        gesture,
        &mut new_id,
        definition::adds_beside_its_rule(&extension),
        Some(&files),
    );
    if edit.is_refused() {
        return Err(edit.refusal);
    }
    KnowledgeEditCommand {
        body_path: body_path.to_path_buf(),
        changes: edit.changes,
        others: edit.others,
    }
    .execute()
}
