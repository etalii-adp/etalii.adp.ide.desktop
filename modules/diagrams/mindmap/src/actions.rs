//! What a person can do to a mind map node (etalii.adp spec 030, FR-018), ported from the standalone host's
//! `MindmapContextActionProvider.cs` at `8b6ccfc`.
//!
//! Which actions a node offers, their labels, icons and shortcuts, and whether a delete asks first and in which words,
//! are the specification's (its context menu and `deletion.confirm`, through `adp-diagram`); what each one does and
//! what it asks is here, because `mindmap.dis` cannot say it: an add creates its node at once under a name read off the
//! siblings it joins (`adp_diagram::sibling_naming`) and then asks for the name in place, a sibling goes right after
//! the node, the link is picked from the project's files and stored relative to the map, and folding is the viewer's
//! own state, never written. Every change is one edit on the project's history.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use adp_cel::{Map, Value};
use adp_context::{Prompt, PromptOption};
use adp_diagram::DiagramDocument;
use adp_diagram::document::DiagramChange;
use adp_diagram::session::{ActionOutcome, ActionTarget, DiagramSession};
use adp_diagram::sibling_naming;
use adp_diagram::workspace::DiagramWorkspace;
use adp_disl::behavior::deletion;
use adp_disl::model::builder;
use adp_disl::{DerivedMenuEntry, DislChange, DislDiagram, DislElement, DislEnv, DislMenuTarget, IdSource};
use adp_fbl::ModelChange;
use adp_layout::FoldState;

use crate::links;
use crate::plugin::{NODE_GONE, NODE_TYPE};

pub const ADD_CHILD: &str = "mindmap.add-child";
pub const ADD_SIBLING: &str = "mindmap.add-sibling";
pub const RENAME: &str = "mindmap.rename";
pub const DELETE: &str = "mindmap.delete";
pub const TOGGLE_FOLD: &str = "mindmap.toggle-fold";
pub const EDIT_NOTES: &str = "mindmap.edit-notes";
pub const LINK: &str = "mindmap.link";
pub const UNLINK: &str = "mindmap.unlink";

/// Every action of a node's menu, which this module runs itself.
pub const ACTIONS: [&str; 8] = [
    ADD_CHILD,
    ADD_SIBLING,
    RENAME,
    DELETE,
    TOGGLE_FOLD,
    EDIT_NOTES,
    LINK,
    UNLINK,
];

/// The name a new node gets when its siblings show no pattern.
pub const FALLBACK_NAME: &str = "Node";

/// The folders the link picker never offers.
const SKIPPED_FOLDERS: [&str; 5] = [".git", "node_modules", "bin", "obj", ".claude"];

/// The viewer state the actions keep: each map's folds, and the node an add made whose name is still being asked.
#[derive(Debug, Default)]
pub struct MindmapActions {
    folds: RefCell<HashMap<PathBuf, FoldState>>,
    /// The node an add made, by the map, the node the add ran on and the add, until its name is answered.
    pending: RefCell<HashMap<(PathBuf, String, String), String>>,
}

impl MindmapActions {
    /// Whether the viewer has folded `id` in the map at `body_path`, seeded from the file's `FOLDED` when first asked.
    pub fn is_folded(&self, document: &DiagramDocument, id: &str) -> bool {
        self.with_folds(document, |folds| folds.is_folded(id))
    }

    fn with_folds<T>(&self, document: &DiagramDocument, read: impl FnOnce(&mut FoldState) -> T) -> T {
        let mut folds = self.folds.borrow_mut();
        let state = folds.entry(document.body_path().to_path_buf()).or_insert_with(|| {
            FoldState::seeded_from(
                document
                    .model()
                    .diagram
                    .nodes()
                    .iter()
                    .filter(|n| n.value_of("folded") == Value::Bool(true))
                    .map(|n| n.id()),
            )
        });
        read(state)
    }

    /// The model of `document` with every node's `self.view.collapsed` this viewer's fold, as menus read it.
    fn model(&self, document: &DiagramDocument) -> DislDiagram {
        let diagram = builder::from(document.fbl(), document.tool().definition.specification()).diagram;
        for node in diagram.nodes() {
            let folded = self.is_folded(document, &node.id());
            node.set_view(Some([("collapsed", Value::Bool(folded))].into_iter().collect()));
        }
        diagram
    }

    /// The menu entry `action_id` of `element`, when its menu offers it and it is available.
    fn offered(document: &DiagramDocument, element: &DislElement, action_id: &str) -> Option<DerivedMenuEntry> {
        document
            .tool()
            .definition
            .menu_entries(&DislMenuTarget::Element(element.clone()), &DislEnv::default())
            .into_iter()
            .flatten()
            .find(|e| e.id == action_id && e.available)
    }

