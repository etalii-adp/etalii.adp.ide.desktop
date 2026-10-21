//! The `natural` id strategy (DISL sections 11.5 and 11.5.1; the standalone host's `DislNaturalIds`) as a binding's
//! reader asks for it: the id an entry whose rule stores none is read with.
//!
//! The natural id is the string forms of the type's `key` attributes, in declaration order after inheritance is
//! flattened, joined by `|`. A binding type is resolved through `persistence.typeMap` to the metamodel type it maps
//! to, and each key attribute is found under the binding's name for it. The type's `prefix` is left to the model
//! builder, so a reference by the natural id resolves under the prefix as every other does. No id is derived - the
//! reader addresses the entry by its place - for a type the metamodel does not have, under another strategy,
//! without a key attribute, or with a key attribute the entry holds no text for.

use std::rc::Rc;

use adp_cel::Value;
use adp_fbl::IdRequest;
use adp_fbl::model::DeriveId;

use super::type_map::TypeMap;
use crate::json;
use crate::loading::specification::DislSpecification;

/// The derivation for `specification`, in the shape `FblOptions::derive_id` takes.
pub fn of(specification: &DislSpecification) -> DeriveId {
    let specification = specification.clone();
    let map = TypeMap::of(&specification);
    Rc::new(move |request: &IdRequest<'_>| {
        let mapping = map.mapping(request.type_name);
        let element_type = specification.metamodel().type_of(&mapping.as_type)?;
        if specification.id_rule_of(&element_type.name).strategy != "natural" {
            return None;
        }
        let keys: Vec<_> = element_type
            .attributes
            .values()
            .filter(|a| json::bool_of(&a.json, "key"))
            .collect();
        if keys.is_empty() {
            return None;
        }
        let mut parts = Vec::with_capacity(keys.len());
        for key in keys {
            let name = mapping.binding_name(&key.name);
            let text = text(request.attributes.get(name)?)?;
            if text.is_empty() {
                return None;
            }
            parts.push(text);
        }
        Some(parts.join("|"))
    })
}

/// The id an element of metamodel type `type_name` holding `attributes` is read back with under the `natural`
/// strategy: its type's prefix and its key attributes joined by `|`; none under another strategy, or where a key holds
/// no text. A host stores a created element's point under it (FBL section 8.3), since the id it created the element
/// with stands in for one the binding derives.
pub fn id_of(specification: &DislSpecification, type_name: &str, attributes: &adp_cel::Map) -> Option<String> {
    let rule = specification.id_rule_of(type_name);
    if rule.strategy != "natural" {
        return None;
    }
    let element_type = specification.metamodel().type_of(type_name)?;
    let keys: Vec<_> = element_type
        .attributes
        .values()
        .filter(|a| json::bool_of(&a.json, "key"))
        .collect();
    if keys.is_empty() {
        return None;
    }
    let mut parts = Vec::with_capacity(keys.len());
    for key in keys {
        let text = text(attributes.get(&key.name)?)?;
        if text.is_empty() {
            return None;
        }
        parts.push(text);
    }
    Some(format!("{}{}", rule.prefix.unwrap_or_default(), parts.join("|")))
}

fn text(value: &Value) -> Option<String> {
    match value {
        Value::Null | Value::List(_) | Value::Map(_) => None,
        Value::String(s) => Some(s.clone()),
        Value::Int(i) => Some(i.to_string()),
        Value::Double(d) => Some(adp_fbl::planning::dotnet_double(*d)),
        Value::Bool(b) => Some(if *b { "True" } else { "False" }.to_owned()),
        other => Some(other.to_string()),
    }
}
