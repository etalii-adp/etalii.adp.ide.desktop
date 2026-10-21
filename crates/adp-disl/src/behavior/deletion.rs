//! What deleting an element takes with it, and whether to ask first (DISL section 9.5; the standalone host's
//! `DeletionPolicy`): the type's `behavior.deletion`, found along its linearisation, with the defaults for a type
//! without one.
//!
//! The changes give every removal: the relations at the element and at every node beneath it (`relations:
//! "delete"`), the nodes beneath it deepest first (`children: "delete"`), then the element itself; a reference to
//! any of them is unset (`references: "unset"`). A `forbid` that applies refuses the deletion; `reconnect`,
//! `reparent` and `delete-referencing` are refused as not supported yet. A derived relation is not removed (section
//! 4.11.5): it follows from its sources. Children written inside their parent go with the parent's own removal:
//! with `nested`, the nodes beneath it get no removal of their own. The confirmation is none when nothing is to be
//! asked: no `confirm`, a `when` that does not hold, or a `count` below its `threshold`.

use adp_cel::{Map, Value};

use super::change::{DislChange, DislTransaction};
use super::operations::variables_of;
use crate::derivation::env::DislEnv;
use crate::derivation::evaluation;
use crate::expressions::contexts;
use crate::json::{self, Json};
use crate::loading::specification::DislSpecification;
use crate::model::element::DislElement;

/// A confirmation to ask before a transaction (section 9.5), its texts evaluated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DislConfirmation {
    pub title: String,
    pub message: String,
    pub confirm_label: String,
    pub danger: bool,
    pub count: Option<i64>,
}

/// The confirmation deleting `element` asks for, or none when it asks nothing.
pub fn confirmation(
    specification: &DislSpecification,
    element: &DislElement,
    selection: Option<&[DislElement]>,
    env: Option<&DislEnv>,
) -> Option<DislConfirmation> {
    let (policy, pointer) = policy(specification, element)?;
    let confirm = json::get(policy, "confirm")?;
    let at = json::pointer(&pointer, "confirm");
    let mut variables = variables_of(contexts::GESTURE_DELETE, &element.diagram(), env);
    variables.insert("self".into(), element.to_value());
    variables.insert(
        "selection".into(),
        Value::list(match selection {
            Some(selection) => selection.iter().map(DislElement::to_value).collect(),
            None => vec![element.to_value()],
        }),
    );
    variables.insert("count".into(), Value::Null);

    let Some(message) = json::get(confirm, "message").filter(|_| confirm.is_object() && !json::has(confirm, "cel"))
    else {
        // A plain Message: asked every time (section 9.5).
        return Some(DislConfirmation {
            title: "Delete".into(),
            message: evaluation::message(specification, confirm, &at, contexts::GESTURE_DELETE, &variables, ""),
            confirm_label: "Delete".into(),
            danger: true,
            count: None,
        });
    };

    let mut count = None;
    if let Some(counted) = json::get(confirm, "count") {
        let Ok(Value::Int(value)) = evaluation::expression(
            specification,
            counted,
            &json::pointer(&at, "count"),
            contexts::GESTURE_DELETE,
            &variables,
        ) else {
            return None;
        };
        count = Some(value);
        variables.insert("count".into(), Value::Int(value));
        let threshold = json::int_of(confirm, "threshold").unwrap_or(0);
        if value < threshold {
            return None;
        }
    }
    if let Some(when) = json::get(confirm, "when")
        && !evaluation::holds(specification, when, &json::pointer(&at, "when"), contexts::GESTURE_DELETE, &variables)
    {
        return None;
    }
    let text = |name: &str, fallback: &str| match json::get(confirm, name) {
        Some(text) => evaluation::message(
            specification,
            text,
            &json::pointer(&at, name),
            contexts::GESTURE_DELETE,
            &variables,
            fallback,
        ),
        None => fallback.to_owned(),
    };
    Some(DislConfirmation {
        title: text("title", "Delete"),
        message: evaluation::message(
            specification,
            message,
            &json::pointer(&at, "message"),
            contexts::GESTURE_DELETE,
            &variables,
            "",
        ),
        confirm_label: text("confirmLabel", "Delete"),
        danger: !matches!(json::get(confirm, "danger"), Some(Json::Bool(false))),
        count,
    })
}