    /// Runs `action_id` on `target`.
    pub fn execute(
        &self,
        session: &DiagramSession,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        ids: &dyn IdSource,
    ) -> Option<ActionOutcome> {
        let ActionTarget::Element(id) = target else {
            return None;
        };
        if !ACTIONS.contains(&action_id) {
            return None;
        }
        let document = workspace.documents().get(session.body_path())?;
        if let Some(refusal) = document.refusal() {
            return Some(ActionOutcome::Refused(refusal));
        }
        let model = self.model(document);
        let Some(element) = model.element_by_id(id) else {
            return Some(ActionOutcome::Refused(NODE_GONE.to_owned()));
        };
        let Some(entry) = Self::offered(document, &element, action_id) else {
            return Some(ActionOutcome::Refused(format!("Unknown action '{action_id}'.")));
        };
        let outcome = match action_id {
            // Add, then edit in place: the node is created at once under a name read off the siblings it joins, and
            // the prompt that follows names it.
            ADD_CHILD => self.add_then_edit(session, workspace, &element, &element.children(), None, ids),
            ADD_SIBLING => match element.parent() {
                Some(parent) => {
                    let siblings = parent.children();
                    let index = siblings.iter().position(|s| s.id() == element.id()).unwrap_or(0) + 1;
                    self.add_then_edit(session, workspace, &element, &siblings, Some((parent, index)), ids)
                }
                None => ActionOutcome::Refused("The root node cannot have a sibling.".into()),
            },
            RENAME => ActionOutcome::Prompt(Prompt::Input {
                title: "Rename node".into(),
                icon: "mdi-pencil-outline".into(),
                label: "Text".into(),
                initial_value: text_of(&element),
                confirm_label: "Rename".into(),
            }),
            // A leaf goes without asking; a branch is confirmed, since it takes its subtree with it. Which, and the
            // words, are the specification's `deletion.confirm`.
            DELETE => {
                let specification = document.tool().definition.specification();
                match deletion::confirmation(specification, &element, None, Some(&DislEnv::default())) {
                    Some(confirmation) => ActionOutcome::Prompt(Prompt::Confirm {
                        title: confirmation.title,
                        icon: entry.icon.clone(),
                        message: confirmation.message,
                        confirm_label: confirmation.confirm_label,
                        danger: confirmation.danger,
                    }),
                    None => done(write(session, workspace, "Delete", remove(&element))),
                }
            }
            // View state, not a command: nothing is written and nothing lands on the history.
            TOGGLE_FOLD => {
                let document = workspace.documents().get(session.body_path())?;
                self.with_folds(document, |folds| folds.toggle(&element.id()));
                ActionOutcome::Done
            }
            EDIT_NOTES => ActionOutcome::Prompt(Prompt::Input {
                title: "Notes".into(),
                icon: "mdi-note-text-outline".into(),
                label: "Notes".into(),
                initial_value: element.value_of("notes").as_str().unwrap_or_default().to_owned(),
                confirm_label: "Save".into(),
            }),
            LINK => ActionOutcome::Prompt(Prompt::Choice {
                title: "Link to".into(),
                icon: "mdi-link-variant".into(),
                confirm_label: "Link".into(),
                options: project_tree(workspace.root()),
                empty_message: "The project has no files to link to.".into(),
                name_field: None,
            }),
            UNLINK => done(write(session, workspace, "Unlink", set(&element, "link", Value::Null))),
            _ => return None,
        };
        Some(outcome)
    }

    fn add_then_edit(
        &self,
        session: &DiagramSession,
        workspace: &mut DiagramWorkspace,
        element: &DislElement,
        siblings: &[DislElement],
        sibling_of: Option<(DislElement, usize)>,
        ids: &dyn IdSource,
    ) -> ActionOutcome {
        let texts: Vec<String> = siblings.iter().map(text_of).collect();
        let name = sibling_naming::next_name(texts.iter().map(String::as_str), FALLBACK_NAME);
        let new_id = match ids.next(NODE_TYPE) {
            Ok(id) if id.starts_with("ID_") => id,
            Ok(id) => format!("ID_{id}"),
            Err(refusal) => return ActionOutcome::Refused(refusal),
        };
        let (parent, index, action) = match sibling_of {
            Some((parent, index)) => (parent.id(), Some(index as i64), ADD_SIBLING),
            None => (element.id(), None, ADD_CHILD),
        };
        let change = add(&parent, &new_id, &name, index);
        if let Err(refusal) = write(session, workspace, "Add", change) {
            return ActionOutcome::Refused(refusal);
        }
        self.pending.borrow_mut().insert(
            (session.body_path().to_path_buf(), element.id(), action.to_owned()),
            new_id,
        );
        ActionOutcome::Prompt(Prompt::Input {
            title: "Rename node".into(),
            icon: "mdi-pencil-outline".into(),
            label: "Text".into(),
            initial_value: name,
            confirm_label: "Rename".into(),
        })
    }

