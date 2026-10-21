//! What changing a node's type makes (DISL section 9.5, `behavior.retype`; the standalone host's `RetypePolicy`): the
//! types it may become, and the values the new type's attributes take from the old element through
//! `attributeMapping`, evaluated in the `retype` context with `self` and `old` both the element as it is before the
//! change. The change carries the mapped values only; a mapping for an attribute the new type does not declare is
//! not evaluated. A type change not declared is refused: from a type with no entry along its linearisation, to a
//! type its entry's `to` does not list, of a derived element, or to a type that is not a concrete node type. The
//! working diagram is not changed.

use adp_cel::Map;

use super::change::{DislChange, DislTransaction};
use super::operations::variables_of;
use crate::derivation::env::DislEnv;
use crate::derivation::evaluation;
use crate::expressions::contexts;
use crate::json::{self, Json};
use crate::loading::specification::DislSpecification;
use crate::model::element::DislElement;

/// The transaction that changes `element`'s type to `type_name`.
pub fn change(
    specification: &DislSpecification,
    element: &DislElement,
    type_name: &str,
    env: Option<&DislEnv>,
) -> DislTransaction {
    let own = element.type_name();
    if element.is_derived() {
        return DislTransaction::refused(format!("{own} is derived, so its type cannot be changed."));
    }
    let target = match specification.metamodel().type_of(type_name) {
        Some(t) if !t.is_relation && !t.is_abstract() => t.clone(),
        _ => {
            return DislTransaction::refused(format!(
                "A {own} cannot become a {type_name}: that is no type of node."
            ));
        }
    };
    let Some((policy, pointer)) = policy(specification, element)
        .filter(|(p, _)| json::strings_of(p, "to").iter().any(|t| t == type_name))
    else {
        return DislTransaction::refused(format!("A {own} cannot become a {type_name}."));
    };

    let mut variables = variables_of(contexts::RETYPE, &element.diagram(), env);
    variables.insert("self".into(), element.to_value());
    variables.insert("old".into(), element.to_value());

    let mut attributes = Map::default();
    for (name, mapping) in json::members(policy, "attributeMapping") {
        if !target.attributes.contains_key(name) {
            continue;
        }
        match evaluation::expression(
            specification,
            mapping,
            &json::pointer(&json::pointer(&pointer, "attributeMapping"), name),
            contexts::RETYPE,
            &variables,
        ) {
            Ok(value) => attributes.insert(name.clone(), value),
            Err(error) => return DislTransaction::refused(error),
        }
    }
    DislTransaction {
        changes: vec![DislChange::Retype {
            element_id: element.id(),
            type_name: type_name.to_owned(),
            attributes,
        }],
        host_actions: Vec::new(),
        refusal: None,
    }
}

/// The retype entry of the first type along the element's linearisation that declares one.
fn policy<'a>(specification: &'a DislSpecification, element: &DislElement) -> Option<(&'a Json, String)> {
    let retype = json::get(json::member(specification.root(), "behavior"), "retype").filter(|r| r.is_object())?;
    element.element_type().linearisation.iter().find_map(|t| {
        json::get(retype, t)
            .filter(|p| p.is_object())
            .map(|p| (p, json::pointer("/behavior/retype", t)))
    })
}