/// The removals deleting `element` makes, in the order they are made; `nested` when the persistence writes a node's
/// children inside it, so that its removal takes them with it.
pub fn changes(specification: &DislSpecification, element: &DislElement, nested: bool) -> DislTransaction {
    let declared = policy(specification, element).map(|(p, _)| p);
    let setting = |name: &str, fallback: &str| {
        declared
            .and_then(|p| json::string_of(p, name))
            .unwrap_or(fallback)
            .to_owned()
    };
    let children = setting("children", "delete");
    let relations = setting("relations", "delete");
    let references = setting("references", "unset");

    let subtree = if element.is_relation() {
        Vec::new()
    } else {
        element.descendants()
    };
    if !subtree.is_empty() && children != "delete" {
        return DislTransaction::refused(if children == "forbid" {
            "This cannot be deleted while it has children.".to_owned()
        } else {
            format!("Deleting with children: \"{children}\" is not supported yet.")
        });
    }

    let mut gone: Vec<DislElement> = subtree.clone();
    gone.push(element.clone());
    let diagram = element.diagram();
    let attached: Vec<DislElement> = diagram
        .relations()
        .into_iter()
        .filter(|r| {
            !r.is_derived()
                && !gone.contains(r)
                && (r.source().is_some_and(|s| gone.contains(&s)) || r.target().is_some_and(|t| gone.contains(&t)))
        })
        .collect();
    if !attached.is_empty() && relations != "delete" {
        return DislTransaction::refused(if relations == "forbid" {
            "This cannot be deleted while relations end at it.".to_owned()
        } else {
            format!("Deleting with relations: \"{relations}\" is not supported yet.")
        });
    }

    let mut changes: Vec<DislChange> = attached
        .iter()
        .map(|r| DislChange::Remove { element_id: r.id() })
        .collect();
    gone.extend(attached);

    let metamodel = specification.metamodel();
    for other in diagram.elements().into_iter().filter(|e| !gone.contains(e)) {
        let mut unset = Map::default();
        for attribute in other.element_type().attributes.values() {
            if metamodel.type_of(&attribute.type_name).is_some() && refers(&other.value_of(&attribute.name), &gone) {
                unset.insert(attribute.name.clone(), Value::Null);
            }
        }
        if unset.is_empty() {
            continue;
        }
        if references != "unset" {
            return DislTransaction::refused(if references == "forbid" {
                "This cannot be deleted while something refers to it.".to_owned()
            } else {
                format!("Deleting with references: \"{references}\" is not supported yet.")
            });
        }
        changes.push(DislChange::Set {
            element_id: other.id(),
            type_name: other.type_name(),
            attributes: unset,
        });
    }

    if !nested {
        changes.extend(subtree.iter().rev().map(|n| DislChange::Remove { element_id: n.id() }));
    }
    changes.push(DislChange::Remove {
        element_id: element.id(),
    });
    DislTransaction {
        changes,
        host_actions: Vec::new(),
        refusal: None,
    }
}

fn refers(value: &Value, gone: &[DislElement]) -> bool {
    match value {
        Value::List(list) => list
            .iter()
            .any(|item| DislElement::from_value(item).is_some_and(|e| gone.contains(&e))),
        other => DislElement::from_value(other).is_some_and(|e| gone.contains(&e)),
    }
}

/// The deletion policy of the first type along the element's linearisation that declares one.
fn policy<'a>(specification: &'a DislSpecification, element: &DislElement) -> Option<(&'a Json, String)> {
    let deletion = json::get(json::member(specification.root(), "behavior"), "deletion").filter(|d| d.is_object())?;
    element
        .element_type()
        .linearisation
        .iter()
        .find_map(|t| json::get(deletion, t).map(|p| (p, json::pointer("/behavior/deletion", t))))
}