    /// Commits the answer `value` to the prompt `action_id` raised on `target`.
    pub fn answer(
        &self,
        session: &DiagramSession,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        value: &str,
    ) -> Option<ActionOutcome> {
        let ActionTarget::Element(id) = target else {
            return None;
        };
        if !ACTIONS.contains(&action_id) {
            return None;
        }
        let document = workspace.documents().get(session.body_path())?;
        let model = self.model(document);
        let Some(element) = model.element_by_id(id) else {
            return Some(ActionOutcome::Refused(NODE_GONE.to_owned()));
        };
        if Self::offered(document, &element, action_id).is_none() {
            return Some(ActionOutcome::Refused(format!("Unknown action '{action_id}'.")));
        }
        let key = (session.body_path().to_path_buf(), element.id(), action_id.to_owned());
        let pending = self.pending.borrow_mut().remove(&key);
        let (label, change) = match (action_id, pending) {
            // The name of the node the add made: a rename of that node, not a second add.
            (ADD_CHILD | ADD_SIBLING, Some(added)) => match model.element_by_id(&added) {
                Some(added) => ("Rename", set(&added, "text", Value::String(value.to_owned()))),
                None => return Some(ActionOutcome::Refused(NODE_GONE.to_owned())),
            },
            (ADD_CHILD, None) => ("Add", add(&element.id(), &crate::document::new_id(), value, None)),
            (ADD_SIBLING, None) => {
                let Some(parent) = element.parent() else {
                    return Some(ActionOutcome::Refused("The root node cannot have a sibling.".into()));
                };
                let index = parent
                    .children()
                    .iter()
                    .position(|s| s.id() == element.id())
                    .unwrap_or(0)
                    + 1;
                (
                    "Add",
                    add(&parent.id(), &crate::document::new_id(), value, Some(index as i64)),
                )
            }
            (RENAME, _) => ("Rename", set(&element, "text", Value::String(value.to_owned()))),
            (DELETE, _) => ("Delete", remove(&element)),
            (EDIT_NOTES, _) => ("Notes", set(&element, "notes", Value::String(value.to_owned()))),
            (LINK, _) => {
                let segments: Vec<&str> = value.split('/').filter(|s| !s.is_empty()).collect();
                let stored = links::to_map_relative(session.body_path(), &segments, workspace.root());
                ("Link", set(&element, "link", Value::String(stored)))
            }
            _ => return Some(ActionOutcome::Refused(format!("Unknown action '{action_id}'."))),
        };
        Some(done(write(session, workspace, label, change)))
    }
}

fn text_of(element: &DislElement) -> String {
    element.value_of("text").as_str().unwrap_or_default().to_owned()
}

fn done(result: Result<(), String>) -> ActionOutcome {
    match result {
        Ok(()) => ActionOutcome::Done,
        Err(refusal) => ActionOutcome::Refused(refusal),
    }
}

/// A new node named `text` under `parent`: the last child, or, with an index, right after the child before it.
fn add(parent: &str, id: &str, text: &str, index: Option<i64>) -> DiagramChange {
    let mut attributes = Map::default();
    attributes.insert("text", Value::String(text.to_owned()));
    DiagramChange::Body(ModelChange::Add {
        type_name: NODE_TYPE.into(),
        id: Some(id.to_owned()),
        attributes,
        parent_id: Some(parent.to_owned()),
        index,
        slot: Some("children".into()),
    })
}

fn set(element: &DislElement, attribute: &str, value: Value) -> DiagramChange {
    DiagramChange::Disl(DislChange::Set {
        element_id: element.id(),
        type_name: element.type_name(),
        attributes: [(attribute, value)].into_iter().collect(),
    })
}

fn remove(element: &DislElement) -> DiagramChange {
    DiagramChange::Disl(DislChange::Remove {
        element_id: element.id(),
    })
}

/// Plans `change` against the map the session shows and runs it as one edit of the project's history.
fn write(
    session: &DiagramSession,
    workspace: &mut DiagramWorkspace,
    label: &str,
    change: DiagramChange,
) -> Result<(), String> {
    let document = workspace
        .documents()
        .get(session.body_path())
        .ok_or_else(|| adp_diagram::document::NOT_OPEN.to_owned())?;
    let edit = document.plan(label, &[change])?;
    workspace.execute(edit)
}

/// The project's files as a tree to pick from, each by its project-relative path, so a link target is picked and never
/// typed (mindmap Requirement 12.7).
pub fn project_tree(root: &Path) -> Vec<PromptOption> {
    if !root.is_dir() {
        return Vec::new();
    }
    options(root, root)
}

fn options(folder: &Path, root: &Path) -> Vec<PromptOption> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut entries: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    entries.sort_by_key(|p| {
        p.file_name()
            .map(|n| n.to_string_lossy().to_uppercase())
            .unwrap_or_default()
    });
    let mut found = Vec::new();
    for entry in entries {
        let name = entry
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let id = links::project_relative(&entry, root).join("/");
        if entry.is_dir() {
            if SKIPPED_FOLDERS.iter().any(|s| s.eq_ignore_ascii_case(&name)) {
                continue;
            }
            let children = options(&entry, root);
            if !children.is_empty() {
                found.push(PromptOption {
                    id,
                    label: name,
                    selectable: true,
                    children,
                    ..PromptOption::default()
                });
            }
        } else if !name.starts_with("~adp-") {
            found.push(PromptOption {
                id,
                label: name,
                selectable: true,
                ..PromptOption::default()
            });
        }
    }
    found
}
