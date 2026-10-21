//! What a person can do to a C4 element or relationship, and the command each answer becomes (etalii.adp spec 030,
//! task T061).
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4/C4ContextActionProvider.cs` at
//! `8b6ccfc`, over its `DiagramActionProvider`. What is offered depends on what is selected and on what C4 says about
//! it: only a container, a component or a node is offered a technology, and an action that does not apply is not
//! offered at all. Every action asks for text first, starting from what is there; the answer becomes a command on the
//! project's history. A name is the one value a prompt refuses blank as it is typed; a commit of a blank rename is
//! written.

use std::path::Path;

use adp_context::{Action, ActionGroup, Prompt, Shortcut};
use adp_diagram::DiagramDocuments;
use adp_history::Command;

use crate::commands::{
    AddC4Element, SetElementDescription, SetElementName, SetElementTechnology, SetRelationshipDescription,
    SetRelationshipTechnology,
};
use crate::model::{C4Element, C4ElementKind, C4Relationship, C4Workspace};
use crate::placement;

pub const RENAME: &str = "c4.rename";
pub const EDIT_DESCRIPTION: &str = "c4.edit-description";
pub const SET_TECHNOLOGY: &str = "c4.set-technology";
pub const RELABEL: &str = "c4.relabel";
pub const SET_PROTOCOL: &str = "c4.set-protocol";

/// What an action on something no longer in the model answers.
pub const GONE: &str = "That element is no longer in this model.";

/// What a name typed blank answers.
pub const NAME_REQUIRED: &str = "Every C4 element needs a name.";

/// The kinds the toolbox can add; the rest live in a deployment environment.
const ADDABLE_KINDS: [C4ElementKind; 4] = [
    C4ElementKind::Person,
    C4ElementKind::SoftwareSystem,
    C4ElementKind::Container,
    C4ElementKind::Component,
];

/// The id of the action that adds an element of `kind`, as the toolbox names it.
pub fn add_action_id(kind: C4ElementKind) -> String {
    format!("c4.add-{}", kind.lower())
}

/// The kind an add action adds, or none when the id is not an add.
pub fn kind_of_add(action_id: &str) -> Option<C4ElementKind> {
    ADDABLE_KINDS.into_iter().find(|kind| add_action_id(*kind) == action_id)
}

/// What `element_id` names: an element, or a relationship by the composite id it goes by, or neither.
pub fn resolve<'a>(
    workspace: &'a C4Workspace,
    element_id: &str,
) -> (Option<&'a C4Element>, Option<&'a C4Relationship>) {
    if element_id.is_empty() {
        return (None, None);
    }
    match workspace.find(element_id) {
        Some(element) => (Some(element), None),
        None => (None, workspace.relationship(element_id)),
    }
}

fn action(id: &str, label: &str, icon: &str) -> Action {
    Action::new(id, label).with_icon(icon)
}

/// The context menu of `element_id` (empty for the diagram itself).
pub fn discover(workspace: &C4Workspace, usable: bool, element_id: &str) -> Vec<ActionGroup> {
    if !usable {
        // A document that could not be read offers nothing: every edit of it would be refused.
        return Vec::new();
    }
    let (element, relationship) = resolve(workspace, element_id);
    if let Some(_relationship) = relationship {
        return vec![ActionGroup {
            actions: vec![
                action(RELABEL, "Relabel…", "mdi-pencil-outline").with_shortcut(Shortcut::key("F2")),
                action(SET_PROTOCOL, "Set technology…", "mdi-transit-connection-variant"),
            ],
        }];
    }
    let Some(element) = element else {
        // No element: the diagram itself, where only the adds that need no parent apply.
        return if element_id.is_empty() {
            vec![ActionGroup {
                actions: add_actions_for(None),
            }]
        } else {
            Vec::new()
        };
    };
    let mut actions = vec![
        action(RENAME, "Rename…", "mdi-pencil-outline").with_shortcut(Shortcut::key("F2")),
        action(
            EDIT_DESCRIPTION,
            if element.description.is_empty() {
                "Add description…"
            } else {
                "Edit description…"
            },
            "mdi-text-box-outline",
        ),
    ];
    if element.kind.has_technology() {
        actions.push(action(
            SET_TECHNOLOGY,
            if element.technology.is_empty() {
                "Set technology…"
            } else {
                "Change technology…"
            },
            "mdi-tools",
        ));
    }
    let adds = add_actions_for(Some(element));
    let mut groups = vec![ActionGroup { actions }];
    if !adds.is_empty() {
        groups.push(ActionGroup { actions: adds });
    }
    groups
}

/// What may be added inside `parent`, or at the top when it is none: the kinds a drop on nothing is not refused, of
/// those that fit inside it.
fn add_actions_for(parent: Option<&C4Element>) -> Vec<Action> {
    ADDABLE_KINDS
        .into_iter()
        .filter(|kind| placement::refuse(&C4Workspace::default(), *kind, "").is_none())
        .filter(|kind| fits_inside(*kind, parent))
        .map(|kind| {
            action(
                &add_action_id(kind),
                &format!("Add {}&", spell(kind).to_lowercase()),
                "mdi-plus",
            )
        })
        .collect()
}

/// C4's containment rule, as the menu needs it.
fn fits_inside(kind: C4ElementKind, parent: Option<&C4Element>) -> bool {
    match kind {
        C4ElementKind::Person | C4ElementKind::SoftwareSystem => parent.is_none(),
        C4ElementKind::Container => parent.is_some_and(|p| p.kind == C4ElementKind::SoftwareSystem),
        C4ElementKind::Component => parent.is_some_and(|p| p.kind == C4ElementKind::Container),
        _ => false,
    }
}

fn spell(kind: C4ElementKind) -> String {
    match kind {
        C4ElementKind::SoftwareSystem => "Software system".to_owned(),
        _ => kind.name().to_owned(),
    }
}

fn input(title: &str, icon: &str, label: &str, initial: &str, confirm: &str) -> Prompt {
    Prompt::Input {
        title: title.to_owned(),
        icon: icon.to_owned(),
        label: label.to_owned(),
        initial_value: initial.to_owned(),
        confirm_label: confirm.to_owned(),
    }
}

/// Runs action `action_id` on `element_id`: the prompt it asks, or why it cannot.
pub fn run(workspace: &C4Workspace, element_id: &str, action_id: &str) -> Result<Prompt, String> {
    if let Some(adding) = kind_of_add(action_id) {
        // Asked before anything is written, so the element arrives named.
        return Ok(input(
            &format!("Add {}", spell(adding).to_lowercase()),
            "mdi-plus",
            "Name",
            "",
            "Add",
        ));
    }
    let (element, relationship) = resolve(workspace, element_id);
    if element.is_none() && relationship.is_none() {
        return Err(GONE.to_owned());
    }
    match (action_id, element, relationship) {
        (RENAME, Some(element), _) => Ok(input(
            "Rename element",
            "mdi-pencil-outline",
            "Name",
            &element.name,
            "Rename",
        )),
        (EDIT_DESCRIPTION, Some(element), _) => Ok(input(
            "Describe element",
            "mdi-text-box-outline",
            "Description",
            &element.description,
            "Save",
        )),
        (SET_TECHNOLOGY, Some(element), _) => Ok(input(
            "Set technology",
            "mdi-tools",
            "Technology",
            &element.technology,
            "Save",
        )),
        (RELABEL, _, Some(relationship)) => Ok(input(
            "Relabel relationship",
            "mdi-pencil-outline",
            "Description",
            &relationship.description,
            "Save",
        )),
        (SET_PROTOCOL, _, Some(relationship)) => Ok(input(
            "Set technology",
            "mdi-transit-connection-variant",
            "Technology",
            &relationship.technology,
            "Save",
        )),
        _ => Err(format!("'{action_id}' does not apply to this selection.")),
    }
}

/// Judges a value as it is typed: only a name is refused blank.
pub fn validate(action_id: &str, value: &str) -> Result<(), String> {
    if (action_id == RENAME || kind_of_add(action_id).is_some()) && value.trim().is_empty() {
        return Err(NAME_REQUIRED.to_owned());
    }
    Ok(())
}

/// The command the answer `value` to action `action_id` on `element_id` becomes, or none when the action does not
/// apply. An add goes inside whatever it was dropped on, and the command judges whether that pairing is legal.
pub fn commit(
    workspace: &C4Workspace,
    body_path: &Path,
    element_id: &str,
    action_id: &str,
    value: &str,
) -> Option<Box<dyn Command<DiagramDocuments>>> {
    let body_path = body_path.to_path_buf();
    if let Some(kind) = kind_of_add(action_id) {
        return Some(Box::new(AddC4Element {
            body_path,
            kind,
            name: value.to_owned(),
            parent_id: element_id.to_owned(),
            identifier: String::new(),
        }));
    }
    let (element, relationship) = resolve(workspace, element_id);
    let value = value.to_owned();
    Some(match (action_id, element, relationship) {
        (RENAME, Some(element), _) => Box::new(SetElementName {
            body_path,
            element_id: element.id.clone(),
            name: value,
        }),
        (EDIT_DESCRIPTION, Some(element), _) => Box::new(SetElementDescription {
            body_path,
            element_id: element.id.clone(),
            description: value,
        }),
        (SET_TECHNOLOGY, Some(element), _) => Box::new(SetElementTechnology {
            body_path,
            element_id: element.id.clone(),
            technology: value,
        }),
        (RELABEL, _, Some(relationship)) => Box::new(SetRelationshipDescription {
            body_path,
            relationship_id: relationship.id(),
            description: value,
        }),
        (SET_PROTOCOL, _, Some(relationship)) => Box::new(SetRelationshipTechnology {
            body_path,
            relationship_id: relationship.id(),
            technology: value,
        }),
        _ => return None,
    })
}
